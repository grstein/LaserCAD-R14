# LCV-030 — eframe::App impl + central panel

- **Status**: Done
- **Phase**: 3
- **Depends on**: LCV-007
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: 24b118a — feat(LCV-030): wire eframe::App into central panel and retire render MODULE

## Problem

The bootstrap window (LCV-007) shows a centered "LaserCAD v2 / Bootstrap window" label. That label was the right shape for proving the toolchain end-to-end, but it occupies the entire central panel and leaves no surface for the render pipeline (Phase 3) to paint into. Every later Phase-3 demand — Camera (LCV-031), pointer wiring (LCV-032), grid (LCV-033), bed (LCV-034), entity painter (LCV-035), selection highlight (LCV-036), preview overlay (LCV-037), snap markers (LCV-038) — extends the same central-panel surface. Without this demand they have nothing to attach to.

This demand replaces the placeholder body of `App::update` with a real `CentralPanel` that allocates the full panel rect as a viewport area and draws a single marker rectangle (the future "drawing surface" boundary). It also retires the `pub const MODULE: &str = "render";` placeholder LCV-001 left in `src/render/mod.rs` and the matching placeholder in `src/app.rs`, both of which only existed so `tests/skeleton.rs::module_tree_is_wired` had names to reference.

User outcome: opening the app shows a window with a clearly-bounded drawing area (not a centered label). The operator can see "this is where my drawing will go" even before any geometry, grid, or bed renders.

## Scope

- `src/app.rs`:
  - Replace `pub struct App;` with `pub struct App { pub document: Document, pub history: History }`. (Camera is **not** added in this demand — LCV-031 adds it. The implementer leaves a `TODO(LCV-031): camera` comment near the struct definition.)
  - Derive `Default` (via `#[derive(Default)]`); `Document::default()` and `History::default()` both exist (LCV-021, LCV-026).
  - Remove `pub const MODULE: &str = "app";` from `src/app.rs`. The placeholder is no longer needed; `tests/skeleton.rs` is updated by AC 6 below.
  - Rewrite `impl eframe::App for App::update`:
    1. `egui::CentralPanel::default().show(ctx, |ui| { ... })`.
    2. Inside the closure: `let (rect, _response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());`. This claims the full panel rect as the viewport area. (`Sense::hover()` is the minimum for now; LCV-032 widens it to `click_and_drag`.)
    3. Get a `Painter` clipped to that rect: `let painter = ui.painter_at(rect);`.
    4. Draw the viewport background: `painter.rect_filled(rect, 0.0, egui::Color32::from_gray(24));` (dark canvas — sufficient marker that the surface is alive without committing to a final theme).
    5. Draw a 1-px border around the viewport: `painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::from_gray(64)), egui::StrokeKind::Inside);` (or whatever current-egui API expects — implementer follows the egui-best-practices skill).
  - No other content in `update` for this demand: no menubar, no toolbar, no statusbar, no labels, no instructions text. The viewport rect is the entire window's contents.
- `src/render/mod.rs`:
  - Remove `pub const MODULE: &str = "render";`. The module header doc-comment stays.
  - The file stays otherwise empty (sub-modules are added by later Phase-3 demands).
- `src/lib.rs::run()`:
  - No change required — `Box::<app::App>::default()` already works once `App: Default`. The implementer verifies the existing closure still compiles after the struct change.
- `tests/skeleton.rs`:
  - The `module_tree_is_wired` test currently references `&app::MODULE` and `&render::MODULE`. Both are removed by this demand. The implementer replaces those two entries with valid module references (e.g., `let _ = std::any::TypeId::of::<lasercad::app::App>();` and a similar witness for `render` — or simply drops those two lines from the tuple if no public item remains in `render`). The intent of the test (every module re-exported by `lib.rs` is addressable) must be preserved.
- File size: `src/app.rs` stays ≤300 LOC. (Current scaffold is ~22 LOC; after this demand it grows to ~40-60 LOC. Well within budget.)
- The kernel-purity rule does **not** apply to `src/app.rs` or `src/render/`; both freely import `egui` and `eframe`.

## Out of scope

- **Camera** (`Camera` struct, `world_to_screen`, `screen_to_world`, zoom, pan, zoom-extents) — owned by **LCV-031**. This demand intentionally renders nothing in world coordinates; the viewport rect is in raw screen pixels.
- **Pointer input** (cursor tracking, mouse-wheel zoom, middle-drag pan, hotkeys) — owned by **LCV-032**. The viewport rect is allocated with `Sense::hover()` here; LCV-032 widens it.
- **Grid renderer** — owned by **LCV-033**.
- **Bed renderer** — owned by **LCV-034**.
- **Entity painter** — owned by **LCV-035**.
- **Selection highlight rendering** — owned by **LCV-036**.
- **Preview overlay** — owned by **LCV-037**.
- **Snap marker rendering** — owned by **LCV-038**.
- **Menubar / toolbar / statusbar / command line / dialogs** — Phase 6 (LCV-065..069).
- **Active tool, toggles, command-line input, cursor state on `App`** — added by **LCV-032** (cursor) and Phase 4 (tools). This demand only adds `document` and `history` to `App`.
- **Settings, autosave, window size restoration** — Phase 5 (LCV-058 / LCV-059).
- **Theme tuning** (final colors, light/dark mode). The dark gray canvas is a marker, not a theme decision — Phase 6 (LCV-071) owns the theme.

## Acceptance criteria

1. `src/app.rs` defines `pub struct App { pub document: Document, pub history: History }` (or equivalent — fields may be reordered or made `pub(crate)` at the implementer's discretion, but both fields exist by those exact names and types). `App` derives `Default` (or a manual `Default` impl produces an empty document and empty history). `App::default()` compiles and runs.
2. `src/app.rs` no longer declares `pub const MODULE: &str = "app";`. `grep -nF 'MODULE' src/app.rs` returns no matches.
3. `src/render/mod.rs` no longer declares `pub const MODULE: &str = "render";`. `grep -nF 'MODULE' src/render/mod.rs` returns no matches. The module's `//!` header is preserved.
4. `impl eframe::App for App::update` calls `egui::CentralPanel::default().show(ctx, |ui| { ... })`. Inside the closure it allocates the full available rect as a viewport area (via `ui.allocate_exact_size(ui.available_size(), egui::Sense::hover())` or equivalent), obtains a `Painter` clipped to that rect, fills the rect with a dark gray (`Color32::from_gray(24)`), and strokes a 1-px border in lighter gray (`Color32::from_gray(64)`).
5. The placeholder centered label ("LaserCAD v2 / Bootstrap window. PLAN.md is the roadmap.") is removed. `grep -nF 'LaserCAD v2' src/app.rs` returns no matches; `grep -nF 'PLAN.md is the roadmap' src/app.rs` returns no matches.
6. `tests/skeleton.rs::module_tree_is_wired` no longer references `&app::MODULE` or `&render::MODULE`. The test continues to compile and pass, with its intent — "every top-level module re-exported by `lib.rs` is addressable" — preserved by some equivalent witness for those two modules (e.g., constructing `lasercad::app::App::default()` and using a public item from `lasercad::render` once one exists in a later demand; in this demand it is acceptable to drop the two lines from the tuple if `render` has no remaining public item).
7. `cargo run` opens the window with no centered label visible; instead a dark gray rectangle (with a thin border) fills the window's client area. (Manual smoke — see Expected tests.)
8. Size: `wc -l src/app.rs` reports `<= 300`; `wc -l src/render/mod.rs` reports `<= 50` (it should remain a slim header).
9. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: test `app_default_constructs_with_empty_document_and_history` in `src/app.rs` under `#[cfg(test)] mod tests`: `let app = App::default(); assert_eq!(app.document.entity_count(), 0); assert!(!app.history.can_undo());`.
- **Static check (AC 2)**: `grep -nF 'MODULE' src/app.rs` returns no matches.
- **Static check (AC 3)**: `grep -nF 'MODULE' src/render/mod.rs` returns no matches.
- **Unit (AC 4)**: no automated egui-Painter test is required; AC 4 is verified by code-review reading `App::update` plus the manual smoke (AC 7). If the implementer wants stronger automated coverage, factor the viewport-rect computation into a `fn allocate_viewport_rect(ui) -> Rect` helper and unit-test the helper with a stub `Ui` — optional, not required.
- **Static check (AC 5)**: two `grep` commands above return no matches.
- **Integration (AC 6)**: `cargo test --test skeleton module_tree_is_wired` passes after the test body is updated.
- **Manual smoke (AC 7)**: run `cargo run` from the repo root. Within 5 seconds, the window opens; the entire client area is a dark gray rectangle with a thin border. No centered text label is visible. Click the OS close button; the process exits cleanly. Record the result in the demand's `Implementation:` line.
- **Size check (AC 8)**: `wc -l src/app.rs` reports `<= 300`; `wc -l src/render/mod.rs` reports `<= 50`.
- **Build gate (AC 9)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **Why retire `MODULE` here**: LCV-001 introduced `pub const MODULE: &str` placeholders in each module so `tests/skeleton.rs` had names to reference and prove the module tree compiled. Task #8 (open) already flagged that pattern as a KISS violation. Phase 3 is when `render/` and `app/` start carrying real public items, so the placeholders retire naturally. Other modules (`io`, `text`, `tools`, `ui`, `util`, `agent`) keep their `MODULE` placeholder until their respective demands replace them.
- **No camera in this demand**: LCV-031 adds `App.camera`. Keeping that out of LCV-030 lets the demand land cleanly without depending on Camera math. The viewport rect is in screen pixels (which is what `Painter` already uses); world↔screen comes in LCV-031.
- **Sense::hover() not Sense::click_and_drag**: this demand does not consume pointer events. LCV-032 widens the sense and reads pointer position.
- **Dark gray canvas, light gray border**: arbitrary marker colors. They make the viewport visible against the default egui chrome without claiming to be the final theme. The theme demand (LCV-071) replaces these values.
- **Window title stays "LaserCAD v2 — bootstrap"**: LCV-007 froze that string. A later demand (Phase 6 / Phase 8) will rename it; not in scope here.
- **`egui::StrokeKind::Inside` argument to `rect_stroke`**: the current egui API (verified via the egui-best-practices skill) requires a `StrokeKind`. If the egui version pinned by Cargo.lock uses a different signature, the implementer follows the docs.rs version that matches `Cargo.lock`.
- Reference: v1's `App.tsx` had a root `<div>` claiming `position: absolute; inset: 0` for the canvas; v2's equivalent is the `CentralPanel` allocate-exact-size call. Same intent, different framework.
