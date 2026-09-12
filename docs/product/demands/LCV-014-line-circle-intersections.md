# LCV-014 — Line-line, line-circle, and circle-circle intersection routines

- **Status**: Done
- **Phase**: 1
- **Depends on**: LCV-011, LCV-012
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: fdade9b — feat(LCV-014): add line/line, line/circle, circle/circle intersection routines

## Problem

Intersection computation is the foundation of the snap engine (LCV-016, snap-to-intersection), the future trim/extend tools (Phase 4), and any "construct geometry from existing geometry" workflow. Without canonical kernel routines, downstream demands would each carry their own implementation — and the floating-point classification of tangents, near-parallels, and near-coincidences would silently diverge. This demand consolidates the four common 2D intersection cases LaserCAD v2 needs.

User outcome: when an operator hovers near where two segments visibly cross, the snap engine offers an exact intersection point that other tools can pick up without re-computing.

## Scope

- New file `src/geometry/intersect.rs` exposing four free functions:
  - `pub fn line_line(a: &Line, b: &Line) -> Option<Vec2>` — intersection of two **segments**. Returns `Some(point)` iff the two segments are non-parallel and the unique infinite-line intersection lies within both segments (with `EPSILON` slack at the endpoints). Returns `None` when the lines are parallel (including coincident) or when the intersection point lies outside either segment.
  - `pub fn line_line_infinite(a: &Line, b: &Line) -> Option<Vec2>` — intersection of two **infinite lines** through the supplied segments' endpoints. Returns `Some(point)` for any non-parallel pair; returns `None` when parallel (including coincident — coincident infinite lines have infinitely many intersection points and the function returns `None` rather than picking one arbitrarily).
  - `pub fn line_circle(line: &Line, circle: &Circle) -> Vec<Vec2>` — intersections of a **segment** with a circle. Returns 0, 1, or 2 points. A tangent counts as 1 (within `EPSILON`). Intersection points that fall outside the segment are excluded; the segment-clipping uses `EPSILON` slack at the endpoints.
  - `pub fn circle_circle(a: &Circle, b: &Circle) -> Vec<Vec2>` — intersections of two circles. Returns 0, 1, or 2 points. Tangent (internal or external) returns 1 within `EPSILON`. Concentric circles (centers within `EPSILON` of each other) return an empty `Vec` regardless of whether radii match.
  - All four functions classify "near-parallel" / "near-tangent" / "near-coincident" using `EPSILON` from LCV-010.
- Update `src/geometry/mod.rs`:
  - Add `pub mod intersect;`.
  - Add `pub use intersect::{line_line, line_line_infinite, line_circle, circle_circle};`.
- Per-module unit tests under `#[cfg(test)] mod tests` in `intersect.rs`. No new file under `tests/`.

## Out of scope

- Arc intersections (line-arc, circle-arc, arc-arc) — deferred. No current consumer in Phase 1; LCV-016's snap engine treats arcs as their parent geometry (center for snap-to-center, endpoints for snap-to-endpoint) and does not require arc-arc intersection.
- Polyline / polygon / spline intersection.
- A general boolean / CSG framework. v2 is not a CSG tool.
- Ordering guarantees on the returned `Vec`. Tests assert the **set** of intersection points (sorted by the test if needed), not the order.
- Coincident-segment overlap reporting (returning the overlap region as a segment). Two coincident segments return `None` from `line_line`; this is documented in the function's doc comment.
- Performance benchmarks. The routines are called per pointer-hover, not in tight loops; correctness wins over speed.

## Acceptance criteria

1. `src/geometry/intersect.rs` exists with exactly the four `pub fn` signatures listed in Scope. Each has a `///` doc comment explaining the `None` / empty-vec cases.
2. `line_line`: clearly crossing segments — `a = Line(Vec2(0,0), Vec2(10,0))`, `b = Line(Vec2(5,-5), Vec2(5,5))` — returns `Some(Vec2(5,0))` within `EPSILON`.
3. `line_line`: parallel-non-coincident — `a = Line(Vec2(0,0), Vec2(10,0))`, `b = Line(Vec2(0,1), Vec2(10,1))` — returns `None`.
4. `line_line`: coincident — `a = Line(Vec2(0,0), Vec2(10,0))`, `b = Line(Vec2(2,0), Vec2(8,0))` — returns `None` (documented contract).
5. `line_line`: infinite-line intersection lies outside segment — `a = Line(Vec2(0,0), Vec2(1,0))`, `b = Line(Vec2(5,-5), Vec2(5,5))` — returns `None` (intersection at `(5,0)` is outside segment `a`).
6. `line_line_infinite`: same `a` and `b` as AC 5 — returns `Some(Vec2(5,0))` within `EPSILON` (infinite lines do intersect).
7. `line_line_infinite`: parallel pair from AC 3 — returns `None`.
8. `line_circle`: secant — `line = Line(Vec2(-10, 0), Vec2(10, 0))`, `circle = Circle(Vec2::default(), 5.0)` — returns a `Vec` of length 2 containing `Vec2(-5,0)` and `Vec2(5,0)` (within `EPSILON`, order-independent).
9. `line_circle`: tangent — `line = Line(Vec2(-10, 5), Vec2(10, 5))`, `circle = Circle(Vec2::default(), 5.0)` — returns a `Vec` of length 1 containing `Vec2(0, 5)` within `EPSILON`.
10. `line_circle`: no intersection — `line = Line(Vec2(-10, 6), Vec2(10, 6))`, `circle = Circle(Vec2::default(), 5.0)` — returns an empty `Vec`.
11. `line_circle`: infinite-line secant but both intersections outside the segment — `line = Line(Vec2(6, -10), Vec2(6, -1))`, `circle = Circle(Vec2(0,0), 10)` (infinite-line intersects circle, but the segment lies entirely outside) — returns an empty `Vec`.
12. `circle_circle`: two intersections — `a = Circle(Vec2::default(), 5.0)`, `b = Circle(Vec2(8, 0), 5.0)` — returns a `Vec` of length 2 containing `Vec2(4.0, 3.0)` and `Vec2(4.0, -3.0)` (within `EPSILON`, order-independent).
13. `circle_circle`: external tangent — `a = Circle(Vec2::default(), 5.0)`, `b = Circle(Vec2(10, 0), 5.0)` — returns a `Vec` of length 1 containing `Vec2(5, 0)` within `EPSILON`.
14. `circle_circle`: internal tangent — `a = Circle(Vec2::default(), 10.0)`, `b = Circle(Vec2(5, 0), 5.0)` — returns a `Vec` of length 1 containing `Vec2(10, 0)` within `EPSILON`.
15. `circle_circle`: separated — `a = Circle(Vec2::default(), 1.0)`, `b = Circle(Vec2(10, 0), 1.0)` — returns an empty `Vec`.
16. `circle_circle`: nested (one inside the other, non-tangent) — `a = Circle(Vec2::default(), 10.0)`, `b = Circle(Vec2(2, 0), 1.0)` — returns an empty `Vec`.
17. `circle_circle`: concentric — `a = Circle(Vec2::default(), 5.0)`, `b = Circle(Vec2::default(), 5.0)` and also `a = Circle(Vec2::default(), 5.0)`, `b = Circle(Vec2::default(), 3.0)` — both return an empty `Vec`.
18. `src/geometry/mod.rs` re-exports all four functions so `use lasercad::geometry::{line_line, line_line_infinite, line_circle, circle_circle};` works from outside the module.
19. `src/geometry/intersect.rs` imports nothing from `egui`, `eframe`, or `rfd`; the file stays under 300 LOC.
20. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: a test that calls each of the four functions with trivial arguments — proves the public API is exactly as declared and reachable through `crate::geometry::intersect::*`.
- **Unit (AC 2)**: test `line_line_crossing_returns_intersection`.
- **Unit (AC 3)**: test `line_line_parallel_returns_none`.
- **Unit (AC 4)**: test `line_line_coincident_returns_none`.
- **Unit (AC 5)**: test `line_line_outside_segment_returns_none`.
- **Unit (AC 6)**: test `line_line_infinite_returns_intersection_outside_segment`.
- **Unit (AC 7)**: test `line_line_infinite_parallel_returns_none`.
- **Unit (AC 8)**: test `line_circle_secant_returns_two_points` (use a small helper to compare a `Vec<Vec2>` against an expected set, ignoring order, with `EPSILON` tolerance).
- **Unit (AC 9)**: test `line_circle_tangent_returns_one_point`.
- **Unit (AC 10)**: test `line_circle_no_intersection_returns_empty`.
- **Unit (AC 11)**: test `line_circle_intersections_outside_segment_returns_empty`.
- **Unit (AC 12)**: test `circle_circle_two_intersections`.
- **Unit (AC 13)**: test `circle_circle_external_tangent`.
- **Unit (AC 14)**: test `circle_circle_internal_tangent`.
- **Unit (AC 15)**: test `circle_circle_separated_returns_empty`.
- **Unit (AC 16)**: test `circle_circle_nested_returns_empty`.
- **Unit (AC 17)**: test `circle_circle_concentric_returns_empty` covering equal-radii and unequal-radii concentric cases.
- **Static check (AC 19)**: `grep -nE '^use (egui|eframe|rfd)' src/geometry/intersect.rs` returns no matches; `wc -l src/geometry/intersect.rs` reports `<= 300`.
- **Build gate (AC 20)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- `line_line` segment-clipping uses `EPSILON` slack so a point that lies exactly at a segment endpoint (within rounding) still counts as inside. This matches the snap engine's expectation in LCV-016.
- The `line_line` vs `line_line_infinite` split exists because the two principal consumers differ: snap-to-intersection (LCV-016) operates on segments and wants the strict version; future trim/extend (Phase 4) operates on extended construction lines and wants the infinite version.
- For `circle_circle`, the standard radical-axis formula is recommended. Edge cases to handle explicitly: `d == 0` (concentric — return empty), `d > r_a + r_b + EPSILON` (separated — return empty), `d < |r_a - r_b| - EPSILON` (nested — return empty), `|d - (r_a + r_b)| <= EPSILON` (external tangent — one point), `|d - |r_a - r_b|| <= EPSILON` (internal tangent — one point), otherwise two points.
- All four functions return owned values (`Option<Vec2>` / `Vec<Vec2>`); no allocator concern since vecs are at most length 2.
- The contract uses **segment** semantics for `line_line` and `line_circle`. Renaming to `segment_segment` and `segment_circle` was considered and rejected — the rest of the kernel uses `Line` to mean a segment with two explicit endpoints, and the `_infinite` suffix on `line_line_infinite` flags the exception clearly.
- Reference: v1 had equivalent routines in `geometry/intersect.ts` (`lineLine`, `lineLineInfinite`, `lineCircle`, `circleCircle`). Behavior is identical; v2 just uses Rust types.
