# App icon

A film frame with a play mark on an amber rounded tile. Original geometry written by hand as SVG
(four shapes); every other file here is drawn from the same geometry. It replaces the upstream
FilmCraft owl.

## Palette

| Colour | Hex | Used for |
|---|---|---|
| Amber (app colour) | `#FDB528` | the tile and the film perforations |
| Near-black | `#141414` | the film frame |
| White | `#FFFFFF` | the play mark |

## Geometry

A 512-unit tile (`viewBox="0 0 512 512"`), rounded square with `rx=112`, no border; the frame is
`x 88–424, y 120–392, rx 40`, with two rows of six perforations and a play triangle in the middle.
macOS renders (`filmcraft.icns`, `filmcraft-macos-512.png`, `filmcraft-1024.png`) put the tile on
Apple's grid (824 of 1024 px, transparent margin). Windows, Linux and the web use the full-bleed
tile. Licence: [LICENSE.txt](LICENSE.txt) (MIT OR Apache-2.0).

## Files

The file names are machine ids (the packaging refers to them), not shown to users.

| File | What |
|---|---|
| `filmcraft.svg` | canonical vector (also copied to `filmcraft-small.svg` and the hicolor scalable icon) |
| `filmcraft-1024.png` | 1024 px render on the macOS grid |
| `filmcraft-macos-512.png` | runtime Dock icon on macOS (embedded by `apps/filmcraft/src/main.rs`) |
| `filmcraft.icns` | macOS bundle icon (`CFBundleIconFile`) |
| `filmcraft.ico` | Windows icon, 16–256 px (embedded in `filmcraft.exe` by `apps/filmcraft/build.rs`) |
| `hicolor/<size>/apps/ai.storyteller.filmcraft.png`, `hicolor/scalable/…svg` | Linux icon theme; the 128 px one is also the in-app wordmark icon (`crates/ui-egui/src/brand.rs`) |

The web app's `apps/filmcraft-web/web/favicon.png` (128 px) comes from the same artwork.

## How the icon reaches each OS

- **macOS:** the window icon set at startup (`ViewportBuilder::with_icon`) is the Dock and app-switcher
  icon when the app runs unbundled. A future `.app` bundle points `CFBundleIconFile` at `filmcraft.icns`.
- **Windows:** `build.rs` embeds `filmcraft.ico` as the exe's resource icon (Explorer, Start menu,
  pinned taskbar), and the runtime icon covers the window, taskbar and Alt-Tab.
- **Linux:** the runtime icon plus `with_app_id(APP_ID)` (`BRAND_ID` in `brand.env`), which Wayland matches to
  the `.desktop` file. Install the `hicolor/` tree under `/usr/share/icons/hicolor/` and the desktop
  file under `/usr/share/applications/`.

## Regenerate

Edit `filmcraft.svg`, then run `packaging/icons.sh` (needs `resvg`, plus `iconutil` on macOS; the
`.ico` is packed by `cargo xtask ico`). Keep each derived file's `.attribution` sidecar and its row
in `ATTRIBUTION.md`.
