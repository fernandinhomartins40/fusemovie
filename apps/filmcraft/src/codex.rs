//! Window › Codex backend: runs `codex app-server` (the JSON-RPC process the Codex IDE extension
//! uses, signed in with ChatGPT, so no API key) and gives it this window as the MCP server
//! `fusemovie` (`filmcraft-cli mcp --bridge`, over the loopback control channel). The chat UI is
//! `filmcraft_ui_egui::codex_panel`.
//!
//! Codex runs with its own home (`CODEX_HOME`, sign-in and history) and without plugins, apps or
//! computer use, so the user's other Codex MCP servers and plugins don't crowd out the editor's
//! tools: the model sees `fusemovie` and image generation. Each ChatGPT account has its own home
//! (`codex-home`, `codex-home-2`, …), so changing accounts never touches a sign-in. The control
//! token reaches the bridge through the environment only (`FILMCRAFT_CONTROL_TOKEN`, passed on
//! with `env_vars`), never a file or a command line.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::mpsc::{Sender, channel};

use filmcraft_ui_egui::codex_panel::{APP_NAME, CodexAccount, CodexEvent, CodexHistory, CodexLink, CodexModel, CodexRequest, CodexThread, NEW_ACCOUNT};
use serde_json::{Value, json};

/// The MCP server name Codex sees (its tool calls read `fusemovie › tool`).
const SERVER: &str = "fusemovie";
/// The environment variable that carries the control credential to the bridge.
pub const TOKEN_VAR: &str = "FILMCRAFT_CONTROL_TOKEN";

/// What Codex is told about where it runs.
const INSTRUCTIONS: &str = "You are running inside {app}, a video editor, as its built-in assistant. \
Act on the user's open project only with the `fusemovie` MCP tools (never the shell or screen control): `command_list` finds the editor's commands \
(timeline, clips, tracks, effects, transitions, captions, markers, titles, audio, export), `command_run` and `command_batch` run them (every edit is undoable), \
`doc_inspect`, `project_inspect` and `sequence_inspect` read the project and the active sequence (ids, times, tracks, clips), \
`media_import` imports files by path, and `render_preview` renders the sequence frame at `seconds` (optionally only a `region` of it, to see details). \
Time is in ticks (254016000000 per second); commands also accept `seconds`, `frame` or `timecode`. \
Inspect before you edit, and check the result with a preview. \
Images the user attaches are references: follow their style, colours and composition, also when you generate an image, and don't import them unless asked. \
A `Reference file:` line gives the path of another attached file (a script, a subtitle file, a clip), which you may import or read with read-only commands. \
When the user wants a new image (a title card, a background, a thumbnail), create it with your image generation tool: {app} imports every generated \
image into the project as media by itself, so never import it again; place it on the timeline with the editor's commands when asked. \
Answer in the user's language, briefly.";

/// `name` next to this executable (the installer puts `filmcraft-cli` and the bundled `codex` there).
fn sibling(name: &str) -> Option<PathBuf> {
    let exe = format!("{name}{}", std::env::consts::EXE_SUFFIX);
    let path = std::env::current_exe().ok()?.parent()?.join(exe);
    path.is_file().then_some(path)
}

/// The first of `names` in a `PATH` directory.
fn on_path(names: &[&str]) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).flat_map(|dir| names.iter().map(move |n| dir.join(n))).find(|p| p.is_file())
}

/// The Codex CLI: `FILMCRAFT_CODEX`, else the copy installed with the app, else `PATH`.
pub fn find_codex() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("FILMCRAFT_CODEX").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(path));
    }
    // npm installs a `codex.cmd` shim on Windows.
    sibling("codex").or_else(|| on_path(if cfg!(windows) { &["codex.exe", "codex.cmd"] } else { &["codex"] }))
}

/// The app's CLI, whose `mcp --bridge` mode is the MCP server: next to the app, else on `PATH`.
fn find_cli() -> Option<PathBuf> {
    sibling("filmcraft-cli").or_else(|| on_path(if cfg!(windows) { &["filmcraft-cli.exe"] } else { &["filmcraft-cli"] }))
}

/// A JSON string, which is also a TOML basic string (for `codex -c key=value`).
fn toml_str(s: &str) -> String {
    Value::String(s.to_owned()).to_string()
}

/// The `codex app-server` arguments that register this window as the `fusemovie` MCP server.
/// The token is never among them: it goes through the environment ([`TOKEN_VAR`]).
fn server_args(cli: &Path, port: u16) -> Vec<String> {
    let args = json!(["mcp", "--bridge", format!("127.0.0.1:{port}")]);
    let config = [
        format!("mcp_servers.{SERVER}.command={}", toml_str(&cli.to_string_lossy())),
        format!("mcp_servers.{SERVER}.args={args}"),
        format!(r#"mcp_servers.{SERVER}.env_vars=["{TOKEN_VAR}"]"#),
        format!("mcp_servers.{SERVER}.startup_timeout_sec=30"),
        // With approval policy "never" (no shell, no file changes), Codex refuses any MCP tool that
        // edits unless the server is pre-approved: editing the project is what this server is for.
        format!(r#"mcp_servers.{SERVER}.default_tools_approval_mode="approve""#),
    ];
    std::iter::once("app-server".to_string())
        .chain(config.into_iter().flat_map(|c| ["-c".to_string(), c]))
        .chain(["--enable".to_string(), "image_generation".to_string()])
        .chain(["apps", "plugins", "computer_use", "multi_agent"].into_iter().flat_map(|f| ["--disable".to_string(), f.to_string()]))
        .collect()
}

/// A fresh control credential: 64 hex characters from the OS's random source, as the control
/// server requires.
pub fn generate_token() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| format!("could not make a control token: {e}"))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Where the Codex accounts live: `%APPDATA%\<brand>` on Windows, the app's data folder elsewhere.
pub fn base_dir() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA").filter(|v| cfg!(windows) && !v.is_empty());
    match appdata {
        Some(dir) => Some(PathBuf::from(dir).join(APP_NAME)),
        None => filmcraft_engine::autosave::default_data_dir(),
    }
}

/// The first account's home; the others add `-2`, `-3`, ….
const HOME: &str = "codex-home";
/// Who is signed in, in each home that has someone: only this app reads it.
const LABEL: &str = "account-label";
/// The file beside the homes naming the one last used.
const ACTIVE: &str = "codex-active-account";
const MAX_ACCOUNTS: usize = 64;

fn valid_home(id: &str) -> bool {
    id == HOME || id.strip_prefix("codex-home-").is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

fn account_label(base: &Path, id: &str) -> Option<String> {
    let label = std::fs::read_to_string(base.join(id).join(LABEL)).ok()?;
    Some(label.trim().to_owned()).filter(|l| !l.is_empty())
}

/// The accounts signed in under `base`.
fn accounts(base: &Path) -> Vec<CodexAccount> {
    let names = std::fs::read_dir(base).into_iter().flatten().flatten().filter_map(|e| e.file_name().into_string().ok());
    let mut list: Vec<CodexAccount> = names.filter(|id| valid_home(id)).filter_map(|id| Some(CodexAccount { label: account_label(base, &id)?, id })).collect();
    list.sort_by(|a, b| a.label.cmp(&b.label));
    list
}

/// The home of `choice` (an account's id, [`NEW_ACCOUNT`], or `None` for the last one used), created
/// and remembered as the last one used.
pub fn account_home(base: &Path, choice: Option<&str>) -> Result<PathBuf, String> {
    let saved = std::fs::read_to_string(base.join(ACTIVE)).ok().map(|s| s.trim().to_owned()).filter(|s| valid_home(s)).unwrap_or_else(|| HOME.to_owned());
    let id = match choice {
        None => saved.clone(),
        Some(NEW_ACCOUNT) => (1..=MAX_ACCOUNTS)
            .map(|n| if n == 1 { HOME.to_owned() } else { format!("{HOME}-{n}") })
            .find(|id| account_label(base, id).is_none())
            .ok_or("too many Codex accounts")?,
        Some(id) if valid_home(id) => id.to_owned(),
        Some(_) => return Err("unknown Codex account".into()),
    };
    let home = base.join(&id);
    std::fs::create_dir_all(&home).map_err(|e| format!("could not create {}: {e}", home.display()))?;
    if id != saved {
        // The chosen model goes along: a new account must not fall back to the catalog's costly default.
        let _ = std::fs::copy(base.join(&saved).join("config.toml"), home.join("config.toml"));
    }
    std::fs::write(base.join(ACTIVE), &id).map_err(|e| format!("could not save the Codex account: {e}"))?;
    Ok(home)
}

/// The `input` of `turn/start`: images as images Codex looks at, other files as paths.
fn turn_input(text: &str, files: &[String]) -> Value {
    use filmcraft_ui_egui::codex_panel::is_reference_image;
    let mut prompt = text.to_owned();
    for file in files.iter().filter(|f| !is_reference_image(f)) {
        prompt.push_str(&format!("\n\nReference file: {file}"));
    }
    let images = files.iter().filter(|f| is_reference_image(f)).map(|f| json!({"type": "localImage", "path": f}));
    Value::Array(std::iter::once(json!({"type": "text", "text": prompt})).chain(images).collect())
}

/// This error means the account ran out, so another account would get through.
fn is_limit(error: &Value) -> bool {
    matches!(error.get("codexErrorInfo").and_then(Value::as_str), Some("usageLimitExceeded" | "rateLimitExceeded"))
}

/// The error when [`find_codex`] finds nothing.
pub const NOT_FOUND: &str = "Codex CLI not found: install it (npm install -g @openai/codex) or set FILMCRAFT_CODEX";

/// Start `codex` for a control server on `port` with `token`, working in `cwd`, with its sign-in
/// and history in `home`.
pub fn start(codex: &Path, port: u16, token: &str, cwd: &Path, home: &Path, ctx: egui::Context) -> Result<CodexLink, String> {
    let cli = find_cli().ok_or("filmcraft-cli was not found next to the app")?;
    let mut cmd = Command::new(codex);
    cmd.args(server_args(&cli, port))
        .current_dir(cwd)
        .env(TOKEN_VAR, token)
        .env("CODEX_HOME", home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        // CREATE_NO_WINDOW: the GUI app has no console for Codex to borrow.
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = cmd.spawn().map_err(|e| format!("could not start Codex ({}): {e}", codex.display()))?;
    let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
        let _ = child.kill();
        return Err("could not connect to Codex".into());
    };
    let (in_tx, in_rx) = channel::<Input>();
    let (ui_tx, ui_rx) = channel::<CodexRequest>();
    let (ev_tx, ev_rx) = channel::<CodexEvent>();
    let lines = in_tx.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if let Ok(v) = serde_json::from_str::<Value>(&line)
                && lines.send(Input::Message(v)).is_err()
            {
                return;
            }
        }
        let _ = lines.send(Input::Exited);
    });
    std::thread::spawn(move || {
        while let Ok(request) = ui_rx.recv() {
            if in_tx.send(Input::Ui(request)).is_err() {
                return;
            }
        }
        let _ = in_tx.send(Input::Closed);
    });
    let cwd = cwd.to_string_lossy().into_owned();
    let home = home.to_path_buf();
    std::thread::spawn(move || {
        let mut driver =
            Driver { stdin, next_id: 0, pending: HashMap::new(), thread: None, turn: None, starting: false, queued: Vec::new(), model: None, events: ev_tx, ctx, cwd, home };
        driver.request("initialize", json!({"clientInfo": {"name": SERVER, "title": APP_NAME, "version": env!("CARGO_PKG_VERSION")}}));
        for input in in_rx {
            match input {
                Input::Ui(request) => driver.ui(request),
                Input::Message(v) => driver.message(&v),
                Input::Exited => {
                    driver.emit(CodexEvent::Exited("Codex stopped.".into()));
                    break;
                }
                Input::Closed => break,
            }
        }
        let _ = child.kill();
        let _ = child.wait();
    });
    Ok(CodexLink { tx: ui_tx, rx: ev_rx })
}

/// shortcut: the newest conversations only; page with `nextCursor` if users want older ones.
const THREAD_PAGE: u32 = 50;

/// The conversations of a `thread/list` reply.
fn threads(result: &Value) -> Vec<CodexThread> {
    let data = result.get("data").and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
    data.iter()
        .filter_map(|t| {
            let title = text(t, "name").filter(|n| !n.trim().is_empty()).or_else(|| text(t, "preview")).unwrap_or_default();
            Some(CodexThread { id: text(t, "id")?, title, updated: t.get("updatedAt").and_then(Value::as_i64).unwrap_or(0) })
        })
        .collect()
}

/// What a resumed conversation said: the user's text, Codex's messages and its tool calls.
fn history(thread: &Value) -> Vec<CodexHistory> {
    let turns = thread.get("turns").and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
    let items = turns.iter().flat_map(|t| t.get("items").and_then(Value::as_array).map_or(&[][..], Vec::as_slice));
    items
        .filter_map(|item| match item.get("type").and_then(Value::as_str)? {
            "userMessage" => {
                let parts = item.get("content").and_then(Value::as_array)?;
                let text: Vec<&str> = parts.iter().filter_map(|c| c.get("text").and_then(Value::as_str)).collect();
                (!text.is_empty()).then(|| CodexHistory::User(text.join("\n")))
            }
            "agentMessage" => text(item, "text").filter(|t| !t.is_empty()).map(CodexHistory::Assistant),
            "mcpToolCall" => Some(CodexHistory::Step(format!("{} › {}", text(item, "server").unwrap_or_default(), text(item, "tool").unwrap_or_default()))),
            "imageGeneration" => Some(CodexHistory::Step("Generating image…".into())),
            _ => None,
        })
        .collect()
}

/// shortcut: one page of `model/list`, enough for today's catalog; follow `nextCursor` if it nears this.
const MODEL_PAGE: u32 = 100;

enum Input {
    Ui(CodexRequest),
    Message(Value),
    Exited,
    /// The window dropped its end.
    Closed,
}

struct Driver {
    stdin: ChildStdin,
    next_id: u64,
    /// Request id → method, to read the matching reply.
    pending: HashMap<u64, &'static str>,
    thread: Option<String>,
    turn: Option<String>,
    /// A `thread/start` is on its way.
    starting: bool,
    /// Messages and their files, waiting for the conversation to start.
    queued: Vec<(String, Vec<String>)>,
    /// The model picked in the chat or saved in Codex's config (`None`: the catalog's default).
    model: Option<String>,
    events: Sender<CodexEvent>,
    ctx: egui::Context,
    cwd: String,
    home: PathBuf,
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// The models of a `model/list` reply and the current one: `configured`, else the catalog's default.
fn models(result: &Value, configured: Option<&str>) -> (Vec<CodexModel>, Option<String>) {
    let data = result.get("data").and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
    let list = data
        .iter()
        .filter_map(|m| {
            let id = text(m, "id")?;
            Some(CodexModel { name: text(m, "displayName").unwrap_or_else(|| id.clone()), description: text(m, "description").unwrap_or_default(), id })
        })
        .collect();
    let default = data.iter().find(|m| m.get("isDefault").and_then(Value::as_bool) == Some(true)).and_then(|m| text(m, "id"));
    (list, configured.map(str::to_owned).or(default))
}

/// How every conversation runs: in the app's Codex folder, read-only, told where it is.
fn thread_params(cwd: &str) -> Value {
    json!({"cwd": cwd, "approvalPolicy": "never", "sandbox": "read-only", "developerInstructions": INSTRUCTIONS.replace("{app}", APP_NAME)})
}

impl Driver {
    fn write(&mut self, v: &Value) {
        if writeln!(self.stdin, "{v}").and_then(|()| self.stdin.flush()).is_err() {
            self.emit(CodexEvent::Error("could not write to Codex".into()));
        }
    }

    fn request(&mut self, method: &'static str, params: Value) {
        self.next_id += 1;
        self.pending.insert(self.next_id, method);
        self.write(&json!({"id": self.next_id, "method": method, "params": params}));
    }

    fn emit(&self, event: CodexEvent) {
        let _ = self.events.send(event);
        self.ctx.request_repaint();
    }

    fn ui(&mut self, request: CodexRequest) {
        match request {
            CodexRequest::SignIn => self.request("account/login/start", json!({"type": "chatgpt"})),
            CodexRequest::SignOut => self.request("account/logout", json!({})),
            CodexRequest::Send { text, files } => match self.thread.clone() {
                Some(thread) => self.turn_start(&thread, &text, &files),
                None => {
                    self.queued.push((text, files));
                    if !self.starting {
                        self.starting = true;
                        self.request("thread/start", thread_params(&self.cwd));
                    }
                }
            },
            CodexRequest::Stop => {
                if let (Some(thread), Some(turn)) = (self.thread.clone(), self.turn.clone()) {
                    self.request("turn/interrupt", json!({"threadId": thread, "turnId": turn}));
                }
            }
            CodexRequest::NewChat => (self.thread, self.turn) = (None, None),
            CodexRequest::ListThreads => self.request("thread/list", json!({"limit": THREAD_PAGE})),
            CodexRequest::Resume(id) => {
                (self.thread, self.turn) = (None, None);
                let mut params = thread_params(&self.cwd);
                params["threadId"] = json!(id);
                self.request("thread/resume", params);
            }
            CodexRequest::SetModel(model) => {
                // Codex's config, in its own home: the choice outlives this run.
                self.request("config/value/write", json!({"keyPath": "model", "value": model, "mergeStrategy": "replace"}));
                self.model = Some(model);
            }
        }
    }

    /// Note who is signed in on this home and report the accounts. A browser still signed in to
    /// another account signs in to that one again: it isn't a new account then.
    fn remember_account(&mut self, who: Option<&str>) {
        let Some((base, id)) = self.home.parent().zip(self.home.file_name().and_then(|n| n.to_str())) else { return };
        let (base, id) = (base.to_path_buf(), id.to_owned());
        let label = self.home.join(LABEL);
        match who {
            Some(who) if who.contains('@') && accounts(&base).iter().any(|a| a.id != id && a.label == who) => {
                self.emit(CodexEvent::Error("This account is already added. To add another, sign out of it in your browser and sign in again.".into()));
            }
            Some(who) => {
                let _ = std::fs::write(&label, who);
            }
            None => {
                let _ = std::fs::remove_file(&label);
            }
        }
        self.emit(CodexEvent::Accounts { list: accounts(&base), current: id });
    }

    fn turn_start(&mut self, thread: &str, message: &str, files: &[String]) {
        // Also for a conversation already running: the model may have changed since it started.
        self.request("turn/start", json!({"threadId": thread, "input": turn_input(message, files), "model": self.model}));
    }

    fn message(&mut self, v: &Value) {
        let method = v.get("method").and_then(Value::as_str);
        match (v.get("id"), method) {
            (Some(id), Some(method)) => self.server_request(id.clone(), method, v.get("params").unwrap_or(&Value::Null)),
            (Some(id), None) => {
                let Some(method) = id.as_u64().and_then(|id| self.pending.remove(&id)) else { return };
                match v.get("error") {
                    Some(error) => self.failed(method, error),
                    None => self.reply(method, v.get("result").unwrap_or(&Value::Null)),
                }
            }
            (None, Some(method)) => self.notification(method, v.get("params").unwrap_or(&Value::Null)),
            (None, None) => {}
        }
    }

    fn reply(&mut self, method: &str, result: &Value) {
        match method {
            "initialize" => {
                self.write(&json!({"method": "initialized"}));
                self.request("account/read", json!({}));
            }
            "account/read" => {
                let account = result.get("account").filter(|a| !a.is_null());
                let who = account.map(|a| text(a, "email").or_else(|| text(a, "type")).unwrap_or_else(|| "signed in".into()));
                self.remember_account(who.as_deref());
                self.emit(CodexEvent::Account(who));
                self.request("config/read", json!({}));
            }
            "config/read" => {
                self.model = result.get("config").and_then(|c| text(c, "model"));
                self.request("model/list", json!({"limit": MODEL_PAGE}));
            }
            "model/list" => {
                let (models, current) = models(result, self.model.as_deref());
                self.emit(CodexEvent::Models { models, current });
            }
            "account/logout" => self.request("account/read", json!({})),
            "account/login/start" => {
                if let Some(url) = text(result, "authUrl") {
                    // The user asked to sign in: their system browser opens ChatGPT's page.
                    self.emit(CodexEvent::Activity("Sign in to ChatGPT in your browser…".into()));
                    if let Err(e) = open::that_detached(&url) {
                        self.emit(CodexEvent::Error(format!("could not open the browser ({e}); sign in at {url}")));
                    }
                }
            }
            "thread/list" => self.emit(CodexEvent::Threads(threads(result))),
            "thread/resume" => {
                let thread = result.get("thread").unwrap_or(&Value::Null);
                self.thread = text(thread, "id");
                self.emit(CodexEvent::Resumed(history(thread)));
            }
            "thread/start" => {
                self.starting = false;
                let Some(thread) = result.get("thread").and_then(|t| text(t, "id")) else {
                    self.emit(CodexEvent::Error("Codex did not start a conversation".into()));
                    self.emit(CodexEvent::TurnFinished);
                    return;
                };
                self.thread = Some(thread.clone());
                for (message, files) in std::mem::take(&mut self.queued) {
                    self.turn_start(&thread, &message, &files);
                }
            }
            _ => {}
        }
    }

    fn failed(&mut self, method: &str, error: &Value) {
        let message = text(error, "message").unwrap_or_else(|| error.to_string());
        if method == "thread/start" {
            self.starting = false;
            self.queued.clear();
        }
        self.emit(CodexEvent::Error(format!("{method}: {message}")));
        if method == "config/read" {
            self.request("model/list", json!({"limit": MODEL_PAGE}));
        }
        if matches!(method, "thread/start" | "turn/start") {
            self.emit(CodexEvent::TurnFinished);
        }
    }

    fn report_error(&mut self, error: &Value) {
        if is_limit(error) {
            self.emit(CodexEvent::LimitReached);
        }
        if let Some(message) = text(error, "message") {
            self.emit(CodexEvent::Error(message));
        }
    }

    fn notification(&mut self, method: &str, p: &Value) {
        let item = p.get("item").unwrap_or(&Value::Null);
        let kind = item.get("type").and_then(Value::as_str).unwrap_or("");
        match method {
            "account/login/completed" => match p.get("success").and_then(Value::as_bool) {
                Some(true) => self.request("account/read", json!({})),
                _ => self.emit(CodexEvent::Error(format!("sign-in failed: {}", text(p, "error").unwrap_or_default()))),
            },
            "account/updated" => self.request("account/read", json!({})),
            "turn/started" => {
                self.turn = p.get("turn").and_then(|t| text(t, "id"));
                self.emit(CodexEvent::TurnStarted);
            }
            "turn/completed" => {
                self.turn = None;
                if let Some(error) = p.get("turn").and_then(|t| t.get("error")) {
                    self.report_error(error);
                }
                self.emit(CodexEvent::TurnFinished);
            }
            "error" if p.get("willRetry").and_then(Value::as_bool) != Some(true) => {
                if let Some(error) = p.get("error") {
                    self.report_error(error);
                }
            }
            "item/agentMessage/delta" => {
                if let (Some(item), Some(delta)) = (text(p, "itemId"), text(p, "delta")) {
                    self.emit(CodexEvent::AssistantDelta { item, delta });
                }
            }
            "item/started" => {
                let line = match kind {
                    "mcpToolCall" => Some(format!("{} › {}", text(item, "server").unwrap_or_default(), text(item, "tool").unwrap_or_default())),
                    "imageGeneration" => Some("Generating image…".into()),
                    "webSearch" => Some("Searching the web…".into()),
                    "commandExecution" => text(item, "command").map(|c| format!("$ {c}")),
                    _ => None,
                };
                if let Some(line) = line {
                    self.emit(CodexEvent::Activity(line));
                }
            }
            "item/completed" => match kind {
                "agentMessage" => {
                    if let (Some(id), Some(text)) = (text(item, "id"), text(item, "text")) {
                        self.emit(CodexEvent::AssistantDone { item: id, text });
                    }
                }
                "imageGeneration" => match text(item, "savedPath") {
                    Some(path) => self.emit(CodexEvent::Image(PathBuf::from(path))),
                    None if item.get("failure").is_some_and(|f| !f.is_null()) => {
                        self.emit(CodexEvent::LimitReached);
                        self.emit(CodexEvent::Error("Image generation failed: the usage limit was reached.".into()));
                    }
                    None => self.emit(CodexEvent::Error("Image generation failed.".into())),
                },
                "mcpToolCall" => {
                    if let Some(message) = item.get("error").and_then(|e| text(e, "message")) {
                        self.emit(CodexEvent::Error(message));
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    /// Codex asking the app something. Only this app's own tools are approved; Codex runs
    /// read-only and never asks to run commands or change files.
    fn server_request(&mut self, id: Value, method: &str, p: &Value) {
        let result = match method {
            "mcpServer/elicitation/request" if text(p, "serverName").as_deref() == Some(SERVER) => {
                let content = if text(p, "mode").as_deref() == Some("url") { Value::Null } else { json!({}) };
                json!({"action": "accept", "content": content})
            }
            "mcpServer/elicitation/request" => json!({"action": "decline"}),
            "item/commandExecution/requestApproval" | "item/fileChange/requestApproval" => json!({"decision": "decline"}),
            _ => {
                self.write(&json!({"id": id, "error": {"code": -32601, "message": format!("This app does not handle {method}")}}));
                return;
            }
        };
        self.write(&json!({"id": id, "result": result}));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_saved_conversations_list_and_reopen() {
        let list = threads(&json!({"data": [
            {"id": "a", "name": null, "preview": "cut the intro", "updatedAt": 100},
            {"id": "b", "name": "Trailer", "preview": "x", "updatedAt": 50},
            {"preview": "no id"},
        ]}));
        assert_eq!(list.iter().map(|t| (t.id.as_str(), t.title.as_str())).collect::<Vec<_>>(), [("a", "cut the intro"), ("b", "Trailer")]);
        let said = history(&json!({"turns": [{"items": [
            {"type": "userMessage", "content": [{"type": "text", "text": "hi"}, {"type": "localImage", "path": "x.png"}]},
            {"type": "reasoning"},
            {"type": "mcpToolCall", "server": "fusemovie", "tool": "sequence_inspect"},
            {"type": "agentMessage", "text": "Done."},
        ]}]}));
        assert_eq!(said, [CodexHistory::User("hi".into()), CodexHistory::Step("fusemovie › sequence_inspect".into()), CodexHistory::Assistant("Done.".into())]);
        assert!(history(&json!({})).is_empty());
    }

    #[test]
    fn codex_turn_input_sends_images_as_images_and_other_files_as_paths() {
        let input = turn_input("like this", &["C:/a/look.PNG".to_string(), "C:/a/script.srt".to_string()]);
        assert_eq!(
            input,
            json!([
                {"type": "text", "text": "like this\n\nReference file: C:/a/script.srt"},
                {"type": "localImage", "path": "C:/a/look.PNG"},
            ])
        );
        assert_eq!(turn_input("hi", &[]), json!([{"type": "text", "text": "hi"}]));
    }

    #[test]
    fn codex_only_running_out_of_usage_is_a_limit() {
        assert!(is_limit(&json!({"message": "x", "codexErrorInfo": "usageLimitExceeded"})));
        assert!(is_limit(&json!({"message": "x", "codexErrorInfo": "rateLimitExceeded"})));
        assert!(!is_limit(&json!({"message": "x", "codexErrorInfo": "badRequest"})));
        assert!(!is_limit(&json!({"message": "x"})));
    }

    #[test]
    fn codex_accounts_keep_their_own_homes() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        let label = |home: &Path, who: &str| std::fs::write(home.join(LABEL), who).unwrap();
        let first = account_home(base, None).unwrap();
        assert_eq!(first, base.join("codex-home"));
        label(&first, "one@example.com");
        std::fs::write(first.join("config.toml"), "model = \"cheap\"\n").unwrap();
        std::fs::write(first.join("auth.json"), "first").unwrap();
        // A new account gets a home of its own, with the chosen model, and is remembered.
        let second = account_home(base, Some(NEW_ACCOUNT)).unwrap();
        assert_eq!(second, base.join("codex-home-2"));
        assert_eq!(std::fs::read_to_string(second.join("config.toml")).unwrap(), "model = \"cheap\"\n");
        assert!(!second.join("auth.json").exists(), "a sign-in never moves");
        assert_eq!(account_home(base, None).unwrap(), second);
        // Until someone signs in there it isn't an account, and is reused rather than piling up.
        assert_eq!(accounts(base).len(), 1);
        assert_eq!(account_home(base, Some(NEW_ACCOUNT)).unwrap(), second);
        label(&second, "two@example.com");
        assert_eq!(accounts(base).iter().map(|a| a.label.as_str()).collect::<Vec<_>>(), ["one@example.com", "two@example.com"]);
        assert_eq!(account_home(base, Some(HOME)).unwrap(), first);
        assert_eq!(std::fs::read_to_string(first.join("auth.json")).unwrap(), "first");
        for bad in ["../evil", "codex-home-", "codex-home-2/..", "other", ""] {
            assert!(account_home(base, Some(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn codex_model_list_reads_the_catalog_and_the_saved_choice_wins() {
        let reply = json!({"data": [
            {"id": "big", "displayName": "Big", "description": "Frontier.", "isDefault": true},
            {"id": "small", "displayName": "Small", "description": "Affordable.", "isDefault": false},
            {"displayName": "no id"},
        ]});
        let (list, current) = models(&reply, None);
        assert_eq!(list.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["big", "small"]);
        assert_eq!(current.as_deref(), Some("big"));
        assert_eq!(models(&reply, Some("small")).1.as_deref(), Some("small"));
        assert_eq!(models(&json!({}), None), (Vec::new(), None));
    }

    #[test]
    fn codex_server_args_register_the_bridge_pre_approved_without_the_token() {
        let cli = r#"C:\Program Files\Fuse "Movie"\filmcraft-cli.exe"#;
        let args = server_args(Path::new(cli), 4567);
        assert_eq!(args.first().map(String::as_str), Some("app-server"));
        let command = args.iter().find_map(|a| a.strip_prefix("mcp_servers.fusemovie.command=")).unwrap_or_default();
        assert_eq!(serde_json::from_str::<String>(command).unwrap_or_default(), cli);
        assert!(args.iter().any(|a| a == r#"mcp_servers.fusemovie.args=["mcp","--bridge","127.0.0.1:4567"]"#));
        assert!(args.iter().any(|a| a == r#"mcp_servers.fusemovie.env_vars=["FILMCRAFT_CONTROL_TOKEN"]"#));
        assert!(args.iter().any(|a| a == "mcp_servers.fusemovie.startup_timeout_sec=30"));
        assert!(args.iter().any(|a| a == r#"mcp_servers.fusemovie.default_tools_approval_mode="approve""#), "or Codex refuses every edit");
        // Every `-c` is followed by its value.
        let configs = args.iter().zip(args.iter().skip(1)).filter(|(flag, _)| *flag == "-c").count();
        assert_eq!(configs, 5);
        assert!(args.windows(2).any(|w| w == ["--enable", "image_generation"]));
        for feature in ["apps", "plugins", "computer_use", "multi_agent"] {
            assert!(args.windows(2).any(|w| w == ["--disable", feature]), "{feature}");
        }
        // The credential only ever travels in the environment.
        let token = generate_token().unwrap();
        assert!(!args.iter().any(|a| a.contains(&token)));
    }

    #[test]
    fn codex_control_tokens_are_64_random_hex_characters() {
        let a = generate_token().unwrap();
        let b = generate_token().unwrap();
        assert_eq!(a.len(), 64);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()), "{a}");
        assert_ne!(a, b);
    }

    #[test]
    fn codex_threads_run_read_only_without_approvals_and_know_the_editor() {
        let p = thread_params("C:/work");
        assert_eq!(p["approvalPolicy"], "never");
        assert_eq!(p["sandbox"], "read-only");
        let instructions = p["developerInstructions"].as_str().unwrap_or_default();
        assert!(instructions.contains(APP_NAME) && instructions.contains("video editor") && instructions.contains("`fusemovie` MCP tools"), "{instructions}");
        assert!(!instructions.contains("{app}"));
    }
}
