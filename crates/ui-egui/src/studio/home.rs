//! Home in the Studio style (`docs/fuse-suite/02-studio-design.md`, mockup 6): the rail, the hero
//! with New project and Open project, the drop zone, the recent projects and "Start with a format".
//! Everything it does goes through the existing commands (`menus::invoke`).

use egui::{Align2, Color32, FontId, Rect, Response, Sense, Stroke, StrokeKind, pos2, vec2};
use serde_json::json;

use crate::FilmcraftApp;
use crate::icons::Icon;
use crate::theme::Tokens;

/// The Studio colours of 02-studio-design.md until the theme has them.
// integrator: switch to theme::Tokens
pub(super) struct Pal {
    pub canvas: Color32,
    pub chrome: Color32,
    pub card: Color32,
    pub border: Color32,
    pub field: Color32,
    pub field_border: Color32,
    pub hover: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub text_faint: Color32,
    pub accent: Color32,
    pub primary_text: Color32,
}

// integrator: switch to theme::Tokens
pub(super) const PAL: Pal = Pal {
    canvas: Color32::from_rgb(19, 20, 20),
    chrome: Color32::from_rgb(29, 29, 30),
    card: Color32::from_rgb(35, 35, 36),
    border: Color32::from_rgb(54, 54, 56),
    field: Color32::from_rgb(28, 28, 29),
    field_border: Color32::from_rgb(72, 72, 74),
    hover: Color32::from_rgb(48, 48, 50),
    text: Color32::from_gray(240),
    text_dim: Color32::from_gray(170),
    text_faint: Color32::from_gray(110),
    accent: Color32::from_rgb(253, 181, 40),
    primary_text: Color32::from_gray(18),
};

pub(super) const RADIUS: f32 = 10.0;
pub(super) const BRAND: &str = env!("FILMCRAFT_BRAND_NAME");
/// Recent projects listed at most.
const RECENT_MAX: usize = 6;
const RECENT_ROW: f32 = 34.0;

/// Compact layout (02-studio-design.md): under 800 points tall or 1280 wide.
pub(super) fn compact(r: Rect) -> bool {
    r.height() < 800.0 || r.width() < 1280.0
}

/// What a click on Home asks for, run once the page is drawn.
enum Act {
    NewProject,
    Open,
    Settings,
    CreateAi,
    Format(usize),
    Recent(String),
    Drop(Vec<String>),
}

/// Home, or the New project page while it is open.
pub(crate) fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui) {
    if super::new_doc::is_open(ui.ctx()) {
        super::new_doc::show(app, ui);
        return;
    }
    let area = ui.available_rect_before_wrap();
    let c = compact(area);
    ui.painter().rect_filled(area, 0.0, PAL.canvas);
    let mut act = None;

    let rail = Rect::from_min_max(area.min, pos2(area.left() + if c { 184.0 } else { 216.0 }, area.bottom()));
    ui.painter().rect_filled(rail, 0.0, PAL.chrome);
    ui.painter().vline(rail.right(), rail.y_range(), Stroke::new(1.0, PAL.border));
    render_rail(app, ui, rail, &mut act);

    let pad = if c { 24.0 } else { 40.0 };
    let body = Rect::from_min_max(pos2(rail.right() + pad, area.top() + pad * 0.8), pos2(area.right() - pad, area.bottom() - pad * 0.8));
    let right_w = if c { 260.0 } else { (body.width() * 0.3).clamp(290.0, 360.0) };
    let gap = if c { 20.0 } else { 36.0 };
    let left = Rect::from_min_max(body.min, pos2(body.right() - right_w - gap, body.bottom()));
    let right = Rect::from_min_max(pos2(body.right() - right_w, body.top()), body.max);
    render_main(app, ui, left, c, &mut act);
    render_formats(app, ui, right, c, &mut act);

    // Files dropped while Home shows start a project from them.
    let dropped: Vec<String> = ui.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_string_lossy().to_string()).collect());
    if !dropped.is_empty() {
        act = Some(Act::Drop(dropped));
    }
    if let Some(a) = act {
        run(app, ui.ctx(), a);
    }
}

fn run(app: &mut FilmcraftApp, ctx: &egui::Context, act: Act) {
    let r = match act {
        Act::NewProject => {
            super::new_doc::open(ctx, None);
            Ok(())
        }
        Act::Open => crate::menus::invoke(app, ctx, "file.open", json!({})).map(|_| ()),
        Act::Settings => crate::menus::invoke(app, ctx, "app.settings", json!({})).map(|_| ()),
        Act::CreateAi => crate::menus::invoke(app, ctx, crate::create_ai::COMMAND, json!({})).map(|_| ()),
        Act::Format(i) => super::new_doc::create(app, ctx, &super::new_doc::Draft::for_format(i, 0)),
        Act::Recent(path) => crate::menus::invoke(app, ctx, "file.open", json!({"path": path})).map(|v| {
            // with unsaved changes Save / Don't Save / Cancel is asked first and Home stays
            if v.get("dialog").is_none() {
                app.ui.mode = crate::state::Mode::Edit;
            }
        }),
        Act::Drop(paths) => drop_files(app, ctx, &paths),
    };
    if let Err(e) = r {
        app.ui.status = e;
    }
}

/// Files dropped on Home: a project file opens; media becomes a new project (named after the first
/// file) with a sequence made from it, as File ▸ New ▸ Project, Import and New Sequence From Clip.
pub(crate) fn drop_files(app: &mut FilmcraftApp, ctx: &egui::Context, paths: &[String]) -> Result<(), String> {
    let paths: Vec<&String> = paths.iter().filter(|p| std::path::Path::new(p).exists()).collect();
    if let Some(project) = paths.iter().find(|p| std::path::Path::new(p).extension().is_some_and(|e| e.eq_ignore_ascii_case("fcproj"))) {
        let v = crate::menus::invoke(app, ctx, "file.open", json!({"path": project}))?;
        if v.get("dialog").is_none() {
            app.ui.mode = crate::state::Mode::Edit;
        }
        return Ok(());
    }
    let Some(first) = paths.first() else { return Err(tl!("Nothing to import: drop video, audio or image files").into()) };
    let name = std::path::Path::new(first).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    super::new_doc::guard_unsaved(app, ctx)?;
    crate::menus::invoke(app, ctx, "file.newProject", json!({"name": name}))?;
    let v = crate::menus::invoke(app, ctx, "mediaBrowser.import", json!({"paths": paths, "imageSequence": false}))?;
    let items: Vec<u64> = v["items"].as_array().map(|a| a.iter().filter_map(|i| i.as_u64()).collect()).unwrap_or_default();
    if items.is_empty() {
        let errs = v["errors"].as_array().map(|e| e.iter().filter_map(|e| e.as_str()).collect::<Vec<_>>().join("; ")).unwrap_or_default();
        return Err(if errs.is_empty() { tl!("Nothing to import: drop video, audio or image files").into() } else { errs });
    }
    crate::menus::invoke(app, ctx, "file.newSequence", json!({"name": name, "fromItems": items}))?;
    app.ui.mode = crate::state::Mode::Edit;
    Ok(())
}

/// A row on the rail: an icon and a label, the amber pill when `selected`, faint when disabled.
fn rail_item(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect, (id, icon, label): (&str, Icon, &str), selected: bool, enabled: bool) -> Response {
    let resp = ui.interact(rect, egui::Id::unique(("studio-home-rail", id)), if enabled { Sense::click() } else { Sense::hover() });
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, label));
    app.auto.add(&format!("home.rail.{id}"), rect, label);
    let ink = if selected {
        PAL.primary_text
    } else if enabled {
        PAL.text
    } else {
        PAL.text_faint
    };
    if selected {
        ui.painter().rect_filled(rect, RADIUS, PAL.accent);
    } else if enabled && resp.hovered() {
        ui.painter().rect_filled(rect, RADIUS, PAL.hover);
    }
    crate::icons::paint(ui.painter(), Rect::from_center_size(pos2(rect.left() + 20.0, rect.center().y), vec2(16.0, 16.0)), icon, ink);
    let font = if selected { Tokens::semibold(13.5) } else { Tokens::ui(13.5) };
    ui.painter().with_clip_rect(rect).text(pos2(rect.left() + 40.0, rect.center().y), Align2::LEFT_CENTER, label, font, ink);
    if enabled { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp }
}

/// Product name, then Home (selected), New project, Open project, Create with AI;
/// Settings at the foot.
fn render_rail(app: &mut FilmcraftApp, ui: &mut egui::Ui, rail: Rect, act: &mut Option<Act>) {
    ui.painter().text(pos2(rail.left() + 22.0, rail.top() + 30.0), Align2::LEFT_CENTER, BRAND, Tokens::semibold(16.0), PAL.text);
    let x = rail.left() + 10.0..=rail.right() - 10.0;
    let row = |y: f32| Rect::from_x_y_ranges(x.clone(), y..=y + 36.0);
    let top = rail.top() + 60.0;
    rail_item(app, ui, row(top), ("home", Icon::Home, tl!("Home")), true, true);
    if rail_item(app, ui, row(top + 42.0), ("newProject", Icon::Plus, tl!("New Project")), false, true).clicked() {
        *act = Some(Act::NewProject);
    }
    if rail_item(app, ui, row(top + 84.0), ("open", Icon::Folder, tl!("Open Project")), false, true).clicked() {
        *act = Some(Act::Open);
    }
    let ai = row(top + 126.0);
    if rail_item(app, ui, ai, ("createAi", Icon::Sparkle, tl!("Create with AI")), false, true).clicked() {
        *act = Some(Act::CreateAi);
    }
    let foot = rail.bottom() - 50.0;
    if foot > ai.bottom() + 16.0 {
        ui.painter().hline(x.clone(), foot - 8.0, Stroke::new(1.0, PAL.border));
        if rail_item(app, ui, row(foot), ("settings", Icon::Gear, tl!("Settings")), false, true).clicked() {
            *act = Some(Act::Settings);
        }
    }
}

/// A rounded icon + label button `h` tall at `min`: amber when `primary`, a card otherwise.
pub(super) fn button(app: &mut FilmcraftApp, ui: &mut egui::Ui, (min, h): (egui::Pos2, f32), id: &str, label: &str, icon: Option<Icon>, primary: bool) -> Response {
    let font = if primary { Tokens::semibold(14.0) } else { Tokens::ui(13.5) };
    let ink = if primary { PAL.primary_text } else { PAL.text };
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, ink);
    let icon_w = if icon.is_some() { 24.0 } else { 0.0 };
    let rect = Rect::from_min_size(min, vec2(galley.size().x + icon_w + 36.0, h));
    let resp = ui.interact(rect, egui::Id::unique(("studio-button", id)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
    app.auto.add(id, rect, label);
    let (fill, stroke) = match (primary, resp.hovered()) {
        (true, false) => (PAL.accent, Stroke::NONE),
        (true, true) => (PAL.accent.gamma_multiply(0.9), Stroke::NONE),
        (false, hov) => (if hov { PAL.hover } else { PAL.card }, Stroke::new(1.0, PAL.field_border)),
    };
    ui.painter().rect(rect, RADIUS, fill, stroke, StrokeKind::Inside);
    let x = rect.left() + 18.0;
    if let Some(icon) = icon {
        crate::icons::paint(ui.painter(), Rect::from_center_size(pos2(x + 8.0, rect.center().y), vec2(16.0, 16.0)), icon, ink);
    }
    ui.painter().galley(pos2(x + icon_w, rect.center().y - galley.size().y / 2.0), galley, ink);
    resp
}

/// Text wrapped to `w`, drawn at `min`; returns its height.
pub(super) fn paragraph(ui: &egui::Ui, min: egui::Pos2, w: f32, text: &str, font: FontId, color: Color32) -> f32 {
    let g = ui.painter().layout(text.to_owned(), font, color, w.max(10.0));
    let h = g.size().y;
    ui.painter().galley(min, g, color);
    h
}

/// The hero, its buttons, the drop zone and the recent projects.
fn render_main(app: &mut FilmcraftApp, ui: &mut egui::Ui, left: Rect, c: bool, act: &mut Option<Act>) {
    let mut y = left.top();
    y += paragraph(ui, pos2(left.left(), y), left.width(), tl!("What shall we create today?"), Tokens::semibold(if c { 22.0 } else { 28.0 }), PAL.text);
    y += 6.0;
    let sub = tlf!("Start a new {name} project, open one, or drop media to begin.", name = BRAND);
    y += paragraph(ui, pos2(left.left(), y), left.width(), &sub, Tokens::ui(14.0), PAL.text_dim);
    y += if c { 16.0 } else { 24.0 };
    let bh = if c { 36.0 } else { 40.0 };
    let new = button(app, ui, (pos2(left.left(), y), bh), "home.newProject", tl!("New Project"), Some(Icon::Plus), true);
    if new.clicked() {
        *act = Some(Act::NewProject);
    }
    if button(app, ui, (pos2(new.rect.right() + 10.0, y), bh), "home.open", tl!("Open Project"), Some(Icon::Folder), false).clicked() {
        *act = Some(Act::Open);
    }
    y += bh + if c { 18.0 } else { 26.0 };

    let recent: Vec<String> = app.session.prefs.general.recent_projects.iter().take(RECENT_MAX).cloned().collect();
    let wanted = recent.len().max(1) as f32;
    let recent_block = 28.0 + wanted * RECENT_ROW;
    let drop_h = (left.bottom() - y - recent_block - 18.0).clamp(96.0, 320.0);
    let drop = Rect::from_min_size(pos2(left.left(), y), vec2(left.width(), drop_h));
    if drop_zone(app, ui, drop, c).clicked() {
        *act = Some(Act::Open);
    }
    y = drop.bottom() + 18.0;

    ui.painter().text(pos2(left.left(), y + 10.0), Align2::LEFT_CENTER, tl!("Recent"), Tokens::semibold(14.0), PAL.text);
    y += 28.0;
    if recent.is_empty() {
        let r = Rect::from_min_size(pos2(left.left(), y), vec2(left.width(), RECENT_ROW));
        crate::icons::paint(ui.painter(), Rect::from_center_size(pos2(r.left() + 10.0, r.center().y), vec2(14.0, 14.0)), Icon::Sequence, PAL.text_faint);
        ui.painter().with_clip_rect(r).text(
            pos2(r.left() + 30.0, r.center().y),
            Align2::LEFT_CENTER,
            tl!("Your recent projects will appear here."),
            Tokens::ui(13.0),
            PAL.text_dim,
        );
        return;
    }
    for (i, path) in recent.iter().enumerate() {
        let r = Rect::from_min_size(pos2(left.left(), y + i as f32 * RECENT_ROW), vec2(left.width(), RECENT_ROW));
        if r.bottom() > left.bottom() + 1.0 {
            break;
        }
        let name = crate::menus::recent_name(path);
        let resp = ui.interact(r, egui::Id::unique(("studio-home-recent", i)), Sense::click()).on_hover_text(path.as_str());
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &name));
        app.auto.add(&format!("home.recent.{i}"), r, &name);
        if resp.hovered() {
            ui.painter().rect_filled(r, RADIUS - 2.0, PAL.hover);
        }
        let p = ui.painter().with_clip_rect(r);
        // shortcut: a project has no thumbnail yet, so every row shows the sequence icon
        crate::icons::paint(&p, Rect::from_center_size(pos2(r.left() + 16.0, r.center().y), vec2(16.0, 16.0)), Icon::Sequence, PAL.text_dim);
        let n = p.text(pos2(r.left() + 36.0, r.center().y), Align2::LEFT_CENTER, &name, Tokens::ui(13.5), PAL.text);
        if let Some(folder) = std::path::Path::new(path).parent() {
            p.text(pos2(n.right() + 12.0, r.center().y), Align2::LEFT_CENTER, folder.to_string_lossy(), Tokens::ui(12.5), PAL.text_faint);
        }
        if resp.clicked() {
            *act = Some(Act::Recent(path.clone()));
        }
    }
}

/// The dashed drop zone, lit while files hover the window; a click opens a project.
fn drop_zone(app: &mut FilmcraftApp, ui: &mut egui::Ui, r: Rect, c: bool) -> Response {
    let resp = ui.interact(r, egui::Id::unique("studio-home-drop"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    let label = tl!("Drop video, audio or images to start a project");
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
    app.auto.add("home.drop", r, label);
    let lit = resp.hovered() || ui.input(|i| !i.raw.hovered_files.is_empty());
    let p = ui.painter();
    if lit {
        p.rect_filled(r, 12.0, PAL.hover);
    }
    p.extend(egui::Shape::dashed_line(&rounded_outline(r.shrink(0.5), 12.0), Stroke::new(1.0, if lit { PAL.accent } else { PAL.field_border }), 6.0, 5.0));
    let ink = if lit { PAL.accent } else { PAL.text_dim };
    let icon = if c { 26.0 } else { 34.0 };
    let block = icon + 14.0 + 20.0 + 20.0;
    let mut y = r.center().y - block / 2.0;
    if r.height() >= block + 16.0 {
        crate::icons::paint(p, Rect::from_center_size(pos2(r.center().x, y + icon / 2.0), vec2(icon, icon)), Icon::Import, ink);
        y += icon + 14.0;
    } else {
        y = r.center().y - 20.0;
    }
    p.text(pos2(r.center().x, y + 10.0), Align2::CENTER_CENTER, label, Tokens::semibold(14.0), PAL.text);
    let or = format!("{} ", tl!("or"));
    let link = tl!("open a project");
    let g_or = p.layout_no_wrap(or, Tokens::ui(13.0), PAL.text_dim);
    let g_link = p.layout_no_wrap(link.to_owned(), Tokens::ui(13.0), PAL.accent);
    let x = r.center().x - (g_or.size().x + g_link.size().x) / 2.0;
    let ly = y + 30.0 - g_or.size().y / 2.0;
    let lx = x + g_or.size().x;
    p.hline(lx..=lx + g_link.size().x, ly + g_link.size().y, Stroke::new(1.0, PAL.accent));
    p.galley(pos2(x, ly), g_or, PAL.text_dim);
    p.galley(pos2(lx, ly), g_link, PAL.accent);
    resp
}

/// The outline of a rounded rectangle as a closed polyline, for the dashed border.
pub(super) fn rounded_outline(r: Rect, radius: f32) -> Vec<egui::Pos2> {
    let rad = radius.min(r.width() / 2.0).min(r.height() / 2.0).max(0.0);
    let corners = [(r.right() - rad, r.top() + rad, -90.0_f32), (r.right() - rad, r.bottom() - rad, 0.0), (r.left() + rad, r.bottom() - rad, 90.0), (r.left() + rad, r.top() + rad, 180.0)];
    let mut pts = Vec::with_capacity(4 * 7 + 1);
    for (cx, cy, start) in corners {
        for k in 0..=6 {
            let a = (start + 15.0 * k as f32).to_radians();
            pts.push(pos2(cx + rad * a.cos(), cy + rad * a.sin()));
        }
    }
    if let Some(&first) = pts.first() {
        pts.push(first);
    }
    pts
}

/// The frame of a `w`×`h` picture fitted in `size` and centred at `c`, at least 2 points a side.
pub(super) fn frame_icon(c: egui::Pos2, size: egui::Vec2, w: u32, h: u32) -> Rect {
    let (w, h) = (w.max(1) as f32, h.max(1) as f32);
    let k = (size.x / w).min(size.y / h);
    Rect::from_center_size(c, vec2((w * k).max(2.0), (h * k).max(2.0)))
}

/// "Start with a format": a card with one row per format; a click creates that project.
fn render_formats(app: &mut FilmcraftApp, ui: &mut egui::Ui, right: Rect, c: bool, act: &mut Option<Act>) {
    let row_h = if c { 46.0 } else { 54.0 };
    let n = super::new_doc::FORMATS.len() as f32;
    let card = Rect::from_min_size(right.min, vec2(right.width(), (44.0 + n * (row_h + 4.0) + 8.0).min(right.height())));
    ui.painter().rect(card, 12.0, PAL.chrome, Stroke::new(1.0, PAL.border), StrokeKind::Inside);
    ui.painter().text(pos2(card.left() + 16.0, card.top() + 24.0), Align2::LEFT_CENTER, tl!("Start with a format"), Tokens::semibold(14.0), PAL.text);
    for (i, f) in super::new_doc::FORMATS.iter().enumerate() {
        let r = Rect::from_min_size(pos2(card.left() + 8.0, card.top() + 44.0 + i as f32 * (row_h + 4.0)), vec2(card.width() - 16.0, row_h));
        if r.bottom() > card.bottom() {
            break;
        }
        let name = crate::i18n::t(f.name);
        let resp = ui.interact(r, egui::Id::unique(("studio-home-format", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, name));
        app.auto.add(&format!("home.format.{i}"), r, name);
        let hov = resp.hovered();
        let p = ui.painter().with_clip_rect(r);
        p.rect(r, RADIUS - 2.0, if hov { PAL.hover } else { PAL.card }, Stroke::new(1.0, PAL.border), StrokeKind::Inside);
        let (w, h, _) = f.presets.first().copied().unwrap_or((1920, 1080, 30.0));
        let frame = frame_icon(pos2(r.left() + 26.0, r.center().y), vec2(26.0, 20.0), w, h);
        p.rect_stroke(frame, 2.0, Stroke::new(1.0, if hov { PAL.accent } else { PAL.text_dim }), StrokeKind::Inside);
        p.vline(r.left() + 50.0, (r.center().y - 12.0)..=(r.center().y + 12.0), Stroke::new(1.0, PAL.border));
        p.text(pos2(r.left() + 62.0, r.center().y - 9.0), Align2::LEFT_CENTER, name, Tokens::semibold(13.0), PAL.text);
        p.text(pos2(r.left() + 62.0, r.center().y + 9.0), Align2::LEFT_CENTER, format!("{w} × {h} · {}", crate::i18n::t(f.note)), Tokens::ui(12.0), PAL.text_dim);
        crate::icons::paint(&p, Rect::from_center_size(pos2(r.right() - 16.0, r.center().y), vec2(12.0, 12.0)), Icon::ChevronRight, PAL.text_dim);
        if resp.clicked() {
            *act = Some(Act::Format(i));
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use std::sync::Arc;

    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    use crate::FilmcraftApp;

    fn ready() -> egui::Id {
        egui::Id::unique("studio-test-ready")
    }

    pub(in crate::studio) fn harness(w: f32, h: f32, app: FilmcraftApp) -> Harness<'static, FilmcraftApp> {
        let mut h = Harness::builder().with_size(egui::vec2(w, h)).build_ui_state(
            |ui, app: &mut FilmcraftApp| {
                // the builder runs a frame before the fonts are installed: draw from the next one
                if !ui.ctx().data(|d| d.get_temp::<bool>(ready()).unwrap_or(false)) {
                    return;
                }
                app.auto.begin_frame();
                egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| super::show(app, ui));
            },
            app,
        );
        crate::theme::install(&h.ctx, &h.state().tokens);
        h.ctx.data_mut(|d| d.insert_temp(ready(), true));
        h.run_steps(3);
        h
    }

    /// Saves the page as a PNG when `FUSEMOVIE_SNAPSHOT_DIR` is set (CI uploads them).
    pub(in crate::studio) fn snapshot(h: &mut Harness<'static, FilmcraftApp>, name: &str) {
        if let Some(dir) = std::env::var_os("FUSEMOVIE_SNAPSHOT_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            h.render().unwrap().save(std::path::Path::new(&dir).join(format!("{name}.png"))).unwrap();
        }
    }

    /// Every widget of the page lies inside the window.
    pub(in crate::studio) fn assert_fits(h: &Harness<'static, FilmcraftApp>, w: f32, hgt: f32, prefix: &str) {
        let els = h.state().auto.query(prefix);
        assert!(!els.is_empty(), "no {prefix} widgets");
        for e in els {
            let [x, y, ew, eh] = e.rect;
            assert!(x >= 0.0 && y >= 0.0 && x + ew <= w + 0.5 && y + eh <= hgt + 0.5, "{} at {:?} outside {w}×{hgt}", e.id, e.rect);
        }
    }

    #[test]
    fn home_renders_at_both_sizes_without_clipping() {
        for (w, hgt, name) in [(1440.0, 900.0, "home-1440x900"), (1100.0, 620.0, "home-1100x620")] {
            let mut h = harness(w, hgt, FilmcraftApp::new(filmcraft_engine::Session::default()));
            assert_fits(&h, w, hgt, "home.");
            for label in ["New Project", "Open Project", "Cinema 2.39:1", "Vertical 9:16", "Settings"] {
                assert!(h.query_all_by_label(label).next().is_some(), "{label} missing at {w}×{hgt}");
            }
            assert!(h.state().auto.find("home.format.4").is_some(), "every format fits at {w}×{hgt}");
            snapshot(&mut h, name);
        }
    }

    #[test]
    fn home_new_project_opens_the_page_and_back_returns() {
        let mut h = harness(1440.0, 900.0, FilmcraftApp::new(filmcraft_engine::Session::default()));
        h.get_all_by_label("New Project").next().unwrap().click();
        h.run_steps(3);
        assert!(super::super::new_doc::is_open(&h.ctx));
        assert!(h.query_by_label("Create").is_some());
        h.get_by_label("Back").click();
        h.run_steps(3);
        assert!(!super::super::new_doc::is_open(&h.ctx));
        assert!(h.state().auto.find("home.format.0").is_some());
    }

    #[test]
    fn home_format_card_creates_that_sequence() {
        let mut h = harness(1440.0, 900.0, FilmcraftApp::new(filmcraft_engine::Session::default()));
        h.get_by_label("Vertical 9:16").click();
        h.run_steps(3);
        let q = h.state().session.active_sequence().expect("a sequence");
        assert_eq!((q.settings.width, q.settings.height), (1080, 1920), "status: {}", h.state().ui.status);
    }

    #[derive(Debug)]
    struct OsFile(std::path::PathBuf);
    impl egui::DroppedFile for OsFile {
        fn path(&self) -> &std::path::Path {
            &self.0
        }
        fn bytes(&self) -> Result<Vec<u8>, String> {
            std::fs::read(&self.0).map_err(|e| e.to_string())
        }
    }

    /// A still dropped on Home becomes a project named after it with a sequence of its size.
    #[test]
    fn home_drop_zone_accepts_files() {
        let dir = tempfile::tempdir().unwrap();
        let still = dir.path().join("beach.png");
        image::RgbaImage::from_pixel(64, 36, image::Rgba([40, 140, 220, 255])).save(&still).unwrap();
        let mut h = harness(1440.0, 900.0, FilmcraftApp::new(filmcraft_engine::Session::default()));
        h.input_mut().dropped_files.push(Arc::new(OsFile(still)));
        h.run_steps(2);
        let s = &h.state().session;
        assert_eq!(s.project.name, "beach", "status: {}", h.state().ui.status);
        let q = s.active_sequence().expect("a sequence from the still");
        assert_eq!((q.settings.width, q.settings.height), (64, 36));
        assert_eq!(q.video_tracks.iter().map(|t| t.items.len()).sum::<usize>(), 1);
        // nothing usable dropped: an error, no project
        let mut h = harness(1440.0, 900.0, FilmcraftApp::new(filmcraft_engine::Session::default()));
        h.input_mut().dropped_files.push(Arc::new(OsFile(dir.path().join("missing.mov"))));
        h.run_steps(2);
        assert!(!h.state().ui.status.is_empty());
        assert!(h.state().session.active_sequence().is_none());
    }

    #[test]
    fn home_outline_closes() {
        let pts = super::rounded_outline(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 60.0)), 12.0);
        assert_eq!(pts.first(), pts.last());
    }
}
