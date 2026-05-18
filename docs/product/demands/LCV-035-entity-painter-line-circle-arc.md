# LCV-035 — Entity painter (line/circle/arc)

- **Status**: Ready
- **Phase**: 3
- **Depends on**: LCV-021, LCV-031
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: <to be filled by demand-manager>

## Problem

The document holds `Vec<Entity>` (LCV-020 / LCV-021) but nothing draws it. Phase 4 tools commit `CreateLine` / `CreateCircle` / `CreateArc` commands that mutate `Document.entities`; without an entity painter, those commits are invisible. Every later demand — selection highlight (LCV-036), preview overlay (LCV-037), SVG export visual verification — assumes operators can see what they have drawn.

This demand ships the routine that walks `&[Entity]` and emits the right egui `Painter` calls for each variant: lines as line segments, circles via `Painter::circle_stroke`, arcs tessellated to polylines.

User outcome: the operator (once Phase 4 lands) draws a line, a circle, and a 90-degree arc; they appear on screen as crisp black 1-pixel strokes at the exact mm positions specified.

## Scope

- New file `src/render/entities.rs` defining:
  - `pub struct PaintOptions { pub stroke: egui::Stroke, pub arc_segments: usize }`.
    - Derives: `Debug`, `Clone`, `Copy`.
    - **Default**: `pub fn default() -> Self { Self { stroke: Stroke::new(1.0, Color32::from_gray(220)), arc_segments: 64 } }` — 1-px light-gray stroke (contrasts with the dark canvas) and 64 segments per full circle for arc tessellation.
  - `pub fn draw_entities(painter: &egui::Painter, rect: egui::Rect, camera: &Camera, entities: &[Entity], options: PaintOptions)`:
    - Iterate `entities`. For each entity, dispatch on the variant.
    - **Line** `Entity::Line(line)`:
      - `let p1 = world_to_screen_offset(rect, camera, line.p1);`
      - `let p2 = world_to_screen_offset(rect, camera, line.p2);`
      - `painter.line_segment([p1, p2], options.stroke);`
    - **Circle** `Entity::Circle(circle)`:
      - `let center = world_to_screen_offset(rect, camera, circle.center);`
      - `let radius_px = (circle.r / camera.mm_per_px) as f32;`
      - `painter.circle_stroke(center, radius_px, options.stroke);` (egui supports circle_stroke natively, no tessellation needed).
    - **Arc** `Entity::Arc(arc)`:
      - Call `arc_polyline(arc, options.arc_segments)` (helper below) to get a `Vec<Vec2>` of world-space sample points.
      - Convert each point via `world_to_screen_offset` and emit consecutive `line_segment` calls (or use `painter.add(egui::Shape::line(...))` if a single batched shape is cheaper — implementer's call; both are acceptable).
  - `pub fn arc_polyline(arc: &Arc, segments: usize) -> Vec<Vec2>`:
    - Sample the arc at `segments + 1` evenly-spaced angles from `arc.start_angle` to `arc.end_angle`, respecting `arc.ccw`. (The `Arc` type from LCV-013 exposes `start_angle`, `end_angle`, `ccw`, `center`, `r`.)
    - Returns a `Vec<Vec2>` of world-space points; consumer connects them with line segments.
    - `segments` is clamped: `segments.max(2)` (a 1-segment arc is degenerate; minimum 2 segments = 3 sample points).
  - Helper: `fn world_to_screen_offset(rect: egui::Rect, camera: &Camera, w: Vec2) -> egui::Pos2` — a local helper computing `camera.world_to_screen(w) + rect.min.to_vec2()`. Private to the module unless tests need it; if so, `pub(crate)`.
- Update `src/render/mod.rs`:
  - Add `pub mod entities;`.
  - Add `pub use entities::{draw_entities, arc_polyline, PaintOptions};`.
- Update `src/app.rs::update`:
  - Call `draw_entities(&painter, rect, &self.camera, &self.document.entities, PaintOptions::default());` after `draw_bed` and before any future preview / selection / snap-marker calls.
- File size: `src/render/entities.rs` stays ≤300 LOC.
- The file imports `egui::{Painter, Pos2, Rect, Stroke, Color32, Shape}` and `crate::geometry::{Vec2, Arc}`, `crate::document::Entity`, `crate::render::Camera`. No `eframe`, no `rfd`.
- Per-module unit tests under `#[cfg(test)] mod tests` in `entities.rs`.

## Out of scope

- **Filled entities** (cut/mark/engrave color presets per entity). v2 entities do not carry color metadata; coloring happens at SVG-export time (LCV-056) based on the user's preset assignment, not on the entity. Renderer is always 1-px light gray.
- **Dash patterns** (`stroke-dasharray` equivalent). v2 entities are solid lines; dashes are reserved for preview / construction lines (LCV-037).
- **Bezier paths**. v2 supports line, circle, arc only. No splines.
- **Hidden entities** (a per-entity visibility flag). v2 entities are always visible.
- **Per-entity stroke width override**. Single `PaintOptions::stroke` applies to all entities in a call. The caller passes different `PaintOptions` if different styles are needed (e.g., selection highlight uses a different stroke — that's LCV-036).
- **Z-order**. Entities are drawn in `Vec` order; later entries paint over earlier ones. No explicit layer system in v2.
- **Hit testing** (which entity is closest to the cursor). That's a geometry-kernel concern owned by Phase 4 (LCV-042 SelectTool); it does not live in the renderer.
- **Performance** beyond "linear scan + one draw call per entity". The v2 document size cap is ~10k entities; at 64 segments per arc that's ~640k line_segment calls in the worst case per frame, which is at egui's limit. Realistic documents have ~100–1000 entities; performance is comfortable. If profiling shows hot spots later, batch into `Painter::add(Shape::line(...))` is the first optimization.

## Acceptance criteria

1. `src/render/entities.rs` exists and defines `pub struct PaintOptions { pub stroke: egui::Stroke, pub arc_segments: usize }` with `Debug`, `Clone`, `Copy` derives.
2. `PaintOptions::default()` returns `arc_segments == 64` and a non-zero stroke width. Asserted via unit test.
3. `pub fn draw_entities(painter: &egui::Painter, rect: egui::Rect, camera: &Camera, entities: &[Entity], options: PaintOptions)` exists with that exact signature.
4. `pub fn arc_polyline(arc: &Arc, segments: usize) -> Vec<Vec2>` exists with that exact signature.
5. **arc_polyline sample count**: `arc_polyline(&arc, n).len() == n + 1` for `n >= 2`. For `n < 2`, `arc_polyline` clamps to `n = 2` and returns 3 points.
6. **arc_polyline endpoints match the arc**: `let pts = arc_polyline(&arc, 64); pts[0]` equals `arc.start_point()` (within `EPSILON`); `pts[64]` equals `arc.end_point()` (within `EPSILON`). For `arc = Arc { center: (0,0), r: 1, start_angle: 0, end_angle: PI/2, ccw: true }`, `pts[0] == Vec2(1, 0)` and `pts[64] == Vec2(0, 1)`.
7. **arc_polyline length approximates the arc length**: the total chord length `sum(|pts[i+1] - pts[i]|)` is within 0.5% of `arc.r * |arc.sweep()|` for `segments = 64`. (This is a numerical sanity check; tighter bounds would test polyline-arc convergence theory and aren't needed.)
8. **arc_polyline respects direction**: for a CCW arc from 0 to PI/2, `pts[1]` is in the first quadrant near `(cos(PI/128), sin(PI/128))`. For the same arc with `ccw = false` (the long way around), `pts[1]` is in the fourth quadrant (going negative-Y first).
9. **draw_entities does not panic** on: empty `entities` slice, a slice containing one of each variant (`Line`, `Circle`, `Arc`), degenerate camera (`mm_per_px = 1e-9` and `1e9`), zero-radius circle, zero-sweep arc. No automated rendering assert is required.
10. `src/render/mod.rs` re-exports `draw_entities`, `arc_polyline`, and `PaintOptions`.
11. `src/app.rs::update` calls `draw_entities` once per frame, passing `&self.document.entities` and `PaintOptions::default()`.
12. **Manual smoke**: with `cargo run`, add three entities to `App::default()` for testing purposes (e.g., a temporary `#[cfg(debug_assertions)]` block that pushes a line, circle, and arc directly into `document.entities` — to be removed before the demand commits, or kept behind a feature flag). Confirm all three render visibly on the grid. **Alternative**: this manual smoke can be deferred to LCV-043 (LineTool ships, then the operator draws lines normally). Implementer picks whichever is less invasive.
13. The file imports nothing from `eframe` or `rfd`. (Importing `egui` is allowed.)
14. Size: `wc -l src/render/entities.rs` reports `<= 300`. (If approaching 300, split into `entities.rs` + `arc_tess.rs`.)
15. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1, 2)**: test `paint_options_default_is_sensible` — asserts `arc_segments == 64` and `stroke.width > 0.0`.
- **Unit (AC 5)**: test `arc_polyline_sample_count` — verifies `n + 1` points for `n = 2, 8, 64, 256` and clamping for `n = 0, 1`.
- **Unit (AC 6)**: test `arc_polyline_endpoints_match_arc` — uses the standard 0→PI/2 CCW arc.
- **Unit (AC 7)**: test `arc_polyline_total_length_approximates_arc_length` — for the 90° unit arc, expected length `PI/2 * 1.0 ≈ 1.5708`; polyline with 64 segments approximates to within 0.5%.
- **Unit (AC 8)**: test `arc_polyline_respects_ccw_flag` — two arcs, same center / radius / start / end, opposite `ccw`; verifies the second sample point is in opposite half-planes.
- **Unit (AC 9)**: test `draw_entities_does_not_panic_on_degenerate_inputs` — see AC 9 pattern in LCV-033/034 (Painter via headless context, or split out a pure helper and test that). The acceptable approach: factor the screen-coord computation into a `pub(crate) fn entity_screen_segments(rect, camera, entity, segments) -> Vec<(Pos2, Pos2)>` helper returning all line segments to draw for an entity, and unit-test that helper for the degenerate cases (empty Vec, finite Vec, no panic).
- **Manual smoke (AC 12)**: see Scope and AC 12. Recorded in `Implementation:` line.
- **Static check (AC 13)**: `grep -nE '^use (eframe|rfd)' src/render/entities.rs` returns no matches.
- **Size check (AC 14)**: `wc -l src/render/entities.rs` reports `<= 300`.
- **Build gate (AC 15)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **64 segments / full circle** for arc tessellation: at 1 mm/px (default zoom) a 10 mm radius arc subtends ~62 pixels; 64 segments / 360° = ~5.6° per segment, which is ~1 px error at the perimeter. Visually crisp without breaking the per-frame shape budget. v1 used 64 as well.
- **`circle_stroke` for full circles**: egui's `Painter::circle_stroke` uses its own anti-aliased curve renderer (not a polyline approximation). At any zoom the circle stays smooth, no segment artifacts. Arcs do not get the same treatment because `circle_stroke` cannot render partial arcs — hence the polyline.
- **Light gray stroke (Color32::from_gray(220)) on dark canvas**: contrasts cleanly with the LCV-030 dark gray (gray(24)) canvas and LCV-034 bed fill (gray(40)). Phase 6 theme (LCV-071) can tune.
- **No per-entity color in v2 kernel**: AutoCAD has color-by-layer; v2 has color-by-export-preset (cut/mark/engrave). The renderer is preset-agnostic. The agent (LCV-078) and export (LCV-056) attach color at the boundary.
- **Helper `world_to_screen_offset`**: every Phase-3 demand (LCV-033..038) needs the same `world_to_screen(w) + rect.min.to_vec2()` translation. Duplicating it in each file is fine (KISS); promoting it to `Camera` is also fine but creates an implicit dependency on `Rect`, which is an egui type. The implementer picks: keep it local per file, OR add a `pub fn world_to_screen_in(&self, rect: Rect, w: Vec2) -> Pos2` to `Camera`. If the latter, the change happens in this demand (since LCV-035 is the first non-grid/bed renderer). Recommend: add the method to `Camera`. Document the choice in code.
- **Hit testing is geometry-kernel territory**: when LCV-042 implements SelectTool, the "which entity is under the cursor" function lives in `geometry/` (pure), not here. The renderer doesn't know about user intent; it only paints.
- Reference: v1's `tools/Renderer.ts` painted line, circle, and arc with very similar logic and the same 64-segment arc approximation. v2 keeps the shape.
