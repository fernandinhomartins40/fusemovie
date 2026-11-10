//! The Studio shell: the Fuse family's editor layout, drawn instead of the docking layout.
//!
//! ```text
//! ┌ top_bar ── ☰ · name · project · undo/redo · command search · status · Codex · Export ────────┐
//! ├ mode_bar ─ Assemble · Edit · Color · Audio · Captions ─────────────────────────────────────────┤
//! │ tools (button + quick strip) │  work area (Program; Source beside it in Assemble) │ right │rail│
//! ├ bottom ─ properties of the selected clip ───────────────────────────────────────────────────────┤
//! ├ bottom ─ Timeline · Markers · Media (resizable) ─────────────────────────────────────────────────┤
//! └ status line: zoom · timecode · fps · size · clips · name ────────────────────────────────────────┘
//! ```
//!
//! Every region draws the existing panels (`panels::show`) and runs the existing commands
//! (`menus::invoke`), so nothing is lost; Window ▸ Classic Layout brings the docking layout back.
//! Sizes and colours follow `docs/fuse-suite/02-studio-design.md` (the Fuse Open Photo repository).

use egui::{Rect, pos2, vec2};

use crate::FilmcraftApp;
use crate::dock::PanelKind;
use crate::state;

pub mod bottom;
pub mod mode_bar;
pub mod right;
pub mod style;
pub mod tools;
pub mod top_bar;

#[cfg(test)]
mod tests;

/// The five work modes of the mode bar. Each picks the work area and the right panel's tabs;
/// the timeline tools stay reachable in every mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Assemble,
    Edit,
    Color,
    Audio,
    Captions,
}

impl Mode {
    pub const ALL: [Mode; 5] = [Mode::Assemble, Mode::Edit, Mode::Color, Mode::Audio, Mode::Captions];

    /// The right panel's tabs in this mode, the first shown by default.
    pub fn panels(self) -> &'static [PanelKind] {
        match self {
            Mode::Assemble => &[PanelKind::Project, PanelKind::EffectControls],
            Mode::Edit => &[PanelKind::EffectControls, PanelKind::Effects],
            Mode::Color => &[PanelKind::LumetriColor, PanelKind::LumetriScopes],
            Mode::Audio => &[PanelKind::EssentialSound, PanelKind::AudioTrackMixer, PanelKind::AudioMeters],
            Mode::Captions => &[PanelKind::Text, PanelKind::Properties, PanelKind::EssentialGraphics],
        }
    }
}

/// What the right rail put in the right panel instead of the mode's tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Panel(PanelKind),
    /// The list of every panel ("Panels…").
    List,
}

/// The Studio shell's view state (not persisted).
pub struct StudioState {
    /// Window ▸ Classic Layout: draw the docking layout instead.
    pub classic: bool,
    pub mode: Mode,
    /// The right panel's tab per mode (an index into [`Mode::panels`]).
    pub tab: [usize; 5],
    /// The rail's pick; `None` gives the right panel back to the mode.
    pub pick: Option<Pick>,
    /// The bottom strip's tab.
    pub bottom: bottom::Tab,
    /// The bottom strip's height as a share of the window's.
    pub strip_frac: f32,
    /// The Codex column is open (the wand button).
    pub codex_open: bool,
    /// The Codex column's width when the user dragged it; `None` = the default for the size.
    pub codex_w: Option<f32>,
    /// The tool list under the tool button is open.
    pub tools_open: bool,
    pub search: top_bar::Search,
}

impl Default for StudioState {
    fn default() -> Self {
        Self {
            classic: false,
            mode: Mode::default(),
            tab: [0; 5],
            pick: None,
            bottom: bottom::Tab::default(),
            strip_frac: style::STRIP_FRAC,
            codex_open: false,
            codex_w: None,
            tools_open: false,
            search: Default::default(),
        }
    }
}

/// Where each region goes this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Regions {
    pub top: Rect,
    pub modes: Rect,
    pub work: Rect,
    pub right: Rect,
    pub rail: Rect,
    pub codex: Option<Rect>,
    pub props: Rect,
    pub strip: Rect,
    pub status: Rect,
}

/// The regions of a `full` window. `codex` = the Codex column's width when open (it then runs down
/// to the status line and the bottom rows sit only under the work area and the right panel).
pub fn regions(full: Rect, compact: bool, codex: Option<f32>, strip_frac: f32) -> Regions {
    let s = style::Sizes::new(compact);
    let top = Rect::from_min_size(full.min, vec2(full.width(), s.top));
    let modes = Rect::from_min_size(top.left_bottom(), vec2(full.width(), s.modes));
    let status = Rect::from_min_max(pos2(full.left(), (full.bottom() - s.status).max(modes.bottom())), full.max);
    let mid = Rect::from_min_max(modes.left_bottom(), status.right_top());
    let rail = Rect::from_min_max(pos2((mid.right() - s.rail).max(mid.left()), mid.top()), mid.max);
    let codex = codex.map(|w| {
        let w = style::codex_width(w, full.width());
        Rect::from_min_max(pos2((rail.left() - w).max(mid.left()), mid.top()), pos2(rail.left(), mid.bottom()))
    });
    // The bottom rows: the whole width, or only left of the Codex column.
    let bottom_right = codex.map_or(mid.right(), |c| c.left());
    let strip_h = style::strip_height(strip_frac, full.height(), mid.height() - s.props, compact);
    let strip = Rect::from_min_max(pos2(mid.left(), mid.bottom() - strip_h), pos2(bottom_right, mid.bottom()));
    let props = Rect::from_min_max(pos2(mid.left(), (strip.top() - s.props).max(mid.top())), pos2(bottom_right, strip.top()));
    let upper_bottom = props.top();
    // Without Codex the rail stops above the bottom rows, as in the mockups.
    let rail = if codex.is_some() { rail } else { Rect::from_min_max(rail.min, pos2(rail.right(), upper_bottom)) };
    let right_edge = codex.map_or(rail.left(), |c| c.left());
    let right = Rect::from_min_max(pos2((right_edge - s.right).max(mid.left()), mid.top()), pos2(right_edge, upper_bottom));
    let work = Rect::from_min_max(mid.min, pos2(right.left(), upper_bottom));
    Regions { top, modes, work, right, rail, codex, props, strip, status }
}

/// Draw the Studio shell in `full`.
pub fn frame(app: &mut FilmcraftApp, ui: &mut egui::Ui, full: Rect) {
    let ctx = ui.ctx().clone();
    let t = app.tokens;
    ui.painter().rect_filled(full, 0.0, t.canvas);
    let compact = style::compact(&ctx);
    let codex = app.studio.codex_open.then(|| app.studio.codex_w.unwrap_or(style::Sizes::new(compact).codex));
    let r = regions(full, compact, codex, app.studio.strip_frac);
    if home_wanted(app) {
        top_bar::show(app, ui, r.top);
        // integrator: home::show — the Home page replaces this placeholder (the classic Import page).
        let body = Rect::from_min_max(r.top.left_bottom(), full.max);
        crate::panels::import_mode::show(app, ui, body);
        return;
    }
    follow_focus(app, ui, &r);
    top_bar::show(app, ui, r.top);
    mode_bar::show(app, ui, r.modes);
    if app.ui.mode == state::Mode::Export {
        let body = Rect::from_min_max(r.modes.left_bottom(), full.max);
        crate::panels::export_mode::show(app, ui, body);
        return;
    }
    work_area(app, ui, r.work);
    right::panel(app, ui, r.right);
    right::rail(app, ui, r.rail);
    if let Some(c) = r.codex {
        codex_column(app, ui, c);
    }
    bottom::props(app, ui, r.props);
    bottom::strip(app, ui, r.strip, full.height());
    bottom::status(app, ui, r.status);
    // Drawn last so its list sits over the panels it opens above.
    tools::show(app, ui, r.work);
}

/// Home instead of the editor: the Home button (Import) or an empty project.
fn home_wanted(app: &FilmcraftApp) -> bool {
    app.ui.mode == state::Mode::Import || (app.session.state.active_sequence.is_none() && app.session.project.items.is_empty())
}

/// A press in a region gives its panel the keyboard focus, as a click in a docked panel does:
/// shortcuts act on the focused panel.
fn follow_focus(app: &mut FilmcraftApp, ui: &egui::Ui, r: &Regions) {
    let Some(pos) = ui.input(|i| i.pointer.any_pressed().then(|| i.pointer.interact_pos()).flatten()) else { return };
    let (source, _) = work_split(app, r.work);
    let focused = if r.strip.contains(pos) {
        Some(bottom::panel_of(app.studio.bottom))
    } else if source.is_some_and(|s| s.contains(pos)) {
        Some(PanelKind::Source)
    } else if r.work.contains(pos) {
        Some(PanelKind::Program)
    } else if r.right.contains(pos) {
        right::shown(app)
    } else {
        None
    };
    if let Some(p) = focused {
        app.ui.focused = p;
    }
}

/// The Source monitor's part of the work area (Assemble only) and the Program monitor's.
pub fn work_split(app: &FilmcraftApp, work: Rect) -> (Option<Rect>, Rect) {
    let left = work.left() + tools::column_width(app, work);
    let inner = Rect::from_min_max(pos2(left, work.top()), work.max);
    if app.studio.mode != Mode::Assemble || inner.width() < 480.0 {
        return (None, inner);
    }
    let mid = inner.center().x;
    (Some(Rect::from_min_max(inner.min, pos2(mid - 3.0, inner.bottom()))), Rect::from_min_max(pos2(mid + 3.0, inner.top()), inner.max))
}

/// The Program monitor (and the Source monitor beside it in Assemble).
fn work_area(app: &mut FilmcraftApp, ui: &mut egui::Ui, work: Rect) {
    let t = app.tokens;
    ui.painter().rect_filled(work, 0.0, t.canvas);
    let (source, program) = work_split(app, work);
    if let Some(s) = source {
        draw_panel(app, ui, PanelKind::Source, s);
    }
    draw_panel(app, ui, PanelKind::Program, program);
}

/// Draw panel `kind` in `rect`, as the dock area draws a panel body.
pub fn draw_panel(app: &mut FilmcraftApp, ui: &mut egui::Ui, kind: PanelKind, rect: Rect) {
    if rect.width() < 1.0 || rect.height() < 1.0 {
        return;
    }
    app.auto.add(&format!("panel.{}", kind.id()), rect, kind.title());
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).id_salt(("studio-panel", kind.id())));
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    crate::panels::show(app, &mut child, kind, rect);
    if kind == PanelKind::Timeline {
        crate::dock_ui::note_body(ui.ctx(), kind, rect, ui.layer_id());
    }
}

/// The Codex column: a left edge to drag, then the slot.
fn codex_column(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    ui.painter().rect_filled(rect, 0.0, t.chrome);
    ui.painter().vline(rect.left(), rect.y_range(), egui::Stroke::new(1.0, t.separator));
    let edge = Rect::from_min_max(pos2(rect.left() - 3.0, rect.top()), pos2(rect.left() + 3.0, rect.bottom()));
    let resp = ui.interact(edge, egui::Id::unique("studio-codex-edge"), egui::Sense::drag()).on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
    if resp.dragged() {
        let w = rect.width() - resp.drag_delta().x;
        app.studio.codex_w = Some(style::codex_width(w, ui.ctx().content_rect().width()));
    }
    let inner = rect.shrink2(vec2(12.0, 12.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).id_salt("studio-codex").layout(egui::Layout::top_down(egui::Align::Min)));
    child.set_clip_rect(inner.intersect(ui.clip_rect()));
    codex_slot(app, &mut child);
}

/// The Codex chat's place in the shell.
// integrator: codex_panel::show_in — replace the placeholder below with the Codex panel.
pub fn codex_slot(app: &mut FilmcraftApp, ui: &mut egui::Ui) {
    let t = app.tokens;
    ui.label(egui::RichText::new("Codex").font(crate::theme::Tokens::semibold(18.0)).color(t.text));
    ui.add_space(8.0);
    ui.label(egui::RichText::new(tl!("The Codex assistant will appear here.")).color(t.text_dim));
}

/// Window ▸ Classic Layout.
pub fn toggle_classic(app: &mut FilmcraftApp) {
    app.studio.classic = !app.studio.classic;
}
