//! Window › Codex: a chat with OpenAI Codex that edits the open project through the app's own MCP
//! tools.
//!
//! This module only draws the chat and talks over channels, so it stays free of process I/O (and
//! keeps building for the web). The desktop app supplies [`HostHooks::codex`](crate::HostHooks):
//! it runs `codex app-server`, signed in with ChatGPT, with this window's control channel as an
//! MCP server, and turns the server's messages into [`CodexEvent`]s.
//!
//! Entry points: [`toggle`] / [`is_open`] for the menu, [`show_in`] to draw the chat in any `Ui`
//! (the classic layout gives it a column on the right, [`show_classic`]; a docked shell can call
//! it in its own panel), and [`poll`] every frame so generated images land in the project even
//! while the chat is closed.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

use egui::{Color32, Frame, Margin, Rect, RichText, Sense, Stroke, Ui, vec2};
use serde_json::json;

use crate::FilmcraftApp;
use crate::theme::Tokens;

/// The product name (`brand.env`), which Codex is told and the chat shows.
pub const APP_NAME: &str = env!("FILMCRAFT_BRAND_NAME");

/// What the chat asks the backend to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodexRequest {
    /// Start ChatGPT sign-in in the system browser.
    SignIn,
    /// Sign out of ChatGPT.
    SignOut,
    /// Send a message with files as references (starts a conversation when there is none).
    Send { text: String, files: Vec<String> },
    /// Interrupt the running turn.
    Stop,
    /// Forget the conversation; the next message starts a new one.
    NewChat,
    /// Use this model from the next message on, and remember it.
    SetModel(String),
    /// List the conversations saved on this computer for this account.
    ListThreads,
    /// Continue a saved conversation.
    Resume(String),
}

/// A conversation saved on this computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexThread {
    pub id: String,
    /// Its name, else the start of its first message.
    pub title: String,
    /// Unix seconds.
    pub updated: i64,
}

/// What a saved conversation said, oldest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodexHistory {
    User(String),
    Assistant(String),
    Step(String),
}

/// What [`CodexStartFn`] gets to add an account, in place of an existing account's id.
pub const NEW_ACCOUNT: &str = "new";

/// A ChatGPT account signed in on this computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexAccount {
    pub id: String,
    pub label: String,
}

/// A model Codex offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexModel {
    pub id: String,
    pub name: String,
    /// The catalog's one line (how capable, how affordable), as Codex words it.
    pub description: String,
}

/// What the backend reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodexEvent {
    /// Who is signed in (`None`: nobody).
    Account(Option<String>),
    /// The accounts signed in on this computer and the one in use, by id.
    Accounts { list: Vec<CodexAccount>, current: String },
    /// This account has run out of its plan's usage.
    LimitReached,
    /// The conversations saved for this account, newest first.
    Threads(Vec<CodexThread>),
    /// A saved conversation was reopened: what it said so far.
    Resumed(Vec<CodexHistory>),
    /// The models on offer and the one in use.
    Models { models: Vec<CodexModel>, current: Option<String> },
    /// Streamed text of the assistant message `item`.
    AssistantDelta { item: String, delta: String },
    /// The finished text of the assistant message `item`.
    AssistantDone { item: String, text: String },
    /// A tool call or other step.
    Activity(String),
    /// An image Codex generated, saved at this path.
    Image(PathBuf),
    TurnStarted,
    TurnFinished,
    Error(String),
    /// The Codex process ended; the link is dead.
    Exited(String),
}

/// The two ends of a running backend.
pub struct CodexLink {
    pub tx: Sender<CodexRequest>,
    pub rx: Receiver<CodexEvent>,
}

/// Starts the backend, on the account with this id, on [`NEW_ACCOUNT`], or (`None`) on the last one
/// used. Called when the chat first opens, after Codex stopped and to change accounts.
pub type CodexStartFn = Box<dyn FnMut(Option<&str>) -> Result<CodexLink, String>>;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Entry {
    User(String),
    Assistant { item: String, text: String },
    /// Consecutive tool calls and other steps (the backend's raw lines), shown as one checklist.
    Steps(Vec<String>),
    /// A turn ended after this many steps.
    Done(usize),
    Error(String),
}

/// The chat's state (not persisted).
#[derive(Default)]
pub struct CodexPanel {
    pub open: bool,
    link: Option<CodexLink>,
    entries: Vec<Entry>,
    input: String,
    busy: bool,
    /// `None` until the backend has said; `Some(None)` when nobody is signed in.
    account: Option<Option<String>>,
    /// Where the running (or last) turn's entries start.
    turn_start: usize,
    /// Sign-in was asked for and the browser is open; cleared by the next account report or error.
    signing_in: bool,
    /// This app's MCP server got at least one tool call in this run.
    mcp: bool,
    models: Vec<CodexModel>,
    /// The model in use, by id.
    model: Option<String>,
    accounts: Vec<CodexAccount>,
    /// The account in use, by id.
    account_id: Option<String>,
    /// The last turn stopped on this account's usage limit.
    limit: bool,
    /// Paths of the files going with the next message.
    attachments: Vec<String>,
    /// Where the chat was last drawn: files dropped there are attachments.
    rect: Option<Rect>,
    /// The saved conversations, as last listed.
    threads: Vec<CodexThread>,
}

impl CodexPanel {
    pub fn is_running(&self) -> bool {
        self.link.is_some()
    }

    pub fn is_busy(&self) -> bool {
        self.busy
    }

    /// Apply one backend event; returns an image to import.
    fn apply(&mut self, event: CodexEvent) -> Option<PathBuf> {
        match event {
            CodexEvent::Account(account) => {
                self.account = Some(account);
                self.signing_in = false;
            }
            CodexEvent::Accounts { list, current } => {
                self.accounts = list;
                self.account_id = Some(current);
            }
            CodexEvent::LimitReached => self.limit = true,
            CodexEvent::Threads(threads) => self.threads = threads,
            CodexEvent::Resumed(history) => {
                self.entries.clear();
                for item in history {
                    match item {
                        CodexHistory::User(text) => self.entries.push(Entry::User(shown_prompt(&text))),
                        CodexHistory::Assistant(text) => {
                            let item = format!("history-{}", self.entries.len());
                            self.entries.push(Entry::Assistant { item, text });
                        }
                        CodexHistory::Step(line) => self.step(line),
                    }
                }
                self.busy = false;
                self.turn_start = self.entries.len();
            }
            CodexEvent::Models { models, current } => {
                self.models = models;
                self.model = current;
            }
            CodexEvent::AssistantDelta { item, delta } => match self.entries.last_mut() {
                Some(Entry::Assistant { item: last, text }) if *last == item => text.push_str(&delta),
                _ => self.entries.push(Entry::Assistant { item, text: delta }),
            },
            CodexEvent::AssistantDone { item, text } => {
                if let Some(Entry::Assistant { text: shown, .. }) =
                    self.entries.iter_mut().rev().find(|e| matches!(e, Entry::Assistant { item: i, .. } if *i == item))
                {
                    *shown = text;
                } else if !text.is_empty() {
                    self.entries.push(Entry::Assistant { item, text });
                }
            }
            CodexEvent::Activity(line) => {
                self.mcp |= line.starts_with(MCP_STEP);
                self.step(line);
            }
            CodexEvent::Image(path) => return Some(path),
            CodexEvent::TurnStarted => {
                if !self.busy {
                    self.turn_start = self.entries.len();
                }
                self.busy = true;
            }
            CodexEvent::TurnFinished => self.finish_turn(),
            CodexEvent::Error(e) => {
                // The backend's fixed messages have catalogue entries.
                self.entries.push(Entry::Error(crate::i18n::t(&e).to_string()));
                self.signing_in = false;
            }
            CodexEvent::Exited(why) => {
                self.entries.push(Entry::Error(crate::i18n::t(&why).to_string()));
                self.reset();
            }
        }
        None
    }

    /// The link is gone (or being replaced): forget what it reported.
    fn reset(&mut self) {
        self.link = None;
        self.busy = false;
        self.account = None;
        self.signing_in = false;
        self.mcp = false;
        self.models.clear();
        self.model = None;
        self.accounts.clear();
        self.account_id = None;
        self.limit = false;
    }

    /// The signed-in accounts other than the one in use.
    fn other_accounts(&self) -> impl Iterator<Item = &CodexAccount> {
        self.accounts.iter().filter(|a| Some(&a.id) != self.account_id.as_ref())
    }

    /// The footer's status and its text: honest about what is up.
    fn status(&self) -> (Status, &'static str) {
        match (&self.account, self.link.is_some()) {
            (_, false) => (Status::Off, tl!("Disconnected")),
            (None, true) => (Status::Waiting, tl!("Connecting…")),
            (Some(None), true) if self.signing_in => (Status::Waiting, tl!("Opening the browser…")),
            (Some(None), true) => (Status::Waiting, tl!("Not signed in")),
            (Some(Some(_)), true) if self.mcp => (Status::Live, tl!("MCP connected")),
            (Some(Some(_)), true) => (Status::Live, tl!("Connected")),
        }
    }

    /// Add a step to the current checklist, or start one.
    fn step(&mut self, line: String) {
        match self.entries.last_mut() {
            Some(Entry::Steps(lines)) => lines.push(line),
            _ => self.entries.push(Entry::Steps(vec![line])),
        }
    }

    /// The turn ended: sum it up when it ran steps without errors.
    fn finish_turn(&mut self) {
        if self.busy {
            let turn = self.entries.get(self.turn_start..).unwrap_or(&[]);
            let steps: usize = turn.iter().map(|e| if let Entry::Steps(lines) = e { lines.len() } else { 0 }).sum();
            if steps > 0 && !turn.iter().any(|e| matches!(e, Entry::Error(_))) {
                self.entries.push(Entry::Done(steps));
            }
        }
        self.busy = false;
    }

    fn request(&mut self, request: CodexRequest) {
        let Some(link) = &self.link else { return };
        if link.tx.send(request).is_err() {
            self.apply(CodexEvent::Exited("Codex stopped.".into()));
        }
    }
}

/// How the backend labels a call to this app's MCP server (`server › tool`).
const MCP_STEP: &str = "fusemovie › ";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Off,
    Waiting,
    Live,
}

/// A step's raw line as a checklist label: `server › tool_name` reads "Tool name".
fn step_label(line: &str) -> String {
    let tool = line.rsplit(" › ").next().unwrap_or(line).trim();
    if tool == line.trim() {
        return crate::i18n::t(tool).to_string();
    }
    let words = tool.replace(['_', '.', '-'], " ");
    let mut chars = words.chars();
    chars.next().map_or_else(String::new, |c| c.to_uppercase().chain(chars).collect())
}

/// Images Codex can look at itself; any other file is a path it may read.
pub fn is_reference_image(path: &str) -> bool {
    std::path::Path::new(path).extension().and_then(|e| e.to_str()).is_some_and(|e| ["png", "jpg", "jpeg", "webp", "gif"].iter().any(|i| e.eq_ignore_ascii_case(i)))
}

/// A message as the user wrote it, a long one cut short.
fn shown_prompt(text: &str) -> String {
    const MAX: usize = 300;
    let text = text.trim();
    match text.char_indices().nth(MAX) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_string(),
    }
}

/// A file's name, for showing.
fn file_name(path: &str) -> String {
    std::path::Path::new(path).file_name().map_or_else(|| path.to_string(), |n| n.to_string_lossy().into_owned())
}

/// Drain the backend's events (every frame, so generated images land even while the chat is
/// closed).
pub fn poll(app: &mut FilmcraftApp) {
    let mut images = Vec::new();
    while let Some(event) = app.codex.link.as_ref().and_then(|link| link.rx.try_recv().ok()) {
        images.extend(app.codex.apply(event));
    }
    for path in images {
        import_image(app, &path);
    }
}

/// Import a generated image into the project as media (the chat says where it went).
fn import_image(app: &mut FilmcraftApp, path: &std::path::Path) {
    let name = file_name(&path.to_string_lossy());
    let bin = app.import_bin().0;
    match app.session.execute("file.import", json!({"paths": [path.to_string_lossy()], "bin": bin})) {
        Ok(_) => app.codex.step(tlf!("Image {name} imported into the project", name)),
        Err(error) => app.codex.entries.push(Entry::Error(tlf!("Could not import image {name}: {error}", name, error))),
    }
}

/// Files as attachments of the next message, each once.
fn attach(app: &mut FilmcraftApp, paths: Vec<String>) {
    for path in paths {
        if !app.codex.attachments.contains(&path) {
            app.codex.attachments.push(path);
        }
    }
}

/// Files dropped on the open chat become attachments (`true`); anywhere else they import as usual.
pub(crate) fn take_drop(app: &mut FilmcraftApp, files: &[egui::DroppedFileHandle], at: Option<egui::Pos2>) -> bool {
    if !app.codex.open || !app.codex.rect.zip(at).is_some_and(|(r, at)| r.contains(at)) {
        return false;
    }
    attach(app, files.iter().filter(|f| f.path().is_absolute()).map(|f| f.path().to_string_lossy().into_owned()).collect());
    true
}

/// Whether the chat is open.
pub fn is_open(app: &FilmcraftApp) -> bool {
    app.codex.open
}

/// Window › Codex: open or close the chat; the first open starts Codex.
pub fn toggle(app: &mut FilmcraftApp) {
    app.codex.open = !app.codex.open;
    if app.codex.open && app.codex.link.is_none() && app.codex.entries.is_empty() {
        start_on(app, None);
    }
}

/// Start the backend on an account, if it isn't running.
fn start_on(app: &mut FilmcraftApp, account: Option<&str>) {
    if app.codex.link.is_some() {
        return;
    }
    let Some(start) = app.hooks.codex.as_mut() else {
        app.codex.entries.push(Entry::Error(tlf!("Codex is not available in this version of {app}.", app = APP_NAME)));
        return;
    };
    match start(account) {
        Ok(link) => app.codex.link = Some(link),
        // The backend's fixed messages (Codex not installed, …) have catalogue entries.
        Err(e) => app.codex.entries.push(Entry::Error(crate::i18n::t(&e).to_string())),
    }
}

/// What the user did this frame, applied after drawing.
enum Act {
    Request(CodexRequest),
    /// Send the composer's text.
    Send(String),
    Start,
    Restart,
    Close,
    /// Ask for files to send with the next message.
    Attach,
    Detach(String),
    /// Move to this account (or [`NEW_ACCOUNT`]), which starts a new conversation.
    SwitchAccount(String),
}

fn run(app: &mut FilmcraftApp, acts: Vec<Act>) {
    for act in acts {
        match act {
            Act::Send(text) => {
                let p = &mut app.codex;
                let text = text.trim().to_string();
                if (text.is_empty() && p.attachments.is_empty()) || p.link.is_none() || p.busy {
                    continue;
                }
                let files = std::mem::take(&mut p.attachments);
                let prompt = if text.is_empty() { "Use the attached files as references.".to_string() } else { text.clone() };
                let shown = std::iter::once(text).filter(|t| !t.is_empty()).chain(files.iter().map(|f| file_name(f))).collect::<Vec<_>>().join("\n");
                p.turn_start = p.entries.len();
                p.entries.push(Entry::User(shown));
                p.busy = true;
                p.limit = false;
                p.request(CodexRequest::Send { text: prompt, files });
            }
            Act::Attach => {
                const EXTS: [&str; 16] = ["png", "jpg", "jpeg", "webp", "gif", "mp4", "mov", "wav", "mp3", "srt", "vtt", "txt", "md", "pdf", "json", "csv"];
                app.pick_ui(crate::native_dialogs::FileDialog::OpenFiles { exts: EXTS.iter().map(|e| e.to_string()).collect() }, attach);
            }
            Act::Detach(path) => app.codex.attachments.retain(|p| *p != path),
            Act::SwitchAccount(id) => {
                let name = app.codex.accounts.iter().find(|a| a.id == id).map(|a| a.label.clone());
                // Dropping the link stops the Codex of the old account.
                app.codex.reset();
                start_on(app, Some(&id));
                if let (Some(name), true) = (name, app.codex.link.is_some()) {
                    app.codex.step(tlf!("Now using {name}. This is a new conversation: Codex doesn't remember the earlier messages.", name));
                }
            }
            Act::Request(request) => {
                let p = &mut app.codex;
                match &request {
                    CodexRequest::SignIn => p.signing_in = true,
                    CodexRequest::SetModel(id) => p.model = Some(id.clone()),
                    CodexRequest::Resume(_) => {
                        p.entries.clear();
                        p.busy = false;
                        p.limit = false;
                    }
                    CodexRequest::NewChat => {
                        p.entries.clear();
                        p.busy = false;
                        p.turn_start = 0;
                    }
                    _ => {}
                }
                p.request(request);
            }
            Act::Start => start_on(app, None),
            Act::Restart => {
                // Dropping the link stops the old Codex process.
                app.codex.reset();
                start_on(app, None);
            }
            Act::Close => app.codex.open = false,
        }
    }
}

/// The classic layout: the chat in a column on the right of `body`, when it is open. Returns
/// what is left of `body` for the dock.
pub(crate) fn show_classic(app: &mut FilmcraftApp, ui: &mut Ui, body: Rect) -> Rect {
    if !app.codex.open {
        app.codex.rect = None;
        return body;
    }
    // Never so wide on a small window that the editor goes.
    let w = 360.0_f32.min(body.width() * 0.4).max(240.0_f32.min(body.width()));
    let column = Rect::from_min_max(egui::pos2(body.max.x - w, body.min.y), body.max);
    let t = app.tokens;
    ui.painter().rect_filled(column, 0.0, t.panel_bg);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(column.shrink(8.0)).id_salt("codex-column").layout(egui::Layout::top_down(egui::Align::Min)));
    child.set_clip_rect(column);
    show_in(app, &mut child);
    Rect::from_min_max(body.min, egui::pos2((column.min.x - 1.0).max(body.min.x), body.max.y))
}

/// Draw the whole chat (header, conversation, composer, status) in `ui`.
pub fn show_in(app: &mut FilmcraftApp, ui: &mut Ui) {
    let t = app.tokens;
    let mut acts = Vec::new();
    let p = &mut app.codex;
    let full = ui.max_rect();
    p.rect = Some(full);
    // The content is drawn in children: without this an enclosing panel would think it is empty.
    ui.expand_to_include_rect(full);
    header(ui, &t, p, &mut acts);
    ui.separator();
    // What the composer and the footer need, measured from the bottom.
    let below = 96.0 + if p.attachments.is_empty() { 0.0 } else { 30.0 };
    let list_h = (ui.available_height() - below).max(60.0);
    egui::ScrollArea::vertical().id_salt("codex-chat").max_height(list_h).min_scrolled_height(list_h).stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 8.0;
        conversation(ui, &t, p, &mut acts);
    });
    ui.separator();
    composer(ui, &t, p, &mut acts);
    footer(ui, &t, p, &mut acts);
    run(app, acts);
}

/// "Codex" … New Chat, Conversations, ⋯, ✕.
fn header(ui: &mut Ui, t: &Tokens, p: &CodexPanel, acts: &mut Vec<Act>) {
    let running = p.link.is_some();
    ui.horizontal(|ui| {
        ui.label(RichText::new(tl!("Codex")).font(Tokens::ui(15.0)).strong().color(t.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("✕").on_hover_text(tl!("Close")).clicked() {
                acts.push(Act::Close);
            }
            let more = ui.small_button("⋯").on_hover_text(tl!("More"));
            egui::Popup::menu(&more).show(|ui| {
                ui.set_min_width(200.0);
                if let Some(Some(who)) = &p.account {
                    ui.label(RichText::new(who).small().color(t.text_dim));
                    ui.separator();
                }
                if running {
                    account_buttons(ui, p, acts);
                    ui.separator();
                    if let Some(signed_in) = p.account.as_ref().map(Option::is_some) {
                        let (label, request) = if signed_in { (tl!("Sign Out"), CodexRequest::SignOut) } else { (tl!("Sign in with ChatGPT"), CodexRequest::SignIn) };
                        if ui.button(label).clicked() {
                            acts.push(Act::Request(request));
                        }
                    }
                }
                if ui.button(if running { tl!("Restart Codex") } else { tl!("Start Codex") }).clicked() {
                    acts.push(if running { Act::Restart } else { Act::Start });
                }
            });
            let history = ui.add_enabled(running, egui::Button::new(tl!("Conversations")).small());
            if history.clicked() {
                acts.push(Act::Request(CodexRequest::ListThreads));
            }
            egui::Popup::menu(&history).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| thread_list(ui, t, p, acts));
            if ui.add_enabled(running, egui::Button::new(tl!("New Chat")).small()).clicked() {
                acts.push(Act::Request(CodexRequest::NewChat));
            }
        });
    });
}

/// The saved conversations, newest first; a click continues one.
fn thread_list(ui: &mut Ui, t: &Tokens, p: &CodexPanel, acts: &mut Vec<Act>) {
    ui.set_width(280.0);
    if p.threads.is_empty() {
        ui.label(RichText::new(tl!("No saved conversations yet.")).small().color(t.text_dim));
        return;
    }
    egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
        for thread in &p.threads {
            let title = shown_prompt(&thread.title);
            if ui.add(egui::Button::new(RichText::new(&title).color(t.text)).frame(false).truncate()).clicked() {
                acts.push(Act::Request(CodexRequest::Resume(thread.id.clone())));
                ui.close();
            }
        }
    });
}

fn card(t: &Tokens) -> Frame {
    Frame::new().fill(t.field_bg).stroke(Stroke::new(1.0, t.field_border)).corner_radius(8.0).inner_margin(Margin::symmetric(10, 8))
}

fn conversation(ui: &mut Ui, t: &Tokens, p: &CodexPanel, acts: &mut Vec<Act>) {
    if p.link.is_some() && p.account == Some(None) {
        sign_in_card(ui, t, p.signing_in, acts);
    } else if p.entries.is_empty() {
        ui.add(egui::Label::new(RichText::new(tl!("Ask Codex to edit the open sequence: cut, trim, add titles, captions, effects or transitions, or generate an image.")).color(t.text_faint)).wrap());
    }
    let last = p.entries.len().saturating_sub(1);
    for (i, entry) in p.entries.iter().enumerate() {
        match entry {
            Entry::User(text) => {
                card(t).fill(t.row_selected).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.add(egui::Label::new(RichText::new(text).color(t.text)).wrap());
                });
            }
            Entry::Assistant { text, .. } => {
                ui.add(egui::Label::new(RichText::new(text).color(t.text)).wrap());
            }
            Entry::Steps(lines) => {
                let running = p.busy && i == last;
                card(t).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    // A Frame inherits the parent's layout: keep the checklist top-down.
                    ui.vertical(|ui| steps(ui, t, i, lines, running));
                });
            }
            Entry::Done(n) => {
                let text = if *n == 1 { tl!("Finished in 1 step.").to_string() } else { tlf!("Finished in {n} steps.", n) };
                ui.add(egui::Label::new(RichText::new(format!("✓ {text}")).color(t.text_dim)).wrap());
            }
            Entry::Error(e) => {
                ui.add(egui::Label::new(RichText::new(e).color(t.danger)).wrap());
            }
        }
    }
    if p.limit {
        card(t).stroke(Stroke::new(1.0, t.accent)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical(|ui| {
                ui.add(egui::Label::new(RichText::new(tl!("This account reached its usage limit. Switch to another account to keep going.")).color(t.text)).wrap());
                ui.add_space(6.0);
                account_buttons(ui, p, acts);
            });
        });
    }
}

/// The other accounts, one click away, and the way to add one.
fn account_buttons(ui: &mut Ui, p: &CodexPanel, acts: &mut Vec<Act>) {
    for account in p.other_accounts() {
        if ui.button(tlf!("Switch to {name}", name = &account.label)).clicked() {
            acts.push(Act::SwitchAccount(account.id.clone()));
        }
    }
    if ui.button(tl!("Add account")).clicked() {
        acts.push(Act::SwitchAccount(NEW_ACCOUNT.to_string()));
    }
}

/// Signed out: what Codex needs and the button that opens ChatGPT's sign-in in the browser.
fn sign_in_card(ui: &mut Ui, t: &Tokens, signing_in: bool, acts: &mut Vec<Act>) {
    card(t).stroke(Stroke::new(1.0, t.accent)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.vertical(|ui| {
            ui.add(egui::Label::new(RichText::new(tl!("Sign in with your ChatGPT account to use Codex.")).color(t.text)).wrap());
            ui.add_space(6.0);
            if signing_in {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new().size(14.0).color(t.accent));
                    ui.label(RichText::new(tl!("Opening the browser…")).color(t.text_dim));
                });
            } else if ui.add(egui::Button::new(RichText::new(tl!("Sign in with ChatGPT")).color(Color32::WHITE)).fill(t.accent)).clicked() {
                acts.push(Act::Request(CodexRequest::SignIn));
            }
        });
    });
}

/// A turn's steps: ✓ when done, a spinner on the running one, and the raw calls folded below.
fn steps(ui: &mut Ui, t: &Tokens, id: usize, lines: &[String], running: bool) {
    for (k, line) in lines.iter().enumerate() {
        ui.horizontal_top(|ui| {
            if running && k + 1 == lines.len() {
                ui.add(egui::Spinner::new().size(14.0).color(t.accent));
            } else {
                ui.label(RichText::new("✓").color(t.accent));
            }
            ui.add(egui::Label::new(RichText::new(step_label(line)).color(t.text)).wrap());
        });
    }
    egui::CollapsingHeader::new(RichText::new(tl!("Run details")).small().color(t.text_dim)).id_salt(("codex-details", id)).show(ui, |ui| {
        for line in lines {
            ui.add(egui::Label::new(RichText::new(line).monospace().small().color(t.text_dim)).wrap());
        }
    });
}

/// Attachments, the input, Attach and Send (Stop while busy).
fn composer(ui: &mut Ui, t: &Tokens, p: &mut CodexPanel, acts: &mut Vec<Act>) {
    let running = p.link.is_some();
    if !p.attachments.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for path in &p.attachments {
                // The whole chip removes it: there is nothing else to do with it here.
                if ui.add(egui::Button::new(RichText::new(format!("✕ {}", file_name(path))).small()).truncate()).on_hover_text(tl!("Remove")).clicked() {
                    acts.push(Act::Detach(path.clone()));
                }
            }
        });
    }
    let edit = ui.add_enabled(
        running,
        egui::TextEdit::multiline(&mut p.input).desired_rows(2).desired_width(f32::INFINITY).hint_text(tl!("What do you want to edit or create?")),
    );
    // Enter sends; Shift+Enter is a new line.
    let enter = edit.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift);
    ui.horizontal(|ui| {
        if ui.add_enabled(running, egui::Button::new(tl!("Attach files"))).clicked() {
            acts.push(Act::Attach);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if p.busy {
                if ui.button(tl!("Stop")).clicked() {
                    acts.push(Act::Request(CodexRequest::Stop));
                }
                ui.add(egui::Spinner::new().size(14.0).color(t.accent));
            } else {
                let send = ui.add_enabled(running, egui::Button::new(RichText::new(tl!("Send")).color(Color32::WHITE)).fill(t.accent));
                if send.clicked() || (enter && running) {
                    acts.push(Act::Send(std::mem::take(&mut p.input)));
                }
            }
        });
    });
}

/// "● Codex CLI · Connected", and the model menu or the button that fixes what is missing.
fn footer(ui: &mut Ui, t: &Tokens, p: &CodexPanel, acts: &mut Vec<Act>) {
    let running = p.link.is_some();
    let (status, text) = p.status();
    let dot = match status {
        Status::Off => t.text_faint,
        Status::Waiting => Color32::from_rgb(0xe0, 0xa0, 0x30),
        Status::Live => Color32::from_rgb(0x3f, 0xb9, 0x50),
    };
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(10.0, 16.0), Sense::hover());
        ui.painter().circle_filled(r.center(), 4.0, dot);
        ui.add(egui::Label::new(RichText::new(format!("{} · {text}", tl!("Codex CLI"))).small().color(t.text_dim)).truncate());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if !running {
                if ui.small_button(tl!("Start Codex")).clicked() {
                    acts.push(Act::Start);
                }
            } else if p.account == Some(None) && !p.signing_in {
                if ui.small_button(tl!("Sign in with ChatGPT")).clicked() {
                    acts.push(Act::Request(CodexRequest::SignIn));
                }
            } else if matches!(p.account, Some(Some(_))) && !p.models.is_empty() {
                model_menu(ui, t, p, acts);
            }
        });
    });
}

/// Which model Codex uses: the cheaper ones cost far less of the user's plan.
fn model_menu(ui: &mut Ui, t: &Tokens, p: &CodexPanel, acts: &mut Vec<Act>) {
    let current = p.model.as_deref().unwrap_or_default();
    let name = p.models.iter().find(|m| m.id == current).map_or(current, |m| m.name.as_str());
    let resp = ui.small_button(format!("{name} ▾")).on_hover_text(tl!("Model"));
    egui::Popup::menu(&resp).show(|ui| {
        ui.set_max_width(300.0);
        for m in &p.models {
            if ui.selectable_label(m.id == current, &m.name).clicked() && m.id != current {
                acts.push(Act::Request(CodexRequest::SetModel(m.id.clone())));
            }
            ui.add(egui::Label::new(RichText::new(&m.description).small().color(t.text_dim)).wrap());
            ui.add_space(4.0);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;

    fn link() -> (CodexLink, Receiver<CodexRequest>, Sender<CodexEvent>) {
        let (tx, requests) = std::sync::mpsc::channel();
        let (events, rx) = std::sync::mpsc::channel();
        (CodexLink { tx, rx }, requests, events)
    }

    fn app() -> FilmcraftApp {
        FilmcraftApp::new(filmcraft_engine::Session::default())
    }

    #[test]
    fn codex_deltas_stream_into_one_message_and_done_replaces_it() {
        let mut p = CodexPanel::default();
        p.apply(CodexEvent::AssistantDelta { item: "a".into(), delta: "Hel".into() });
        p.apply(CodexEvent::AssistantDelta { item: "a".into(), delta: "lo".into() });
        assert_eq!(p.entries, vec![Entry::Assistant { item: "a".into(), text: "Hello".into() }]);
        p.apply(CodexEvent::Activity("tool".into()));
        p.apply(CodexEvent::AssistantDone { item: "a".into(), text: "Hello!".into() });
        p.apply(CodexEvent::AssistantDelta { item: "b".into(), delta: "Next".into() });
        assert_eq!(p.entries.first(), Some(&Entry::Assistant { item: "a".into(), text: "Hello!".into() }));
        assert_eq!(p.entries.len(), 3);
    }

    #[test]
    fn codex_consecutive_steps_group_and_a_clean_turn_is_summed_up() {
        let mut p = CodexPanel::default();
        p.apply(CodexEvent::TurnStarted);
        p.apply(CodexEvent::Activity("fusemovie › sequence_inspect".into()));
        p.apply(CodexEvent::Activity("fusemovie › command_run".into()));
        p.apply(CodexEvent::AssistantDelta { item: "a".into(), delta: "ok".into() });
        p.apply(CodexEvent::Activity("Generating image…".into()));
        p.apply(CodexEvent::TurnFinished);
        assert_eq!(
            p.entries,
            vec![
                Entry::Steps(vec!["fusemovie › sequence_inspect".into(), "fusemovie › command_run".into()]),
                Entry::Assistant { item: "a".into(), text: "ok".into() },
                Entry::Steps(vec!["Generating image…".into()]),
                Entry::Done(3),
            ]
        );
        p.apply(CodexEvent::TurnFinished);
        assert_eq!(p.entries.len(), 4, "a stray second TurnFinished adds nothing");
        // A failed turn has no summary.
        p.apply(CodexEvent::TurnStarted);
        p.apply(CodexEvent::Activity("fusemovie › x".into()));
        p.apply(CodexEvent::Error("boom".into()));
        p.apply(CodexEvent::TurnFinished);
        assert_eq!(p.entries.iter().filter(|e| matches!(e, Entry::Done(_))).count(), 1);
    }

    #[test]
    fn codex_status_follows_the_link_sign_in_and_first_mcp_call() {
        let (link, _requests, _events) = link();
        let mut p = CodexPanel::default();
        assert_eq!(p.status(), (Status::Off, "Disconnected"));
        p.link = Some(link);
        assert_eq!(p.status(), (Status::Waiting, "Connecting…"));
        p.apply(CodexEvent::Account(None));
        assert_eq!(p.status().1, "Not signed in");
        p.signing_in = true;
        assert_eq!(p.status().1, "Opening the browser…");
        p.apply(CodexEvent::Error("sign-in failed".into()));
        assert!(!p.signing_in, "an error ends the wait");
        p.apply(CodexEvent::Account(Some("me@example.com".into())));
        assert_eq!(p.status(), (Status::Live, "Connected"));
        p.apply(CodexEvent::Activity("Generating image…".into()));
        assert_eq!(p.status().1, "Connected", "only a call to the app's own server proves MCP");
        p.apply(CodexEvent::Activity("fusemovie › doc_inspect".into()));
        assert_eq!(p.status(), (Status::Live, "MCP connected"));
        assert_eq!(p.apply(CodexEvent::Image("x.png".into())), Some(PathBuf::from("x.png")));
        p.apply(CodexEvent::Exited("gone".into()));
        assert_eq!(p.status(), (Status::Off, "Disconnected"));
        assert!(!p.mcp && !p.busy);
    }

    #[test]
    fn codex_steps_read_as_labels() {
        assert_eq!(step_label("fusemovie › command_run"), "Command run");
        assert_eq!(step_label("fusemovie › doc.inspect"), "Doc inspect");
        assert_eq!(step_label("Generating image…"), "Generating image…");
        assert_eq!(step_label("$ ls -la"), "$ ls -la");
        assert_eq!(step_label("fusemovie › "), "");
        assert_eq!(shown_prompt(&"x".repeat(400)).chars().count(), 301);
    }

    #[test]
    fn codex_attachments_go_with_the_next_message() {
        let (link, requests, _events) = link();
        let mut app = app();
        app.codex.link = Some(link);
        attach(&mut app, vec!["C:\\refs\\look.png".into(), "C:\\refs\\look.png".into(), "C:\\refs\\script.srt".into()]);
        assert_eq!(app.codex.attachments.len(), 2, "each once");
        run(&mut app, vec![Act::Detach("C:\\refs\\script.srt".into())]);
        run(&mut app, vec![Act::Send("make it like this".into())]);
        assert_eq!(requests.try_recv().ok(), Some(CodexRequest::Send { text: "make it like this".into(), files: vec!["C:\\refs\\look.png".into()] }));
        assert!(app.codex.attachments.is_empty(), "only for that message");
        let Some(Entry::User(shown)) = app.codex.entries.last() else { panic!("no message") };
        assert!(shown.starts_with("make it like this") && shown.contains("look.png"), "{shown}");
        // Files alone are enough to send, and nothing at all isn't.
        app.codex.busy = false;
        run(&mut app, vec![Act::Send("  ".into())]);
        assert!(requests.try_recv().is_err());
        attach(&mut app, vec!["/refs/a.png".into()]);
        run(&mut app, vec![Act::Send(String::new())]);
        assert!(matches!(requests.try_recv(), Ok(CodexRequest::Send { files, .. }) if files.len() == 1));
    }

    #[test]
    fn codex_without_a_backend_says_so() {
        let mut app = app();
        toggle(&mut app);
        assert!(is_open(&app));
        assert!(matches!(app.codex.entries.last(), Some(Entry::Error(e)) if e.contains("not available")));
        toggle(&mut app);
        assert!(!is_open(&app));
    }

    #[test]
    fn codex_generated_images_are_imported_into_the_project() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("title.png");
        std::fs::write(&path, crate::control::encode_png(&[255, 0, 0, 255].repeat(4), 2, 2).unwrap()).unwrap();
        let (link, _requests, events) = link();
        let mut app = app();
        app.codex.link = Some(link);
        let before = app.session.execute("project.inspect", json!({})).unwrap().to_string();
        events.send(CodexEvent::Image(path.clone())).unwrap();
        poll(&mut app);
        let after = app.session.execute("project.inspect", json!({})).unwrap().to_string();
        assert!(!before.contains("title") && after.contains("title"), "{after}");
        assert!(matches!(app.codex.entries.last(), Some(Entry::Steps(lines)) if lines.iter().any(|l| l.contains("title.png"))), "{:?}", app.codex.entries);
    }

    #[test]
    fn codex_resumed_conversation_comes_back_as_the_chat_showed_it() {
        let mut p = CodexPanel::default();
        p.entries.push(Entry::User("old".into()));
        p.apply(CodexEvent::Resumed(vec![
            CodexHistory::User("cut the intro".into()),
            CodexHistory::Step("fusemovie › command_run".into()),
            CodexHistory::Step("fusemovie › render_preview".into()),
            CodexHistory::Assistant("Done.".into()),
        ]));
        assert_eq!(
            p.entries,
            vec![
                Entry::User("cut the intro".into()),
                Entry::Steps(vec!["fusemovie › command_run".into(), "fusemovie › render_preview".into()]),
                Entry::Assistant { item: "history-2".into(), text: "Done.".into() },
            ]
        );
    }

    fn harness(app: FilmcraftApp) -> egui_kittest::Harness<'static, FilmcraftApp> {
        egui_kittest::Harness::builder().with_size(vec2(380.0, 640.0)).build_ui_state(|ui, app: &mut FilmcraftApp| show_in(app, ui), app)
    }

    /// Signed out the chat shows the sign-in button, which sends the request and waits ("Opening
    /// the browser…") until the account is reported.
    #[test]
    fn codex_panel_signs_in_through_the_link() {
        let (link, requests, events) = link();
        let mut app = app();
        app.codex.link = Some(link);
        app.codex.open = true;
        let mut h = harness(app);
        h.run_steps(2);
        assert!(h.query_by_label_contains("Connecting…").is_some(), "no account report yet");
        events.send(CodexEvent::Account(None)).unwrap();
        poll(h.state_mut());
        h.run_steps(2);
        h.get_all_by_label("Sign in with ChatGPT").next().expect("the sign-in button").click();
        h.run_steps(2);
        assert_eq!(requests.try_recv().ok(), Some(CodexRequest::SignIn));
        assert!(h.query_all_by_label_contains("Opening the browser…").next().is_some());
        assert!(h.query_by_label("Sign in with ChatGPT").is_none(), "one request at a time");
        events.send(CodexEvent::Account(Some("me@example.com".into()))).unwrap();
        poll(h.state_mut());
        h.run_steps(2);
        assert!(h.query_by_label_contains("Connected").is_some());
    }

    /// At the limit the chat offers the other accounts; picking one restarts Codex on it.
    #[test]
    fn codex_limit_card_switches_accounts() {
        let (link, _requests, events) = link();
        let asked = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Option<String>>::new()));
        let seen = asked.clone();
        let mut app = app();
        app.codex.link = Some(link);
        app.codex.open = true;
        app.hooks.codex = Some(Box::new(move |account| {
            seen.lock().unwrap().push(account.map(str::to_owned));
            let (tx, _) = std::sync::mpsc::channel();
            let (_, rx) = std::sync::mpsc::channel();
            Ok(CodexLink { tx, rx })
        }));
        let mut h = harness(app);
        let account = |id: &str, label: &str| CodexAccount { id: id.into(), label: label.into() };
        events.send(CodexEvent::Account(Some("one@example.com".into()))).unwrap();
        events.send(CodexEvent::Accounts { list: vec![account("codex-home", "one@example.com"), account("codex-home-2", "two@example.com")], current: "codex-home".into() }).unwrap();
        poll(h.state_mut());
        h.run_steps(2);
        assert!(h.query_by_label("Switch to two@example.com").is_none(), "only at the limit, or in the menu");
        events.send(CodexEvent::LimitReached).unwrap();
        poll(h.state_mut());
        h.run_steps(2);
        assert!(h.query_by_label("Switch to one@example.com").is_none(), "not the account in use");
        h.get_by_label("Switch to two@example.com").click();
        h.run_steps(2);
        assert_eq!(*asked.lock().unwrap(), [Some("codex-home-2".to_string())]);
        assert!(!h.state().codex.limit, "the new account starts clean");
        assert!(h.state().codex.link.is_some());
    }

    /// The footer lists Codex's models and asks for the one picked.
    #[test]
    fn codex_footer_picks_a_model() {
        let model = |id: &str, name: &str| CodexModel { id: id.into(), name: name.into(), description: format!("{name} description") };
        let (link, requests, events) = link();
        let mut app = app();
        app.codex.link = Some(link);
        app.codex.open = true;
        let mut h = harness(app);
        events.send(CodexEvent::Account(Some("me".into()))).unwrap();
        events.send(CodexEvent::Models { models: vec![model("big", "BigModel"), model("small", "SmallModel")], current: Some("big".into()) }).unwrap();
        poll(h.state_mut());
        h.run_steps(2);
        h.get_by_label_contains("BigModel").click();
        h.run_steps(2);
        assert!(h.query_by_label("SmallModel description").is_some(), "the menu says what each model is for");
        h.get_by_label("SmallModel").click();
        h.run_steps(2);
        assert_eq!(requests.try_recv().ok(), Some(CodexRequest::SetModel("small".into())));
        assert_eq!(h.state().codex.model.as_deref(), Some("small"));
    }
}
