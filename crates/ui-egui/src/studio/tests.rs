//! Headless tests of the Studio shell (kittest). Names contain `studio` so CI's filter runs them.

use egui::{Event, Modifiers, PointerButton, Pos2, Rect, pos2, vec2};
use egui_kittest::Harness;
use filmcraft_engine::Session;
use serde_json::json;

use super::*;
use crate::state::Tool;

fn app() -> FilmcraftApp {
    let mut s = Session::default();
    s.execute("file.openDemoProject", json!({})).unwrap();
    FilmcraftApp::new(s)
}

fn harness_with(app: FilmcraftApp, w: f32, h: f32) -> Harness<'static, FilmcraftApp> {
    let mut h = Harness::builder().with_size(vec2(w, h)).with_max_steps(10_000).build_eframe(move |_cc| app);
    for _ in 0..6 {
        h.step();
    }
    h
}

fn harness(w: f32, h: f32) -> Harness<'static, FilmcraftApp> {
    harness_with(app(), w, h)
}

fn rect_of(h: &Harness<'static, FilmcraftApp>, id: &str) -> Option<Rect> {
    let [x, y, w, hh] = h.state().auto.elements.iter().find(|e| e.id == id)?.rect;
    Some(Rect::from_min_size(pos2(x, y), vec2(w, hh)))
}

fn click(h: &mut Harness<'static, FilmcraftApp>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.step();
    h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.step();
    h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.step();
    h.step();
}

fn click_id(h: &mut Harness<'static, FilmcraftApp>, id: &str) {
    let r = rect_of(h, id).unwrap_or_else(|| panic!("{id} is not drawn"));
    click(h, r.center());
}

#[test]
fn studio_is_the_default_layout_and_theme() {
    let h = harness(1440.0, 900.0);
    assert!(!h.state().studio.classic);
    assert_eq!(h.state().tokens.kind, crate::theme::ThemeKind::Studio);
    assert_eq!(h.state().tokens.accent, egui::Color32::from_rgb(253, 181, 40), "the factory highlight keeps the amber");
}

#[test]
fn studio_shell_renders_at_1440_and_1100() {
    for (w, hh) in [(1440.0, 900.0), (1100.0, 620.0)] {
        let mut h = harness(w, hh);
        for mode in Mode::ALL {
            h.state_mut().studio.mode = mode;
            h.run_steps(3);
            for id in ["studio.menu", "studio.search", "studio.export", "studio.mode.assemble", "studio.tool.Razor", "panel.Program", "panel.Timeline", "studio.status"] {
                let r = rect_of(&h, id).unwrap_or_else(|| panic!("{id} missing at {w}×{hh} in {mode:?}"));
                assert!(r.left() >= 0.0 && r.right() <= w + 1.0 && r.bottom() <= hh + 1.0, "{id} {r:?} outside {w}×{hh}");
            }
            let right = right::shown(h.state()).unwrap();
            assert!(rect_of(&h, &format!("panel.{}", right.id())).is_some(), "{right:?} drawn in {mode:?}");
        }
    }
}

#[test]
fn studio_mode_switch_changes_the_right_panel() {
    let mut h = harness(1440.0, 900.0);
    assert!(rect_of(&h, "panel.Project").is_some(), "Assemble shows the Project bin");
    assert!(rect_of(&h, "panel.Source").is_some(), "and the Source monitor");
    click_id(&mut h, "studio.mode.color");
    assert_eq!(h.state().studio.mode, Mode::Color);
    assert!(rect_of(&h, "panel.LumetriColor").is_some());
    assert!(rect_of(&h, "panel.Project").is_none() && rect_of(&h, "panel.Source").is_none());
    click_id(&mut h, "studio.right.tab.LumetriScopes");
    assert!(rect_of(&h, "panel.LumetriScopes").is_some());
    click_id(&mut h, "studio.mode.audio");
    assert!(rect_of(&h, "panel.EssentialSound").is_some());
    click_id(&mut h, "studio.mode.captions");
    assert!(rect_of(&h, "panel.Text").is_some());
}

#[test]
fn studio_quick_tool_click_sets_the_timeline_tool() {
    let mut h = harness(1440.0, 900.0);
    for tool in tools::QUICK {
        click_id(&mut h, &format!("studio.tool.{tool:?}"));
        assert_eq!(h.state().ui.tool, tool);
    }
    click_id(&mut h, "studio.tools.button");
    assert!(h.state().studio.tools_open);
    click_id(&mut h, "studio.tools.list.RateStretch");
    assert_eq!(h.state().ui.tool, Tool::RateStretch);
    assert!(!h.state().studio.tools_open);
}

#[test]
fn studio_classic_layout_toggle() {
    let mut h = harness(1440.0, 900.0);
    let ctx = h.ctx.clone();
    crate::menus::invoke(h.state_mut(), &ctx, "window.classicLayout", json!({})).unwrap();
    h.run_steps(4);
    assert!(h.state().studio.classic);
    assert!(rect_of(&h, "header.home").is_some(), "the classic header is back");
    assert!(rect_of(&h, "studio.menu").is_none());
    assert!(crate::menus::menu_items(h.state()).iter().any(|it| it.id == "window.classicLayout" && it.checked == Some(true)));
    crate::menus::invoke(h.state_mut(), &ctx, "window.classicLayout", json!({})).unwrap();
    h.run_steps(4);
    assert!(rect_of(&h, "studio.menu").is_some());
}

#[test]
fn studio_compact_sizes_and_the_codex_column() {
    let full = Rect::from_min_size(Pos2::ZERO, vec2(1100.0, 620.0));
    let r = regions(full, true, None, style::STRIP_FRAC);
    assert_eq!((r.top.height(), r.modes.height(), r.status.height(), r.props.height(), r.rail.width()), (44.0, 42.0, 28.0, 36.0, 60.0));
    assert!(r.work.height() >= 120.0 && r.strip.height() >= 112.0);
    assert_eq!(r.strip.right(), full.right(), "without Codex the strip runs the whole width");
    let r = regions(full, true, Some(340.0), style::STRIP_FRAC);
    let codex = r.codex.unwrap();
    assert_eq!((codex.width(), codex.bottom()), (340.0, r.status.top()), "Codex runs down to the status line");
    assert_eq!(r.strip.right(), codex.left(), "and the strip sits only under the work area");
    let n = regions(Rect::from_min_size(Pos2::ZERO, vec2(1440.0, 900.0)), false, None, style::STRIP_FRAC);
    assert_eq!((n.top.height(), n.modes.height(), n.status.height()), (52.0, 54.0, 34.0));
    assert!((n.strip.height() - 900.0 * 0.38).abs() < 0.5, "the Timeline takes 38% by default");
    let h = harness(1100.0, 620.0);
    assert!(style::compact(&h.ctx));
    assert_eq!(rect_of(&h, "studio.mode.assemble").map(|r| r.height()), Some(30.0));
}

#[test]
fn studio_codex_wand_opens_the_column() {
    let mut h = harness(1440.0, 900.0);
    click_id(&mut h, "studio.codex");
    assert!(h.state().studio.codex_open);
    let strip = rect_of(&h, "panel.Timeline").unwrap();
    assert!(strip.right() <= 1440.0 - 80.0 - 430.0 + 1.0, "the Timeline stops at the Codex column");
    click_id(&mut h, "studio.codex");
    assert!(!h.state().studio.codex_open);
}

#[test]
fn studio_command_search_runs_a_command() {
    let mut h = harness(1440.0, 900.0);
    click_id(&mut h, "studio.search");
    h.event(Event::Text("Classic Layout".into()));
    h.run_steps(3);
    click_id(&mut h, "studio.search.window.classicLayout");
    assert!(h.state().studio.classic);
}

#[test]
fn studio_rail_picks_and_the_strip_resizes() {
    let mut h = harness(1440.0, 900.0);
    click_id(&mut h, "studio.rail.history");
    assert_eq!(h.state().studio.pick, Some(Pick::Panel(PanelKind::History)));
    assert!(rect_of(&h, "panel.History").is_some());
    click_id(&mut h, "studio.rail.markers");
    assert_eq!(h.state().studio.bottom, bottom::Tab::Markers);
    assert!(rect_of(&h, "panel.Markers").is_some() && rect_of(&h, "panel.Timeline").is_none());
    click_id(&mut h, "studio.strip.timeline");
    let edge = rect_of(&h, "studio.strip.edge").unwrap().center();
    let before = h.state().studio.strip_frac;
    h.event(Event::PointerMoved(edge));
    h.step();
    h.event(Event::PointerButton { pos: edge, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    for k in 1..=6 {
        h.event(Event::PointerMoved(edge - vec2(0.0, 15.0 * k as f32)));
        h.step();
    }
    h.event(Event::PointerButton { pos: edge - vec2(0.0, 90.0), button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
    assert!(h.state().studio.strip_frac > before + 0.05, "dragging the edge up grows the Timeline");
}

/// Photo's shell froze on a click (`ctx.data` inside `ctx.data_mut`); click everywhere and close
/// whatever opened.
#[test]
fn studio_click_sweep_does_not_freeze() {
    for (w, hh) in [(1440.0, 900.0), (1100.0, 620.0)] {
        let mut h = harness(w, hh);
        let mut y = 20.0;
        while y < hh {
            let mut x = 20.0;
            while x < w {
                click(&mut h, pos2(x, y));
                h.key_press(egui::Key::Escape);
                h.step();
                // a command that leaves the editor (Export, Home, Classic) comes back
                let a = h.state_mut();
                a.ui.mode = crate::state::Mode::Edit;
                a.studio.classic = false;
                a.dialog = None;
                x += 97.0;
            }
            y += 61.0;
        }
        h.run_steps(2);
    }
}

#[test]
fn studio_home_shows_for_an_empty_project() {
    let h = harness_with(FilmcraftApp::new(Session::default()), 1440.0, 900.0);
    if h.state().session.project.items.is_empty() {
        assert!(rect_of(&h, "studio.mode.assemble").is_none(), "Home instead of the editor");
        assert!(rect_of(&h, "studio.menu").is_some(), "with the top bar");
    }
}

/// Offscreen PNGs of every mode at both sizes into `docs/redesign/fusemovie/`, when
/// `FUSE_STUDIO_SNAPSHOTS` is set (it needs a wgpu adapter).
#[test]
fn studio_snapshots() {
    if std::env::var_os("FUSE_STUDIO_SNAPSHOTS").is_none() {
        return;
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/redesign/fusemovie");
    std::fs::create_dir_all(&dir).unwrap();
    for (w, hh) in [(1440.0, 900.0), (1100.0, 620.0)] {
        for (codex, mode) in Mode::ALL.into_iter().map(|m| (false, m)).chain([(true, Mode::Assemble)]) {
            let mut a = app();
            a.studio.mode = mode;
            a.studio.codex_open = codex;
            let mut h = harness_with(a, w, hh);
            h.run_steps(20);
            let image = h.render().unwrap();
            let name = format!("{}{}-{}x{}.png", mode_bar::label(mode).to_ascii_lowercase(), if codex { "-codex" } else { "" }, w as u32, hh as u32);
            image.save(dir.join(name)).unwrap();
        }
    }
}
