//! Product identity: name, reverse-DNS id and company from `brand.env` (repository root,
//! exported by `build.rs`), and the wordmark (the app icon next to the product name).

/// The product name shown in the interface (window title, menus, dialogs, `{app}` in UI text).
pub const APP_NAME: &str = env!("FILMCRAFT_BRAND_NAME");
/// Reverse-DNS app id (Linux desktop entry, Wayland app id).
pub const APP_ID: &str = env!("FILMCRAFT_BRAND_ID");
/// The company that ships the product.
pub const COMPANY: &str = env!("FILMCRAFT_BRAND_COMPANY");

/// The app icon (`assets/app-icon/`, original work), 128 px.
const ICON_PNG: &[u8] = include_bytes!("../../../assets/app-icon/hicolor/128x128/apps/ai.storyteller.filmcraft.png");

/// The app icon, decoded once and kept as a texture.
pub fn icon(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let id = egui::Id::unique("app-icon-texture");
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(t);
    }
    let img = image::load_from_memory_with_format(ICON_PNG, image::ImageFormat::Png).ok()?.to_rgba8();
    let color = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
    let tex = ctx.load_texture("app-icon", color, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
    Some(tex)
}

/// Draw the wordmark (the icon, then [`APP_NAME`] in bold) `height` points tall with its left edge
/// at `left_center` (vertically centred), in light ink on dark themes (`dark = true`). Returns the
/// drawn rect.
pub fn paint_wordmark(ui: &egui::Ui, left_center: egui::Pos2, height: f32, dark: bool) -> Option<egui::Rect> {
    let tex = icon(ui.ctx())?;
    let icon_rect = egui::Rect::from_min_size(egui::pos2(left_center.x, left_center.y - height / 2.0), egui::vec2(height, height));
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    ui.painter().image(tex.id(), icon_rect, uv, egui::Color32::WHITE);
    let ink = if dark { egui::Color32::from_gray(245) } else { egui::Color32::from_rgb(0x14, 0x14, 0x14) };
    let text_rect = ui.painter().text(
        egui::pos2(icon_rect.max.x + height * 0.35, left_center.y),
        egui::Align2::LEFT_CENTER,
        APP_NAME,
        crate::theme::Tokens::semibold(height * 0.8),
        ink,
    );
    Some(icon_rect.union(text_rect))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brand_comes_from_brand_env() {
        let env = include_str!("../../../brand.env");
        let get = |k: &str| env.lines().find_map(|l| l.strip_prefix(k)?.strip_prefix('=')).map(str::trim);
        assert_eq!(Some(APP_NAME), get("BRAND_NAME"));
        assert_eq!(Some(APP_ID), get("BRAND_ID"));
        assert_eq!(Some(COMPANY), get("BRAND_COMPANY"));
        assert!(!APP_NAME.is_empty() && !APP_NAME.contains("FilmCraft"));
    }

    /// No update check and no upstream (vendor) address anywhere in the interface: the menus have
    /// no update or vendor-link command, the update command is unknown, and no source file or
    /// catalog row of this crate or the desktop app names an upstream address.
    #[test]
    fn brand_has_no_update_check_or_upstream_links() {
        let mut app = crate::FilmcraftApp::new(filmcraft_engine::Session::default());
        let ids: Vec<String> = crate::menus::menu_items(&app).into_iter().map(|it| it.id).collect();
        for gone in ["app.checkForUpdates", "help.discord", "help.website", "help.appPage", "help.github", "help.reportIssue", "help.filmcraftHelp"] {
            assert!(!ids.iter().any(|id| id == gone), "{gone} is a menu command");
        }
        assert!(crate::menus::invoke(&mut app, &egui::Context::default(), "app.checkForUpdates", serde_json::json!({})).is_err());
        // split so this file does not match itself
        let upstream = [concat!("story", "told"), concat!("getart", "craft"), concat!("discord", ".gg"), concat!("craft", "_update")];
        let mut dirs = vec![
            std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/src")),
            std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../apps/filmcraft/src")),
        ];
        let mut files = 0;
        while let Some(d) = dirs.pop() {
            for p in std::fs::read_dir(&d).into_iter().flatten().flatten().map(|e| e.path()) {
                if p.is_dir() {
                    dirs.push(p);
                    continue;
                }
                if !p.extension().is_some_and(|e| e == "rs" || e == "tsv") {
                    continue;
                }
                files += 1;
                let text = std::fs::read_to_string(&p).unwrap_or_default().to_lowercase();
                for line in text.lines().filter(|l| !l.starts_with('#')) {
                    for u in upstream {
                        assert!(!line.contains(u), "{}: {line}", p.display());
                    }
                }
            }
        }
        assert!(files > 100, "scanned {files} files");
    }

    /// `{app}` in UI text and catalogs is filled with the product name in every language, and no
    /// catalog spells out the upstream name.
    #[test]
    fn brand_fills_app_in_every_language() {
        use crate::i18n::Language;
        for l in Language::ALL {
            for s in ["About {app}", "Quit {app}", "{app} hit an error", "An untranslated {app} label"] {
                let shown = l.tr(s);
                assert!(shown.contains(APP_NAME) && !shown.contains("{app}"), "{}: {s} → {shown}", l.code());
            }
        }
        assert_eq!(Language::En.tr("About {app}"), format!("About {APP_NAME}"));
        assert_eq!(Language::PtBr.tr("About {app}"), format!("Sobre o {APP_NAME}"));
        crate::i18n::set_current(Language::En);
        assert_eq!(crate::i18n::t("{app} Project"), format!("{APP_NAME} Project"));
        let mut dir = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src/i18n")).into_iter().flatten().flatten().map(|e| e.path()).collect::<Vec<_>>();
        dir.retain(|p| p.extension().is_some_and(|e| e == "tsv"));
        assert!(dir.len() > 10);
        for p in dir {
            let text = std::fs::read_to_string(&p).unwrap_or_default();
            for line in text.lines().filter(|l| !l.starts_with('#')) {
                assert!(!line.contains("FilmCraft") && !line.contains("ArtCraft"), "{}: {line}", p.display());
            }
        }
    }

    #[test]
    fn brand_wordmark_draws_the_icon_and_name() {
        let ctx = egui::Context::default();
        let mut drawn = None;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| drawn = paint_wordmark(ui, egui::pos2(10.0, 50.0), 20.0, true));
        out.textures_delta.clear();
        let r = drawn.expect("icon decodes");
        assert!(r.width() > 40.0 && (r.height() - 20.0).abs() < 4.0, "{r:?}");
    }
}
