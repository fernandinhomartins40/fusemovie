//! Mode bar: Assemble · Edit · Color · Audio · Captions, centred under the top bar, the active
//! one in amber. A mode picks the work area and the right panel's tabs.

use egui::{Rect, Sense, Stroke, pos2, vec2};

use super::{Mode, style};
use crate::FilmcraftApp;
use crate::icons::{self, Icon};
use crate::theme::Tokens;

/// The English source string of `mode` (`studio.mode` context in the catalogs: "Edit" here is
/// the cutting mode, not the Edit menu).
pub fn label(mode: Mode) -> &'static str {
    match mode {
        Mode::Assemble => "Assemble",
        Mode::Edit => "Edit",
        Mode::Color => "Color",
        Mode::Audio => "Audio",
        Mode::Captions => "Captions",
    }
}

/// `mode`'s label in the interface language.
pub fn shown(mode: Mode) -> &'static str {
    crate::i18n::tr_ctx("studio.mode", label(mode))
}

fn icon(mode: Mode) -> Icon {
    match mode {
        Mode::Assemble => Icon::Film,
        Mode::Edit => Icon::Razor,
        Mode::Color => Icon::Adjust,
        Mode::Audio => Icon::Audio,
        Mode::Captions => Icon::Captions,
    }
}

pub fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    let p = ui.painter().clone();
    p.rect_filled(rect, 0.0, t.chrome);
    p.hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, t.separator));
    let compact = style::compact(ui.ctx());
    let font = Tokens::ui(if compact { 13.5 } else { 15.0 });
    let (icon_w, gap, pad, sep_gap) = (18.0, 9.0, if compact { 12.0 } else { 18.0 }, if compact { 8.0 } else { 14.0 });
    let items: Vec<_> = Mode::ALL.iter().map(|&m| (m, p.layout_no_wrap(shown(m).to_owned(), font.clone(), t.text))).collect();
    let widths: Vec<f32> = items.iter().map(|(_, g)| pad * 2.0 + icon_w + gap + g.size().x).collect();
    let total = widths.iter().sum::<f32>() + (items.len() - 1) as f32 * (sep_gap * 2.0 + 1.0);
    let mut x = (rect.center().x - total / 2.0).max(rect.left() + 8.0);
    let h = (rect.height() - 12.0).max(20.0);
    let exporting = app.ui.mode == crate::state::Mode::Export;
    for (i, ((mode, galley), w)) in items.into_iter().zip(widths).enumerate() {
        if i > 0 {
            x += sep_gap;
            p.vline(x, (rect.center().y - 11.0)..=(rect.center().y + 11.0), Stroke::new(1.0, t.separator));
            x += 1.0 + sep_gap;
        }
        let r = Rect::from_min_size(pos2(x, rect.center().y - h / 2.0), vec2(w, h));
        x += w;
        let resp = ui.interact(r, egui::Id::unique(("studio-mode", i)), Sense::click());
        let active = app.studio.mode == mode && !exporting;
        resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, active, shown(mode)));
        app.auto.add(&format!("studio.mode.{}", label(mode).to_ascii_lowercase()), r, label(mode));
        let ink = if active { t.primary_text } else { t.text };
        if active {
            p.rect_filled(r, style::RADIUS, t.primary_bg);
        } else if resp.hovered() {
            p.rect_filled(r, style::RADIUS, t.hover);
        }
        let ir = Rect::from_min_size(pos2(r.left() + pad, r.center().y - icon_w / 2.0), vec2(icon_w, icon_w));
        icons::paint(&p, ir, icon(mode), ink);
        p.galley_with_override_text_color(pos2(ir.right() + gap, r.center().y - galley.size().y / 2.0), galley, ink);
        if resp.clicked() {
            set_mode(app, mode);
        }
    }
}

/// Switch to `mode` (leaving the Export page), handing the right panel back to the mode.
pub fn set_mode(app: &mut FilmcraftApp, mode: Mode) {
    app.studio.mode = mode;
    app.studio.pick = None;
    if app.ui.mode != crate::state::Mode::Edit {
        app.ui.mode = crate::state::Mode::Edit;
    }
}
