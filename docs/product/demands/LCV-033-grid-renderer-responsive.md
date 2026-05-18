# LCV-033 — Grid renderer (responsive minor/major)

- **Status**: Ready
- **Phase**: 3
- **Depends on**: LCV-031
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: <to be filled by demand-manager>

## Problem

An empty dark gray viewport tells the operator nothing about scale. When they zoom in to draw a 5 mm slot or zoom out to see a 400 mm bed, they need visible reference lines to eyeball distances and to confirm "I am at the right zoom level". Without a grid, every drawing tool flies blind — the operator cannot tell whether a click is 2 mm off or 200 mm off from the intended position. AGENTS.md §"Architecture" lists `render/grid.rs` as a first-class submodule; this demand ships it.

The grid must be **responsive**: at any zoom, the minor spacing on screen stays comfortably readable (target: 8–80 pixels between minor lines). A fixed 1-mm grid would be invisible at 0.01 mm/px (way too dense) or invisible at 100 mm/px (way too sparse). Snapping the spacing to a power-of-10 / power-of-5 ladder keeps the grid usable across the full zoom range.

User outcome: the operator zooms from "10 m bed view" down to "1 mm slot detail" and the grid stays readable throughout — minor lines about a finger-width apart, major lines every 5 or 10 minor lines, all aligned with world coordinates.

## Scope

- New file `src/render/grid.rs` defining:
  - `pub fn draw_grid(painter: &egui::Painter, camera: &Camera)` — the public entry point. Iterates over visible grid lines and draws them.
  - `pub fn pick_minor_spacing_mm(mm_per_px: f64) -> f64` — pure function that returns the minor grid spacing in mm given the current zoom. Exposed publicly (or `pub(crate)`) so unit tests can pin the spacing ladder.
    - **Spacing ladder**: choose the smallest member of the set `{ 10^k, 2 * 10^k, 5 * 10^k }` for integer `k` such that the resulting screen spacing (`spacing_mm / mm_per_px`) is `>= MIN_SCREEN_PX`, where `MIN_SCREEN_PX = 8.0`. This is the standard "1-2-5" decade ladder used by most CAD tools and matches v1.
    - Bounded: `k` is clamped to `[-6, 6]` (1 µm minor to 1 km minor) to avoid degenerate output at insane zoom.
  - `pub fn pick_major_step(minor_spacing_mm: f64) -> u32` — pure function returning how many minor cells make one major cell. **Fixed = 10** (every tenth minor line is major). The function exists for testability; its body is `10`.
  - Drawing rules inside `draw_grid`:
    - Compute `minor_mm = pick_minor_spacing_mm(camera.mm_per_px)`.
    - Compute the world-space bounds of the visible viewport: top-left corner = `camera.screen_to_world(rect.top_left)`, bottom-right = `camera.screen_to_world(rect.bottom_right)` (with Y-flip handled correctly).
    - Iterate world X from `floor(world_left / minor_mm) * minor_mm` to `world_right`, stepping by `minor_mm`. For each X, draw a vertical line from world (X, world_bottom) to (X, world_top). Use `Painter::line_segment` with world-space coords converted via `camera.world_to_screen`.
    - Same for Y from `floor(world_bottom / minor_mm) * minor_mm` upward.
    - **Minor lines**: `Stroke::new(0.5, Color32::from_gray(48))` — thin and dark, just barely visible against the LCV-030 dark gray canvas.
    - **Major lines** (every 10th line, i.e., when `(world_coord / minor_mm).round() as i64 % 10 == 0`): `Stroke::new(1.0, Color32::from_gray(96))` — slightly thicker and lighter.
    - **Origin lines** (world X == 0 or Y == 0, within `EPSILON`): treated as major; no special axis highlight in this demand (axis labels arrive with a future demand, not scoped here).
    - **Clipping**: only draw lines whose endpoints fall within (or cross) the viewport rect. The simpler approach is to compute the world bounds first and only iterate within those; lines outside the viewport are skipped by construction.
- `draw_grid` takes `&Painter` (not `&mut`); `Painter` is internally `Clone` and the caller (`App::update`) gives a fresh one via `ui.painter_at(rect)`.
- The function signature does **not** take the viewport rect explicitly; it reads `camera.viewport_size_px` and derives the rect's world bounds. (The rect's screen position relative to the egui canvas is irrelevant — `Painter::line_segment` takes screen-space `Pos2` from `camera.world_to_screen`, which already knows the viewport size.) **Correction**: `world_to_screen` returns coordinates relative to the viewport's geometric center, expressed in the painter's clip rect coordinate system. If the painter's clip rect is offset from the egui canvas origin, the implementer either (a) passes the rect into `draw_grid` and offsets the world_to_screen result by `rect.min.to_vec2()`, or (b) tracks the rect's origin on the Camera. **Decision**: pass the rect explicitly. Final signature: `pub fn draw_grid(painter: &egui::Painter, rect: egui::Rect, camera: &Camera)`. The implementer adds `rect: egui::Rect` and translates `world_to_screen`'s output by `rect.min.to_vec2()` before passing to `Painter::line_segment`. The same pattern repeats in LCV-034..038.
- Update `src/render/mod.rs`:
  - Add `pub mod grid;`.
  - Add `pub use grid::{draw_grid, pick_minor_spacing_mm, pick_major_step};` (or only `draw_grid` if the helpers are kept `pub(crate)` for testing only — implementer's call; the tests need at least crate-level visibility).
- Update `src/app.rs::update`: after the viewport rect is allocated and `camera.viewport_size_px` is set, call `lasercad::render::grid::draw_grid(&painter, rect, &self.camera);`. (Adjust `use` path to match how the implementer re-exports it.)
- File size: `src/render/grid.rs` stays ≤300 LOC.
- The file imports `egui::{Painter, Pos2, Rect, Stroke, Color32}` and `crate::geometry::Vec2`, `crate::render::Camera`. No `eframe`, no `rfd`.
- Per-module unit tests under `#[cfg(test)] mod tests` in `grid.rs`.

## Out of scope

- **Axis labels** ("X 0", "Y 100") at major-line intersections — deferred. Not needed for v2 Phase 3.
- **Axis highlight** (a different color for X=0 and Y=0 lines) — deferred. Major-line styling is sufficient for now.
- **Isometric grid / dot grid / polar grid**. v2 is orthogonal-cartesian only.
- **Snap-to-grid** — that's a snap-engine concern (LCV-016 already shipped without grid snap; grid snap is a future demand if requested).
- **Grid origin offset** (aligning the grid to the bed corner instead of world (0, 0)) — owned by LCV-058 settings if it materializes. v2 grid is world-origin-centered.
- **Disabling the grid** (a toggle). Phase 6 toolbar (LCV-066) may add a "grid on/off" button; the grid renderer itself stays unconditional in this demand. The toolbar will gate the `draw_grid` call.
- **Background color of the canvas** (the dark gray fill from LCV-030) — owned by LCV-030 and Phase 6 theme (LCV-071).
- **Performance** beyond "linear scan within visible bounds is fine". A 1280×800 viewport at maximum reasonable zoom shows ~200 minor lines per axis × 2 axes = 400 line_segment calls per frame. egui handles 10k+ shapes per frame without breaking a sweat.

## Acceptance criteria

1. `src/render/grid.rs` exists and defines `pub fn draw_grid(painter: &egui::Painter, rect: egui::Rect, camera: &Camera)`, `pub(crate) fn pick_minor_spacing_mm(mm_per_px: f64) -> f64`, and `pub(crate) fn pick_major_step(_minor_spacing_mm: f64) -> u32`. (Visibility may be `pub` if the implementer prefers; `pub(crate)` is the minimum.)
2. **Spacing ladder**: `pick_minor_spacing_mm(1.0)` returns one of `{10, 20, 50}` (a value where screen spacing ≥ 8 px at mm_per_px = 1.0). Specifically: at mm_per_px = 1.0, the smallest 1-2-5 value whose product `spacing / mm_per_px >= 8.0` is `10.0` (10 mm / 1 mm/px = 10 px ≥ 8 px). Asserts `pick_minor_spacing_mm(1.0) == 10.0`.
3. **Spacing ladder at zoom-in**: `pick_minor_spacing_mm(0.1)` returns `1.0` (1 mm / 0.1 mm/px = 10 px ≥ 8 px; smaller 1-2-5 values 0.5 → 5 px, 0.2 → 2 px, 0.1 → 1 px all fail the floor). Asserts `pick_minor_spacing_mm(0.1) == 1.0`.
4. **Spacing ladder at zoom-out**: `pick_minor_spacing_mm(10.0)` returns `100.0` (100 mm / 10 mm/px = 10 px). Asserts `pick_minor_spacing_mm(10.0) == 100.0`.
5. **Spacing ladder at intermediate zoom**: `pick_minor_spacing_mm(0.5)` returns `5.0` (5 / 0.5 = 10 px). Asserts.
6. **Spacing ladder at very small zoom**: `pick_minor_spacing_mm(0.001)` returns a value such that `value / 0.001 >= 8.0` and value is in the 1-2-5 ladder. At mm_per_px = 0.001 the answer is `0.01` (10 px). Asserts.
7. **Spacing ladder clamping**: `pick_minor_spacing_mm(1e-9)` does not panic and returns the smallest clamped value (`1e-6` per the `k >= -6` clamp). `pick_minor_spacing_mm(1e9)` returns the largest clamped value (`1e6` per the `k <= 6` clamp). The function does not loop indefinitely.
8. **Major step is fixed at 10**: `pick_major_step(any)` returns `10` for any input. (Single assert sufficient.)
9. **draw_grid does not panic** on any of the following: empty viewport (`viewport_size_px = [0, 0]`), tiny zoom (`mm_per_px = 1e-9`), huge zoom (`mm_per_px = 1e9`), camera centered at a large world coord (`center_world = (1e6, 1e6)`). No automated rendering assert is required; the function returns normally without panic.
10. `src/render/mod.rs` re-exports `draw_grid` (and helpers if the implementer chose `pub` visibility).
11. `src/app.rs::update` calls `draw_grid` once per frame, passing the viewport painter, viewport rect, and `&self.camera`.
12. **Manual smoke**: `cargo run` opens the window with a visible orthogonal grid. Zooming in via mouse wheel shows the grid spacing adapt (the screen spacing stays in roughly the 8–80 px band). Major lines (every 10th) are visibly thicker / brighter than minor lines.
13. The file imports nothing from `eframe` or `rfd`. (Importing `egui` is allowed.)
14. Size: `wc -l src/render/grid.rs` reports `<= 300`.
15. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: test `grid_public_api_exists` — calls `pick_minor_spacing_mm(1.0)` and `pick_major_step(10.0)` and verifies they compile and return finite values.
- **Unit (AC 2)**: test `minor_spacing_at_unit_zoom_is_10mm`.
- **Unit (AC 3)**: test `minor_spacing_at_zoom_in_is_1mm`.
- **Unit (AC 4)**: test `minor_spacing_at_zoom_out_is_100mm`.
- **Unit (AC 5)**: test `minor_spacing_at_half_unit_zoom_is_5mm`.
- **Unit (AC 6)**: test `minor_spacing_at_micro_zoom`.
- **Unit (AC 7)**: test `minor_spacing_clamped_at_extreme_zoom` — verifies bounded output and no panic for `mm_per_px = 1e-9` and `1e9`.
- **Unit (AC 8)**: test `major_step_is_ten`.
- **Unit (AC 9)**: test `draw_grid_does_not_panic_on_degenerate_cameras` — constructs an egui context for a `Painter` (via `egui::Context::default()` and a stub `Painter` from `Painter::new(...)`), calls `draw_grid` with each degenerate camera. The test relies on egui's headless Painter API; if that API is too awkward, the implementer instead exposes a `fn grid_world_bounds(rect, camera) -> (Vec2, Vec2)` internal helper and unit-tests that the bounds are finite for each degenerate camera. **The exact technique is open**; the AC is: `cargo test --all` passes without panic for the four listed camera configurations.
- **Manual smoke (AC 12)**: `cargo run`; verify a visible grid. Zoom in and out; confirm minor spacing stays readable. Record success in the demand's `Implementation:` line.
- **Static check (AC 13)**: `grep -nE '^use (eframe|rfd)' src/render/grid.rs` returns no matches.
- **Size check (AC 14)**: `wc -l src/render/grid.rs` reports `<= 300`.
- **Build gate (AC 15)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **1-2-5 ladder**: classic "engineering decade" spacing. Each decade is split as 1, 2, 5 — never 3 or 7. At any zoom there is always exactly one valid choice (the smallest one that clears the screen-pixel floor). v1's `tools/Grid.ts` used the same ladder.
- **MIN_SCREEN_PX = 8**: a reasonable lower bound for visibility on a non-HiDPI display. egui already scales for DPI via `ctx.pixels_per_point()`; if HiDPI users report the grid too fine, a future demand can multiply by `pixels_per_point()`. Not in scope here.
- **Major every 10**: the standard. AutoCAD's default major-every-five is rarer than major-every-ten in modern tools (Inkscape, FreeCAD, Fusion 360 all default to ten). Choosing 10 keeps the code simpler (no second 1-2-5 lookup).
- **No `EPSILON` comparison for major lines**: at zoom 0.5 (5 mm minor), the world coordinates of minor lines are exact multiples of 5 mm by construction (`floor(world_left / 5) * 5`). The integer-modulo check `(world_coord / minor).round() as i64 % 10 == 0` is robust as long as `world_coord` is an exact multiple of `minor`, which it is by construction. No epsilon dance needed.
- **Origin highlight deferred**: future visual polish demand (Phase 6) can color X=0 and Y=0 differently. The Phase-3 grid is plain — major-vs-minor is enough information.
- **Performance budget**: a 1280×800 viewport at `mm_per_px = 0.5` and `minor = 5` shows ~256 lines per axis (1280 px / 10 px screen spacing). That's ~512 line_segment calls per frame. Comfortably under egui's per-frame shape budget.
- **Painter clip rect**: `ui.painter_at(rect)` returns a painter whose clip rect is `rect`. Any line_segment outside the rect is clipped automatically. The implementer can either rely on that clipping or pre-clip world coordinates to the viewport bounds for efficiency. Pre-clipping is preferred (fewer shapes; less work).
- Reference: v1's `tools/Grid.ts` rendered the same 1-2-5 grid with major-every-10. v2 keeps the look.
