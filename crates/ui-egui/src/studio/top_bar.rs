//! Top bar: ☰ (every menu of the menu bar), product name, project name, undo / redo, command
//! search, save status, the Codex wand and the amber Export button.

use egui::{Align2, Rect, Sense, Stroke, StrokeKind, pos2, vec2};

use super::style;
use crate::FilmcraftApp;
use crate::icons::{self, Icon};
use crate::theme::Tokens;

/// The command search field's text and whether its results are showing.
#[derive(Default)]
pub struct Search {
    pub query: String,
    /// The results list is showing (from typing until a pick, Esc or a click elsewhere).
    pub open: bool,
}

/// Results the search shows at most.
const MAX_RESULTS: usize = 10;

pub fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    let ctx = ui.ctx().clone();
    ui.painter().rect_filled(rect, 0.0, t.chrome);
    ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, t.separator));
    let compact = style::compact(&ctx);
    let cy = rect.center().y;
    let mut run: Option<String> = None;

    // Left: ☰, name, project, undo / redo.
    let mut x = rect.left() + 10.0;
    let menu = Rect::from_min_size(pos2(x, cy - 17.0), vec2(34.0, 34.0));
    let resp = style::icon_button(ui, menu, egui::Id::unique("studio-menu"), Icon::Hamburger, false, &t, tl!("Menu"));
    app.auto.add("studio.menu", menu, "Menu");
    egui::Popup::menu(&resp).show(|ui| {
        if let Some(id) = crate::menus::menu_tree(app, ui) {
            run = Some(id);
        }
    });
    x = menu.right() + 10.0;
    let name = ui.painter().text(pos2(x, cy), Align2::LEFT_CENTER, style::APP_NAME, Tokens::semibold(if compact { 16.0 } else { 18.0 }), t.text);
    x = name.right() + 12.0;
    ui.painter().vline(x, (cy - 10.0)..=(cy + 10.0), Stroke::new(1.0, t.separator));
    x += 12.0;
    let project = app.session.project.name.clone();
    let mut job = egui::text::LayoutJob::simple_singleline(project, Tokens::ui(13.0), t.text_dim);
    job.wrap = egui::text::TextWrapping::truncate_at_width(if compact { 140.0 } else { 220.0 });
    let galley = ui.painter().layout_job(job);
    let w = galley.size().x;
    ui.painter().galley(pos2(x, cy - galley.size().y / 2.0), galley, t.text_dim);
    x += w + 14.0;
    for (icon, id, tip, enabled) in
        [(Icon::Undo, "edit.undo", tl!("Undo"), app.session.history.can_undo()), (Icon::Redo, "edit.redo", tl!("Redo"), app.session.history.can_redo())]
    {
        let r = Rect::from_min_size(pos2(x, cy - 15.0), vec2(30.0, 30.0));
        let resp = ui.add_enabled_ui(enabled, |ui| style::icon_button(ui, r, egui::Id::unique(("studio-history", id)), icon, false, &t, tip)).inner;
        app.auto.add(&format!("studio.{id}"), r, tip);
        if resp.clicked() {
            run = Some(id.to_string());
        }
        x += 34.0;
    }
    let left_end = x + 8.0;

    // Right: Export, the Codex wand, the save status.
    let mut rx = rect.right() - 12.0;
    let label = tl!("Export");
    let galley = ui.painter().layout_no_wrap(label.to_owned(), Tokens::semibold(14.0), t.primary_text);
    let export = Rect::from_min_max(pos2(rx - galley.size().x - 48.0, cy - 17.0), pos2(rx, cy + 17.0));
    let resp = ui.interact(export, egui::Id::unique("studio-export"), Sense::click()).on_hover_text(tl!("Export"));
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
    app.auto.add("studio.export", export, "Export");
    let fill = if resp.hovered() { t.accent_hover } else { t.primary_bg };
    ui.painter().rect_filled(export, style::RADIUS, fill);
    icons::paint(ui.painter(), Rect::from_center_size(pos2(export.left() + 20.0, cy), vec2(16.0, 16.0)), Icon::Export, t.primary_text);
    ui.painter().galley(pos2(export.left() + 36.0, cy - galley.size().y / 2.0), galley, t.primary_text);
    if resp.clicked() {
        app.ui.mode = crate::state::Mode::Export;
    }
    rx = export.left() - 10.0;
    let wand = Rect::from_min_max(pos2(rx - 34.0, cy - 17.0), pos2(rx, cy + 17.0));
    if style::icon_button(ui, wand, egui::Id::unique("studio-codex"), Icon::Sparkle, app.studio.codex_open, &t, "Codex").clicked() {
        app.studio.codex_open = !app.studio.codex_open;
    }
    app.auto.add("studio.codex", wand, "Codex");
    rx = wand.left() - 14.0;
    if !compact {
        let text = if app.session.is_dirty() { tl!("Not saved") } else { tl!("Saved") };
        let r = ui.painter().text(pos2(rx, cy), Align2::RIGHT_CENTER, text, Tokens::ui(13.0), t.text_dim);
        if app.session.is_dirty() {
            ui.painter().circle_filled(pos2(r.left() - 8.0, cy), 3.5, t.accent);
        }
        rx = r.left() - 24.0;
    }

    // Centre: the command search, centred when it fits.
    let room = rx - left_end;
    if room >= 140.0 {
        let w = room.min(380.0);
        let x = (rect.center().x - w / 2.0).clamp(left_end, rx - w);
        let field = Rect::from_min_size(pos2(x, cy - 16.0), vec2(w, 32.0));
        if let Some(id) = search(app, ui, field) {
            run = Some(id);
        }
    }
    if let Some(id) = run {
        super::run(app, &ctx, &id);
    }
}

/// The command search: type to list menu commands (in any language, or by their English name),
/// Enter or a click runs one. Returns the command to run.
fn search(app: &mut FilmcraftApp, ui: &mut egui::Ui, field: Rect) -> Option<String> {
    let t = app.tokens;
    let p = ui.painter();
    p.rect(field, style::RADIUS, t.field_bg, Stroke::new(1.0, t.field_border), StrokeKind::Inside);
    icons::paint(p, Rect::from_center_size(pos2(field.left() + 18.0, field.center().y), vec2(15.0, 15.0)), Icon::Search, t.text_dim);
    let edit = Rect::from_min_max(pos2(field.left() + 34.0, field.top() + 4.0), pos2(field.right() - 10.0, field.bottom() - 4.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(edit).id_salt("studio-search").layout(egui::Layout::left_to_right(egui::Align::Center)));
    let resp = child.add(
        egui::TextEdit::singleline(&mut app.studio.search.query)
            .frame(egui::Frame::NONE)
            .hint_text(tl!("Search command"))
            .font(Tokens::ui(13.0))
            .desired_width(edit.width()),
    );
    app.auto.add_response("studio.search", &resp, "Search command");
    let query = app.studio.search.query.trim().to_owned();
    if resp.has_focus() && resp.changed() {
        app.studio.search.open = true;
    }
    if query.is_empty() || !app.studio.search.open {
        return None;
    }
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.studio.search.query.clear();
        app.studio.search.open = false;
        return None;
    }
    let found: Vec<(String, String)> = crate::menus::menu_items(app)
        .into_iter()
        .filter(|it| it.enabled && (crate::i18n::matches_query(&it.label, &query) || it.path.iter().any(|s| crate::i18n::matches_query(s, &query))))
        .take(MAX_RESULTS)
        .map(|it| {
            let path: Vec<&str> = it.path.iter().map(|s| crate::i18n::t(s)).collect();
            (it.id, if path.is_empty() { it.label } else { format!("{} ▸ {}", path.join(" ▸ "), it.label) })
        })
        .collect();
    let mut pick = None;
    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        pick = found.first().map(|(id, _)| id.clone());
    }
    let area = egui::Area::new(egui::Id::unique("studio-search-results")).order(egui::Order::Foreground).fixed_pos(field.left_bottom() + vec2(0.0, 4.0)).show(
        ui.ctx(),
        |ui| {
            egui::Frame::popup(ui.style()).fill(t.card).show(ui, |ui| {
                ui.set_width(field.width() - 16.0);
                if found.is_empty() {
                    ui.label(egui::RichText::new(tl!("No commands found")).color(t.text_dim));
                }
                for (id, label) in &found {
                    let row = ui.add(egui::Button::new(label.as_str()).frame(false).truncate());
                    app.auto.add_response(&format!("studio.search.{id}"), &row, label);
                    if row.clicked() {
                        pick = Some(id.clone());
                    }
                }
            });
        },
    );
    if area.response.clicked_elsewhere() && !resp.hovered() {
        app.studio.search.open = false;
    }
    if pick.is_some() {
        app.studio.search.query.clear();
        app.studio.search.open = false;
    }
    pick
}
