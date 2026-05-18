# LCV-015 — Axis-aligned Rect with contains/crosses predicates

- **Status**: Done
- **Phase**: 1
- **Depends on**: LCV-011, LCV-012, LCV-013
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: implementer-rust (task #38); b7a9882

## Problem

AutoCAD's selection workflow distinguishes two modes by drag direction:
- Left-to-right drag = **window-box** selection (entity selected only if fully inside the rectangle).
- Right-to-left drag = **crossing-box** selection (entity selected if any part touches the rectangle).

The future SelectTool (LCV-042) needs to ask "does this rectangle fully contain entity X?" and "does this rectangle cross entity X?" for every line, circle, and arc in the document. Without a shared `Rect` type and the eight contains/crosses predicates, the SelectTool would inline the math (>300 LOC, blowing the per-file cap and tangling rendering with geometry). This demand factors the predicates into the kernel where they can be unit-tested in isolation.

## Scope

- New file `src/geometry/rect.rs` defining:
  - `pub struct Rect { pub min: Vec2, pub max: Vec2 }` (axis-aligned; `min.x <= max.x` and `min.y <= max.y` always).
  - Derives: `Copy`, `Clone`, `Debug`, `PartialEq`. No `Eq`, no `Hash`.
  - Constructor `pub fn new(p1: Vec2, p2: Vec2) -> Self` — auto-normalizes so `min = (min(p1.x, p2.x), min(p1.y, p2.y))` and `max = (max(p1.x, p2.x), max(p1.y, p2.y))`. Accepts any two corner points in any order; cannot be `const fn` because `f64::min` / `f64::max` are not `const`.
  - Methods:
    - `width(&self) -> f64`, `height(&self) -> f64`.
    - `bbox(&self) -> (Vec2, Vec2)` — returns `(self.min, self.max)`. Trivial but kept for API symmetry with other kernel types.
    - `contains_point(&self, p: Vec2) -> bool` — `true` iff `min.x <= p.x <= max.x` and `min.y <= p.y <= max.y` (inclusive bounds; no `EPSILON` slack — picking is a UI concern, the kernel predicate is exact).
    - `contains_line(&self, line: &Line) -> bool` — `true` iff both endpoints are contained.
    - `crosses_line(&self, line: &Line) -> bool` — `true` iff any part of the segment overlaps the rectangle, including endpoint touches and segments fully inside. Implementation hint: returns `true` if `contains_line` is true, **or** if the segment intersects any of the four rect edges (use `intersect::line_line` from LCV-014).
    - `contains_circle(&self, c: &Circle) -> bool` — `true` iff the circle's bbox is fully inside the rectangle (i.e., `min.x <= c.center.x - c.r` and `c.center.x + c.r <= max.x`, similarly for y). For axis-aligned `Rect`, the circle-bbox test is exact for full containment.
    - `crosses_circle(&self, c: &Circle) -> bool` — `true` iff the rectangle and circle overlap. Standard "closest point on rect to circle center" test: `let cp = Vec2::new(c.center.x.clamp(min.x, max.x), c.center.y.clamp(min.y, max.y)); cp.distance(c.center) <= c.r + EPSILON`.
    - `contains_arc(&self, a: &Arc) -> bool` — `true` iff the arc's bbox is fully inside the rectangle. Uses `Arc::bbox()` from LCV-013 (which already accounts for cardinal extreme angles).
    - `crosses_arc(&self, a: &Arc) -> bool` — `true` iff either (a) `contains_arc(a)` is true, or (b) the arc's bbox intersects the rectangle's bbox **and** at least one of the following holds: `start_point` or `end_point` is inside the rectangle, OR the arc's parent circle crosses any of the four rect edges at a point whose angle is contained in the arc's sweep. Implementation hint: enumerate the four edges (as `Line`s), compute `line_circle` against the parent `Circle`, filter the resulting points by `arc.contains_angle((p - arc.center).y.atan2((p - arc.center).x))`.
- Update `src/geometry/mod.rs`:
  - Add `pub mod rect;` and `pub use rect::Rect;`.
- Per-module unit tests under `#[cfg(test)] mod tests` in `rect.rs`. No new file under `tests/`.

## Out of scope

- The SelectTool itself — owned by **LCV-042** (Phase 4).
- Render-side picking, hit-testing for individual entities by a click point (point-pick rather than box-pick) — that lives in `render/` once the renderer arrives.
- A "fuzzy" pixel-tolerance picking API. The kernel predicates are exact; pixel tolerance is applied at the UI layer.
- Rect-rect predicates (intersection of two rects). Not needed by any current consumer.
- Oriented (rotated) rectangles. v2's selection box is always axis-aligned in world coordinates.
- Polygon containment. v2 has no polygon entity in Phase 1.

## Acceptance criteria

1. `src/geometry/rect.rs` exists and defines `pub struct Rect { pub min: Vec2, pub max: Vec2 }` with `#[derive(Copy, Clone, Debug, PartialEq)]` (no `Eq`, no `Hash`).
2. `Rect::new(Vec2::new(5.0, 3.0), Vec2::new(1.0, 7.0))` produces `min = Vec2::new(1.0, 3.0)`, `max = Vec2::new(5.0, 7.0)` (auto-normalization).
3. `width()` and `height()` return `4.0` and `4.0` respectively for the rect in AC 2.
4. `bbox()` returns `(self.min, self.max)`.
5. `contains_point`: **PASS** — for `Rect::new(Vec2(0,0), Vec2(10,10))`, `contains_point(Vec2(5,5))` returns `true`; boundary `contains_point(Vec2(10,10))` returns `true`. **FAIL** — `contains_point(Vec2(11,5))` returns `false`.
6. `contains_line`: **PASS** — for the same rect, `contains_line(&Line::new(Vec2(2,2), Vec2(8,8)))` returns `true`. **FAIL** — `contains_line(&Line::new(Vec2(2,2), Vec2(15,8)))` returns `false` (one endpoint outside).
7. `crosses_line`: **PASS** — for the same rect, `crosses_line(&Line::new(Vec2(-5,5), Vec2(15,5)))` returns `true` (segment crosses through). **FAIL** — `crosses_line(&Line::new(Vec2(-5,5), Vec2(-1,5)))` returns `false` (segment entirely to the left). **PASS** (additional): `crosses_line(&Line::new(Vec2(2,2), Vec2(8,8)))` returns `true` (segment fully inside also counts as crossing).
8. `contains_circle`: **PASS** — for the same rect, `contains_circle(&Circle::new(Vec2(5,5), 3.0))` returns `true`. **FAIL** — `contains_circle(&Circle::new(Vec2(5,5), 6.0))` returns `false` (circle pokes out).
9. `crosses_circle`: **PASS** — for the same rect, `crosses_circle(&Circle::new(Vec2(11,5), 2.0))` returns `true` (circle overlaps right edge). **FAIL** — `crosses_circle(&Circle::new(Vec2(15,5), 2.0))` returns `false` (circle entirely outside, distance > radius).
10. `contains_arc`: **PASS** — for the same rect, `contains_arc(&Arc { center: Vec2(5,5), r: 2.0, start_angle: 0.0, end_angle: PI/2, ccw: true })` returns `true` (quarter-arc bbox `((5,5), (7,7))` fits inside). **FAIL** — same arc with `center: Vec2(9,5), r: 2.0` returns `false` (the bbox `((9,5),(11,7))` exceeds the right edge).
11. `crosses_arc`: **PASS** — for the same rect, an arc whose endpoint pokes inside returns `true` (e.g., `Arc { center: Vec2(11, 5), r: 3.0, start_angle: PI, end_angle: 3*PI/2, ccw: true }` — endpoint `(11, 2)` and starts at `(8, 5)` which is inside the rect). **FAIL** — an arc entirely outside the rectangle (e.g., `Arc { center: Vec2(20, 20), r: 1.0, start_angle: 0.0, end_angle: PI/2, ccw: true }`) returns `false`.
12. `src/geometry/mod.rs` re-exports `Rect` so `use lasercad::geometry::Rect;` works from outside the module.
13. `src/geometry/rect.rs` imports nothing from `egui`, `eframe`, or `rfd`; the file stays under 300 LOC. The file may import from `crate::geometry::{Vec2, Line, Circle, Arc, EPSILON, intersect}`.
14. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: a test that constructs `Rect { min: Vec2::default(), max: Vec2::new(1.0, 1.0) }` via the struct literal (proves fields are `pub`).
- **Unit (AC 2)**: test `new_auto_normalizes_corners`.
- **Unit (AC 3)**: test `width_and_height_of_normalized_rect`.
- **Unit (AC 4)**: test `bbox_returns_min_max_tuple`.
- **Unit (AC 5)**: test `contains_point_inside_boundary_outside` covering one PASS and one FAIL.
- **Unit (AC 6)**: test `contains_line_both_endpoints_inside_vs_one_outside` covering one PASS and one FAIL.
- **Unit (AC 7)**: test `crosses_line_through_vs_outside_vs_fully_inside` covering the through-PASS, outside-FAIL, and fully-inside-PASS cases.
- **Unit (AC 8)**: test `contains_circle_inscribed_vs_overflows`.
- **Unit (AC 9)**: test `crosses_circle_overlap_vs_clear`.
- **Unit (AC 10)**: test `contains_arc_quarter_inside_vs_overflows`.
- **Unit (AC 11)**: test `crosses_arc_endpoint_inside_vs_entirely_outside`.
- **Static check (AC 13)**: `grep -nE '^use (egui|eframe|rfd)' src/geometry/rect.rs` returns no matches; `wc -l src/geometry/rect.rs` reports `<= 300`.
- **Build gate (AC 14)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- The `crosses_arc` implementation is the trickiest predicate. The hint in the Scope section ("enumerate the four edges, intersect each as a `Line` against the parent `Circle`, filter resulting points by `Arc::contains_angle`") is the recommended path because it reuses LCV-014 and LCV-013 instead of writing fresh arc-vs-line code. Estimated LOC for the predicate: ~40 lines including doc comment.
- The "300 LOC budget" math for this file: struct + constructor + 9 methods ≈ 9 × 20 LOC = 180 LOC + doc comments. Estimated total ~250 LOC. If the implementer hits 300 LOC, the recommended split is moving `crosses_arc` and `crosses_circle` into a sibling file `rect_predicates.rs` re-exported by `rect.rs` — flag this in the demand's `Implementation:` line if it happens.
- `contains_point` uses inclusive bounds without `EPSILON` slack. Rationale: the SelectTool will inflate the user's drag rectangle by a pixel-radius in screen space *before* converting to world coordinates; the kernel does not bake in a tolerance.
- The `Rect` type from this demand is **kernel** Rect (world-space, mm). The renderer's `Viewport` may carry a separate pixel-space rectangle type; do not conflate them.
- Reference: v1's `geometry/rect.ts` had `contains{Point,Line,Circle,Arc}` and `crosses{Line,Circle,Arc}`. Same names, same semantics.
