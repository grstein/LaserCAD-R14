# LCV-038 — Snap marker rendering

- **Status**: Done
- **Phase**: 3
- **Depends on**: LCV-031, LCV-016
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: 7db1ddb — feat(LCV-038): snap marker rendering — endpoint/midpoint/center/intersection shapes

## Problem

The snap engine (LCV-016) returns a `SnapResult { point, kind, primary_idx, secondary_idx }` whenever the cursor is within tolerance of a snap candidate. Without a visible marker at the snapped point, the operator cannot tell whether the snap is active — they would click and discover after the fact whether they landed on the endpoint or the eyeballed position. AutoCAD R14 conventionally draws a distinct marker shape per snap kind (square for endpoint, X for intersection, triangle for midpoint, circle for center); muscle memory from AutoCAD users carries over directly. v1 used the same convention.

This demand ships the renderer that draws one marker shape per `SnapKind` at the snapped world point. The caller (Phase-4 tools via LCV-054) computes the SnapResult once per frame and passes it to `draw_snap_marker`.

User outcome: as the operator moves the cursor near a line endpoint, a small orange square appears at the endpoint. Near a circle center, an orange circle outline appears. Near a line-line intersection, an orange X. The visual language tells the operator "the next click will land here, snapped to this feature".

## Scope

- New file `src/render/snaps.rs` defining:
  - `pub fn draw_snap_marker(painter: &egui::Painter, rect: egui::Rect, camera: &Camera, snap: &SnapResult)`:
    - Convert `snap.point` (world) to screen-space via `camera.world_to_screen(snap.point) + rect.min.to_vec2()`.
    - Dispatch on `snap.kind`:
      - `Endpoint` → `endpoint_marker(painter, screen_pos, marker_size_px, color)` — draws a filled square.
      - `Midpoint` → `midpoint_marker(...)` — draws a filled triangle (pointing up).
      - `Center` → `center_marker(...)` — draws an unfilled circle (stroke only — visually different from a filled circle so it doesn't read as a "dot").
      - `Intersection` → `intersection_marker(...)` — draws an X (two crossed line segments).
    - **Marker size**: `MARKER_SIZE_PX: f32 = 8.0` (constant in the module). Markers are sized in screen pixels, not world units — they stay a constant size on screen regardless of zoom.
    - **Marker color**: `MARKER_COLOR: Color32 = Color32::from_rgb(255, 160, 0)` — high-contrast orange. Distinguishable from entity gray, selection cyan, and preview amber. Visible against dark canvas and against the lighter bed fill.
  - **Pure shape helpers** (each is a small `pub(crate) fn` that returns a geometry primitive but does not touch the `Painter`):
    - `pub(crate) fn endpoint_marker_corners(center: egui::Pos2, size_px: f32) -> [egui::Pos2; 4]` — returns the four corners of the square `[top_left, top_right, bottom_right, bottom_left]`.
    - `pub(crate) fn midpoint_marker_corners(center: egui::Pos2, size_px: f32) -> [egui::Pos2; 3]` — returns the three vertices of an upward-pointing equilateral triangle inscribed in the size_px box.
    - `pub(crate) fn intersection_marker_segments(center: egui::Pos2, size_px: f32) -> [[egui::Pos2; 2]; 2]` — returns the two line segments forming the X (both diagonals of the size_px box).
    - `pub(crate) fn center_marker_radius(size_px: f32) -> f32` — returns the radius of the center-marker circle (`size_px / 2.0`).
  - The shape helpers are **the actually testable units** for this demand. `draw_snap_marker` itself is a thin orchestrator: it calls the right helper, then emits Painter calls (`rect_filled`, `add(Shape::convex_polygon(...))`, `line_segment`, `circle_stroke`).
- Update `src/render/mod.rs`:
  - Add `pub mod snaps;`.
  - Add `pub use snaps::draw_snap_marker;`.
- Update `src/app.rs`:
  - Add `pub active_snap: Option<SnapResult>` to the `App` struct. Defaults to `None`. Will be written by Phase 4 (LCV-054) when a tool computes a live snap.
  - In `App::update`, after `draw_preview` (LCV-037), call `if let Some(snap) = &self.active_snap { draw_snap_marker(&painter, rect, &self.camera, snap); }`. In this demand, `active_snap` remains `None` (LCV-054 writes it).
- File size: `src/render/snaps.rs` stays ≤300 LOC.
- The file imports `egui::{Painter, Pos2, Rect, Stroke, Color32, Shape}` and `crate::geometry::{Vec2, SnapResult, SnapKind}`, `crate::render::Camera`. No `eframe`, no `rfd`.
- Per-module unit tests under `#[cfg(test)] mod tests` in `snaps.rs`.

## Out of scope

- **Snap tooltip text** ("endpoint", "midpoint of line 3") next to the marker. AutoCAD shows them; v2 does not in Phase 3. Future visual-polish demand (Phase 6 LCV-071) may add.
- **Snap mode toggle UI** (a button or hotkey to disable per-kind snaps): the snap toggle and the per-kind enable/disable live elsewhere (LCV-053 ortho-style toggle for ortho; a future "OSnap" toggle demand for per-kind snap toggling). This demand draws whatever the caller passes — if the caller passes `None`, nothing draws.
- **Snap "lock" preview** (a clearer "this snap will commit on click" hint, like a colored outline around the snapped entity). Future demand if needed.
- **Animated markers** (pulsing, fading). Static; rejected.
- **Snap engine itself** — that's LCV-016, already shipped.
- **Tool integration** (when to invoke the snap engine, what tolerance to use, when to write `app.active_snap`) — owned by **LCV-054**. This demand only paints the marker.
- **`SnapEntity` → `Entity` migration** (LCV-016 used a transitional `SnapEntity` enum because LCV-020 hadn't landed yet — see LCV-016 Notes). The migration is owned by a follow-up demand the project-manager schedules; not in scope here.
- **Multi-snap display** (showing endpoint + midpoint candidates simultaneously when both are within tolerance). LCV-016 returns one best SnapResult; this demand draws one marker. If multi-snap display is needed in the future, a separate demand expands the API.

## Acceptance criteria

1. `src/render/snaps.rs` exists and defines `pub fn draw_snap_marker(painter: &egui::Painter, rect: egui::Rect, camera: &Camera, snap: &SnapResult)` with that exact signature.
2. **endpoint_marker_corners**: `endpoint_marker_corners(Pos2::new(100.0, 100.0), 8.0)` returns the four corners `[Pos2::new(96.0, 96.0), Pos2::new(104.0, 96.0), Pos2::new(104.0, 104.0), Pos2::new(96.0, 104.0)]` (top-left first, clockwise). Asserted within `f32::EPSILON`.
3. **midpoint_marker_corners**: returns three points forming an upward-pointing equilateral-ish triangle inscribed in the size_px box. The exact vertex coordinates are implementer-chosen (any well-defined inscribed triangle is acceptable); the test asserts: (a) three points, (b) one vertex strictly above the other two (the "top" of an upward-pointing triangle), (c) the bottom two vertices are symmetric about the marker center's X coordinate.
4. **intersection_marker_segments**: returns two line segments `[[Pos2::new(96.0, 96.0), Pos2::new(104.0, 104.0)], [Pos2::new(96.0, 104.0), Pos2::new(104.0, 96.0)]]` (the two diagonals of the size_px square). Asserted within `f32::EPSILON`.
5. **center_marker_radius**: `center_marker_radius(8.0)` returns `4.0` (exactly half the size_px).
6. **Marker size is constant regardless of camera**: the marker shape helpers take `size_px: f32` directly; they do not consult `Camera`. The unit tests (AC 2–5) exercise this — the same `size_px = 8.0` input produces the same shape regardless of any camera state. (This is implicit in the function signatures; no separate assert is needed.)
7. **Marker color is high-contrast and not gray**: the marker color (encoded as a module-level constant `pub(crate) const MARKER_COLOR: Color32` or `pub(crate) fn marker_color() -> Color32`) has R > 200, G in (100, 200), and B < 100 (i.e., orange-band — pins the band, not the exact value, so theme tuning doesn't break the test).
8. **draw_snap_marker does not panic** for any of the four `SnapKind` variants with any camera state, including degenerate camera (`mm_per_px = 1e-9`, `1e9`) and large world coordinates (`snap.point = Vec2::new(1e6, 1e6)`). The function returns normally.
9. **draw_snap_marker dispatches correctly per SnapKind**: encoded as a trace test. The implementer either (a) exposes a `pub(crate) enum MarkerShape { Square, Triangle, Circle, X }` and a `pub(crate) fn marker_shape_for(kind: SnapKind) -> MarkerShape` mapping function that the test asserts (Endpoint → Square, Midpoint → Triangle, Center → Circle, Intersection → X); or (b) passes a closure-based callback to `draw_snap_marker` that records the shape drawn. Either approach satisfies the AC.
10. `src/render/mod.rs` re-exports `draw_snap_marker`.
11. `src/app.rs` `App` struct carries `pub active_snap: Option<SnapResult>` defaulting to `None`. `App::default().active_snap.is_none()` is `true`.
12. `src/app.rs::update` calls `draw_snap_marker` when `active_snap` is `Some`, after `draw_preview`.
13. The file imports nothing from `eframe` or `rfd`. (Importing `egui` is allowed.)
14. Size: `wc -l src/render/snaps.rs` reports `<= 300`.
15. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: test `draw_snap_marker_function_exists` — calls `draw_snap_marker` with a default-constructed `SnapResult`; verifies signature.
- **Unit (AC 2)**: test `endpoint_marker_corners_at_origin_with_size_8`.
- **Unit (AC 2, second case)**: test `endpoint_marker_corners_offset` — at `(50, 75)` with size 6, asserts corners `[(47, 72), (53, 72), (53, 78), (47, 78)]`.
- **Unit (AC 3)**: test `midpoint_marker_is_upward_triangle` — asserts three points, top vertex has the smallest Y (egui's Y-down convention; "up" on screen = smaller Y), bottom two share their Y and are symmetric in X.
- **Unit (AC 4)**: test `intersection_marker_is_two_diagonals`.
- **Unit (AC 5)**: test `center_marker_radius_is_half_size`.
- **Unit (AC 7)**: test `marker_color_is_orange_band`.
- **Unit (AC 8)**: test `draw_snap_marker_does_not_panic_on_degenerate_inputs` — same pattern as LCV-035 AC 9. Either via headless Painter or via the marker-shape helpers being purely numeric (which they are by design — the test calls `endpoint_marker_corners` etc. with extreme inputs and asserts finite results).
- **Unit (AC 9)**: test `marker_shape_dispatch` — for each of the four `SnapKind` variants, the dispatch function returns the expected `MarkerShape` (or callback records the expected shape).
- **Integration (AC 11)**: extend `tests/skeleton.rs` to construct `lasercad::app::App::default()` and assert `app.active_snap.is_none()`.
- **Static check (AC 13)**: `grep -nE '^use (eframe|rfd)' src/render/snaps.rs` returns no matches.
- **Size check (AC 14)**: `wc -l src/render/snaps.rs` reports `<= 300`.
- **Build gate (AC 15)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **Marker shape per kind** (square / triangle / circle / X): matches AutoCAD R14's default OSNAP marker conventions. v1 used the same shapes. Operators familiar with AutoCAD or any AutoCAD clone will recognize them.
- **Why pure shape helpers**: testing `Painter` calls directly requires a stub Painter or `egui_kittest`. The egui-best-practices skill says: don't add UI testing infra preemptively. Instead, the shape geometry is computed in pure helpers (`endpoint_marker_corners`, etc.) and unit-tested as plain math; `draw_snap_marker` is a thin orchestrator that's covered by manual smoke. This is the same pattern as LCV-032 (helpers + manual smoke) and applies throughout Phase 3.
- **8-pixel marker size**: matches v1 and AutoCAD R14's default. Small enough to not obscure detail; large enough to be visible. HiDPI: egui scales by `pixels_per_point()` automatically when rendering shapes derived from `Pos2`, so a "8 pixel" marker stays roughly 8 logical pixels regardless of DPI. If users on 4K displays report the marker too small, multiply by `pixels_per_point()` in a future demand.
- **Orange color** (`rgb(255, 160, 0)`): distinct from entity gray, selection cyan, preview amber. Amber and orange are close — the saving grace is that the snap marker is a small fixed-shape at the cursor, while the preview is an extended stroke; visual confusion is minimal in practice.
- **Center marker as stroke (not fill)**: prevents it from reading as a "dot" (which AutoCAD reserves for the "point" entity). Outline-only circles are unambiguously "center snap".
- **Triangle direction**: upward-pointing for `Midpoint`. v1 used the same. Some CAD tools use a downward triangle for "perpendicular"; v2 doesn't have perpendicular snap yet (out of scope per LCV-016) so the direction is free to be "up" for midpoint.
- **`active_snap` writer**: LCV-054 (snap integration into tools) will read the cursor's world position, call `snap()` from LCV-016, and assign the result to `app.active_snap`. This demand only ships the field and the painter.
- Reference: v1's `tools/SnapRenderer.tsx` drew the same four shapes in orange at the snap point. v2 keeps the visual language.
