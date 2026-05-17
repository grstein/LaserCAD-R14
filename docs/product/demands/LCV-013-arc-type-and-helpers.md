# LCV-013 — Arc value type with bbox, endpoints, and contains_angle

- **Status**: Done
- **Phase**: 1
- **Depends on**: LCV-010
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: implementer-rust (task #36); caad4e4

## Problem

Arcs are essential for laser-cut artwork: rounded corners, fillets, curved cut paths, dial markings. AutoCAD's `ARC` entity is canonically CCW with explicit start/end angles, and that orientation matters for SVG export (the `sweep-flag` of the `<path d="A ...">` command depends on it) and for trim/extend semantics. Later demands need a single shared `Arc` type that explicitly carries the orientation, the parametric endpoints, an axis-aware bounding box (the bbox of a quarter-arc is **not** the parent circle's bbox), and a `contains_angle` predicate that handles the wrap-around case (e.g., sweeping from 350° to 10° CCW crosses 0).

## Scope

- New file `src/geometry/arc.rs` defining:
  - `pub struct Arc { pub center: Vec2, pub r: f64, pub start_angle: f64, pub end_angle: f64, pub ccw: bool }`. All angles in radians, CCW from +X (AutoCAD convention).
  - Derives: `Copy`, `Clone`, `Debug`, `PartialEq`. No `Eq`, no `Hash`.
  - Constructor `pub const fn new(center: Vec2, r: f64, start_angle: f64, end_angle: f64, ccw: bool) -> Self`. No normalization (angles stored as supplied); the methods below handle wrap-around internally.
  - Methods:
    - `start_point(&self) -> Vec2` — `center + (r*cos(start_angle), r*sin(start_angle))`.
    - `end_point(&self) -> Vec2` — same formula at `end_angle`.
    - `sweep_angle(&self) -> f64` — the magnitude of the angular sweep in radians, in `[0.0, 2.0 * PI]`. Computed as the angular distance from `start_angle` to `end_angle` in the direction implied by `ccw`. A start equal to end (within `EPSILON`) returns `0.0`; a full circle (start/end identical and the implementation chooses) returns `2.0 * PI` when the arc explicitly represents a closed loop — implementer's call which sentinel to choose, but pick one and document it. Default rule: equal angles → `0.0` (zero-length arc). To draw a full circle, callers use the `Circle` type.
    - `arc_length(&self) -> f64` — `r * sweep_angle()`.
    - `contains_angle(&self, angle: f64) -> bool` — `true` iff the input angle (modulo `2π`) lies within the sweep from `start_angle` to `end_angle` in the direction implied by `ccw`. Both endpoints are inclusive (within `EPSILON` of the boundary angle). Wrap-around (e.g., start `350°`, end `10°`, `ccw=true`) is handled correctly.
    - `bbox(&self) -> (Vec2, Vec2)` — axis-aligned bounding box of the arc. The implementation must consider the four cardinal extreme angles (`0`, `π/2`, `π`, `3π/2`) and include each one in the bbox iff `contains_angle(extreme)` is true; otherwise the bbox is the min/max of `start_point` and `end_point`. A full-circle arc (sweep `>= 2π - EPSILON`) returns the parent circle's bbox.
- Update `src/geometry/mod.rs`:
  - Add `pub mod arc;` and `pub use arc::Arc;`.
- Per-module unit tests under `#[cfg(test)] mod tests` in `arc.rs`. No new file under `tests/`.

## Out of scope

- Arc-arc intersection — deferred (no current consumer; not in Phase 1).
- Line-arc and circle-arc intersection — deferred (no current consumer in Phase 1; LCV-014 only covers line-line / line-circle / circle-circle).
- Snap to arc endpoints / midpoint / center — owned by **LCV-016**.
- Rect-arc predicates — owned by **LCV-015**.
- Rendering and SVG export.
- The `Entity::Arc` variant in the document model — owned by LCV-020.
- A representation for closed (full-circle) arcs — use `Circle` instead. `Arc` is strictly for proper arcs (sweep `< 2π`); the full-circle case is handled by bbox only for robustness.

## Acceptance criteria

1. `src/geometry/arc.rs` exists and defines `pub struct Arc { pub center: Vec2, pub r: f64, pub start_angle: f64, pub end_angle: f64, pub ccw: bool }` with `#[derive(Copy, Clone, Debug, PartialEq)]` (no `Eq`, no `Hash`).
2. For a quarter arc `center=Vec2::default(), r=1.0, start=0.0, end=PI/2, ccw=true`: `start_point()` equals `Vec2::new(1.0, 0.0)` (within `EPSILON`); `end_point()` equals `Vec2::new(0.0, 1.0)` (within `EPSILON`).
3. For the same arc: `sweep_angle()` equals `PI/2` (within `EPSILON`); `arc_length()` equals `PI/2` (within `EPSILON`, since `r=1`).
4. For the same arc: `contains_angle(PI/4)` returns `true`; `contains_angle(3.0 * PI / 4.0)` returns `false`; `contains_angle(0.0)` returns `true` (start boundary, inclusive); `contains_angle(PI/2)` returns `true` (end boundary, inclusive).
5. For the same arc: `bbox()` equals `(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0))` (within `EPSILON`) — the bbox is **not** the parent circle's bbox; only `start_point`, `end_point`, and any cardinal extreme angles within the sweep contribute.
6. Wrap-around CCW: arc `center=Vec2::default(), r=1.0, start_angle=350° (in radians), end_angle=10° (in radians), ccw=true`: `sweep_angle()` equals `20°` in radians (within `EPSILON`); `contains_angle(0.0)` returns `true`; `contains_angle(PI)` returns `false`; `contains_angle(5° in radians)` returns `true`.
7. Reverse direction (CW): arc `center=Vec2::default(), r=1.0, start_angle=PI/2, end_angle=0.0, ccw=false` (a quarter arc swept clockwise from `(0,1)` to `(1,0)`): `sweep_angle()` equals `PI/2`; `contains_angle(PI/4)` returns `true`; `contains_angle(3.0 * PI / 4.0)` returns `false`.
8. A near-full-circle arc (sweep `>= 2π - EPSILON`, e.g., `start_angle=0.0, end_angle=2*PI - EPSILON/2, ccw=true`) returns `bbox()` equal to the parent circle's bbox `(Vec2::new(-1.0, -1.0), Vec2::new(1.0, 1.0))` (within `EPSILON`).
9. A bbox that includes a cardinal extreme: arc `center=Vec2::default(), r=1.0, start_angle=-PI/4, end_angle=PI/4, ccw=true` (sweep crosses angle `0`, which is the `+X` extreme): `bbox()` `max.x` equals `1.0` (within `EPSILON`); `min.x` equals `cos(PI/4)` (within `EPSILON`); `min.y` equals `-sin(PI/4)` and `max.y` equals `sin(PI/4)` (within `EPSILON`).
10. Zero-length sweep: arc with `start_angle == end_angle` (within `EPSILON`): `sweep_angle()` returns `0.0`; `arc_length()` returns `0.0`; `contains_angle(start_angle)` returns `true` (single-point boundary).
11. `src/geometry/mod.rs` re-exports `Arc` so `use lasercad::geometry::Arc;` works from outside the module.
12. `src/geometry/arc.rs` imports nothing from `egui`, `eframe`, or `rfd`; the file stays under 300 LOC.
13. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: a test that constructs `Arc { center: Vec2::default(), r: 1.0, start_angle: 0.0, end_angle: 1.0, ccw: true }` via the struct literal.
- **Unit (AC 2)**: test `quarter_arc_endpoints`.
- **Unit (AC 3)**: test `quarter_arc_sweep_and_length`.
- **Unit (AC 4)**: test `quarter_arc_contains_angle_inside_outside_and_boundaries` covering inside, outside, start boundary, end boundary.
- **Unit (AC 5)**: test `quarter_arc_bbox_is_corner_not_parent_circle`.
- **Unit (AC 6)**: test `ccw_wrap_around_zero` covering sweep, contains-true at 0, contains-false at π, contains-true near end.
- **Unit (AC 7)**: test `cw_quarter_arc` covering sweep and contains-angle inside/outside.
- **Unit (AC 8)**: test `near_full_circle_bbox_matches_parent_circle`.
- **Unit (AC 9)**: test `bbox_includes_cardinal_extreme_inside_sweep`.
- **Unit (AC 10)**: test `zero_length_arc_sweep_zero_and_boundary_contains`.
- **Static check (AC 12)**: `grep -nE '^use (egui|eframe|rfd)' src/geometry/arc.rs` returns no matches; `wc -l src/geometry/arc.rs` reports `<= 300`.
- **Build gate (AC 13)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- Angle convention: radians, CCW from +X. Negative or `> 2π` angles are accepted by the constructor and normalized inside the methods (modulo `2π` arithmetic). This matches AutoCAD's `ARC` entity (`50` / `51` DXF group codes are start/end angle in degrees; v2 stores radians).
- Wrap-around correctness is the trickiest acceptance criterion. The recommended implementation: normalize all angles to `[0, 2π)`, then `sweep_angle` for CCW is `(end - start).rem_euclid(2π)`, for CW is `(start - end).rem_euclid(2π)`. `contains_angle(a)` normalizes `a`, computes `delta = (a - start).rem_euclid(2π)` for CCW (or `(start - a).rem_euclid(2π)` for CW), and returns `delta <= sweep_angle + EPSILON`.
- `bbox` cardinal-extreme test: for each of `0, π/2, π, 3π/2`, ask `contains_angle(theta)`; if true, include `center + (r*cos(theta), r*sin(theta))` in the bbox; always include `start_point` and `end_point`.
- Full-circle sentinel: the demand chooses "equal start/end → zero-sweep, not full circle" so the type is unambiguous. Callers that want a full circle use the `Circle` type from LCV-012. The full-circle bbox criterion (AC 8) is a robustness guard for the boundary case where a caller deliberately sets `end_angle = start_angle + 2π - tiny` to trace a near-closed curve.
- The transitional Arc consumers in Phase 1 are LCV-015 (Rect predicates) and LCV-016 (snap engine). Both rely on `bbox`, `start_point`, `end_point`, and `contains_angle`.
- Reference: v1 stored arcs as `{ center, r, startAngle, endAngle, ccw }` in `geometry/arc.ts`. Field names align (snake_case in Rust).
