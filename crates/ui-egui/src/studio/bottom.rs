//! The bottom rows: the selected clip's properties bar, the bottom strip (Timeline · Markers ·
//! Media, the classic panels, resizable from its top edge) and the status line.

use egui::{Align2, Rect, Sense, Stroke, StrokeKind, pos2, vec2};

use super::style;
use crate::FilmcraftApp;
use crate::dock::PanelKind;
use crate::icons::{self, Icon};
use crate::theme::Tokens;

/// The bottom strip's tab.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Timeline,
    Markers,
    Media,
}

impl Tab {
    pub const ALL: [Tab; 3] = [Tab::Timeline, Tab::Markers, Tab::Media];
    fn label(self) -> &'static str {
        match self {
            Tab::Timeline => "Timeline",
            Tab::Markers => "Markers",
            Tab::Media => "Media",
        }
    }
    fn icon(self) -> Icon {
        match self {
            Tab::Timeline => Icon::Sequence,
            Tab::Markers => Icon::Marker,
            Tab::Media => Icon::Folder,
        }
    }
}

/// The classic panel a tab shows.
pub fn panel_of(tab: Tab) -> PanelKind {
    match tab {
        Tab::Timeline => PanelKind::Timeline,
        Tab::Markers => PanelKind::Markers,
        Tab::Media => PanelKind::MediaBrowser,
    }
}

/// A time as the sequence shows it (timecode, feet+frames or frames).
fn time_text(app: &FilmcraftApp, t: filmcraft_time::Tick) -> String {
    match app.session.active_sequence() {
        Some(q) => {
            let s = &q.settings;
            filmcraft_time::format_time(t, s.frame_rate, s.drop_frame, s.display, i64::from(s.sample_rate))
        }
        None => String::new(),
    }
}

/// The properties bar: the selected clip's name, start, duration and speed, and its switches.
pub fn props(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    let ctx = ui.ctx().clone();
    ui.painter().rect_filled(rect, 0.0, t.chrome);
    ui.painter().hline(rect.x_range(), rect.top() + 0.5, Stroke::new(1.0, t.separator));
    let cy = rect.center().y;
    let clip = app.session.state.selection.first().and_then(|id| app.session.active_sequence()?.find_item(*id).map(|(_, c)| c.clone()));
    let Some(clip) = clip else {
        ui.painter().text(pos2(rect.left() + 16.0, cy), Align2::LEFT_CENTER, tl!("No clip selected"), Tokens::ui(13.0), t.text_faint);
        return;
    };
    let mut x = rect.left() + 16.0;
    let swatch = crate::menus::label_color(&app.session.prefs.labels, clip.label);
    ui.painter().rect_filled(Rect::from_center_size(pos2(x + 5.0, cy), vec2(10.0, 10.0)), 3.0, swatch);
    x += 18.0;
    let mut job = egui::text::LayoutJob::simple_singleline(clip.name.clone(), Tokens::semibold(13.0), t.text);
    job.wrap = egui::text::TextWrapping::truncate_at_width(220.0);
    let g = ui.painter().layout_job(job);
    let w = g.size().x;
    ui.painter().galley(pos2(x, cy - g.size().y / 2.0), g, t.text);
    x += w + 20.0;
    let speed = format!("{} %", (clip.speed * 100.0 * if clip.reverse { -1.0 } else { 1.0 }).round());
    for (label, value) in [(tl!("Start"), time_text(app, clip.start)), (tl!("Duration"), time_text(app, clip.duration)), (tl!("Speed"), speed)] {
        let l = ui.painter().text(pos2(x, cy), Align2::LEFT_CENTER, format!("{label}:"), Tokens::ui(12.5), t.text_dim);
        let v = ui.painter().text(pos2(l.right() + 6.0, cy), Align2::LEFT_CENTER, value, Tokens::mono(12.5), t.text);
        x = v.right() + 18.0;
        if x > rect.right() - 260.0 {
            break;
        }
    }
    let mut x = (rect.right() - 250.0).max(x);
    for (label, id, on) in [(tl!("Enabled"), "clip.enable", clip.enabled), (tl!("Speed/Duration…"), "clip.speedDuration", false)] {
        let resp = style::pill(ui, pos2(x, cy - 13.0), egui::Id::unique(("studio-props", id)), label, on, &t);
        app.auto.add_response(&format!("studio.props.{id}"), &resp, label);
        x = resp.rect.right() + 8.0;
        if resp.clicked() {
            super::run(app, &ctx, id);
        }
    }
}

/// The bottom strip: its tabs at the left, the tab's panel beside them, a drag edge on top.
pub fn strip(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect, window_h: f32) {
    let t = app.tokens;
    ui.painter().rect_filled(rect, 0.0, t.dock);
    ui.painter().hline(rect.x_range(), rect.top() + 0.5, Stroke::new(1.0, t.separator));
    let edge = Rect::from_min_max(pos2(rect.left(), rect.top() - 3.0), pos2(rect.right(), rect.top() + 3.0));
    let resp = ui.interact(edge, egui::Id::unique("studio-strip-edge"), Sense::drag()).on_hover_cursor(egui::CursorIcon::ResizeVertical);
    app.auto.add("studio.strip.edge", edge, "Resize the bottom strip");
    if resp.dragged() && window_h > 0.0 {
        app.studio.strip_frac = ((rect.height() - resp.drag_delta().y) / window_h).clamp(0.15, 0.75);
    }
    let compact = style::compact(ui.ctx());
    let tabs_w = if compact { 48.0 } else { 124.0 };
    let card = Rect::from_min_size(rect.min + vec2(8.0, 8.0), vec2(tabs_w - 8.0, (Tab::ALL.len() as f32 * 36.0 + 8.0).min(rect.height() - 16.0)));
    ui.painter().rect(card, style::RADIUS, t.card, Stroke::new(1.0, t.card_border), StrokeKind::Inside);
    for (i, tab) in Tab::ALL.into_iter().enumerate() {
        let r = Rect::from_min_size(card.min + vec2(4.0, 4.0 + i as f32 * 36.0), vec2(card.width() - 8.0, 34.0));
        if r.bottom() > card.bottom() {
            break;
        }
        let on = app.studio.bottom == tab;
        let name = crate::i18n::t(tab.label());
        let resp = ui.interact(r, egui::Id::unique(("studio-strip-tab", i)), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, on, name));
        app.auto.add(&format!("studio.strip.{}", tab.label().to_ascii_lowercase()), r, tab.label());
        if on || resp.hovered() {
            ui.painter().rect_filled(r, 8.0, if on { t.pill_active_bg } else { t.hover });
        }
        let ink = if on { t.accent } else { t.text };
        let icon_x = if compact { r.center().x } else { r.left() + 16.0 };
        icons::paint(ui.painter(), Rect::from_center_size(pos2(icon_x, r.center().y), vec2(16.0, 16.0)), tab.icon(), ink);
        if !compact {
            ui.painter().text(pos2(r.left() + 32.0, r.center().y), Align2::LEFT_CENTER, name, Tokens::ui(13.0), ink);
        }
        let resp = if compact { resp.on_hover_text(name) } else { resp };
        if resp.clicked() {
            app.studio.bottom = tab;
        }
    }
    let body = Rect::from_min_max(pos2(rect.left() + tabs_w + 6.0, rect.top() + 1.0), rect.max);
    super::draw_panel(app, ui, panel_of(app.studio.bottom), body);
}

/// The fps as people say it: 25, 29.97, 23.976.
fn fps_text(fps: f64) -> String {
    let s = format!("{fps:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// "Fit · 00:00:12:05 · 25 fps · 1920 × 1080 · 14 clips · Sequence 01".
pub fn status_text(app: &FilmcraftApp) -> String {
    let zoom = match app.ui.program.zoom {
        Some(z) if z.is_finite() => format!("{} %", (z * 100.0).round()),
        _ => tl!("Fit").to_owned(),
    };
    let Some(q) = app.session.active_sequence() else { return zoom };
    let s = &q.settings;
    let clips: usize = q.all_tracks().map(|tr| tr.items.len()).sum();
    let name = app.session.state.active_sequence.and_then(|id| app.session.project.item(id)).map(|i| i.name.clone()).unwrap_or_default();
    let fps = tlf!("{fps} fps", fps = fps_text(s.frame_rate.as_f64()));
    let clips = tlf!("{n} clips", n = clips);
    [zoom, time_text(app, app.session.playhead()), fps, format!("{} × {}", s.width, s.height), clips, name].join("  ·  ")
}

/// The status line: the sequence's facts at the left, the running job and the last message at the right.
pub fn status(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect) {
    let t = app.tokens;
    ui.painter().rect_filled(rect, 0.0, t.chrome);
    ui.painter().hline(rect.x_range(), rect.top() + 0.5, Stroke::new(1.0, t.separator));
    let text = status_text(app);
    let left = ui.painter().text(pos2(rect.left() + 16.0, rect.center().y), Align2::LEFT_CENTER, &text, Tokens::ui(12.5), t.text_dim);
    app.auto.add("studio.status", left, &text);
    let right = app.job_status(ui, rect.shrink2(vec2(10.0, 0.0)), &t).unwrap_or(rect.right() - 12.0);
    let message = app.status_message(ui.ctx());
    if !message.is_empty() && right - left.right() > 80.0 {
        let mut job = egui::text::LayoutJob::simple_singleline(message.clone(), Tokens::ui(12.0), t.text_dim);
        job.wrap = egui::text::TextWrapping::truncate_at_width(right - left.right() - 40.0);
        let g = ui.painter().layout_job(job);
        let at = pos2(right - 10.0 - g.size().x, rect.center().y - g.size().y / 2.0);
        let r = Rect::from_min_size(at, g.size());
        ui.painter().galley(at, g, t.text_dim);
        if ui.interact(r, egui::Id::unique("studio-status-message"), Sense::click()).on_hover_text(&message).clicked() {
            app.ui.status.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn studio_fps_text_drops_trailing_zeros() {
        assert_eq!(fps_text(25.0), "25");
        assert_eq!(fps_text(30000.0 / 1001.0), "29.97");
        assert_eq!(fps_text(24000.0 / 1001.0), "23.976");
    }
}
