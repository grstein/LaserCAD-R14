# LCV-034 — Bed renderer (rectangle + dark outer overlay)

- **Status**: Done
- **Phase**: 3
- **Depends on**: LCV-031
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: ee8cea4 — feat(LCV-034): bed renderer — rectangle fill, border, outer overlay

## Problem

LaserGRBL drives a physical laser machine with a fixed bed (e.g., 400×400 mm). Geometry placed outside the bed will not cut. Without a visible bed boundary in the viewport, the operator has no way to know whether their drawing fits the machine until they hit "Export SVG" and load it into LaserGRBL — too late. AutoCAD R14 doesn't have a bed concept, but every laser-CAD tool does, and v1 had one too.

This demand draws (a) a light-filled rectangle at the bed coordinates and (b) a darker translucent overlay covering everything outside the bed. The contrast makes "in bounds" obvious at a glance and "out of bounds" visually discouraged.

User outcome: the operator opens the app and sees a 400×400 mm rectangle (the default bed) sitting on the grid. Anything they draw inside it is in the printable area; anything outside is shown in a dimmed region.

## Scope

- New file `src/render/bed.rs` defining:
  - `pub struct Bed { pub size_mm: [f64; 2], pub origin_world: Vec2 }`.
    - Derives: `Debug`, `Clone`, `PartialEq`.
    - `size_mm[0]` = width along world X; `size_mm[1]` = height along world Y. Both must be strictly positive (no debug-assert needed; drawing handles zero gracefully by drawing nothing).
    - `origin_world` is the world-space coordinate of the **lower-left** corner of the bed (so the bed spans `[origin.x, origin.x + size_mm[0]]` × `[origin.y, origin.y + size_mm[1]]`).
    - **Default**: `pub fn default() -> Self { Self { size_mm: [400.0, 400.0], origin_world: Vec2::new(0.0, 0.0) } }` — 400×400 mm bed at origin. Matches a common GRBL hobbyist machine size.
  - `pub fn draw_bed(painter: &egui::Painter, rect: egui::Rect, camera: &Camera, bed: &Bed)`:
    - Compute the bed's four world-space corners and convert to screen-space via `camera.world_to_screen` + `rect.min.to_vec2()` (same offset pattern as LCV-033).
    - Draw the bed rectangle as a filled rect: `Painter::rect_filled(bed_screen_rect, 0.0, Color32::from_gray(40))` — slightly lighter than the LCV-030 canvas background (`gray(24)`) so the bed reads as "the active area".
    - Draw a thin stroke around the bed: `Painter::rect_stroke(bed_screen_rect, 0.0, Stroke::new(1.5, Color32::from_gray(160)), StrokeKind::Inside)` — a light gray border at ~1.5 px.
    - Draw a dark overlay outside the bed by painting four rectangles (top strip, bottom strip, left strip, right strip) that together cover `rect \ bed_screen_rect`. Color: `Color32::from_rgba_unmultiplied(0, 0, 0, 96)` — black at ~37% alpha. (Translucent so the grid stays partially visible underneath.)
    - **Order of operations**: caller must invoke `draw_bed` BEFORE `draw_entities` and AFTER `draw_grid`, so the bed sits over the grid but under any drawn geometry. (Documented in the function's `///` doc-comment.)
  - The function takes `&Bed` by reference; it does not mutate.
- Update `src/render/mod.rs`:
  - Add `pub mod bed;`.
  - Add `pub use bed::{Bed, draw_bed};`.
- Update `src/app.rs`:
  - Add `pub bed: Bed` to the `App` struct. `Bed::default()` slots into `#[derive(Default)]`.
  - In `App::update`, call `lasercad::render::bed::draw_bed(&painter, rect, &self.camera, &self.bed);` after `draw_grid` and before any future entity painting.
- File size: `src/render/bed.rs` stays ≤300 LOC.
- The file imports `egui::{Painter, Pos2, Rect, Stroke, Color32, StrokeKind}` and `crate::geometry::Vec2`, `crate::render::Camera`. No `eframe`, no `rfd`.
- Per-module unit tests under `#[cfg(test)] mod tests` in `bed.rs`.

## Out of scope

- **Configurable bed size via settings**: owned by **LCV-058** (settings store). This demand ships a hard-coded default of 400×400 mm. LCV-058 will let the user override `App.bed` from a settings dialog.
- **Multiple bed presets** (a dropdown of "K40", "Ortur LM3", "Atomstack X20" sizes). Future Phase-5 feature; defer.
- **Bed orientation / rotation**. The bed is axis-aligned in v2.
- **Bed origin alignment to the laser machine's home position**. The world origin == bed lower-left in this demand; if the user's machine homes top-right, that's a coordinate-transform concern for the SVG export demand (LCV-056), not the renderer.
- **Bed work-area vs. mechanical-area distinction** (some machines have a slightly smaller usable area than the full bed). v2 treats the bed as a single rectangle.
- **Out-of-bed warning** (drawing entities outside the bed in red, or a warning toast). Phase 6 feature; defer.
- **Grid alignment to the bed**. The grid is world-origin-aligned (LCV-033); if the bed origin matches world origin (default), they align. If the user moves the bed, the grid does not follow. Documented as a known limitation.
- **Custom bed colors / theme**: owned by Phase 6 (LCV-071 theme demand).

## Acceptance criteria

1. `src/render/bed.rs` exists and defines `pub struct Bed { pub size_mm: [f64; 2], pub origin_world: Vec2 }` with `Debug`, `Clone`, `PartialEq` derives.
2. `Bed::default()` returns `Bed { size_mm: [400.0, 400.0], origin_world: Vec2::new(0.0, 0.0) }`. Asserted via unit test.
3. **Corner accessor**: the file exposes a helper (either inherent method `pub fn corners(&self) -> [Vec2; 4]` or top-level function `pub fn bed_corners(bed: &Bed) -> [Vec2; 4]`) returning the four world-space corners in `[bottom_left, bottom_right, top_right, top_left]` order. For `Bed::default()`, corners are `[Vec2::new(0,0), Vec2::new(400,0), Vec2::new(400,400), Vec2::new(0,400)]`. Asserted via unit test.
4. `pub fn draw_bed(painter: &egui::Painter, rect: egui::Rect, camera: &Camera, bed: &Bed)` exists with that exact signature.
5. **No-panic guarantee**: `draw_bed` does not panic for any of the following: `camera.viewport_size_px == [0.0, 0.0]`, `camera.mm_per_px == 1e-9` (huge zoom-in), `camera.mm_per_px == 1e9` (huge zoom-out), `bed.size_mm == [0.0, 0.0]` (degenerate), `bed.size_mm == [1e9, 1e9]` (absurdly large). No automated rendering assert is required; the function returns normally.
6. `src/render/mod.rs` re-exports `Bed` and `draw_bed`.
7. `src/app.rs` `App` struct carries `pub bed: Bed`. `App::default().bed == Bed::default()`. `App::update` calls `draw_bed` once per frame after `draw_grid`.
8. **Manual smoke**: `cargo run` opens the window with the grid visible and a 400×400 mm rectangle (lighter-fill, thin border) clearly identifying the bed. The area outside the bed is visibly darker than the grid area inside the bed. Pressing `F` (zoom-extents on empty document) leaves the bed at the same world coordinates (default view).
9. The file imports nothing from `eframe` or `rfd`. (Importing `egui` is allowed.)
10. Size: `wc -l src/render/bed.rs` reports `<= 300`.
11. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: test `bed_struct_constructs` — instantiates `Bed { size_mm: [100.0, 50.0], origin_world: Vec2::new(10.0, 20.0) }` via struct literal.
- **Unit (AC 2)**: test `bed_default_is_400x400_at_origin`.
- **Unit (AC 3)**: test `bed_corners_default` — asserts the four corners of `Bed::default()` match the expected array.
- **Unit (AC 3, second case)**: test `bed_corners_offset` — `Bed { size_mm: [100.0, 50.0], origin_world: Vec2::new(10.0, 20.0) }` → corners `[(10,20), (110,20), (110,70), (10,70)]`.
- **Unit (AC 5)**: test `draw_bed_does_not_panic_on_degenerate_inputs` — see the AC 9 pattern in LCV-033. Either (a) construct an egui Painter via `egui::Context::default()` (if practical) and call `draw_bed` with each degenerate configuration, or (b) factor the world-bounds computation into a `pub(crate) fn bed_screen_rect(rect, camera, bed) -> egui::Rect` helper and unit-test it returns a finite `Rect` (possibly empty / inverted) for each degenerate case. Either approach satisfies the AC.
- **Manual smoke (AC 8)**: `cargo run`; verify the bed renders. Record success in the demand's `Implementation:` line.
- **Static check (AC 9)**: `grep -nE '^use (eframe|rfd)' src/render/bed.rs` returns no matches.
- **Size check (AC 10)**: `wc -l src/render/bed.rs` reports `<= 300`.
- **Build gate (AC 11)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **400×400 mm default**: chosen because it matches the most common entry-level GRBL machine size (Ortur LM2/LM3, Atomstack A5/A10, NEJE Master 2). Users with smaller (100×100 K40) or larger (XCarve 600×900) machines override via LCV-058 settings.
- **Lower-left origin**: matches AutoCAD / standard math convention (Y up, origin at bottom-left). v1 used the same convention. LaserGRBL itself uses the same convention for SVG import.
- **Outer overlay via four strips**: the alternative is drawing one large overlay rect over the entire viewport and then painting the bed rect on top (the bed rect "punches through"). egui's `Painter` does not support compositing modes well, so the four-strip approach is simpler and renders correctly with normal source-over alpha.
- **Order of operations** (`draw_grid` then `draw_bed` then `draw_entities`): documented in the function's doc-comment. Reviewer enforces in code review of LCV-035+.
- **Translucent overlay color** (`rgba(0, 0, 0, 96)`): 37.6% black on top of the dark gray canvas yields a noticeably-but-not-overwhelmingly dim outside region. The grid remains visible underneath, which is intentional — the operator can still see scale outside the bed.
- **No user-visible bed corner labels** ("0,0" / "400,400") in this demand. Phase 6 visual polish (LCV-071) may add them. The bed is recognizable from its visual contrast without labels.
- Reference: v1's `tools/Bed.tsx` rendered the bed as a `<rect>` and a `<rect mask="bed-hole">` overlay. The math is the same; the framework is different.
