//! Studio measures (`docs/fuse-suite/02-studio-design.md`, normal / compact) and the shared
//! button painting. Colours come from `theme::Tokens`.

use egui::{Rect, Response, Sense, Stroke, StrokeKind, pos2, vec2};

use crate::icons::{self, Icon};
use crate::theme::Tokens;

/// The product name, from `brand.env` at the repository root (exported by `build.rs`).
pub const APP_NAME: &str = env!("FILMCRAFT_BRAND_NAME");

/// The bottom strip's default share of the window height: the Timeline needs room for tracks.
pub const STRIP_FRAC: f32 = 0.38;
/// Corner radius of cards and pills.
pub const RADIUS: f32 = 10.0;

/// Laptop-sized window (a 1366×768 screen at 125% is about 1093×614): the chrome shrinks.
pub fn compact(ctx: &egui::Context) -> bool {
    let r = ctx.content_rect();
    r.height() < 800.0 || r.width() < 1280.0
}

/// Region sizes for normal or compact windows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sizes {
    pub top: f32,
    pub modes: f32,
    pub status: f32,
    pub props: f32,
    pub rail: f32,
    pub right: f32,
    pub codex: f32,
    /// The bottom strip's least height.
    pub strip_min: f32,
}

impl Sizes {
    pub fn new(compact: bool) -> Self {
        if compact {
            Sizes { top: 44.0, modes: 42.0, status: 28.0, props: 36.0, rail: 60.0, right: 300.0, codex: 340.0, strip_min: 112.0 }
        } else {
            Sizes { top: 52.0, modes: 54.0, status: 34.0, props: 42.0, rail: 80.0, right: 360.0, codex: 430.0, strip_min: 170.0 }
        }
    }
}

/// The Codex column's width: at least 260, at most 60% of the window.
pub fn codex_width(w: f32, window: f32) -> f32 {
    let max = (window * 0.6).max(260.0);
    if w.is_finite() { w.clamp(260.0, max) } else { 260.0 }
}

/// The bottom strip's height: `frac` of the window, at least the size's minimum, leaving the
/// work area at least 120 of the `room` above the status line.
pub fn strip_height(frac: f32, window: f32, room: f32, compact: bool) -> f32 {
    let min = Sizes::new(compact).strip_min;
    let frac = if frac.is_finite() { frac.clamp(0.15, 0.75) } else { STRIP_FRAC };
    let max = (room - 120.0).max(0.0);
    (window * frac).max(min).min(max)
}

/// A square icon button: amber fill when `active`, hover tint otherwise.
pub fn icon_button(ui: &mut egui::Ui, rect: Rect, id: egui::Id, icon: Icon, active: bool, t: &Tokens, tip: &str) -> Response {
    let resp = ui.interact(rect, id, Sense::click()).on_hover_text(tip);
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, active, tip));
    let p = ui.painter();
    let fill = if active {
        t.primary_bg
    } else if resp.hovered() {
        t.hover
    } else {
        egui::Color32::TRANSPARENT
    };
    p.rect_filled(rect, 8.0, fill);
    let ink = if active { t.primary_text } else if resp.hovered() { t.text } else { t.icon };
    let s = (rect.height() * 0.5).clamp(12.0, 20.0);
    icons::paint(p, Rect::from_center_size(rect.center(), vec2(s, s)), icon, ink);
    resp
}

/// A text pill (tabs, toggles): amber outline and text when `on`.
pub fn pill(ui: &mut egui::Ui, at: egui::Pos2, id: egui::Id, label: &str, on: bool, t: &Tokens) -> Response {
    let font = Tokens::ui(12.5);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, t.text);
    let rect = Rect::from_min_size(at, vec2(galley.size().x + 20.0, 26.0));
    let resp = ui.interact(rect, id, Sense::click());
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, on, label));
    let p = ui.painter();
    let fill = if resp.hovered() { t.hover } else { t.card };
    let border = if on { t.accent } else { t.card_border };
    p.rect(rect, 8.0, fill, Stroke::new(1.0, border), StrokeKind::Inside);
    let ink = if on { t.accent } else { t.text };
    p.galley_with_override_text_color(pos2(rect.left() + 10.0, rect.center().y - galley.size().y / 2.0), galley, ink);
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn studio_sizes_follow_the_design_doc() {
        let (n, c) = (Sizes::new(false), Sizes::new(true));
        assert_eq!((n.top, n.modes, n.status, n.props, n.rail, n.codex), (52.0, 54.0, 34.0, 42.0, 80.0, 430.0));
        assert_eq!((c.top, c.modes, c.status, c.props, c.rail, c.codex), (44.0, 42.0, 28.0, 36.0, 60.0, 340.0));
    }

    #[test]
    fn studio_codex_width_stays_between_260_and_60_percent() {
        assert_eq!(codex_width(430.0, 1440.0), 430.0);
        assert_eq!(codex_width(100.0, 1440.0), 260.0);
        assert_eq!(codex_width(2000.0, 1000.0), 600.0);
        assert_eq!(codex_width(f32::NAN, 1000.0), 260.0);
    }

    #[test]
    fn studio_strip_height_keeps_room_for_the_monitor() {
        assert_eq!(strip_height(0.38, 900.0, 800.0, false), 342.0);
        assert_eq!(strip_height(0.1, 900.0, 800.0, false), 170.0, "never below the minimum");
        assert_eq!(strip_height(0.75, 900.0, 500.0, false), 380.0, "the work area keeps 120");
        assert_eq!(strip_height(f32::NAN, 1000.0, 900.0, false), 380.0);
    }
}
