//! The right side: the contextual panel (the mode's panels as tabs, or what the rail picked) and
//! the icon rail (History, Info, Markers, Panels…). Every panel is the classic one (`panels::show`).

use egui::{Align2, Rect, Sense, Stroke, StrokeKind, pos2, vec2};

use super::{Pick, bottom, style};
use crate::FilmcraftApp;
use crate::dock::PanelKind;
use crate::icons::{self, Icon};
use crate::theme::Tokens;

const TAB_H: f32 = 40.0;

/// Panels the shell shows elsewhere (work area, bottom strip): not offered in "Panels…", so no
/// panel is drawn twice in one frame.
const ELSEWHERE: [PanelKind; 5] = [PanelKind::Program, PanelKind::Source, PanelKind::Timeline, PanelKind::Markers, PanelKind::MediaBrowser];

/// The panels "Panels…" lists.
pub fn listed() -> impl Iterator<Item = PanelKind> {
    PanelKind::ALL.into_iter().filter(|p| !ELSEWHERE.contains(p))
}

/// The panel the right side shows now, if any.
pub fn shown(app: &FilmcraftApp) -> Option<PanelKind> {
    match app.studio.pick {
        Some(Pick::Panel(p)) => Some(p),
        Some(Pick::List) => None,
        None => {
            let panels = app.studio.mode.panels();
            let i = app.studio.tab[app.studio.mode as usize].min(panels.len().saturating_sub(1));
            panels.get(i).copied()
        }
    }
}

pub fn panel(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    if rect.width() < 40.0 || rect.height() < 40.0 {
        return;
    }
    let t = app.tokens;
    ui.painter().rect_filled(rect, 0.0, t.chrome);
    ui.painter().vline(rect.left() + 0.5, rect.y_range(), Stroke::new(1.0, t.separator));
    let head = Rect::from_min_size(rect.min, vec2(rect.width(), TAB_H));
    ui.painter().hline(head.x_range(), head.bottom() - 0.5, Stroke::new(1.0, t.separator));
    let body = Rect::from_min_max(pos2(rect.left() + 1.0, head.bottom()), rect.max);
    match app.studio.pick {
        None => {
            let mode = app.studio.mode;
            let mut x = head.left() + 10.0;
            for (i, p) in mode.panels().iter().enumerate() {
                let on = shown(app) == Some(*p);
                let resp = style::pill(ui, pos2(x, head.top() + 7.0), egui::Id::unique(("studio-right-tab", i)), crate::i18n::t(p.title()), on, &t);
                app.auto.add_response(&format!("studio.right.tab.{}", p.id()), &resp, p.title());
                x = resp.rect.right() + 6.0;
                if resp.clicked() {
                    app.studio.tab[mode as usize] = i;
                }
            }
        }
        Some(pick) => {
            let title = match pick {
                Pick::Panel(p) => crate::i18n::t(p.title()),
                Pick::List => tl!("Panels"),
            };
            ui.painter().text(pos2(head.left() + 14.0, head.center().y), Align2::LEFT_CENTER, title, Tokens::semibold(14.0), t.text);
            let close = Rect::from_center_size(pos2(head.right() - 22.0, head.center().y), vec2(26.0, 26.0));
            if style::icon_button(ui, close, egui::Id::unique("studio-right-close"), Icon::Close, false, &t, tl!("Close")).clicked() {
                app.studio.pick = None;
            }
        }
    }
    match app.studio.pick {
        Some(Pick::List) => panel_list(app, ui, body.shrink(8.0)),
        _ => {
            if let Some(p) = shown(app) {
                super::draw_panel(app, ui, p, body);
            }
        }
    }
}

/// "Panels…": every panel the shell doesn't show elsewhere; a click shows it here.
fn panel_list(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).id_salt("studio-panel-list").layout(egui::Layout::top_down_justified(egui::Align::Min)));
    egui::ScrollArea::vertical().auto_shrink(false).show(&mut child, |ui| {
        for p in listed() {
            let row = ui.add(egui::Button::new(crate::i18n::t(p.title())).min_size(vec2(0.0, 28.0)).frame(false));
            app.auto.add_response(&format!("studio.panels.{}", p.id()), &row, p.title());
            if row.clicked() {
                app.studio.pick = Some(Pick::Panel(p));
            }
        }
    });
}

/// (icon, English label) of the rail's tiles, top to bottom.
const TILES: [(Icon, &str); 4] = [(Icon::Stopwatch, "History"), (Icon::Info, "Info"), (Icon::Marker, "Markers"), (Icon::Grid, "Panels")];

pub fn rail(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    ui.painter().rect_filled(rect, 0.0, t.canvas);
    let compact = style::compact(ui.ctx());
    let (w, h) = if compact { (44.0, 44.0) } else { (64.0, 66.0) };
    let mut y = rect.top() + 10.0;
    for (icon, label) in TILES {
        let r = Rect::from_min_size(pos2(rect.center().x - w / 2.0, y), vec2(w, h));
        y += h + 8.0;
        if r.bottom() > rect.bottom() {
            break;
        }
        let on = match label {
            "History" => app.studio.pick == Some(Pick::Panel(PanelKind::History)),
            "Info" => app.studio.pick == Some(Pick::Panel(PanelKind::Info)),
            "Markers" => app.studio.bottom == bottom::Tab::Markers,
            _ => app.studio.pick == Some(Pick::List),
        };
        let name = crate::i18n::tr_ctx("studio.rail", label);
        let resp = ui.interact(r, egui::Id::unique(("studio-rail", label)), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, on, name));
        app.auto.add(&format!("studio.rail.{}", label.to_ascii_lowercase()), r, label);
        let fill = if resp.hovered() { t.hover } else { t.card };
        let border = if on { t.accent } else { t.card_border };
        ui.painter().rect(r, 12.0, fill, Stroke::new(1.0, border), StrokeKind::Inside);
        let tint = if on { t.accent } else { t.icon };
        let icon_y = if compact { r.center().y } else { r.top() + 25.0 };
        icons::paint(ui.painter(), Rect::from_center_size(pos2(r.center().x, icon_y), vec2(20.0, 20.0)), icon, tint);
        if !compact {
            ui.painter().text(pos2(r.center().x, r.bottom() - 15.0), Align2::CENTER_CENTER, name, Tokens::ui(11.0), if on { t.accent } else { t.text });
        }
        let resp = if compact { resp.on_hover_text(name) } else { resp };
        if resp.clicked() {
            match label {
                "History" => app.studio.pick = toggle(app.studio.pick, Pick::Panel(PanelKind::History)),
                "Info" => app.studio.pick = toggle(app.studio.pick, Pick::Panel(PanelKind::Info)),
                "Markers" => {
                    app.studio.bottom = if on { bottom::Tab::Timeline } else { bottom::Tab::Markers };
                }
                _ => app.studio.pick = toggle(app.studio.pick, Pick::List),
            }
        }
    }
}

/// A tile click: pick `key`, or give the panel back to the mode when it is already picked.
pub fn toggle(current: Option<Pick>, key: Pick) -> Option<Pick> {
    if current == Some(key) { None } else { Some(key) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn studio_rail_tiles_toggle_and_the_list_skips_panels_shown_elsewhere() {
        let h = Pick::Panel(PanelKind::History);
        assert_eq!(toggle(None, h), Some(h));
        assert_eq!(toggle(Some(h), h), None);
        assert_eq!(toggle(Some(h), Pick::List), Some(Pick::List));
        assert!(listed().all(|p| !ELSEWHERE.contains(&p)));
        assert!(listed().any(|p| p == PanelKind::LumetriColor));
    }
}
