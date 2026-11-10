//! The tool column at the left of the work area: the active tool's button (it opens every
//! timeline tool in two columns), the quick strip of the 8 most used tools (active in amber, the
//! shortcut in the tooltip) and the floating options pill beside it. The tools are the Timeline's
//! own (`state::Tool`, the Tools panel's); picking one sets `app.ui.tool` as that panel does.

use egui::{Rect, Stroke, StrokeKind, pos2, vec2};

use super::style;
use crate::FilmcraftApp;
use crate::state::Tool;

/// The quick strip, top to bottom.
pub const QUICK: [Tool; 8] = [Tool::Selection, Tool::Razor, Tool::Ripple, Tool::Slip, Tool::Pen, Tool::Type, Tool::Zoom, Tool::Hand];

const PAD: f32 = 6.0;

fn cell(compact: bool) -> f32 {
    if compact { 30.0 } else { 36.0 }
}

/// Quick-strip columns that fit `height` below the tool button.
fn columns(height: f32, compact: bool) -> usize {
    let c = cell(compact);
    let rows = (((height - PAD * 3.0 - c - 4.0) / c).floor().max(1.0)) as usize;
    QUICK.len().div_ceil(rows).max(1)
}

/// The column's width (with its gutter) in `work`.
pub fn column_width(_app: &FilmcraftApp, work: Rect) -> f32 {
    let compact = work.height() < 330.0 || work.width() < 900.0;
    columns(work.height(), compact) as f32 * cell(compact) + PAD * 2.0 + 12.0
}

pub fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui, work: Rect) {
    let t = app.tokens;
    let compact = work.height() < 330.0 || work.width() < 900.0;
    let c = cell(compact);
    let cols = columns(work.height(), compact);
    let card = Rect::from_min_size(work.min + vec2(6.0, 6.0), vec2(cols as f32 * c + PAD * 2.0, work.height() - 12.0));
    let rows = QUICK.len().div_ceil(cols);
    let card = Rect::from_min_size(card.min, vec2(card.width(), (PAD * 3.0 + c + 4.0 + rows as f32 * c).min(card.height())));
    ui.painter().rect(card, style::RADIUS, t.card, Stroke::new(1.0, t.card_border), StrokeKind::Inside);
    let ctx = ui.ctx().clone();

    // The tool button: the active tool; it opens the full list.
    let button = Rect::from_min_size(card.min + vec2(PAD, PAD), vec2(card.width() - PAD * 2.0, c + 4.0));
    let tip = crate::panels::tools::tooltip(app, app.ui.tool);
    let resp = style::icon_button(ui, button, egui::Id::unique("studio-tool-button"), app.ui.tool.icon(), app.studio.tools_open, &t, &tip);
    app.auto.add("studio.tools.button", button, &tip);
    if resp.clicked() {
        app.studio.tools_open = !app.studio.tools_open;
    }
    let sep_y = button.bottom() + PAD / 2.0;
    ui.painter().hline((card.left() + PAD)..=(card.right() - PAD), sep_y, Stroke::new(1.0, t.separator));

    // The quick strip.
    for (i, tool) in QUICK.into_iter().enumerate() {
        let (row, col) = (i / cols, i % cols);
        let r = Rect::from_min_size(pos2(card.left() + PAD + col as f32 * c, sep_y + PAD / 2.0 + row as f32 * c), vec2(c, c)).shrink(2.0);
        if r.bottom() > card.bottom() {
            break;
        }
        let tip = crate::panels::tools::tooltip(app, tool);
        let resp = style::icon_button(ui, r, egui::Id::unique(("studio-quick-tool", i)), tool.icon(), app.ui.tool == tool, &t, &tip);
        app.auto.add_response(&format!("studio.tool.{tool:?}"), &resp, tool.label());
        if resp.clicked() {
            app.ui.tool = tool;
        }
    }
    options_pill(app, ui, &ctx, pos2(card.right() + 10.0, work.top() + 8.0), work.right() - 8.0);
    if app.studio.tools_open {
        tool_list(app, ui, &ctx, button);
    }
}

/// Every timeline tool in two columns, next to the tool button.
fn tool_list(app: &mut FilmcraftApp, ui: &mut egui::Ui, ctx: &egui::Context, button: Rect) {
    let t = app.tokens;
    let area = egui::Area::new(egui::Id::unique("studio-tool-list")).order(egui::Order::Foreground).fixed_pos(button.right_top() + vec2(10.0, 0.0)).show(ctx, |ui| {
        egui::Frame::popup(ui.style()).fill(t.card).corner_radius(style::RADIUS).show(ui, |ui| {
            egui::Grid::new("studio-tool-grid").num_columns(2).spacing(vec2(6.0, 2.0)).show(ui, |ui| {
                for (i, tool) in Tool::ALL.into_iter().enumerate() {
                    let text = crate::panels::tools::tooltip(app, tool);
                    let row = ui.add(egui::Button::selectable(app.ui.tool == tool, text).min_size(vec2(200.0, 26.0)));
                    app.auto.add_response(&format!("studio.tools.list.{tool:?}"), &row, tool.label());
                    if row.clicked() {
                        app.ui.tool = tool;
                        app.studio.tools_open = false;
                    }
                    if i % 2 == 1 {
                        ui.end_row();
                    }
                }
            });
        });
    });
    if area.response.clicked_elsewhere() && !ui.input(|i| i.pointer.interact_pos().is_some_and(|p| button.contains(p))) {
        app.studio.tools_open = false;
    }
}

/// The options pill: the active tool's name and the timeline switches every tool uses.
fn options_pill(app: &mut FilmcraftApp, ui: &mut egui::Ui, ctx: &egui::Context, at: egui::Pos2, right: f32) {
    let t = app.tokens;
    let name = app.ui.language.tr(app.ui.tool.label()).to_owned();
    let galley = ui.painter().layout_no_wrap(name, crate::theme::Tokens::semibold(12.5), t.text);
    let snap = (tl!("Snap"), "sequence.snap", app.session.state.snapping);
    let linked = (tl!("Linked Selection"), "sequence.linkedSelection", app.session.state.linked_selection);
    let widths: f32 = [snap.0, linked.0].iter().map(|l| ui.painter().layout_no_wrap((*l).to_owned(), crate::theme::Tokens::ui(12.5), t.text).size().x + 26.0).sum();
    let w = galley.size().x + 24.0 + widths + 8.0;
    if at.x + w > right {
        return;
    }
    let pill = Rect::from_min_size(at, vec2(w, 36.0));
    ui.painter().rect(pill, 18.0, t.card, Stroke::new(1.0, t.card_border), StrokeKind::Inside);
    let mut x = pill.left() + 14.0;
    ui.painter().galley(pos2(x, pill.center().y - galley.size().y / 2.0), galley.clone(), t.text);
    x += galley.size().x + 10.0;
    for (label, id, on) in [snap, linked] {
        let resp = style::pill(ui, pos2(x, pill.top() + 5.0), egui::Id::unique(("studio-option", id)), label, on, &t);
        app.auto.add_response(&format!("studio.option.{id}"), &resp, label);
        x = resp.rect.right() + 6.0;
        if resp.clicked() {
            super::run(app, ctx, id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn studio_quick_strip_wraps_into_columns_when_short() {
        assert_eq!(columns(600.0, false), 1);
        assert_eq!(columns(200.0, true), 2);
        assert!(columns(10.0, true) <= QUICK.len());
    }
}
