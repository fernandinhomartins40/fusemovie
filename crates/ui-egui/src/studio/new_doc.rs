//! New project as a full page in the Studio style (mockup 7): formats, their presets, a preview
//! with rulers, the settings on the right, Back and Create at the foot. Create runs
//! `file.newProject` then `file.newSequence` (and `file.saveAs` when a folder is given).

use egui::{Align2, Rect, Sense, Stroke, StrokeKind, pos2, vec2};
use serde_json::json;

use super::home::{RADIUS, button, compact, frame_icon, paragraph};
use crate::FilmcraftApp;
use crate::icons::Icon;
use crate::theme::Tokens;

/// A family of frame sizes: its name, a note (what it is for) and its presets (width, height, fps).
pub(crate) struct Format {
    pub name: &'static str,
    pub note: &'static str,
    pub presets: &'static [(u32, u32, f64)],
}

pub(crate) const FORMATS: &[Format] = &[
    Format { name: "YouTube 1080p 16:9", note: "Full HD", presets: &[(1920, 1080, 29.97), (1920, 1080, 25.0), (1920, 1080, 60.0), (1280, 720, 30.0)] },
    Format { name: "4K UHD", note: "16:9", presets: &[(3840, 2160, 23.976), (3840, 2160, 25.0), (3840, 2160, 29.97), (3840, 2160, 60.0)] },
    Format { name: "Vertical 9:16", note: "Reels, TikTok, Shorts", presets: &[(1080, 1920, 30.0), (1080, 1920, 60.0), (720, 1280, 30.0)] },
    Format { name: "Square 1:1", note: "Feed", presets: &[(1080, 1080, 30.0), (1080, 1080, 25.0)] },
    Format { name: "Cinema 2.39:1", note: "Scope", presets: &[(2048, 858, 24.0), (4096, 1716, 24.0), (1920, 804, 23.976)] },
];

const RATES: [f64; 7] = [23.976, 24.0, 25.0, 29.97, 30.0, 50.0, 60.0];
const SAMPLE_RATES: [u32; 3] = [44_100, 48_000, 96_000];

/// The page's settings while it is open (kept in egui's memory: its presence means "open").
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Draft {
    pub name: String,
    /// Folder to save the project in; empty leaves it unsaved.
    pub location: String,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub sample_rate: u32,
    pub format: usize,
    pub error: String,
}

impl Draft {
    /// The settings of preset `p` of format `f` (the first of each when out of range).
    pub(crate) fn for_format(f: usize, p: usize) -> Self {
        let format = FORMATS.get(f).map_or(0, |_| f);
        let (width, height, fps) = FORMATS.get(format).and_then(|x| x.presets.get(p).or(x.presets.first())).copied().unwrap_or((1920, 1080, 29.97));
        Self { name: tl!("Untitled").into(), location: String::new(), width, height, fps, sample_rate: 48_000, format, error: String::new() }
    }

    /// The preset of the chosen format these settings match.
    fn preset(&self) -> Option<usize> {
        FORMATS.get(self.format)?.presets.iter().position(|&(w, h, f)| (w, h) == (self.width, self.height) && (f - self.fps).abs() < 1e-6)
    }
}

fn id() -> egui::Id {
    egui::Id::unique("studio-new-doc")
}

pub(crate) fn is_open(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<Draft>(id()).is_some())
}

/// Show the page next time Home draws, with `draft` or the first format.
pub(crate) fn open(ctx: &egui::Context, draft: Option<Draft>) {
    let d = draft.unwrap_or_else(|| Draft::for_format(0, 0));
    ctx.data_mut(|m| m.insert_temp(id(), d));
}

pub(crate) fn close(ctx: &egui::Context) {
    ctx.data_mut(|m| m.remove::<Draft>(id()));
}

fn fps_label(f: f64) -> String {
    if f.fract() == 0.0 { format!("{f:.0}") } else { format!("{f}") }
}

fn summary(d: &Draft) -> String {
    format!("{} × {} · {} fps · {} kHz", d.width, d.height, fps_label(d.fps), f64::from(d.sample_rate) / 1000.0)
}

/// One project is open at a time: with unsaved changes, ask Save / Don't Save / Cancel (Close
/// Project) and refuse for now, so nothing is lost.
pub(crate) fn guard_unsaved(app: &mut FilmcraftApp, ctx: &egui::Context) -> Result<(), String> {
    if !app.session.is_dirty() {
        return Ok(());
    }
    let _ = crate::menus::invoke(app, ctx, "file.closeProject", json!({}));
    Err(tl!("Save or discard the changes to the open project first").into())
}

/// Make the project as File ▸ New ▸ Project and New ▸ Sequence do, then save it in the chosen folder.
pub(crate) fn create(app: &mut FilmcraftApp, ctx: &egui::Context, d: &Draft) -> Result<(), String> {
    let name = d.name.trim();
    let name = if name.is_empty() { tl!("Untitled") } else { name };
    let save = match d.location.trim() {
        "" => None,
        dir => {
            let dir = std::path::Path::new(dir);
            if !dir.is_dir() {
                return Err(tlf!("The folder {path} does not exist", path = dir.display()));
            }
            if std::path::Path::new(name).file_name().is_none_or(|f| f != std::ffi::OsStr::new(name)) {
                return Err(tl!("A project name cannot contain a folder separator").into());
            }
            let path = dir.join(format!("{name}.fcproj"));
            if path.exists() {
                return Err(tlf!("{path} already exists: choose another name or folder", path = path.display()));
            }
            Some(path)
        }
    };
    guard_unsaved(app, ctx)?;
    crate::menus::invoke(app, ctx, "file.newProject", json!({"name": name}))?;
    crate::menus::invoke(app, ctx, "file.newSequence", json!({"name": name, "width": d.width, "height": d.height, "fps": d.fps, "sampleRate": d.sample_rate}))?;
    if let Some(path) = save {
        crate::menus::invoke(app, ctx, "file.saveAs", json!({"path": path.to_string_lossy()}))?;
    }
    app.ui.mode = crate::state::Mode::Edit;
    close(ctx);
    Ok(())
}

/// The New project page.
pub(crate) fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let mut d = ctx.data(|m| m.get_temp::<Draft>(id())).unwrap_or_else(|| Draft::for_format(0, 0));
    let area = ui.available_rect_before_wrap();
    let c = compact(area);
    ui.painter().rect_filled(area, 0.0, t.canvas);

    let foot = Rect::from_min_max(pos2(area.left(), area.bottom() - if c { 56.0 } else { 64.0 }), area.max);
    let formats = Rect::from_min_max(area.min, pos2(area.left() + if c { 184.0 } else { 224.0 }, foot.top()));
    let presets = Rect::from_min_max(pos2(formats.right(), area.top()), pos2(formats.right() + if c { 176.0 } else { 220.0 }, foot.top()));
    let settings = Rect::from_min_max(pos2(area.right() - if c { 290.0 } else { 360.0 }, area.top()), pos2(area.right(), foot.top()));
    let center = Rect::from_min_max(pos2(presets.right(), area.top()), pos2(settings.left(), foot.top()));
    for r in [formats, presets, settings, foot] {
        ui.painter().rect_filled(r, 0.0, t.chrome);
    }
    let line = Stroke::new(1.0, t.separator);
    ui.painter().vline(formats.right(), formats.y_range(), line);
    ui.painter().vline(presets.right(), presets.y_range(), line);
    ui.painter().vline(settings.left(), settings.y_range(), line);
    ui.painter().hline(foot.x_range(), foot.top(), line);

    render_formats(app, ui, formats, &mut d);
    render_presets(app, ui, presets, c, &mut d);
    render_preview(ui, center, c, &d);
    render_settings(app, ui, settings, c, &mut d);

    // Foot: Back, the summary, Create.
    let bh = if c { 34.0 } else { 36.0 };
    let back = button(app, ui, (pos2(foot.left() + 20.0, foot.center().y - bh / 2.0), bh), "newDoc.back", tl!("Back"), None, false);
    let ch = if c { 38.0 } else { 42.0 };
    let label = tl!("Create");
    let cw = ui.painter().layout_no_wrap(label.to_owned(), Tokens::semibold(14.0), t.text).size().x + 24.0 + 36.0;
    let create_at = pos2(foot.right() - 20.0 - cw, foot.center().y - ch / 2.0);
    let made = button(app, ui, (create_at, ch), "newDoc.create", label, Some(Icon::Plus), true);
    let text = if d.error.is_empty() { summary(&d) } else { d.error.clone() };
    let color = if d.error.is_empty() { t.text_dim } else { app.tokens.danger };
    let room = Rect::from_min_max(pos2(back.rect.right() + 20.0, foot.top()), pos2(made.rect.left() - 20.0, foot.bottom()));
    ui.painter().with_clip_rect(room).text(pos2(room.right(), foot.center().y), Align2::RIGHT_CENTER, text, Tokens::ui(13.0), color);

    if back.clicked() {
        close(&ctx);
        return;
    }
    if made.clicked() {
        if let Err(e) = create(app, &ctx, &d) {
            d.error = e;
        } else {
            return;
        }
    }
    ctx.data_mut(|m| m.insert_temp(id(), d));
}

fn section_title(ui: &egui::Ui, r: Rect, text: &str) {
    ui.painter().text(pos2(r.left() + 20.0, r.top() + 30.0), Align2::LEFT_CENTER, text, Tokens::semibold(14.0), t.text);
}

/// The formats: a row each, the chosen one in amber; a click picks its first preset.
fn render_formats(app: &mut FilmcraftApp, ui: &mut egui::Ui, r: Rect, d: &mut Draft) {
    section_title(ui, r, tl!("Choose a format"));
    for (i, f) in FORMATS.iter().enumerate() {
        let row = Rect::from_min_size(pos2(r.left() + 10.0, r.top() + 56.0 + i as f32 * 40.0), vec2(r.width() - 20.0, 36.0));
        let name = crate::i18n::t(f.name);
        let resp = ui.interact(row, egui::Id::unique(("studio-newdoc-format", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, name));
        app.auto.add(&format!("newDoc.format.{i}"), row, name);
        let on = d.format == i;
        let ink = if on { t.primary_text } else { t.text };
        if on {
            ui.painter().rect_filled(row, RADIUS, t.accent);
        } else if resp.hovered() {
            ui.painter().rect_filled(row, RADIUS, t.hover);
        }
        let (w, h, _) = f.presets.first().copied().unwrap_or((16, 9, 30.0));
        ui.painter().rect_stroke(frame_icon(pos2(row.left() + 18.0, row.center().y), vec2(18.0, 16.0), w, h), 2.0, Stroke::new(1.0, ink), StrokeKind::Inside);
        ui.painter().with_clip_rect(row).text(pos2(row.left() + 38.0, row.center().y), Align2::LEFT_CENTER, name, Tokens::ui(13.0), ink);
        if resp.clicked() {
            *d = Draft { name: std::mem::take(&mut d.name), location: std::mem::take(&mut d.location), ..Draft::for_format(i, 0) };
        }
    }
}

/// The chosen format's presets as cards: size over rate and audio; a click fills the settings.
fn render_presets(app: &mut FilmcraftApp, ui: &mut egui::Ui, r: Rect, c: bool, d: &mut Draft) {
    let Some(f) = FORMATS.get(d.format) else { return };
    section_title(ui, r, &tlf!("Presets ({n})", n = f.presets.len()));
    let chosen = d.preset();
    let card_h = if c { 64.0 } else { 76.0 };
    for (i, &(w, h, fps)) in f.presets.iter().enumerate() {
        let card = Rect::from_min_size(pos2(r.left() + 10.0, r.top() + 56.0 + i as f32 * (card_h + 8.0)), vec2(r.width() - 20.0, card_h));
        if card.bottom() > r.bottom() {
            break;
        }
        let label = format!("{w} × {h} · {} fps", fps_label(fps));
        let resp = ui.interact(card, egui::Id::unique(("studio-newdoc-preset", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
        app.auto.add(&format!("newDoc.preset.{i}"), card, &label);
        let on = chosen == Some(i);
        let stroke = Stroke::new(1.0, if on { t.accent } else { t.separator });
        ui.painter().rect(card, RADIUS, if resp.hovered() { t.hover } else { t.card }, stroke, StrokeKind::Inside);
        let p = ui.painter().with_clip_rect(card);
        let top = card.top() + if c { 16.0 } else { 20.0 };
        p.rect_stroke(frame_icon(pos2(card.left() + 22.0, top), vec2(20.0, 14.0), w, h), 2.0, Stroke::new(1.0, t.text_dim), StrokeKind::Inside);
        p.text(pos2(card.left() + 40.0, top), Align2::LEFT_CENTER, format!("{w} × {h}"), Tokens::semibold(13.0), t.text);
        p.text(pos2(card.left() + 12.0, top + if c { 26.0 } else { 32.0 }), Align2::LEFT_CENTER, format!("{} fps · 48 kHz", fps_label(fps)), Tokens::ui(12.0), t.text_dim);
        if resp.clicked() {
            (d.width, d.height, d.fps, d.sample_rate) = (w, h, fps, 48_000);
            d.error.clear();
        }
    }
}

/// The title, then the frame at its aspect between a width ruler and a height ruler.
fn render_preview(ui: &egui::Ui, r: Rect, c: bool, d: &Draft) {
    let pad = if c { 24.0 } else { 36.0 };
    let inner = r.shrink2(vec2(pad, pad * 0.8));
    let mut y = inner.top();
    y += paragraph(ui, pos2(inner.left(), y), inner.width(), tl!("Set up your next project."), Tokens::semibold(if c { 22.0 } else { 28.0 }), t.text);
    y += 4.0;
    y += paragraph(ui, pos2(inner.left(), y), inner.width(), tl!("Pick a format, adjust the settings and create your project."), Tokens::ui(14.0), t.text_dim);
    y += if c { 14.0 } else { 22.0 };
    ui.painter().hline(inner.x_range(), y, Stroke::new(1.0, t.separator));
    y += if c { 12.0 } else { 18.0 };
    ui.painter().text(pos2(inner.left(), y + 10.0), Align2::LEFT_CENTER, tl!("Preview"), Tokens::semibold(14.0), t.text);
    y += 30.0;

    // The frame, fitted under the width ruler and right of the height ruler, the summary below.
    let ruler = 28.0;
    let space = Rect::from_min_max(pos2(inner.left() + ruler, y + ruler), pos2(inner.right(), inner.bottom() - 30.0));
    if space.width() < 20.0 || space.height() < 20.0 {
        return;
    }
    let frame = frame_icon(space.center(), space.size(), d.width, d.height);
    ui.painter().rect(frame, 4.0, t.card, Stroke::new(1.0, t.field_border), StrokeKind::Inside);
    ui.painter().text(frame.center(), Align2::CENTER_CENTER, aspect(d.width, d.height), Tokens::ui(13.0), t.text_faint);
    let ink = Stroke::new(1.0, t.text_dim);
    let p = ui.painter();
    // width ruler
    let ry = frame.top() - ruler / 2.0;
    let wl = p.layout_no_wrap(format!("{} px", d.width), Tokens::ui(12.5), t.text);
    let gap = wl.size().x / 2.0 + 8.0;
    let cx = frame.center().x;
    for (a, b) in [(frame.left(), cx - gap), (cx + gap, frame.right())] {
        if b > a {
            p.hline(a..=b, ry, ink);
        }
    }
    p.vline(frame.left(), (ry - 5.0)..=(ry + 5.0), ink);
    p.vline(frame.right(), (ry - 5.0)..=(ry + 5.0), ink);
    p.galley(pos2(cx - wl.size().x / 2.0, ry - wl.size().y / 2.0), wl, t.text);
    // height ruler, its label turned on its side
    let rx = frame.left() - ruler / 2.0;
    let hl = p.layout_no_wrap(format!("{} px", d.height), Tokens::ui(12.5), t.text);
    let gap = hl.size().x / 2.0 + 8.0;
    let cy = frame.center().y;
    for (a, b) in [(frame.top(), cy - gap), (cy + gap, frame.bottom())] {
        if b > a {
            p.vline(rx, a..=b, ink);
        }
    }
    p.hline((rx - 5.0)..=(rx + 5.0), frame.top(), ink);
    p.hline((rx - 5.0)..=(rx + 5.0), frame.bottom(), ink);
    let (hw, hh) = (hl.size().x, hl.size().y);
    p.add(egui::epaint::TextShape::new(pos2(rx - hh / 2.0, cy + hw / 2.0), hl, t.text).with_angle(-std::f32::consts::FRAC_PI_2));
    p.text(pos2(frame.center().x, frame.bottom() + 18.0), Align2::CENTER_CENTER, summary(d), Tokens::ui(13.0), t.text_dim);
}

/// "16:9", or "2.39:1" when the reduced ratio has no small integer form.
fn aspect(w: u32, h: u32) -> String {
    let (mut a, mut b) = (w.max(1), h.max(1));
    while b != 0 {
        (a, b) = (b, a % b);
    }
    let (x, y) = (w.max(1) / a, h.max(1) / a);
    if x <= 32 && y <= 32 { format!("{x}:{y}") } else { format!("{:.2}:1", f64::from(w) / f64::from(h.max(1))) }
}

/// Studio fields: dark fill, 1 px border, 30 points tall, 13.5 px text.
fn field_style(ui: &mut egui::Ui) {
    ui.style_mut().override_font_id = Some(Tokens::ui(13.5));
    let s = ui.spacing_mut();
    s.interact_size.y = 30.0;
    s.button_padding = vec2(10.0, 6.0);
    s.item_spacing = vec2(8.0, 6.0);
    let v = ui.visuals_mut();
    v.override_text_color = Some(t.text);
    v.extreme_bg_color = t.field_bg;
    v.selection.stroke = Stroke::new(1.0, t.accent);
    for (w, border) in [(&mut v.widgets.inactive, t.field_border), (&mut v.widgets.hovered, t.text_faint), (&mut v.widgets.active, t.accent), (&mut v.widgets.open, t.accent)] {
        w.bg_fill = t.field_bg;
        w.weak_bg_fill = t.field_bg;
        w.bg_stroke = Stroke::new(1.0, border);
        w.corner_radius = 8.0.into();
        w.fg_stroke.color = t.text;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, t.separator);
}

fn field_label(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).font(Tokens::ui(12.5)).color(t.text_dim));
}

/// Name, location, size, rate, pixel aspect and sample rate.
fn render_settings(app: &mut FilmcraftApp, ui: &mut egui::Ui, r: Rect, c: bool, d: &mut Draft) {
    let inner = Rect::from_min_max(pos2(r.left() + 20.0, r.top() + 14.0), pos2(r.right() - 20.0, r.bottom() - 10.0));
    let before = d.clone();
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        field_style(ui);
        let w = inner.width();
        ui.label(egui::RichText::new(tl!("Project settings")).font(Tokens::semibold(16.0)).color(t.text));
        ui.add_space(if c { 0.0 } else { 6.0 });
        field_label(ui, tl!("Name"));
        let r = ui.add(egui::TextEdit::singleline(&mut d.name).desired_width(w).margin(vec2(10.0, 7.0)));
        app.auto.add_response("newDoc.name", &r, tl!("Name"));
        field_label(ui, tl!("Location"));
        let r = ui.add(egui::TextEdit::singleline(&mut d.location).desired_width(w).margin(vec2(10.0, 7.0)).hint_text(tl!("Not saved yet")));
        // shortcut: the folder is typed or pasted; a folder picker can come with the integrated shell
        app.auto.add_response("newDoc.location", &r, tl!("Location"));
        ui.add_space(if c { 2.0 } else { 8.0 });
        let half = (w - 10.0) / 2.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            for (key, label, v) in [("width", tl!("Width"), &mut d.width), ("height", tl!("Height"), &mut d.height)] {
                ui.vertical(|ui| {
                    ui.set_width(half);
                    field_label(ui, label);
                    let r = ui.add_sized(vec2(half, 30.0), egui::DragValue::new(v).range(16..=16_384).suffix(" px"));
                    app.auto.add_response(&format!("newDoc.{key}"), &r, label);
                });
            }
        });
        field_label(ui, tl!("Frame Rate"));
        let r = craft_ui::ComboBox::from_id_salt("studio-newdoc-fps").width(w).selected_text(format!("{} fps", fps_label(d.fps))).show_ui(ui, |ui| {
            for f in RATES {
                ui.selectable_value(&mut d.fps, f, format!("{} fps", fps_label(f)));
            }
        });
        app.auto.add_response("newDoc.fps", &r.response, tl!("Frame Rate"));
        field_label(ui, tl!("Pixel Aspect Ratio"));
        ui.add_enabled_ui(false, |ui| {
            craft_ui::ComboBox::from_id_salt("studio-newdoc-par").width(w).selected_text(tl!("Square Pixels (1.0)")).show_ui(ui, |_| {});
        })
        .response
        .on_disabled_hover_text(tl!("Sequences use square pixels"));
        field_label(ui, tl!("Audio Sample Rate"));
        let r = craft_ui::ComboBox::from_id_salt("studio-newdoc-rate").width(w).selected_text(format!("{} Hz", d.sample_rate)).show_ui(ui, |ui| {
            for s in SAMPLE_RATES {
                ui.selectable_value(&mut d.sample_rate, s, format!("{s} Hz"));
            }
        });
        app.auto.add_response("newDoc.sampleRate", &r.response, tl!("Audio Sample Rate"));
    });
    if *d != before {
        d.error.clear();
    }
}

#[cfg(test)]
mod tests {
    use egui_kittest::kittest::Queryable;
    use filmcraft_time::FrameRate;

    use super::super::home::tests::{assert_fits, harness, snapshot};
    use crate::FilmcraftApp;

    fn page(w: f32, h: f32) -> egui_kittest::Harness<'static, FilmcraftApp> {
        let mut h = harness(w, h, FilmcraftApp::new(filmcraft_engine::Session::default()));
        super::open(&h.ctx, None);
        h.run_steps(3);
        h
    }

    #[test]
    fn new_doc_renders_at_both_sizes_without_clipping() {
        for (w, hgt, name) in [(1440.0, 900.0, "new-project-1440x900"), (1100.0, 620.0, "new-project-1100x620")] {
            let mut h = page(w, hgt);
            assert_fits(&h, w, hgt, "newDoc.");
            for id in ["newDoc.create", "newDoc.back", "newDoc.sampleRate", "newDoc.format.4", "newDoc.preset.3"] {
                assert!(h.state().auto.find(id).is_some(), "{id} missing at {w}×{hgt}");
            }
            snapshot(&mut h, name);
        }
    }

    #[test]
    fn new_doc_preset_click_fills_settings_and_create_makes_the_sequence() {
        let mut h = page(1440.0, 900.0);
        h.get_by_label("4K UHD").click();
        h.run_steps(2);
        h.get_by_label("3840 × 2160 · 25 fps").click();
        h.run_steps(2);
        let d = h.ctx.data(|m| m.get_temp::<super::Draft>(super::id())).unwrap();
        assert_eq!((d.format, d.width, d.height, d.fps, d.sample_rate), (1, 3840, 2160, 25.0, 48_000));
        assert_eq!(d.preset(), Some(1));
        h.get_by_label("Create").click();
        h.run_steps(3);
        assert!(!super::is_open(&h.ctx), "status: {}", h.state().ui.status);
        let q = h.state().session.active_sequence().expect("a sequence");
        assert_eq!((q.settings.width, q.settings.height, q.settings.frame_rate, q.settings.sample_rate), (3840, 2160, FrameRate::from_f64(25.0), 48_000));
        assert_eq!(h.state().session.project.name, "Untitled");
    }

    #[test]
    fn new_doc_create_saves_in_the_folder_and_refuses_to_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = FilmcraftApp::new(filmcraft_engine::Session::default());
        let ctx = egui::Context::default();
        let mut d = super::Draft::for_format(4, 2);
        d.name = "Trailer".into();
        d.location = dir.path().to_string_lossy().into();
        super::create(&mut app, &ctx, &d).unwrap();
        assert!(dir.path().join("Trailer.fcproj").exists());
        let q = app.session.active_sequence().unwrap();
        assert_eq!((q.settings.width, q.settings.height, q.settings.frame_rate), (1920, 804, FrameRate::from_f64(23.976)));
        assert!(super::create(&mut app, &ctx, &d).is_err(), "the file exists");
        d.name = "a/b".into();
        assert!(super::create(&mut app, &ctx, &d).is_err());
        d.location = dir.path().join("nope").to_string_lossy().into();
        d.name = "Other".into();
        assert!(super::create(&mut app, &ctx, &d).is_err());
    }

    #[test]
    fn new_doc_aspect_reads_like_a_ratio() {
        assert_eq!(super::aspect(1920, 1080), "16:9");
        assert_eq!(super::aspect(1080, 1920), "9:16");
        assert_eq!(super::aspect(2048, 858), "2.39:1");
        assert_eq!(super::aspect(0, 0), "1:1");
    }
}
