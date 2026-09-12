# LCV-011 — Line value type with bbox and geometric helpers

- **Status**: Done
- **Phase**: 1
- **Depends on**: LCV-010
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: c6f09f9 — feat(LCV-011): add Line geometry primitive with bbox and closest-point helpers

## Problem

Lines are the most common primitive in laser-cut artwork (cut paths, fold marks, registration crosshairs). Later demands cover line drawing (Phase 4), line-line / line-circle intersection (LCV-014), Rect predicates (LCV-015), the snap engine (LCV-016), and SVG export. All of them need the same `Line` value type and the same handful of helpers — endpoint queries, midpoint, parametric point, bounding box, closest-point/distance — to avoid reinventing these calculations per demand. Without a shared `Line`, downstream demands either duplicate the math (with the inevitable subtle disagreements) or carry around bare `(Vec2, Vec2)` tuples that fail to document intent.

## Scope

- New file `src/geometry/line.rs` defining:
  - `pub struct Line { pub p1: Vec2, pub p2: Vec2 }`.
  - Derives: `Copy`, `Clone`, `Debug`, `PartialEq`. No `Eq`, no `Hash` (transitively f64).
  - Constructor `pub const fn new(p1: Vec2, p2: Vec2) -> Self`.
  - Methods:
    - `length(&self) -> f64` — Euclidean length of the segment.
    - `length_squared(&self) -> f64`.
    - `direction(&self) -> Option<Vec2>` — unit vector from `p1` to `p2`; `None` when the segment is degenerate (length `<= EPSILON`).
    - `midpoint(&self) -> Vec2`.
    - `bbox(&self) -> (Vec2, Vec2)` — `(min, max)` axis-aligned bounding box; returns the corners regardless of `p1` / `p2` ordering.
    - `point_at(&self, t: f64) -> Vec2` — parametric: `t=0.0` gives `p1`, `t=1.0` gives `p2`. `t` is **not clamped** — values outside `[0, 1]` return the extrapolated infinite-line point. This is intentional; callers that need clamping wrap the call.
    - `closest_point(&self, p: Vec2) -> Vec2` — for a degenerate line (length `<= EPSILON`) returns `p1`; otherwise returns the foot of the perpendicular from `p` clamped to the segment `[p1, p2]`.
    - `distance_to_point(&self, p: Vec2) -> f64` — `(p - self.closest_point(p)).length()`.
- Update `src/geometry/mod.rs`:
  - Add `pub mod line;` and `pub use line::Line;`.
- Per-module unit tests under `#[cfg(test)] mod tests` in `line.rs`. No new file under `tests/`.

## Out of scope

- Line-line and line-circle intersection — owned by **LCV-014**.
- Rect-line predicates (contains / crosses) — owned by **LCV-015**.
- Snap to endpoints / midpoint — owned by **LCV-016**.
- Rendering — Phase 3.
- The `Entity::Line` variant in the document model — owned by **LCV-020** (Phase 2). The relationship between `Entity::Line` and the kernel's `geometry::Line` is decided by LCV-020; LCV-011 only ships the kernel value type.
- An infinite-line type. The implementer may **not** add a separate `InfiniteLine` struct — `point_at` with unclamped `t` and `closest_point` covering the segment-clamped case are sufficient until a future demand proves otherwise.
- Polylines, polygons, splines.

## Acceptance criteria

1. `src/geometry/line.rs` exists and defines `pub struct Line { pub p1: Vec2, pub p2: Vec2 }` with `#[derive(Copy, Clone, Debug, PartialEq)]` (no `Eq`, no `Hash`).
2. `Line::new(Vec2::new(0.0, 0.0), Vec2::new(3.0, 4.0)).length()` returns exactly `5.0`; `.length_squared()` returns exactly `25.0`.
3. `Line::new(Vec2::default(), Vec2::default()).direction()` returns `None`; `Line::new(Vec2::new(0.0, 0.0), Vec2::new(2.0, 0.0)).direction().unwrap().approx_eq(Vec2::new(1.0, 0.0), EPSILON)` is `true`.
4. `Line::new(Vec2::new(2.0, 4.0), Vec2::new(6.0, 8.0)).midpoint()` equals `Vec2::new(4.0, 6.0)` (within `EPSILON`).
5. `bbox` is invariant under endpoint ordering: `Line::new(Vec2::new(5.0, 1.0), Vec2::new(2.0, 7.0)).bbox()` equals `Line::new(Vec2::new(2.0, 7.0), Vec2::new(5.0, 1.0)).bbox()` and equals `(Vec2::new(2.0, 1.0), Vec2::new(5.0, 7.0))`.
6. `point_at(0.0)` returns `p1`; `point_at(1.0)` returns `p2`; `point_at(0.5)` equals `midpoint()`; `point_at(2.0)` for the segment from `(0,0)` to `(1,0)` returns `Vec2::new(2.0, 0.0)` (extrapolation beyond `p2`, no clamping).
7. `closest_point` for a point off the line equals the foot of the perpendicular: for the segment from `(0,0)` to `(10,0)`, `closest_point(Vec2::new(3.0, 5.0))` equals `Vec2::new(3.0, 0.0)` (within `EPSILON`).
8. `closest_point` clamps to the segment endpoints: for the same segment, `closest_point(Vec2::new(-2.0, 1.0))` equals `Vec2::new(0.0, 0.0)`; `closest_point(Vec2::new(15.0, -1.0))` equals `Vec2::new(10.0, 0.0)`.
9. `closest_point` on a degenerate segment (`p1 == p2`) returns `p1` regardless of the query point.
10. `distance_to_point` returns `0.0` (within `EPSILON`) when the query point lies on the segment; for the segment `(0,0)`→`(10,0)` and query `(3.0, 5.0)`, returns exactly `5.0`.
11. `src/geometry/mod.rs` re-exports `Line` so `use lasercad::geometry::Line;` works from outside the module.
12. `src/geometry/line.rs` imports nothing from `egui`, `eframe`, or `rfd`; the file stays under 300 LOC.
13. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: a test that constructs `Line { p1: Vec2::default(), p2: Vec2::new(1.0, 0.0) }` via the struct literal (proves fields are `pub`).
- **Unit (AC 2)**: test `length_and_length_squared_345`.
- **Unit (AC 3)**: test `direction_none_for_degenerate_and_unit_for_axis_aligned` covering both branches.
- **Unit (AC 4)**: test `midpoint_of_known_segment`.
- **Unit (AC 5)**: test `bbox_is_order_invariant` comparing both orderings.
- **Unit (AC 6)**: test `point_at_endpoints_midpoint_and_extrapolated` covering `t = 0.0, 0.5, 1.0, 2.0`.
- **Unit (AC 7)**: test `closest_point_foot_of_perpendicular`.
- **Unit (AC 8)**: test `closest_point_clamps_to_endpoints` covering both ends.
- **Unit (AC 9)**: test `closest_point_on_degenerate_segment_returns_p1`.
- **Unit (AC 10)**: test `distance_to_point_zero_on_segment_and_perpendicular_distance` covering both branches.
- **Static check (AC 12)**: `grep -nE '^use (egui|eframe|rfd)' src/geometry/line.rs` returns no matches; `wc -l src/geometry/line.rs` reports `<= 300`.
- **Build gate (AC 13)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- `direction` returns `Option<Vec2>` rather than panicking on a degenerate segment, matching the `Vec2::normalize` convention from LCV-010. Consistent error handling reduces cognitive load for callers.
- `point_at` is intentionally unclamped. The two principal consumers (line-line intersect in LCV-014, snap-to-midpoint in LCV-016) need both behaviors; the segment-clamping consumers already have `closest_point`.
- AutoCAD's `LINE` entity has two endpoints in CCS; v2 stores `Vec2` directly because layers do not get separate coordinate systems in v2 (one global mm-space).
- The kernel `Line` is a value type. Persistence and history identification of a particular line in the document is the `Entity` / `Document` layer's job (LCV-020+).
- Reference: v1 stored lines as `{ p1, p2 }` literal pairs in `geometry/line.ts`; the v2 type keeps the same field names so cross-reference is easy.
