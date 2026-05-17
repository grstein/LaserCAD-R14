# LCV-012 — Circle value type with bbox and geometric helpers

- **Status**: Done
- **Phase**: 1
- **Depends on**: LCV-010
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: implementer-rust (task #40); 4ff9ee4

## Problem

Circles are a core CAD primitive — registration holes, drill marks, rounded mount points, and the basis for the Arc primitive (LCV-013). Later demands cover circle drawing (Phase 4), line-circle / circle-circle intersection (LCV-014), Rect predicates (LCV-015), and snap-to-center (LCV-016). Without a shared `Circle` type and its helpers (bbox, point-at-angle, containment), each demand would reinvent the math and risk subtle divergence (e.g., snap deciding a point is inside while Rect predicates deem it outside).

## Scope

- New file `src/geometry/circle.rs` defining:
  - `pub struct Circle { pub center: Vec2, pub r: f64 }`.
  - Derives: `Copy`, `Clone`, `Debug`, `PartialEq`. No `Eq`, no `Hash`.
  - Constructor `pub const fn new(center: Vec2, r: f64) -> Self`. The constructor is **panic-free** — `r` is not validated. A `///` doc comment notes that a meaningful circle requires `r > 0.0`; callers that need validation perform it themselves (the constructor stays `const` and trivial so it can be used in tests and `static` contexts).
  - Methods:
    - `circumference(&self) -> f64` — `2.0 * PI * r`.
    - `area(&self) -> f64` — `PI * r * r`.
    - `bbox(&self) -> (Vec2, Vec2)` — `(center - (r,r), center + (r,r))`.
    - `point_at_angle(&self, angle_rad: f64) -> Vec2` — `center + (r*cos(angle), r*sin(angle))`. Angle is in radians, measured CCW from the positive X axis (AutoCAD convention).
    - `contains_point(&self, p: Vec2, eps: f64) -> bool` — `true` iff `(p - center).length() <= r + eps`. The boundary counts as "inside" within `eps`; callers that want strict interior pass a negative `eps` or test `distance_to_point` directly.
    - `distance_to_point(&self, p: Vec2) -> f64` — **signed** distance: `(p - center).length() - r`. Positive when `p` is outside the circle, `0.0` on the circle, negative when inside.
- Update `src/geometry/mod.rs`:
  - Add `pub mod circle;` and `pub use circle::Circle;`.
- Per-module unit tests under `#[cfg(test)] mod tests` in `circle.rs`. No new file under `tests/`.

## Out of scope

- Line-circle and circle-circle intersection — owned by **LCV-014**.
- Rect-circle predicates (contains / crosses) — owned by **LCV-015**.
- Snap to center / quadrant — owned by **LCV-016**.
- Arc type (a sweep of a circle) — owned by **LCV-013**.
- Ellipses, conic sections. v2 has no ellipse primitive.
- Validation of `r` in the constructor (would require a `Result` and is not justified by any current consumer; document instead).
- Rendering and tool integration.
- The `Entity::Circle` variant in the document model — owned by LCV-020.

## Acceptance criteria

1. `src/geometry/circle.rs` exists and defines `pub struct Circle { pub center: Vec2, pub r: f64 }` with `#[derive(Copy, Clone, Debug, PartialEq)]` (no `Eq`, no `Hash`).
2. `Circle::new(Vec2::default(), 5.0).bbox()` equals `(Vec2::new(-5.0, -5.0), Vec2::new(5.0, 5.0))` exactly.
3. `Circle::new(Vec2::new(10.0, 20.0), 3.0).bbox()` equals `(Vec2::new(7.0, 17.0), Vec2::new(13.0, 23.0))` exactly.
4. `Circle::new(Vec2::default(), 5.0).point_at_angle(0.0)` equals `Vec2::new(5.0, 0.0)` (within `EPSILON`).
5. `Circle::new(Vec2::default(), 5.0).point_at_angle(std::f64::consts::FRAC_PI_2)` equals `Vec2::new(0.0, 5.0)` (within `EPSILON`).
6. `Circle::new(Vec2::default(), 5.0).point_at_angle(std::f64::consts::PI)` equals `Vec2::new(-5.0, 0.0)` (within `EPSILON`).
7. `Circle::new(Vec2::default(), 5.0).contains_point(Vec2::default(), EPSILON)` returns `true` (center is contained).
8. `Circle::new(Vec2::default(), 5.0).contains_point(Vec2::new(5.0, 0.0), EPSILON)` returns `true` (on-boundary, within `eps`).
9. `Circle::new(Vec2::default(), 5.0).contains_point(Vec2::new(5.0 + 1e-6, 0.0), EPSILON)` returns `false` (outside by more than `eps`).
10. `Circle::new(Vec2::default(), 5.0).distance_to_point(Vec2::new(6.0, 0.0))` returns exactly `1.0` (outside).
11. `Circle::new(Vec2::default(), 5.0).distance_to_point(Vec2::new(4.0, 0.0))` returns exactly `-1.0` (inside; sign negative).
12. `Circle::new(Vec2::default(), 5.0).distance_to_point(Vec2::new(5.0, 0.0))` returns `0.0` within `EPSILON` (on boundary).
13. `Circle::new(Vec2::default(), 2.0).circumference()` equals `4.0 * std::f64::consts::PI` (within `EPSILON`); `.area()` equals `4.0 * std::f64::consts::PI` (within `EPSILON`).
14. `src/geometry/mod.rs` re-exports `Circle` so `use lasercad::geometry::Circle;` works from outside the module.
15. `src/geometry/circle.rs` imports nothing from `egui`, `eframe`, or `rfd`; the file stays under 300 LOC.
16. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: a test that constructs `Circle { center: Vec2::default(), r: 1.0 }` via the struct literal (proves fields are `pub`).
- **Unit (AC 2, 3)**: test `bbox_origin_and_offset_circle` covering both cases.
- **Unit (AC 4, 5, 6)**: test `point_at_angle_cardinals` covering 0, π/2, π (and as a bonus, 3π/2 to verify `(0, -5)` for thoroughness).
- **Unit (AC 7)**: test `contains_point_center`.
- **Unit (AC 8, 9)**: test `contains_point_boundary_and_outside`.
- **Unit (AC 10, 11, 12)**: test `signed_distance_inside_outside_and_boundary` covering all three sign cases.
- **Unit (AC 13)**: test `circumference_and_area_of_radius_2`.
- **Static check (AC 15)**: `grep -nE '^use (egui|eframe|rfd)' src/geometry/circle.rs` returns no matches; `wc -l src/geometry/circle.rs` reports `<= 300`.
- **Build gate (AC 16)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- Angles are radians, CCW from +X, matching the kernel contract in [`AGENTS.md`](../../AGENTS.md) § Units and types. Degree↔radian conversion is a UI-layer concern (Phase 6).
- `distance_to_point` is **signed**. This matches the convention used by GIS / CSG libraries (negative = interior) and is the contract LCV-014 (intersection-of-tangent test) and LCV-015 (Rect-crosses-Circle) will depend on. Callers wanting an unsigned distance call `.abs()`.
- The `contains_point(p, eps)` API mirrors the `Vec2::approx_eq(p, eps)` pattern from LCV-010 — explicit tolerance per call, no hidden global. Callers typically pass `EPSILON`.
- The constructor is intentionally `const fn` and non-validating so it can be used in `const` contexts (e.g., test fixtures). The doc comment is the single source of truth for the "r > 0" guidance.
- Reference: v1 stored circles as `{ center: Vec2, r: number }` in `geometry/circle.ts`. Field names align.
