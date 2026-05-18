# LCV-031 — Camera (world↔screen, zoom, pan, zoom-extents)

- **Status**: Ready
- **Phase**: 3
- **Depends on**: LCV-021, LCV-030
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: <to be filled by demand-manager>

## Problem

Every Phase-3 renderer (grid, bed, entities, preview, snap markers) and every Phase-4 tool (pointer-to-world translation, snap lookup) needs a single source of truth for "where in the world is the cursor" and "where on the screen does this millimeter coordinate land". Without it, each renderer would invent its own pan/zoom math, the grid would not stay aligned with entities, and snap markers would drift from the cursor on zoom. AGENTS.md §"Units and types" pins the contract: millimeters are canonical in the document and kernel; pixels appear only inside `render/camera`.

This demand ships the `Camera` struct that owns that boundary. It exposes `world_to_screen`, `screen_to_world`, zoom in/out, pan, and zoom-extents. The next demand (LCV-032) plugs cursor and input events into it; LCV-033..038 paint through it.

User outcome: the operator zooms the wheel and the drawing scales around the cursor (not the center of the window); pans with the middle button and the drawing tracks the cursor 1:1; presses `F` (zoom-extents) and the camera fits the document's `bounds()` with a comfortable margin. A line drawn at world (0, 0) stays at world (0, 0) regardless of camera state.

## Scope

- New file `src/render/camera.rs` defining:
  - `pub struct Camera { pub center_world: Vec2, pub mm_per_px: f64, pub viewport_size_px: [f32; 2] }`.
    - Derives: `Debug`, `Clone`, `PartialEq`. No `Copy` (the struct is small but tools and renderers borrow it; cloning is cheap enough that the explicit call keeps intent clear).
    - **Field semantics**:
      - `center_world`: the world-space (mm) coordinate that maps to the geometric center of the viewport.
      - `mm_per_px`: the zoom factor — millimeters per screen pixel. Smaller value = more zoomed in. Must be strictly positive; debug-asserted via `debug_assert!(self.mm_per_px > 0.0)` inside `world_to_screen` / `screen_to_world`.
      - `viewport_size_px`: the current viewport size in screen pixels. Written by `App::update` every frame before calling any rendering function; readers may assume it is current.
  - `impl Default for Camera`: `center_world = Vec2::new(0.0, 0.0); mm_per_px = 1.0; viewport_size_px = [0.0, 0.0]`. The zero-sized viewport is a sentinel: `App::update` overwrites it on every tick before any draw call uses it. (Tests that exercise transforms construct a non-zero viewport explicitly.)
  - Public methods:
    - `pub fn world_to_screen(&self, w: Vec2) -> egui::Pos2`.
      - Formula: `let half = Vec2::new(self.viewport_size_px[0] as f64 / 2.0, self.viewport_size_px[1] as f64 / 2.0); let dx = (w.x - self.center_world.x) / self.mm_per_px; let dy = (w.y - self.center_world.y) / self.mm_per_px; egui::Pos2::new((half.x + dx) as f32, (half.y - dy) as f32)`.
      - **Y is flipped**: world Y goes up (mathematical convention, matches v1 and AutoCAD); screen Y goes down (egui convention). The minus sign on `dy` does the flip.
    - `pub fn screen_to_world(&self, s: egui::Pos2) -> Vec2`.
      - Inverse of the above. Used by pointer handlers (LCV-032) and by tools (Phase 4).
    - `pub fn zoom_in(&mut self, factor: f64)` — multiplies `mm_per_px` by `1.0 / factor` (factor > 1.0 means zoom-in). Caller passes a positive factor (typically `1.1` per wheel notch).
    - `pub fn zoom_out(&mut self, factor: f64)` — multiplies `mm_per_px` by `factor` (factor > 1.0 means zoom-out).
    - `pub fn zoom_around(&mut self, screen_anchor: egui::Pos2, factor: f64)` — zoom in/out around a specific screen point (the cursor). Computes the world point under `screen_anchor`, applies the zoom, then shifts `center_world` so the same world point is still under `screen_anchor`. `factor < 1.0` zooms out; `factor > 1.0` zooms in. (Single method; LCV-032 uses this for wheel-zoom-at-cursor and the menu Zoom In / Zoom Out actions reuse it with `screen_anchor` = viewport center.)
    - `pub fn pan(&mut self, delta_screen_px: egui::Vec2)` — translates `center_world` by `-delta_screen_px * mm_per_px` (the world stays under the cursor as the cursor drags). The minus sign accounts for "drag right → world shifts right under cursor → camera center moves left in world space".
    - `pub fn zoom_extents(&mut self, bounds: Option<(Vec2, Vec2)>, viewport_size_px: [f32; 2])` — fits the bounding box to the viewport with 10% padding on each side. If `bounds` is `None` (empty document), sets `center_world = (0, 0)` and `mm_per_px = 1.0` (the default view). Also updates `self.viewport_size_px = viewport_size_px` as a side effect (so a caller can pass the current viewport size and not have to set it separately).
      - Math: take the larger of `(max.x - min.x) / viewport_width_px` and `(max.y - min.y) / viewport_height_px`, multiply by `1.0 / 0.8` (the inverse of "80% of viewport = drawing area"), use that as `mm_per_px`. `center_world = ((min.x + max.x) / 2.0, (min.y + max.y) / 2.0)`. If width or height is zero (a single point or a horizontal/vertical line), fall back to `mm_per_px = 1.0`.
- Update `src/render/mod.rs`:
  - Add `pub mod camera;`.
  - Add `pub use camera::Camera;`.
- Update `src/app.rs`:
  - Add `pub camera: Camera` to the `App` struct. `Camera::default()` slots into the `#[derive(Default)]`. The `TODO(LCV-031)` comment from LCV-030 is removed.
  - In `App::update`, after allocating the viewport rect, set `self.camera.viewport_size_px = [rect.width(), rect.height()];` before any rendering call. (No rendering happens yet in this demand — LCV-033 onward consumes the camera. But the write happens here so subsequent demands inherit the wiring.)
- File size: `src/render/camera.rs` stays ≤300 LOC.
- The file imports `egui::{Pos2, Vec2 as EguiVec2}` (egui's pixel-space Vec2, aliased to avoid collision with `geometry::Vec2`). It also imports `crate::geometry::Vec2`. No `eframe`, no `rfd`.
- Per-module unit tests under `#[cfg(test)] mod tests` in `camera.rs`.

## Out of scope

- **Cursor tracking on the camera struct**: the cursor's last-known world position lives on `App`, not on `Camera`. Camera is stateless w.r.t. pointer history.
- **Pointer-event handling** (mouse wheel → `zoom_around`, middle-drag → `pan`, `F` key → `zoom_extents`) — owned by **LCV-032**. This demand provides the math; LCV-032 wires the events.
- **Animation / easing** on zoom and pan. v2 snaps instantly to the new camera state; smoothing is decorative and rejected.
- **Multi-viewport / split-pane**. v2 has one viewport.
- **Camera persistence across sessions**. Reopening the app starts at the default view; loading a file calls `zoom_extents`. Settings persistence is owned by LCV-058.
- **Constraining `mm_per_px` to a min/max range** (clamping extreme zoom). Defer until a user reports a problem — at extreme zoom the rendering becomes degenerate but does not panic (`debug_assert > 0.0` is the only invariant).
- **Rotation** of the camera. v2 is 2D axis-aligned; rotation would invalidate every snap math assumption. Rejected.

## Acceptance criteria

1. `src/render/camera.rs` exists and defines `pub struct Camera { pub center_world: Vec2, pub mm_per_px: f64, pub viewport_size_px: [f32; 2] }` with `Debug`, `Clone`, `PartialEq` derives.
2. `Camera::default()` produces `center_world = Vec2::new(0.0, 0.0)`, `mm_per_px = 1.0`, `viewport_size_px = [0.0, 0.0]`.
3. **World→screen round-trip**: for `Camera { center_world: Vec2::new(0.0, 0.0), mm_per_px: 1.0, viewport_size_px: [800.0, 600.0] }`, `camera.world_to_screen(Vec2::new(0.0, 0.0))` equals `egui::Pos2::new(400.0, 300.0)` (viewport center). `camera.screen_to_world(egui::Pos2::new(400.0, 300.0))` equals `Vec2::new(0.0, 0.0)` (within `EPSILON`).
4. **Y-axis flip**: with the same camera, `camera.world_to_screen(Vec2::new(0.0, 10.0))` equals `egui::Pos2::new(400.0, 290.0)` (world Y up = screen Y down).
5. **Zoom changes scale**: with `mm_per_px = 2.0` (zoomed out), `world_to_screen(Vec2::new(10.0, 0.0))` equals `Pos2::new(405.0, 300.0)` (10 mm = 5 px at this zoom). With `mm_per_px = 0.5` (zoomed in), the same call returns `Pos2::new(420.0, 300.0)`.
6. **Pan shifts center_world**: calling `camera.pan(egui::Vec2::new(100.0, 0.0))` with `mm_per_px = 1.0` shifts `center_world.x` by exactly `-100.0` (drag-right → world center moves left in world space by 100 mm).
7. **zoom_around preserves the anchor**: with any camera, `let world_under_anchor = camera.screen_to_world(anchor); camera.zoom_around(anchor, 2.0); let world_after = camera.screen_to_world(anchor); assert_eq!(world_under_anchor, world_after);` (within `EPSILON`).
8. **zoom_extents on a known bbox**: with `bounds = Some((Vec2::new(0.0, 0.0), Vec2::new(100.0, 50.0)))` and `viewport_size_px = [800.0, 600.0]`, after `zoom_extents`, `camera.center_world` equals `Vec2::new(50.0, 25.0)` (within `EPSILON`). `camera.mm_per_px` equals `max(100.0/800.0, 50.0/600.0) / 0.8 = max(0.125, 0.0833...) / 0.8 = 0.15625` (within `EPSILON`).
9. **zoom_extents on None**: `camera.zoom_extents(None, [800.0, 600.0])` leaves `center_world = (0, 0)` and `mm_per_px = 1.0`. Viewport size is updated to `[800.0, 600.0]`.
10. **zoom_extents on zero-extent bounds** (single point or zero-width line): `bounds = Some((Vec2::new(5.0, 5.0), Vec2::new(5.0, 5.0)))` results in `center_world = Vec2::new(5.0, 5.0)` and `mm_per_px = 1.0` (fallback).
11. **Round-trip after arbitrary pan + zoom**: for any starting camera, after a sequence `camera.pan(p); camera.zoom_around(anchor, f); camera.pan(p2);`, the identity `camera.screen_to_world(camera.world_to_screen(w)) == w` holds (within `EPSILON`) for any `w`.
12. `src/render/mod.rs` re-exports `Camera`.
13. `src/app.rs` `App` struct carries `pub camera: Camera`. `App::default().camera == Camera::default()`. Inside `App::update`, before any rendering call, `self.camera.viewport_size_px` is assigned `[rect.width(), rect.height()]` where `rect` is the viewport rect allocated in LCV-030.
14. `src/render/camera.rs` imports nothing from `eframe` or `rfd`. (Importing `egui` is allowed — render IS the UI side.)
15. Size: `wc -l src/render/camera.rs` reports `<= 300`.
16. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1, 2)**: test `camera_default_values` — asserts the three default fields.
- **Unit (AC 3)**: test `world_to_screen_and_back_at_origin`.
- **Unit (AC 4)**: test `y_axis_is_flipped`.
- **Unit (AC 5)**: test `zoom_changes_scale` — two camera instances with different `mm_per_px`, same world point, asserts the expected pixel positions.
- **Unit (AC 6)**: test `pan_shifts_center_world`.
- **Unit (AC 7)**: test `zoom_around_preserves_world_under_anchor` — picks a non-trivial anchor (e.g., `(100, 100)`) and asserts the screen→world value matches before/after.
- **Unit (AC 8)**: test `zoom_extents_fits_known_bbox`.
- **Unit (AC 9)**: test `zoom_extents_on_empty_bounds_uses_default_view`.
- **Unit (AC 10)**: test `zoom_extents_on_degenerate_bounds_falls_back`.
- **Unit (AC 11)**: test `roundtrip_after_pan_zoom_pan` — applies a random-ish sequence and asserts the identity over a small grid of world points.
- **Integration (AC 13)**: extend `tests/skeleton.rs` (or add a new integration test) constructing `lasercad::app::App::default()` and asserting `app.camera.mm_per_px == 1.0` — proves the `App` field is wired through the public surface.
- **Static check (AC 14)**: `grep -nE '^use (eframe|rfd)' src/render/camera.rs` returns no matches.
- **Size check (AC 15)**: `wc -l src/render/camera.rs` reports `<= 300`.
- **Build gate (AC 16)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **`egui::Vec2` vs `geometry::Vec2` naming collision**: the module imports `crate::geometry::Vec2` directly and qualifies egui's vector as `egui::Vec2` at call sites, or aliases via `use egui::Vec2 as EguiVec2`. Either pattern is fine; the implementer picks one and stays consistent.
- **Why `mm_per_px` and not `px_per_mm`**: at extreme zoom-out, `px_per_mm` would be a very small fraction; `mm_per_px` stays a tidy number. Either works mathematically; this picks the form that matches the AGENTS.md naming convention ("millimeters canonical, pixels only inside `render/camera`").
- **Y-flip**: world Y up matches v1 (`tools/Camera.ts`) and AutoCAD. egui's `Pos2` has Y growing downward. The flip is a single minus sign in `world_to_screen` / `screen_to_world`.
- **80% margin in `zoom_extents`**: leaves a 10% breathing strip on each side of the drawing. Matches AutoCAD's default Z→E behavior. Tunable later if a user complains.
- **`debug_assert!(mm_per_px > 0.0)`**: pinned because dividing by zero or negative zoom would silently produce NaN coordinates and a blank viewport. Asserting in debug + propagating the bad state in release is the right tradeoff (no panic in release; reviewer catches the bug in development).
- **Setting `viewport_size_px` in `App::update`**: the dependency direction is "App writes the camera size each frame; renderers read it". This keeps `Camera` ignorant of egui's UI lifecycle and lets render functions take `&Camera` (immutable) instead of `&mut Camera`.
- **No motion smoothing / inertia**: v2 is a precision tool; instant camera response matches AutoCAD R14 and avoids the surprise of "I let go of the mouse but the camera is still moving".
- Reference: v1's `tools/Camera.ts` had `worldToScreen`, `screenToWorld`, `zoomIn`, `zoomOut`, `pan`, `zoomToExtents` with essentially the same math; v2 keeps the same shape, adds `zoom_around`, and lives in Rust.
