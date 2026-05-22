# LCV-071 — Theme + visual polish

- **Status**: Ready
- **Phase**: 6
- **Depends on**: LCV-030
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

`App::update` currently hardcodes raw `Color32::from_gray` literals for the canvas background
(`from_gray(24)`) and border. There is no central theme authority: any future demand that adds a
panel, toolbar, or dialog has to invent its own colors, risking a visually incoherent result.
Default egui renders a light-gray window — hostile to the laser-cutting workflow, where the
operator needs to read bright cut-line geometry against a dark field. This demand establishes a
single `src/ui/theme.rs` module that owns the four canonical color values, calls
`ctx.set_visuals()` once per frame to enforce them, and exposes `CANVAS_BG` so the viewport fill
in `App::update` references a named constant instead of a bare gray literal.

User outcome: the application opens with a consistent dark CAD theme (near-black canvas, dark
panels, light gray text, blue accent) without requiring any downstream demands to hardcode color
values.

## Scope

- New file `src/ui/theme.rs` (≤ 120 LOC):
  - `pub const CANVAS_BG: egui::Color32` — viewport background color, rgb(26, 26, 26) (`#1a1a1a`).
  - `pub fn apply_theme(ctx: &egui::Context)` — builds a `Visuals` starting from
    `egui::Visuals::dark()`, overrides the four named colors below, then calls
    `ctx.set_visuals(visuals)`:
    - **Panel fill** `panel_fill` = rgb(37, 37, 37) (`#252525`)
    - **Text** `override_text_color` = `Some(Color32::from_rgb(208, 208, 208))` (`#d0d0d0`)
    - **Accent** `selection.bg_fill` = rgb(79, 163, 224) (`#4fa3e0`)
    - **Hyperlink** `hyperlink_color` = rgb(79, 163, 224) (same accent)
  - `#[cfg(test)] mod tests` with at least the tests listed under Expected tests.
- `src/ui/mod.rs`:
  - Add `pub mod theme;`.
  - Add `pub use theme::{apply_theme, CANVAS_BG};`.
  - Remove the `pub const MODULE: &str = "ui";` placeholder — this demand retires it (the same
    pattern LCV-030 established for `app::MODULE`, LCV-031 for `render::Camera`). The skeleton
    test (`tests/skeleton.rs`) witness for the `ui` module changes from `&ui::MODULE` to
    `lasercad::ui::CANVAS_BG` (see AC 6 and Expected tests).
- `src/app.rs`:
  - At the very top of `App::update`, before the `CentralPanel::default().show(...)` call, add:
    `crate::ui::apply_theme(ctx);`
  - Replace `painter.rect_filled(rect, 0.0, egui::Color32::from_gray(24))` with
    `painter.rect_filled(rect, 0.0, crate::ui::CANVAS_BG)`.
  - No other changes to `app.rs`.
- `tests/skeleton.rs`:
  - Replace `&ui::MODULE` in the `module_tree_is_wired` tuple with `lasercad::ui::CANVAS_BG` (or
    any equivalent expression that forces the compiler to resolve a real `lasercad::ui` export).
    The test's intent — every top-level module in `lib.rs` is addressable — must be preserved.

## Out of scope

- Grid color tuning — `src/render/grid.rs` keeps its `Color32::from_gray(48/96)` values unchanged.
- Entity painter stroke color — `src/render/entities.rs` keeps `Color32::from_gray(220)`.
- Light mode / dark-mode toggle. One static dark theme only.
- Per-layer or per-entity color assignments — SVG export color-by-preset is LCV-056.
- Menubar, toolbar, statusbar, panels — those demands (LCV-065..LCV-070) add their own widgets;
  `apply_theme` sets base egui visuals they will inherit automatically.
- HiDPI / DPI-scale color compensation.
- Window title changes.

## Acceptance criteria

1. `src/ui/theme.rs` exists, is ≤ 120 LOC (`wc -l src/ui/theme.rs` ≤ 120), and imports nothing
   from `eframe` or `rfd` (`grep -nE '^use (eframe|rfd)' src/ui/theme.rs` returns no matches).
2. `lasercad::ui::CANVAS_BG` is accessible (resolves without a deep path) and its RGB channels are
   exactly (26, 26, 26): `CANVAS_BG.r() == 26 && CANVAS_BG.g() == 26 && CANVAS_BG.b() == 26`.
3. `lasercad::ui::apply_theme` is accessible and calling it on a freshly constructed
   `egui::Context::default()` does not panic.
4. After `apply_theme(&ctx)` is called, the context's visuals satisfy:
   `ctx.style().visuals.panel_fill == Color32::from_rgb(37, 37, 37)` and
   `ctx.style().visuals.selection.bg_fill == Color32::from_rgb(79, 163, 224)`.
5. `src/ui/mod.rs` re-exports `apply_theme` and `CANVAS_BG` — both resolve at the
   `lasercad::ui::*` path (verified by `cargo doc --no-deps --quiet` producing docs for both
   items without errors, or equivalently by the `module_tree_is_wired` test compiling with the
   new witness).
6. `src/ui/mod.rs` no longer contains `pub const MODULE: &str = "ui";`.
   `grep -nF 'MODULE' src/ui/mod.rs` returns no matches.
7. `src/app.rs::update` calls `crate::ui::apply_theme(ctx)` before the `CentralPanel` show
   closure. `grep -nF 'apply_theme' src/app.rs` returns at least one match.
8. `src/app.rs` no longer hardcodes `Color32::from_gray(24)` for the canvas fill.
   `grep -nF 'from_gray(24)' src/app.rs` returns no matches.
9. `tests/skeleton.rs::module_tree_is_wired` compiles and passes after `&ui::MODULE` is replaced
   with a witness that addresses a real `lasercad::ui` export.
10. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and
    `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 2)** — `canvas_bg_rgb_matches_spec` in `src/ui/theme.rs #[cfg(test)] mod tests`:
  ```
  assert_eq!(CANVAS_BG.r(), 26);
  assert_eq!(CANVAS_BG.g(), 26);
  assert_eq!(CANVAS_BG.b(), 26);
  ```
- **Unit (AC 3)** — `apply_theme_does_not_panic`:
  ```
  let ctx = egui::Context::default();
  apply_theme(&ctx);  // must not panic
  ```
- **Unit (AC 4)** — `apply_theme_sets_panel_fill` and `apply_theme_sets_selection_accent`:
  ```
  let ctx = egui::Context::default();
  apply_theme(&ctx);
  let v = ctx.style().visuals.clone();
  assert_eq!(v.panel_fill,           egui::Color32::from_rgb(37, 37, 37));
  assert_eq!(v.selection.bg_fill,    egui::Color32::from_rgb(79, 163, 224));
  ```
  (In egui 0.29, `ctx.style()` returns `Arc<Style>`; `style.visuals` is a plain field. If the
  API requires a closure — `ctx.style(|s| …)` — the implementer adapts accordingly.)
- **Static check (AC 6)** — `grep -nF 'MODULE' src/ui/mod.rs` returns no matches.
- **Static check (AC 7)** — `grep -nF 'apply_theme' src/app.rs` returns at least one match.
- **Static check (AC 8)** — `grep -nF 'from_gray(24)' src/app.rs` returns no matches.
- **Integration (AC 9)** — `cargo test --test skeleton module_tree_is_wired` passes after the
  skeleton is updated per scope.
- **Manual smoke** — `cargo run`: the window opens with a near-black canvas, dark panels, light
  gray text, and a blue highlight on any active/selected egui widget. No default egui off-white
  panels visible. Click the OS close button; the process exits cleanly.
- **Build gate (AC 10)** — `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **`egui::Visuals::dark()` as base**: egui 0.29's `Visuals` struct has ~30 fields. Starting from
  `Visuals::dark()` and overriding only four values is forward-compatible; a full struct literal
  would break with upstream field additions.
- **`apply_theme` called every frame**: calling `ctx.set_visuals()` each frame is idiomatic egui
  practice (used for theme toggles in egui's own demos). The runtime stores visuals in an
  `Arc<Data>` under `RwLock`; repeated identical writes are cheap. A one-shot `bool` field on
  `App` is also acceptable but adds app state without benefit.
- **Retiring `ui::MODULE`**: `src/ui/theme.rs` gives the `ui` module a real public item, so the
  `MODULE` placeholder retires — the same pattern LCV-030 established for `app::MODULE`.
- **`CANVAS_BG` vs `Color32::from_gray(24)`**: `from_gray(24)` = rgb(24, 24, 24). The demand
  specifies rgb(26, 26, 26) (`#1a1a1a`) — slightly lighter, named. The implementer replaces the
  old literal.
- **Accent `#4fa3e0`** matches the selection-halo color in `src/render/selection.rs` (LCV-036),
  giving visual coherence between egui selection widgets and the CAD selection highlight.
- **Grid and entity colors out of scope**: `Color32::from_gray(48/96)` in `grid.rs` and
  `Color32::from_gray(220)` in `entities.rs` remain untouched. Those are render-layer concerns;
  a future micro-demand can thread `CANVAS_BG` and palette constants through the render layer if
  the product owner approves.
- **LOC budget**: the file needs `pub const`, one `pub fn`, four color overrides, module doc
  comment, and tests — well within the 120 LOC cap.
