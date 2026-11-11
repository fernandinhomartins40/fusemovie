//! File › Create with AI: the user gives clips and a briefing and Codex assembles a first cut, an
//! ordinary editable sequence, while they watch. The app only sets the stage and shows the work:
//! Codex edits through the same MCP tools as any chat ([`RECIPE`] tells it how).
//!
//! A run makes a new project, imports the material into its bin, makes the empty sequence
//! "Assembly" in the first video clip's format, starts a new conversation and locks the editor (only
//! the Codex column and the progress bar stay live) until the turn ends or the user stops it.
//! Codex's commands arrive one at a time, even inside a batch, so [`hold`] paces them and each can
//! be seen; the cursor only shows where on the Timeline the last one landed, Codex doesn't use the
//! mouse.

use egui::{Align2, Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use filmcraft_project::{ClipId, ItemId, Sequence, TrackId};
use filmcraft_time::Tick;
use serde_json::{Value, json};

use crate::FilmcraftApp;
use crate::codex_panel;
use crate::icons::Icon;
use crate::native_dialogs::FileDialog;
use crate::theme::Tokens;

/// The model a run starts on when Codex offers it. The dialog can change it.
const DEFAULT_MODEL: &str = "gpt-6.1-sol";
/// Seconds between Codex's edits during a run, so they appear one by one.
const PACE: f64 = 0.12;
pub(crate) const COMMAND: &str = "file.createWithAi";
/// Files taken from a folder or a drop at most (a run looks at every clip).
const MAX_MEDIA: usize = 500;

const VIDEO_EXTS: [&str; 11] = ["mp4", "mov", "m4v", "mkv", "webm", "avi", "mxf", "mts", "m2ts", "mpg", "mpeg"];
const AUDIO_EXTS: [&str; 7] = ["wav", "mp3", "m4a", "aac", "flac", "ogg", "opus"];
const IMAGE_EXTS: [&str; 6] = ["png", "jpg", "jpeg", "tif", "tiff", "webp"];

/// The stages Codex announces with a `[[n]]` tag, in order. Keep them in step with [`RECIPE`].
const STEPS: [&str; 6] = ["Looking at the clips", "Transcribing the speech", "Assembling the cut", "Captions and titles", "Music and color", "Checking the result"];
/// The same without speech to text: in step with [`RECIPE_NO_SPEECH`].
const STEPS_NO_SPEECH: [&str; 5] = ["Looking at the clips", "Assembling the cut", "Titles", "Music and color", "Checking the result"];

/// How Codex does it (docs/fusemovie/ORQUESTRACAO.md, section 4).
const RECIPE: &str = r#"Assemble a first cut from the media in this project, following the briefing below. The project is
already open: every clip is imported in the Project bin and the empty sequence "{sequence}" is active.
Work only through this app's tools. Write to the user in {language}.

Progress: begin each message with the tag of the stage you are starting, once per stage and in order:
[[1]] inventory, [[2]] transcription, [[3]] assembly, [[4]] captions and titles, [[5]] music and color,
[[6]] checking. Keep messages to a sentence or two.

1. Inventory. Read the clips with `project_inspect` (ids, durations, sizes). Look at each clip with
   `render_preview` and its `item` parameter at a few evenly spaced times: at most 6 frames per clip,
   `max_side` 512. Note what each clip shows, which are takes of the same moment, and which parts are
   unusable (out of focus, camera moving to position, slate).

2. Transcription. Run `transcript.generate` on the clips with speech, then read the words and their
   times. Speech decides where a cut can fall: never cut inside a word.

3. Assembly. Build the cut in the order the briefing asks, with `command_batch`: place each chosen
   range on the timeline (`timeline.place`), trim heads and tails (`timeline.trim`, `timeline.razor`),
   keep dialogue on the first audio track. Prefer straight cuts; add a transition
   (`sequence.applyVideoTransition`) only where time or place changes. Leave no gaps. If the briefing
   gives a duration, meet it within 5%.

4. Captions and titles. Create captions from the transcript (`transcript.createCaptions`), two lines
   at most. Add the titles the briefing asks for as editable text (`graphics.newText`): opening title,
   names, closing card. Never burn text into an image.

5. Music and color. If a music file is in the bin, put it on its own audio track under the dialogue
   and lower it while people speak (`clip.audioGain`). Make the shots match: adjust exposure, white
   balance and contrast per clip with `lumetri.setSection`, or `lumetri.applyMatch` against the best
   shot of the scene. Adjustments only; do not render or export.

6. Check. Read `sequence_inspect` for gaps, overlaps and clips cut inside a word. Look at
   `render_preview` of the sequence at every cut and at each title. Fix what you find.

Use `command_list` to confirm a command's parameters before the first time you use it. Don't save,
export or close the project: the user saves it when you finish. End with a brief note of the
structure you built, what you checked and what is still rough.

Briefing:
{briefing}"#;

/// [`RECIPE`] for a build without speech to text: no transcription stage and no captions, five tags.
const RECIPE_NO_SPEECH: &str = r#"Assemble a first cut from the media in this project, following the briefing below. The project is
already open: every clip is imported in the Project bin and the empty sequence "{sequence}" is active.
Work only through this app's tools. Write to the user in {language}.

Progress: begin each message with the tag of the stage you are starting, once per stage and in order:
[[1]] inventory, [[2]] assembly, [[3]] titles, [[4]] music and color, [[5]] checking. Keep messages to
a sentence or two.

1. Inventory. Read the clips with `project_inspect` (ids, durations, sizes). Look at each clip with
   `render_preview` and its `item` parameter at a few evenly spaced times: at most 6 frames per clip,
   `max_side` 512. Note what each clip shows, which are takes of the same moment, and which parts are
   unusable (out of focus, camera moving to position, slate).

2. Assembly. Build the cut in the order the briefing asks, with `command_batch`: place each chosen
   range on the timeline (`timeline.place`), trim heads and tails (`timeline.trim`, `timeline.razor`),
   keep dialogue on the first audio track. Prefer straight cuts; add a transition
   (`sequence.applyVideoTransition`) only where time or place changes. Leave no gaps. If the briefing
   gives a duration, meet it within 5%.

3. Titles. Add the titles the briefing asks for as editable text (`graphics.newText`): opening title,
   names, closing card. Never burn text into an image.

4. Music and color. If a music file is in the bin, put it on its own audio track under the dialogue
   and lower it while people speak (`clip.audioGain`). Make the shots match: adjust exposure, white
   balance and contrast per clip with `lumetri.setSection`, or `lumetri.applyMatch` against the best
   shot of the scene. Adjustments only; do not render or export.

5. Check. Read `sequence_inspect` for gaps and overlaps. Look at `render_preview` of the sequence at
   every cut and at each title. Fix what you find.

Use `command_list` to confirm a command's parameters before the first time you use it. Don't save,
export or close the project: the user saves it when you finish. End with a brief note of the
structure you built, what you checked and what is still rough.

Briefing:
{briefing}"#;

/// Sent before the same recipe when a run continues on another account or after a limit.
const RESUME: &str = "This job was interrupted, and you don't have the earlier conversation. The active sequence already
holds part of the work: inspect it with `sequence_inspect`, `project_inspect` and `render_preview`,
work out which stages are done, and continue from there without redoing them.\n\n";

#[derive(Default)]
pub struct CreateAi {
    dialog: Option<Dialog>,
    run: Option<Run>,
    /// No request from Codex runs before this time (`FilmcraftApp::now`).
    hold_until: f64,
}

#[derive(Default)]
struct Dialog {
    /// The clips (and stills and sounds) to edit.
    media: Vec<String>,
    briefing: String,
    /// A finished video whose style and pacing to follow.
    reference: Option<String>,
    music: Option<String>,
    error: Option<String>,
    /// The model chosen here; `None` until the user picks one.
    model: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
enum Stage {
    /// Getting the speech model before Codex starts.
    Downloading,
    /// The project is ready; Codex isn't signed in or connected yet.
    Waiting,
    Working,
    /// The account ran out: waiting for another one (or another try).
    Paused,
    /// Stop was pressed; Codex is ending its turn.
    Stopping,
    Stopped,
    Done,
    Failed(String),
}

struct Run {
    stage: Stage,
    model: Option<String>,
    /// The model to go back to at the end, when the run changed it.
    restore_model: Option<String>,
    /// Sent once: later sends are resumes.
    sent: bool,
    started: f64,
    cursor: Cursor,
    /// Speech to text is there: the recipe with transcription and captions.
    speech: bool,
    /// The briefing as Codex gets it.
    briefing: String,
    clips: usize,
    /// The progress bar has been drawn (the speech model downloads only once it says so).
    drawn: bool,
}

impl Run {
    fn new(briefing: String, clips: usize, speech: bool, model: Option<String>, started: f64) -> Self {
        Run { stage: Stage::Waiting, model, restore_model: None, sent: false, started, cursor: Cursor::default(), speech, briefing, clips, drawn: false }
    }
}

/// A place on the Timeline: a time, on a track when the command names one.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Spot {
    time: Tick,
    track: Option<TrackId>,
}

/// The cursor that follows the edits.
#[derive(Default)]
struct Cursor {
    /// Where it is drawn now, on the screen.
    at: Option<Pos2>,
    target: Option<Spot>,
    label: &'static str,
    /// When it last landed, for the ring.
    landed: f64,
}

impl CreateAi {
    /// The editor is locked: a run is going.
    pub(crate) fn locked(&self) -> bool {
        self.run.as_ref().is_some_and(|r| matches!(r.stage, Stage::Downloading | Stage::Waiting | Stage::Working | Stage::Paused | Stage::Stopping))
    }

    pub(crate) fn dialog_open(&self) -> bool {
        self.dialog.is_some()
    }
}

/// File › Create with AI.
pub(crate) fn open(app: &mut FilmcraftApp) {
    if app.create_ai.locked() {
        return;
    }
    app.create_ai.dialog = Some(Dialog::default());
    // Start now, so the models are there by the time the user looks for them.
    codex_panel::ensure_started(app);
}

fn has_ext(path: &str, exts: &[&str]) -> bool {
    std::path::Path::new(path).extension().and_then(|e| e.to_str()).is_some_and(|e| exts.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

fn is_media(path: &str) -> bool {
    has_ext(path, &VIDEO_EXTS) || has_ext(path, &AUDIO_EXTS) || has_ext(path, &IMAGE_EXTS)
}

fn media_exts() -> Vec<String> {
    VIDEO_EXTS.iter().chain(&AUDIO_EXTS).chain(&IMAGE_EXTS).map(|e| e.to_string()).collect()
}

/// The media files in `paths`: a folder gives the media directly inside it, by name.
fn expand(paths: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for path in paths {
        let p = std::path::Path::new(&path);
        if p.is_dir() {
            let mut inside: Vec<String> = std::fs::read_dir(p)
                .map(|dir| dir.filter_map(Result::ok).map(|e| e.path()).filter(|f| f.is_file()).map(|f| f.to_string_lossy().into_owned()).filter(|f| is_media(f)).collect())
                .unwrap_or_default();
            inside.sort();
            out.extend(inside);
        } else if is_media(&path) {
            out.push(path);
        }
    }
    out.truncate(MAX_MEDIA);
    out
}

/// Add `paths` (files or folders) to the dialog's clips, each once.
fn add_media(app: &mut FilmcraftApp, paths: Vec<String>) {
    let found = expand(paths);
    let Some(d) = app.create_ai.dialog.as_mut() else { return };
    if found.is_empty() {
        d.error = Some(tl!("No video, audio or image files there.").into());
        return;
    }
    for path in found {
        if d.media.len() < MAX_MEDIA && !d.media.contains(&path) {
            d.media.push(path);
        }
    }
    d.error = None;
}

/// Dropped files: the dialog's clips while it is open; nothing at all while the editor is locked.
pub(crate) fn take_drop(app: &mut FilmcraftApp, files: &[egui::DroppedFileHandle]) -> bool {
    if app.create_ai.locked() {
        return true;
    }
    if !app.create_ai.dialog_open() {
        return false;
    }
    add_media(app, files.iter().filter(|f| f.path().is_absolute()).map(|f| f.path().to_string_lossy().into_owned()).collect());
    true
}

/// Keep the keyboard from the editor's shortcuts while the dialog or a run has the app. A text field
/// (the briefing, the chat) keeps its keys; Esc closes the dialog.
pub(crate) fn filter_keys(app: &mut FilmcraftApp, ctx: &egui::Context) {
    let dialog = app.create_ai.dialog_open();
    if !dialog && !app.create_ai.locked() {
        return;
    }
    let typing = ctx.egui_wants_keyboard_input();
    if typing {
        return;
    }
    if dialog && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.create_ai.dialog = None;
    }
    ctx.input_mut(|i| i.events.retain(|e| !matches!(e, egui::Event::Key { .. })));
}

/// How long to hold Codex's next request; `None` when it can run now.
pub(crate) fn hold(app: &FilmcraftApp, now: f64) -> Option<f64> {
    (app.create_ai.hold_until > now).then_some(app.create_ai.hold_until - now)
}

/// What the user sees for an edit; `None` for commands that only read.
fn edit_label(command: &str) -> Option<&'static str> {
    let area = command.split('.').next().unwrap_or(command);
    Some(match (area, command) {
        (_, "file.import" | "mediaBrowser.import") => tl!("Importing"),
        (_, "timeline.place" | "source.insert" | "source.overwrite") => tl!("Placing a clip"),
        (_, "timeline.razor") => tl!("Cutting"),
        (_, "transcript.createCaptions") | ("captions", _) => tl!("Captions"),
        (_, c) if c.contains("Transition") => tl!("Transition"),
        ("graphics", _) => tl!("Writing the title"),
        ("lumetri" | "color", _) => tl!("Adjusting the color"),
        (_, "clip.audioGain") | ("audio" | "essentialSound", _) => tl!("Adjusting the audio"),
        ("timeline", _) => tl!("Adjusting"),
        _ => return None,
    })
}

/// A track as commands name it: its id, or "V2" / "A1".
fn track_in(seq: &Sequence, v: Option<&Value>) -> Option<TrackId> {
    let v = v?;
    if let Some(id) = v.as_u64() {
        return seq.track(TrackId(id)).map(|t| t.id);
    }
    let mut chars = v.as_str()?.chars();
    let tracks = match chars.next()? {
        'v' | 'V' => &seq.video_tracks,
        'a' | 'A' => &seq.audio_tracks,
        _ => return None,
    };
    let i: usize = chars.as_str().parse().ok()?;
    tracks.get(i.checked_sub(1)?).map(|t| t.id)
}

/// Where on the Timeline a command's own parameters point, if they say: the clip it names, else
/// its time (`time` ticks, `frame` or `seconds`) and `track`.
fn spot_in(app: &FilmcraftApp, params: &Value) -> Option<Spot> {
    let seq = app.session.active_sequence()?;
    let clip = params.get("clip").and_then(Value::as_u64).or_else(|| params.get("clips").and_then(Value::as_array).and_then(|a| a.first()).and_then(Value::as_u64));
    if let Some((track, it)) = clip.and_then(|c| seq.find_item(ClipId(c))) {
        let time = if params.get("edge").and_then(Value::as_str) == Some("out") { it.end() } else { it.start };
        return Some(Spot { time, track: Some(track) });
    }
    let time = if let Some(t) = params.get("time").and_then(Value::as_i64) {
        Some(Tick(t))
    } else if let Some(f) = params.get("frame").and_then(Value::as_i64) {
        Some(app.session.sequence_rate().tick_of(f))
    } else {
        params.get("seconds").and_then(Value::as_f64).filter(|s| s.is_finite()).map(Tick::from_seconds_f64)
    };
    let track = track_in(seq, params.get("track"));
    if time.is_none() && track.is_none() {
        return None;
    }
    Some(Spot { time: time.unwrap_or_else(|| app.session.playhead()), track })
}

/// A Timeline place on the screen, from the Timeline's last layout; `None` while it isn't drawn.
fn spot_pos(app: &FilmcraftApp, spot: Spot) -> Option<Pos2> {
    let layout = app.tl.layout.as_ref()?;
    let c = layout.content;
    let x = layout.x_of(spot.time);
    let x = if x.is_finite() { x.max(c.left()).min(c.right().max(c.left())) } else { c.left() };
    let row = spot.track.and_then(|t| layout.rows.iter().find(|r| r.track == t)).map(|r| layout.shown(r)).filter(|r| r.height() > 0.0);
    Some(pos2(x, row.map_or(c.center().y, |r| r.center().y)))
}

/// A control request is about to run: during a run, pace the next one and send the cursor to where
/// this edit lands on the Timeline (the playhead when its parameters don't say).
pub(crate) fn observe(app: &mut FilmcraftApp, ctx: &egui::Context, method: &str, params: &Value) {
    if method != "engine.execute" || !matches!(app.create_ai.run.as_ref().map(|r| &r.stage), Some(Stage::Working)) {
        return;
    }
    let command = params.get("command").or_else(|| params.get("id")).and_then(Value::as_str);
    let Some(label) = command.and_then(edit_label) else { return };
    let args = params.get("params").cloned().unwrap_or(Value::Null);
    let spot = spot_in(app, &args).unwrap_or(Spot { time: app.session.playhead(), track: None });
    let now = app.now();
    app.create_ai.hold_until = now + PACE;
    if let Some(run) = app.create_ai.run.as_mut() {
        run.cursor.label = label;
        run.cursor.target = Some(spot);
        run.cursor.landed = now;
    }
    ctx.request_repaint();
}

/// The dialog's Create button: makes the project and starts the run.
fn start(app: &mut FilmcraftApp, ctx: &egui::Context) {
    let Some(d) = app.create_ai.dialog.as_mut() else { return };
    if d.media.is_empty() {
        d.error = Some(tl!("Add the clips to edit first.").into());
        return;
    }
    if d.briefing.trim().is_empty() {
        d.error = Some(tl!("Write the briefing: what the video should be.").into());
        return;
    }
    let (media, reference, music, model) = (d.media.clone(), d.reference.clone(), d.music.clone(), d.model.clone());
    let briefing = with_extras(d.briefing.trim(), reference.as_deref(), music.as_deref());
    if app.studio.classic {
        // The run shows in the Studio shell's Codex column.
        crate::studio::toggle_classic(app);
    }
    codex_panel::ensure_started(app);
    let mut all = media.clone();
    all.extend(reference.into_iter().chain(music));
    let made = if app.codex.is_running() { new_project(app, ctx, &all) } else { Err(stopped_reason(app)) };
    if let Err(e) = made {
        if let Some(d) = app.create_ai.dialog.as_mut() {
            d.error = Some(e);
        }
        return;
    }
    app.create_ai.dialog = None;
    app.codex.open = true;
    app.studio.bottom = crate::studio::bottom::Tab::Timeline;
    codex_panel::new_chat(app);
    let model = model.or_else(|| app.codex.has_model(DEFAULT_MODEL).then(|| DEFAULT_MODEL.to_string()));
    let (speech, download) = speech_plan(app);
    let mut run = Run::new(briefing, media.len(), speech, model, app.now());
    if download {
        run.stage = Stage::Downloading;
    }
    app.create_ai.run = Some(run);
}

/// The briefing with the optional reference and music named, as Codex gets it.
fn with_extras(briefing: &str, reference: Option<&str>, music: Option<&str>) -> String {
    let name = |p: &str| std::path::Path::new(p).file_name().map_or_else(|| p.to_string(), |n| n.to_string_lossy().into_owned());
    let mut text = briefing.to_string();
    if let Some(r) = reference {
        text = format!("{text}\n\nReference video (in the bin; follow its style and pacing, don't use it in the cut): {}", name(r));
    }
    if let Some(m) = music {
        text = format!("{text}\n\nMusic (in the bin): {}", name(m));
    }
    text
}

/// A new project with `paths` in its bin and the empty sequence "Assembly", in the format of the
/// first clip with a picture.
fn new_project(app: &mut FilmcraftApp, ctx: &egui::Context, paths: &[String]) -> Result<(), String> {
    crate::studio::new_doc::guard_unsaved(app, ctx)?;
    let name = paths
        .first()
        .and_then(|p| std::path::Path::new(p).parent())
        .and_then(|d| d.file_name())
        .map_or_else(|| tl!("Untitled").to_string(), |n| n.to_string_lossy().into_owned());
    crate::menus::invoke(app, ctx, "file.newProject", json!({"name": name}))?;
    let v = crate::menus::invoke(app, ctx, "file.import", json!({"paths": paths}))?;
    let items: Vec<u64> = v.get("items").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_u64).collect()).unwrap_or_default();
    if items.is_empty() {
        let errors = v.get("errors").and_then(Value::as_array).map(|e| e.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("; ")).unwrap_or_default();
        return Err(if errors.is_empty() { tl!("No video, audio or image files there.").into() } else { errors });
    }
    let format = items.iter().filter_map(|i| app.session.project.item(ItemId(*i))).filter_map(|it| it.as_media()).find_map(|m| m.info.video.as_ref().map(|v| (v.width, v.height, v.frame_rate.sane().as_f64())));
    let mut params = json!({"name": tl!("Assembly")});
    if let (Some((w, h, fps)), Some(o)) = (format, params.as_object_mut()) {
        if w > 0 && h > 0 {
            o.insert("width".into(), json!(w));
            o.insert("height".into(), json!(h));
        }
        if fps.is_finite() && fps > 0.0 {
            o.insert("fps".into(), json!(fps));
        }
    }
    crate::menus::invoke(app, ctx, "file.newSequence", params)?;
    app.ui.mode = crate::state::Mode::Edit;
    Ok(())
}

/// Whether speech to text can run (`.0`) and whether its model has to be downloaded first (`.1`).
fn speech_plan(app: &mut FilmcraftApp) -> (bool, bool) {
    let Ok(v) = app.session.execute("transcript.models", json!({})) else { return (false, false) };
    if v.get("available").and_then(Value::as_bool) != Some(true) {
        return (false, false);
    }
    let default = v.get("default").and_then(Value::as_str).unwrap_or_default();
    let installed = v
        .get("models")
        .and_then(Value::as_array)
        .is_some_and(|m| m.iter().any(|m| m.get("id").and_then(Value::as_str) == Some(default) && m.get("installed").and_then(Value::as_bool) == Some(true)));
    (installed, !installed)
}

fn steps(speech: bool) -> &'static [&'static str] {
    if speech { &STEPS } else { &STEPS_NO_SPEECH }
}

/// The recipe in the UI's language, with the briefing.
fn recipe(run: &Run, resume: bool) -> String {
    let language = crate::i18n::current().bcp47();
    let base = if run.speech { RECIPE } else { RECIPE_NO_SPEECH };
    // the briefing last: what the user wrote is never read as a placeholder
    let text = base.replace("{language}", language).replace("{sequence}", tl!("Assembly")).replace("{briefing}", &run.briefing);
    if resume { format!("{RESUME}{text}") } else { text }
}

/// Send the recipe, on the run's model.
fn send(app: &mut FilmcraftApp) {
    let Some(run) = app.create_ai.run.as_ref() else { return };
    let (text, wanted, clips) = (recipe(run, run.sent), run.model.clone(), run.clips);
    let previous = app.codex.switch_model(wanted);
    codex_panel::send_as(app, tlf!("Assembling a first cut from {n} clips", n = clips), text, Vec::new());
    if let Some(run) = app.create_ai.run.as_mut() {
        run.sent = true;
        run.stage = Stage::Working;
        if run.restore_model.is_none() {
            run.restore_model = previous;
        }
    }
}

/// Move the run along what Codex reports. Every frame.
pub(crate) fn tick(app: &mut FilmcraftApp) {
    let Some((stage, drawn)) = app.create_ai.run.as_ref().map(|r| (r.stage.clone(), r.drawn)) else { return };
    if app.create_ai.locked() {
        // the edits are on the Timeline: keep it in sight
        app.studio.bottom = crate::studio::bottom::Tab::Timeline;
    }
    let (running, signed_in, busy, limit) = (app.codex.is_running(), app.codex.is_signed_in(), app.codex.is_busy(), app.codex.limit_reached());
    let next = match stage {
        Stage::Downloading if drawn => {
            // shortcut: the download runs on the UI thread and the window waits for it (the bar
            // says so first); run it as an engine job once `transcript.downloadModel` has one
            let ok = app.session.execute("transcript.downloadModel", json!({})).is_ok();
            if let Some(run) = app.create_ai.run.as_mut() {
                run.speech = ok;
            }
            Some(Stage::Waiting)
        }
        Stage::Waiting | Stage::Paused if !running => Some(Stage::Failed(stopped_reason(app))),
        Stage::Waiting if signed_in => {
            send(app);
            None
        }
        // Another account (the switch clears the limit), signed in: carry on from the sequence.
        Stage::Paused if signed_in && !limit => {
            send(app);
            None
        }
        Stage::Working if !running => Some(Stage::Failed(stopped_reason(app))),
        Stage::Working if !busy && limit => Some(Stage::Paused),
        Stage::Working if !busy => Some(match codex_panel::last_turn_error(app) {
            Some(e) => Stage::Failed(e),
            None => Stage::Done,
        }),
        Stage::Stopping if !busy || !running => Some(Stage::Stopped),
        _ => None,
    };
    if let Some(next) = next {
        let ended = !matches!(next, Stage::Paused | Stage::Waiting);
        if let Some(run) = app.create_ai.run.as_mut() {
            run.stage = next;
        }
        if ended {
            restore_model(app);
        }
    }
}

fn stopped_reason(app: &FilmcraftApp) -> String {
    codex_panel::last_error(app).unwrap_or_else(|| tl!("Codex stopped.").to_string())
}

fn restore_model(app: &mut FilmcraftApp) {
    if let Some(model) = app.create_ai.run.as_mut().and_then(|r| r.restore_model.take()) {
        app.codex.switch_model(Some(model));
    }
}

/// The last stage Codex announced (an index into [`steps`]), from its messages.
fn stage_of(app: &FilmcraftApp, total: usize) -> Option<usize> {
    app.codex.assistant_texts().rev().find_map(codex_panel::stage_tag).filter(|n| *n <= total).map(|n| n - 1)
}

/// The dialog and, during a run, the lock, the progress bar and the cursor. Every frame.
pub(crate) fn show(app: &mut FilmcraftApp, ctx: &egui::Context) {
    tick(app);
    if app.create_ai.dialog_open() {
        dialog(app, ctx);
    }
    if app.create_ai.run.is_some() {
        if app.create_ai.locked() {
            lock(app, ctx);
            cursor(app, ctx);
        }
        progress(app, ctx);
    }
}

// ---- the dialog ----------------------------------------------------------------------------------

/// What the dialog's controls asked for, applied after drawing.
enum Ask {
    PickFiles,
    PickFolder,
    ClearMedia,
    PickReference,
    ClearReference,
    PickMusic,
    ClearMusic,
    Create,
    Close,
    Model(String),
}

fn file_name(path: &str) -> String {
    std::path::Path::new(path).file_name().map_or_else(|| path.to_string(), |n| n.to_string_lossy().into_owned())
}

fn dialog(app: &mut FilmcraftApp, ctx: &egui::Context) {
    let Some(mut d) = app.create_ai.dialog.take() else { return };
    let t = app.tokens;
    let mut asks = Vec::new();
    let mut auto: Vec<(&str, Rect, &str)> = Vec::new();
    let codex = &app.codex;
    let frame = egui::Frame::new().fill(t.card).stroke(Stroke::new(1.0, t.card_border)).corner_radius(14.0).inner_margin(22.0);
    egui::Modal::new(egui::Id::unique("create-ai-dialog")).frame(frame).show(ctx, |ui| {
        ui.set_width((ctx.content_rect().width() - 80.0).clamp(280.0, 540.0));
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
            crate::icons::paint(ui.painter(), r, Icon::Sparkle, t.accent);
            ui.label(RichText::new(tl!("Create with AI")).font(Tokens::semibold(20.0)).color(t.text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let close = crate::icons::button(ui, Icon::Close, 20.0, false, &t, tl!("Close"));
                let close = crate::widgets::accessible_name(close, tl!("Close"));
                auto.push(("createAi.close", close.rect, tl!("Close")));
                if close.clicked() {
                    asks.push(Ask::Close);
                }
            });
        });
        ui.add_space(4.0);
        let intro = tl!("Give Codex your clips and a briefing, and it assembles a first cut you can keep editing: cuts, titles, captions, music and color.");
        ui.add(egui::Label::new(RichText::new(intro).size(13.5).color(t.text_dim)).wrap());
        ui.add_space(12.0);
        let area = drop_area(ui, &t, &d.media);
        auto.push(("createAi.media", area.rect, tl!("Choose clips")));
        if area.clicked() {
            asks.push(Ask::PickFiles);
        }
        ui.horizontal(|ui| {
            let folder = ui.button(tl!("Choose a folder"));
            auto.push(("createAi.folder", folder.rect, tl!("Choose a folder")));
            if folder.clicked() {
                asks.push(Ask::PickFolder);
            }
            if !d.media.is_empty() && ui.button(tl!("Remove")).clicked() {
                asks.push(Ask::ClearMedia);
            }
        });
        ui.add_space(10.0);
        ui.label(RichText::new(tl!("Briefing")).strong().color(t.text));
        let hint = tl!("What the video is about, who it is for, the order of the scenes, the length, the titles and names to show…");
        let edit = ui.add(egui::TextEdit::multiline(&mut d.briefing).desired_rows(4).desired_width(f32::INFINITY).hint_text(hint));
        auto.push(("createAi.briefing", edit.rect, tl!("Briefing")));
        ui.add_space(8.0);
        let (pick, clear, r) = optional_file(ui, &t, tl!("Reference video (optional)"), d.reference.as_deref());
        auto.push(("createAi.reference", r, tl!("Reference video (optional)")));
        if pick {
            asks.push(Ask::PickReference);
        }
        if clear {
            asks.push(Ask::ClearReference);
        }
        let (pick, clear, r) = optional_file(ui, &t, tl!("Music (optional)"), d.music.as_deref());
        auto.push(("createAi.music", r, tl!("Music (optional)")));
        if pick {
            asks.push(Ask::PickMusic);
        }
        if clear {
            asks.push(Ask::ClearMusic);
        }
        if codex.has_models() {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(tl!("Model")).size(13.0).color(t.text_dim));
                let chosen = d.model.clone().or_else(|| codex.has_model(DEFAULT_MODEL).then(|| DEFAULT_MODEL.to_string()));
                if let Some(id) = codex_panel::model_picker(ui, &t, codex, chosen.as_deref()) {
                    asks.push(Ask::Model(id));
                }
            });
        }
        ui.add_space(10.0);
        let warning = tl!("This can take over an hour and use a lot of your plan. The editor stays locked until it finishes, and you can stop whenever you want.");
        ui.add(egui::Label::new(RichText::new(warning).size(12.5).color(t.text_dim)).wrap());
        if let Some(error) = &d.error {
            ui.add_space(6.0);
            ui.add(egui::Label::new(RichText::new(error).size(13.0).color(t.danger)).wrap());
        }
        ui.add_space(14.0);
        let ready = !d.media.is_empty() && !d.briefing.trim().is_empty();
        let create = create_button(ui, &t, ready);
        auto.push(("createAi.create", create.rect, tl!("Create with AI")));
        if create.clicked() {
            asks.push(Ask::Create);
        }
    });
    app.create_ai.dialog = Some(d);
    for (id, rect, label) in auto {
        app.auto.add(id, rect, label);
    }
    for ask in asks {
        let d = app.create_ai.dialog.as_mut();
        match ask {
            Ask::PickFiles => app.pick_ui(FileDialog::OpenFiles { exts: media_exts() }, add_media),
            Ask::PickFolder => app.pick_ui(FileDialog::Folder { at: None }, add_media),
            Ask::ClearMedia => {
                if let Some(d) = d {
                    d.media.clear();
                }
            }
            Ask::PickReference => app.pick_ui(FileDialog::OpenFiles { exts: VIDEO_EXTS.iter().map(|e| e.to_string()).collect() }, |app, paths| {
                if let (Some(d), Some(path)) = (app.create_ai.dialog.as_mut(), paths.into_iter().next()) {
                    d.reference = Some(path);
                }
            }),
            Ask::ClearReference => {
                if let Some(d) = d {
                    d.reference = None;
                }
            }
            Ask::PickMusic => app.pick_ui(FileDialog::OpenFiles { exts: AUDIO_EXTS.iter().map(|e| e.to_string()).collect() }, |app, paths| {
                if let (Some(d), Some(path)) = (app.create_ai.dialog.as_mut(), paths.into_iter().next()) {
                    d.music = Some(path);
                }
            }),
            Ask::ClearMusic => {
                if let Some(d) = d {
                    d.music = None;
                }
            }
            Ask::Create => start(app, ctx),
            Ask::Close => app.create_ai.dialog = None,
            Ask::Model(id) => {
                if let Some(d) = d {
                    d.model = Some(id);
                }
            }
        }
    }
}

/// The drop area: what to do, or the clips taken so far.
fn drop_area(ui: &mut Ui, t: &Tokens, media: &[String]) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 110.0), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tl!("Choose clips")));
    let hov = resp.hovered() || ui.input(|i| !i.raw.hovered_files.is_empty());
    let p = ui.painter();
    p.rect(r, 12.0, if hov { t.hover } else { t.field_bg }, Stroke::new(1.0, if hov { t.accent } else { t.field_border }), StrokeKind::Inside);
    let c = r.center();
    if media.is_empty() {
        crate::icons::paint(p, Rect::from_center_size(c - vec2(0.0, 26.0), vec2(22.0, 22.0)), Icon::Folder, if hov { t.accent } else { t.text_dim });
        p.text(c + vec2(0.0, 4.0), Align2::CENTER_CENTER, tl!("Drag your clips or a folder here"), Tokens::ui(14.0), t.text);
        p.text(c + vec2(0.0, 26.0), Align2::CENTER_CENTER, tl!("or click to choose files"), Tokens::ui(13.0), t.accent);
    } else {
        let count = if media.len() == 1 { tl!("1 clip").to_string() } else { tlf!("{n} clips", n = media.len()) };
        p.text(c - vec2(0.0, 12.0), Align2::CENTER_CENTER, count, Tokens::semibold(15.0), t.text);
        let names = media.iter().take(3).map(|m| file_name(m)).collect::<Vec<_>>().join(", ");
        let more = if media.len() > 3 { "…" } else { "" };
        let line = p.layout(format!("{names}{more}"), Tokens::ui(12.5), t.text_dim, r.width() - 24.0);
        p.galley(pos2(c.x - line.size().x / 2.0, c.y + 4.0), line, t.text_dim);
    }
    resp
}

/// "Label … [Choose… | file name] [Remove]": returns (choose clicked, remove clicked, choose's rect).
fn optional_file(ui: &mut Ui, t: &Tokens, label: &str, path: Option<&str>) -> (bool, bool, Rect) {
    let mut out = (false, false, Rect::NOTHING);
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(13.0).color(t.text_dim));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if path.is_some() && ui.button(tl!("Remove")).clicked() {
                out.1 = true;
            }
            let choose = ui.add(egui::Button::new(path.map_or_else(|| tl!("Choose…").to_string(), file_name)).truncate());
            out.0 = choose.clicked();
            out.2 = choose.rect;
        });
    });
    out
}

/// The big amber button with the wand.
fn create_button(ui: &mut Ui, t: &Tokens, enabled: bool) -> egui::Response {
    let label = tl!("Create with AI");
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 50.0), if enabled { Sense::click() } else { Sense::hover() });
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, label));
    let fill = match (enabled, resp.hovered()) {
        (false, _) => t.field_border,
        (true, true) => t.accent_hover,
        (true, false) => t.accent,
    };
    let ink = if enabled { t.primary_text } else { t.text_faint };
    let p = ui.painter();
    p.rect_filled(r, 12.0, fill);
    let g = p.layout_no_wrap(label.to_string(), Tokens::semibold(16.0), ink);
    let x = r.center().x - (g.size().x + 30.0) / 2.0;
    crate::icons::paint(p, Rect::from_center_size(pos2(x + 10.0, r.center().y), vec2(20.0, 20.0)), Icon::Sparkle, ink);
    p.galley(pos2(x + 30.0, r.center().y - g.size().y / 2.0), g, ink);
    if enabled { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp }
}

// ---- the run -------------------------------------------------------------------------------------

/// Take the pointer over everything but the Codex column (the progress bar sits above this).
fn lock(app: &FilmcraftApp, ctx: &egui::Context) {
    let s = ctx.content_rect();
    let rects = match app.codex.dock() {
        Some(d) => vec![
            Rect::from_min_max(s.min, pos2(d.left(), s.bottom())),
            Rect::from_min_max(pos2(d.right(), s.top()), s.max),
            Rect::from_min_max(pos2(d.left(), s.top()), pos2(d.right(), d.top())),
            Rect::from_min_max(pos2(d.left(), d.bottom()), pos2(d.right(), s.bottom())),
        ],
        None => vec![s],
    };
    for (i, rect) in rects.into_iter().enumerate() {
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            continue;
        }
        egui::Area::new(egui::Id::unique(("create-ai-lock", i))).order(egui::Order::Foreground).fixed_pos(rect.min).constrain(false).show(ctx, |ui| {
            ui.allocate_rect(rect, Sense::click_and_drag());
        });
    }
}

/// The pointer over the Timeline, with what it is doing; drawn over everything, takes nothing.
fn cursor(app: &mut FilmcraftApp, ctx: &egui::Context) {
    let Some(spot) = app.create_ai.run.as_ref().and_then(|r| r.cursor.target) else { return };
    let Some(goal) = spot_pos(app, spot) else { return };
    let (now, t) = (app.now(), app.tokens);
    let Some(run) = app.create_ai.run.as_mut() else { return };
    let c = &mut run.cursor;
    let dt = ctx.input(|i| i.stable_dt).min(0.1);
    let k = 1.0 - (-dt * 9.0).exp();
    let at = c.at.unwrap_or(goal);
    let pos = at + (goal - at) * k;
    c.at = Some(pos);
    let ring = ((now - c.landed) / 0.45) as f32;
    // Only while something moves: a run can last an hour, on any computer.
    if pos.distance(goal) > 0.5 || ring < 1.0 {
        ctx.request_repaint();
    }
    let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("create-ai-cursor")));
    if ring < 1.0 && pos.distance(goal) < 12.0 {
        p.circle_stroke(goal, 6.0 + ring * 22.0, Stroke::new(1.5, t.accent.gamma_multiply(1.0 - ring)));
    }
    paint_pointer(&p, pos, t.accent, t.primary_text);
    let text = p.layout_no_wrap(c.label.to_string(), Tokens::ui(12.0), t.primary_text);
    let chip = Rect::from_min_size(pos + vec2(16.0, 18.0), text.size() + vec2(14.0, 8.0));
    p.rect_filled(chip, 7.0, t.accent);
    p.galley(chip.min + vec2(7.0, 4.0), text, t.primary_text);
}

/// An arrow pointer with its tip at `tip`.
fn paint_pointer(p: &egui::Painter, tip: Pos2, fill: Color32, line: Color32) {
    let pts = [(0.0, 0.0), (0.0, 17.0), (4.5, 13.0), (8.0, 20.0), (11.0, 18.5), (7.5, 12.0), (13.0, 12.0)].map(|(x, y)| tip + vec2(x, y)).to_vec();
    p.add(egui::Shape::convex_polygon(pts[..3].to_vec(), fill, Stroke::NONE));
    p.add(egui::Shape::convex_polygon(vec![pts[0], pts[2], pts[5], pts[6]], fill, Stroke::NONE));
    p.add(egui::Shape::convex_polygon(vec![pts[2], pts[3], pts[4], pts[5]], fill, Stroke::NONE));
    let mut outline = pts;
    outline.push(tip);
    p.add(egui::Shape::line(outline, Stroke::new(1.0, line)));
}

/// What the bar's buttons asked for, applied after drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Bar {
    Stop,
    Retry,
    Save,
    Discard,
    Dismiss,
}

/// The bar at the foot of the editor: the stage and its way out.
fn progress(app: &mut FilmcraftApp, ctx: &egui::Context) {
    let t = app.tokens;
    let Some(run) = app.create_ai.run.as_ref() else { return };
    let (stage, names, started) = (run.stage.clone(), steps(run.speech), run.started);
    let minutes = ((app.now() - started) / 60.0).max(0.0) as u32;
    let step = stage_of(app, names.len());
    let s = ctx.content_rect();
    let room = match app.codex.dock() {
        Some(d) if d.left() > s.left() + 200.0 => Rect::from_min_max(s.min, pos2(d.left(), s.bottom())),
        _ => s,
    };
    let mut clicked = None;
    egui::Area::new(egui::Id::unique("create-ai-progress")).order(egui::Order::Tooltip).pivot(Align2::CENTER_BOTTOM).fixed_pos(pos2(room.center().x, room.bottom() - 48.0)).show(ctx, |ui| {
        egui::Frame::new().fill(t.card).stroke(Stroke::new(1.0, t.card_border)).corner_radius(12.0).inner_margin(egui::Margin::symmetric(14, 9)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                bar_content(app, ui, &t, &stage, (step, names), minutes, &mut clicked);
            });
        });
    });
    if let Some(run) = app.create_ai.run.as_mut() {
        run.drawn = true;
    }
    if stage == Stage::Downloading {
        // the next frame downloads, under this bar
        ctx.request_repaint();
    } else if app.create_ai.locked() {
        // The elapsed minutes stay right without keeping frames coming.
        ctx.request_repaint_after(std::time::Duration::from_secs(30));
    }
    if let Some(bar) = clicked {
        act(app, ctx, bar);
    }
}

fn act(app: &mut FilmcraftApp, ctx: &egui::Context, bar: Bar) {
    match bar {
        Bar::Stop => {
            if let Some(run) = app.create_ai.run.as_mut() {
                run.stage = Stage::Stopping;
            }
            app.codex.stop_turn();
        }
        Bar::Retry => {
            app.codex.clear_limit();
            send(app);
        }
        Bar::Save => {
            if let Err(e) = crate::menus::invoke(app, ctx, "file.saveAs", json!({})) {
                app.ui.status = e;
            }
            app.create_ai.run = None;
        }
        Bar::Discard => {
            if let Err(e) = app.session.execute("file.closeProject", json!({"force": true})) {
                app.ui.status = e.to_string();
            }
            app.create_ai.run = None;
        }
        Bar::Dismiss => app.create_ai.run = None,
    }
}

fn bar_content(app: &mut FilmcraftApp, ui: &mut Ui, t: &Tokens, stage: &Stage, (step, names): (Option<usize>, &[&str]), minutes: u32, clicked: &mut Option<Bar>) {
    let mut button = |app: &mut FilmcraftApp, ui: &mut Ui, id: &str, label: &str, primary: bool, then: Bar| {
        if crate::panels::file_dialogs::button(app, ui, id, label, primary) {
            *clicked = Some(then);
        }
    };
    let text = |ui: &mut Ui, s: &str| {
        ui.add(egui::Label::new(RichText::new(s).font(Tokens::ui(13.0)).color(t.text)).wrap_mode(egui::TextWrapMode::Extend));
    };
    match stage {
        Stage::Downloading => {
            ui.add(egui::Spinner::new().size(16.0).color(t.accent));
            text(ui, tl!("Downloading the speech model"));
        }
        Stage::Waiting => {
            ui.add(egui::Spinner::new().size(16.0).color(t.accent));
            text(ui, tl!("Waiting for Codex: sign in on the panel if it asks"));
            button(app, ui, "createAi.stop", tl!("Stop"), false, Bar::Stop);
        }
        Stage::Working => {
            ui.add(egui::Spinner::new().size(16.0).color(t.accent));
            match step.and_then(|s| names.get(s).map(|name| (s, name))) {
                Some((s, name)) => {
                    let at = tlf!("Step {n} of {total}", n = s + 1, total = names.len());
                    ui.label(RichText::new(at).font(Tokens::ui(12.5)).color(t.text_dim));
                    text(ui, crate::i18n::t(name));
                }
                None => text(ui, tl!("Getting started")),
            }
            if minutes > 0 {
                ui.label(RichText::new(tlf!("{n} min", n = minutes)).font(Tokens::ui(12.5)).color(t.text_dim));
            }
            button(app, ui, "createAi.stop", tl!("Stop"), false, Bar::Stop);
        }
        Stage::Paused => {
            text(ui, tl!("Usage limit reached: switch accounts on the panel to continue"));
            button(app, ui, "createAi.retry", tl!("Try again"), false, Bar::Retry);
            button(app, ui, "createAi.stop", tl!("Stop"), false, Bar::Stop);
        }
        Stage::Stopping => {
            ui.add(egui::Spinner::new().size(16.0).color(t.accent));
            text(ui, tl!("Stopping…"));
        }
        Stage::Stopped => {
            text(ui, tl!("Stopped. Keep what was done?"));
            button(app, ui, "createAi.keep", tl!("Keep"), false, Bar::Dismiss);
            button(app, ui, "createAi.discard", tl!("Discard"), false, Bar::Discard);
        }
        Stage::Done => {
            let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
            crate::icons::paint(ui.painter(), r, Icon::Sparkle, t.accent);
            text(ui, tl!("Your first cut is ready"));
            button(app, ui, "createAi.close", tl!("Close"), false, Bar::Dismiss);
            button(app, ui, "createAi.save", tl!("Save Project…"), true, Bar::Save);
        }
        Stage::Failed(e) => {
            ui.set_max_width(520.0);
            ui.add(egui::Label::new(RichText::new(e).size(13.0).color(t.danger)).wrap());
            button(app, ui, "createAi.close", tl!("Close"), false, Bar::Dismiss);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::{Receiver, Sender};

    use egui_kittest::kittest::Queryable;
    use filmcraft_engine::Session;

    use super::*;
    use codex_panel::{CodexEvent, CodexLink, CodexModel, CodexRequest};

    fn running_app(session: Session) -> (FilmcraftApp, Receiver<CodexRequest>, Sender<CodexEvent>) {
        let (tx, requests) = std::sync::mpsc::channel();
        let (events, rx) = std::sync::mpsc::channel();
        let mut app = FilmcraftApp::new(session);
        app.codex.connect(CodexLink { tx, rx });
        (app, requests, events)
    }

    fn demo() -> Session {
        let mut s = Session::default();
        s.execute("file.openDemoProject", json!({})).unwrap();
        s
    }

    fn png(dir: &std::path::Path) -> String {
        let path = dir.join("frame.png");
        std::fs::write(&path, crate::control::encode_png(&[200; 64 * 36 * 4], 64, 36).unwrap()).unwrap();
        path.to_string_lossy().into_owned()
    }

    /// Open the dialog with `path` and a briefing and start; the speech model is never downloaded here.
    fn begin(app: &mut FilmcraftApp, path: &str) {
        let ctx = egui::Context::default();
        open(app);
        add_media(app, vec![path.to_string()]);
        app.create_ai.dialog.as_mut().unwrap().briefing = "A one-minute story of the day".into();
        start(app, &ctx);
        assert!(app.create_ai.locked(), "{:?}", app.create_ai.dialog.as_ref().and_then(|d| d.error.clone()));
        let run = app.create_ai.run.as_mut().unwrap();
        if run.stage == Stage::Downloading {
            run.stage = Stage::Waiting;
        }
    }

    /// A run as `start` leaves it once Codex works, on an already open project.
    fn working(app: &mut FilmcraftApp, events: &Sender<CodexEvent>, speech: bool) {
        let mut run = Run::new("brief".into(), 3, speech, None, 0.0);
        run.stage = Stage::Working;
        app.create_ai.run = Some(run);
        events.send(CodexEvent::Account(Some("me@example.com".into()))).unwrap();
        events.send(CodexEvent::TurnStarted).unwrap();
        codex_panel::poll(app);
    }

    fn harness(app: FilmcraftApp) -> egui_kittest::Harness<'static, FilmcraftApp> {
        let mut h = egui_kittest::Harness::builder().with_size(vec2(1440.0, 900.0)).with_max_steps(10_000).build_eframe(move |_cc| app);
        for _ in 0..6 {
            h.step();
        }
        h
    }

    #[test]
    fn codex_create_ai_edits_have_labels_and_reads_do_not() {
        assert_eq!(edit_label("timeline.place"), Some("Placing a clip"));
        assert_eq!(edit_label("timeline.razor"), Some("Cutting"));
        assert_eq!(edit_label("timeline.trim"), Some("Adjusting"));
        assert_eq!(edit_label("sequence.applyVideoTransition"), Some("Transition"));
        assert_eq!(edit_label("graphics.newText"), Some("Writing the title"));
        assert_eq!(edit_label("transcript.createCaptions"), Some("Captions"));
        assert_eq!(edit_label("lumetri.setSection"), Some("Adjusting the color"));
        assert_eq!(edit_label("clip.audioGain"), Some("Adjusting the audio"));
        assert_eq!(edit_label("file.import"), Some("Importing"));
        assert!(edit_label("project.inspect").is_none());
        assert!(edit_label("command.list").is_none());
    }

    #[test]
    fn codex_create_ai_spots_come_from_the_command_parameters() {
        let app = FilmcraftApp::new(demo());
        let seq = app.session.active_sequence().unwrap();
        let v1 = seq.video_tracks[0].id;
        let rate = app.session.sequence_rate();
        assert_eq!(spot_in(&app, &json!({"time": 1000, "track": "V1"})), Some(Spot { time: Tick(1000), track: Some(v1) }));
        assert_eq!(spot_in(&app, &json!({"frame": 24})), Some(Spot { time: rate.tick_of(24), track: None }));
        assert_eq!(spot_in(&app, &json!({"seconds": 2.0, "track": v1.0})).map(|s| s.track), Some(Some(v1)));
        let clip = seq.video_tracks.iter().flat_map(|t| t.items.iter()).next().unwrap();
        assert_eq!(spot_in(&app, &json!({"clip": clip.id.0, "edge": "out"})).map(|s| s.time), Some(clip.end()));
        assert_eq!(spot_in(&app, &json!({"seconds": f64::NAN})), None, "nothing usable: the caller falls back to the playhead");
        assert_eq!(spot_in(&app, &json!({"track": "Q9"})), None);
        assert_eq!(spot_in(&app, &json!({})), None);
    }

    /// No clips or no briefing: the dialog says what is missing and nothing starts.
    #[test]
    fn codex_create_ai_dialog_needs_material_and_briefing() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, _requests, _events) = running_app(Session::default());
        let ctx = egui::Context::default();
        open(&mut app);
        start(&mut app, &ctx);
        assert!(app.create_ai.dialog.as_ref().unwrap().error.as_deref().is_some_and(|e| e.contains("clips")));
        add_media(&mut app, vec![dir.path().join("notes.txt").to_string_lossy().into_owned()]);
        assert!(app.create_ai.dialog.as_ref().unwrap().media.is_empty(), "only media is taken");
        add_media(&mut app, vec![png(dir.path())]);
        start(&mut app, &ctx);
        assert!(app.create_ai.dialog.as_ref().unwrap().error.as_deref().is_some_and(|e| e.contains("briefing")));
        assert!(app.create_ai.run.is_none() && app.session.project.items.is_empty());
        // A folder gives its media.
        app.create_ai.dialog = Some(Dialog::default());
        add_media(&mut app, vec![dir.path().to_string_lossy().into_owned()]);
        assert_eq!(app.create_ai.dialog.as_ref().unwrap().media.len(), 1);
    }

    /// The dialog draws from File › Create with AI, and Esc closes it.
    #[test]
    fn codex_create_ai_dialog_opens_and_escape_closes_it() {
        let mut h = harness(FilmcraftApp::new(Session::default()));
        let ctx = h.ctx.clone();
        crate::menus::invoke(h.state_mut(), &ctx, COMMAND, json!({})).unwrap();
        for _ in 0..3 {
            h.step();
        }
        assert!(h.state().create_ai.dialog_open());
        assert!(h.state().auto.find("createAi.create").is_some());
        assert!(h.query_all_by_label("Choose clips").next().is_some(), "the drop area");
        h.event(egui::Event::Key { key: egui::Key::Escape, physical_key: Some(egui::Key::Escape), pressed: true, repeat: false, modifiers: egui::Modifiers::NONE });
        for _ in 0..3 {
            h.step();
        }
        assert!(!h.state().create_ai.dialog_open());
    }

    /// While a run has the editor its shortcuts get no keys; the dialog gets only Esc, which closes it.
    #[test]
    fn codex_create_ai_keyboard_is_blocked() {
        let mut app = FilmcraftApp::new(Session::default());
        app.create_ai.dialog = Some(Dialog::default());
        let mut h = egui_kittest::Harness::builder().build_ui_state(
            |ui, (app, seen): &mut (FilmcraftApp, Vec<egui::Key>)| {
                filter_keys(app, ui.ctx());
                seen.extend(ui.input(|i| i.events.iter().filter_map(|e| if let egui::Event::Key { key, pressed: true, .. } = e { Some(*key) } else { None }).collect::<Vec<_>>()));
            },
            (app, Vec::new()),
        );
        let key = |key| egui::Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers: egui::Modifiers::NONE };
        h.event(key(egui::Key::Space));
        h.step();
        assert!(h.state().1.is_empty() && h.state().0.create_ai.dialog_open());
        h.event(key(egui::Key::Escape));
        h.step();
        assert!(h.state().1.is_empty() && !h.state().0.create_ai.dialog_open());
        let mut run = Run::new(String::new(), 1, false, None, 0.0);
        run.stage = Stage::Working;
        h.state_mut().0.create_ai.run = Some(run);
        h.event(key(egui::Key::Space));
        h.event(key(egui::Key::Escape));
        h.step();
        assert!(h.state().1.is_empty(), "{:?}", h.state().1);
        h.state_mut().0.create_ai.run = None;
        h.event(key(egui::Key::Space));
        h.step();
        assert_eq!(h.state().1, [egui::Key::Space], "no run, no lock");
    }

    /// Starting makes a project with the material in its bin and the empty sequence "Assembly";
    /// once signed in the recipe goes out on the recipe's model, and the old model comes back.
    #[test]
    fn codex_create_ai_start_makes_the_project_and_runs_the_recipe() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, requests, events) = running_app(Session::default());
        let model = |id: &str| CodexModel { id: id.into(), name: id.into(), description: String::new() };
        events.send(CodexEvent::Models { models: vec![model(DEFAULT_MODEL), model("cheap")], current: Some("cheap".into()) }).unwrap();
        codex_panel::poll(&mut app);
        let path = png(dir.path());
        begin(&mut app, &path);
        let s = &app.session;
        assert!(s.project.items.values().any(|i| i.name == "frame.png"), "the clip is in the bin");
        let seq_item = s.state.active_sequence.and_then(|id| s.project.item(id)).expect("a sequence");
        assert_eq!(seq_item.name, "Assembly");
        let seq = s.active_sequence().unwrap();
        assert!(seq.video_tracks.iter().chain(&seq.audio_tracks).all(|t| t.items.is_empty()), "empty");
        assert!(app.codex.open && !app.studio.classic && app.ui.mode == crate::state::Mode::Edit);
        assert_eq!(requests.try_recv().ok(), Some(CodexRequest::NewChat));
        // Signed out: it waits, without sending anything.
        tick(&mut app);
        assert!(requests.try_recv().is_err());
        events.send(CodexEvent::Account(Some("me@example.com".into()))).unwrap();
        codex_panel::poll(&mut app);
        tick(&mut app);
        assert_eq!(requests.try_recv().ok(), Some(CodexRequest::SetModel(DEFAULT_MODEL.into())));
        let Ok(CodexRequest::Send { text, files }) = requests.try_recv() else { panic!("no recipe") };
        assert!(files.is_empty(), "the clips are in the bin");
        assert!(text.contains("[[1]]") && text.contains("\"Assembly\"") && text.contains("A one-minute story") && !text.contains("{language}") && !text.contains("{briefing}"));
        assert_eq!(app.create_ai.run.as_ref().unwrap().stage, Stage::Working);
        events.send(CodexEvent::TurnFinished).unwrap();
        codex_panel::poll(&mut app);
        tick(&mut app);
        assert!(!app.create_ai.locked(), "the end unlocks");
        assert_eq!(app.create_ai.run.as_ref().unwrap().stage, Stage::Done);
        assert_eq!(requests.try_recv().ok(), Some(CodexRequest::SetModel("cheap".into())), "the user's model comes back");
    }

    /// An edit holds the next request and moves the cursor; a read does neither.
    #[test]
    fn codex_create_ai_hold_paces_the_next_edit() {
        let (mut app, _requests, events) = running_app(demo());
        let ctx = egui::Context::default();
        working(&mut app, &events, true);
        assert!(hold(&app, 0.0).is_none());
        observe(&mut app, &ctx, "engine.execute", &json!({"command": "timeline.razor", "params": {"time": 5000}}));
        assert!(hold(&app, 0.0).is_some(), "the next request waits");
        assert!(hold(&app, app.now() + 1.0).is_none(), "but not for long");
        let cursor = &app.create_ai.run.as_ref().unwrap().cursor;
        assert_eq!((cursor.label, cursor.target.map(|s| s.time)), ("Cutting", Some(Tick(5000))));
        app.create_ai.hold_until = 0.0;
        observe(&mut app, &ctx, "engine.execute", &json!({"command": "project.inspect"}));
        assert!(hold(&app, 0.0).is_none());
        // Outside a run nothing is held.
        app.create_ai.run = None;
        observe(&mut app, &ctx, "engine.execute", &json!({"command": "timeline.razor"}));
        assert!(hold(&app, 0.0).is_none());
    }

    /// The cursor goes to the edit's place on the Timeline, inside it.
    #[test]
    fn codex_create_ai_cursor_lands_in_the_timeline() {
        let (mut app, _requests, events) = running_app(demo());
        working(&mut app, &events, true);
        let mut h = harness(app);
        let ctx = h.ctx.clone();
        let v1 = h.state().session.active_sequence().unwrap().video_tracks[0].id;
        observe(h.state_mut(), &ctx, "engine.execute", &json!({"command": "timeline.place", "params": {"item": 1, "seconds": 1.0, "track": "V1"}}));
        for _ in 0..30 {
            h.step();
        }
        let app = h.state();
        let content = app.tl.layout.as_ref().expect("the Timeline is drawn").content;
        let spot = app.create_ai.run.as_ref().unwrap().cursor.target.unwrap();
        assert_eq!(spot.track, Some(v1));
        let goal = spot_pos(app, spot).unwrap();
        assert!(content.contains(goal), "{goal:?} outside {content:?}");
        let at = app.create_ai.run.as_ref().unwrap().cursor.at.expect("drawn");
        assert!(content.expand(1.0).contains(at), "{at:?} outside {content:?}");
        assert_eq!(app.studio.bottom, crate::studio::bottom::Tab::Timeline);
    }

    /// The `[[3]]` tag shows as "Etapa 3 de 6" on the bar and is gone from the chat.
    #[test]
    fn codex_create_ai_stage_tag_shows_the_step() {
        let (mut app, _requests, events) = running_app(demo());
        app.ui.language = crate::i18n::Language::PtBr;
        app.session.prefs.general.interface_language = "pt-br".into();
        app.codex.open = true;
        working(&mut app, &events, true);
        events.send(CodexEvent::AssistantDone { item: "a".into(), text: "[[3]] Agora a montagem.".into() }).unwrap();
        codex_panel::poll(&mut app);
        assert_eq!(stage_of(&app, STEPS.len()), Some(2));
        let mut h = harness(app);
        codex_panel::poll(h.state_mut());
        for _ in 0..4 {
            h.step();
        }
        assert!(h.query_all_by_label("Etapa 3 de 6").next().is_some());
        assert!(h.query_all_by_label_contains("Agora a montagem").next().is_some());
        assert!(h.query_all_by_label_contains("[[3]]").next().is_none());
        crate::i18n::set_current(crate::i18n::Language::En);
    }

    /// Out of usage the run waits; the sequence carries it to the next account.
    #[test]
    fn codex_create_ai_limit_pauses_and_another_account_resumes() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, requests, events) = running_app(Session::default());
        events.send(CodexEvent::Account(Some("one@example.com".into()))).unwrap();
        codex_panel::poll(&mut app);
        begin(&mut app, &png(dir.path()));
        tick(&mut app);
        while requests.try_recv().is_ok() {}
        events.send(CodexEvent::LimitReached).unwrap();
        events.send(CodexEvent::TurnFinished).unwrap();
        codex_panel::poll(&mut app);
        tick(&mut app);
        assert_eq!(app.create_ai.run.as_ref().unwrap().stage, Stage::Paused);
        assert!(app.create_ai.locked(), "still locked while it waits");
        app.codex.clear_limit();
        tick(&mut app);
        let Ok(CodexRequest::Send { text, .. }) = requests.try_recv() else { panic!("no resume") };
        assert!(text.starts_with("This job was interrupted"), "{text}");
        assert_eq!(app.create_ai.run.as_ref().unwrap().stage, Stage::Working);
    }

    /// Stop ends the turn, then Keep leaves the work and Discard closes the project.
    #[test]
    fn codex_create_ai_stop_offers_keep_or_discard() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, requests, events) = running_app(Session::default());
        let ctx = egui::Context::default();
        events.send(CodexEvent::Account(Some("one@example.com".into()))).unwrap();
        codex_panel::poll(&mut app);
        begin(&mut app, &png(dir.path()));
        tick(&mut app);
        while requests.try_recv().is_ok() {}
        act(&mut app, &ctx, Bar::Stop);
        assert_eq!(requests.try_recv().ok(), Some(CodexRequest::Stop));
        tick(&mut app);
        assert!(app.create_ai.locked(), "until the turn ends");
        events.send(CodexEvent::TurnFinished).unwrap();
        codex_panel::poll(&mut app);
        tick(&mut app);
        assert!(!app.create_ai.locked());
        assert_eq!(app.create_ai.run.as_ref().unwrap().stage, Stage::Stopped);
        assert!(!app.session.project.items.is_empty());
        act(&mut app, &ctx, Bar::Discard);
        assert!(app.create_ai.run.is_none());
        assert!(app.session.project.items.is_empty(), "discarded");
        // Keep leaves the project as it is.
        begin(&mut app, &png(dir.path()));
        app.create_ai.run.as_mut().unwrap().stage = Stage::Stopped;
        act(&mut app, &ctx, Bar::Dismiss);
        assert!(app.create_ai.run.is_none() && !app.session.project.items.is_empty());
    }
}
