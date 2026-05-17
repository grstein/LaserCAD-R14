# LCV-010 — Vec2 type and kernel EPSILON constant

- **Status**: Ready
- **Phase**: 1
- **Depends on**: LCV-007
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: <to be filled by demand-manager>

## Problem

Every later kernel demand (Line, Circle, Arc, intersections, snap, Rect) needs to talk about 2D points in millimeters and needs a shared tolerance to decide when two floating-point quantities count as "the same". Without a single canonical `Vec2` and a single canonical `EPSILON`, each follow-up demand would invent its own type and its own tolerance, producing silent disagreements at module boundaries (e.g., snap deciding a point is "on" a line that intersect deems "off"). This demand fixes the bottom of the geometry kernel so every later demand has one type to import.

The user pain this prevents: a CAD operator drawing two segments that "look" coincident but later fail to snap or trim because two kernel modules disagree on what "equal" means at 1e-6 mm.

## Scope

- New file `src/geometry/vec2.rs` defining:
  - `pub struct Vec2 { pub x: f64, pub y: f64 }`.
  - Derives: `Copy`, `Clone`, `Debug`, `Default`, `PartialEq`. No `Eq`, no `Hash` (f64 forbids both).
  - `Default` returns the origin `Vec2 { x: 0.0, y: 0.0 }`.
  - Constructor `pub const fn new(x: f64, y: f64) -> Self`.
  - Operator impls: `Add`, `Sub`, scalar `Mul<f64>` (both `Vec2 * f64` and `f64 * Vec2`), scalar `Div<f64>`, unary `Neg`. Output of each is `Vec2`.
  - Methods: `length(&self) -> f64`, `length_squared(&self) -> f64`, `distance(&self, other: Vec2) -> f64`, `distance_squared(&self, other: Vec2) -> f64`, `dot(&self, other: Vec2) -> f64`, `cross(&self, other: Vec2) -> f64` (2D scalar cross: `self.x * other.y - self.y * other.x`), `normalize(&self) -> Option<Vec2>` (returns `None` when `length() <= EPSILON`), `lerp(&self, other: Vec2, t: f64) -> Vec2` (no clamping of `t`), `approx_eq(&self, other: Vec2, eps: f64) -> bool` (true iff both coordinate differences are `<= eps` in absolute value).
- New file `src/geometry/epsilon.rs` defining:
  - `pub const EPSILON: f64 = 1e-9;` with a `///` doc comment explaining: this is the canonical kernel epsilon for length and area comparisons, expressed in the kernel's units (millimeters / mm²). 1e-9 mm sits well above f64's mantissa precision (~2e-16 relative) on bed-sized coordinates (~1 m = 1000 mm), giving ~10 orders of magnitude of headroom for accumulated rounding. UI-side tolerances (pixel snap radius) are NOT this epsilon — they live in `render/` and are expressed in screen pixels.
- Update `src/geometry/mod.rs`:
  - Add `pub mod vec2;` and `pub mod epsilon;`.
  - Add `pub use vec2::Vec2;` and `pub use epsilon::EPSILON`.
  - **Remove** the placeholder `pub const MODULE: &str = "geometry";` line that LCV-001 added. Its purpose was to give `tests/skeleton.rs` something to reference until the real surface arrived; LCV-010 supplies the real surface.
- Update `tests/skeleton.rs`:
  - Replace the `&geometry::MODULE` reference with `&geometry::EPSILON` (or any other always-present real public item such as a constructed `geometry::Vec2`). The test must still compile and pass; its purpose (verify the module surface is importable) is unchanged.
  - Do not touch the references to the other 9 modules. They keep their `MODULE` placeholders until their own Phase-1+ demands replace them.
- Per-module unit tests live in `#[cfg(test)] mod tests` inside `vec2.rs`. No new integration test file in `tests/` for this demand.

## Out of scope

- Any tool, render, document, or command integration. LCV-010 is the type definition only.
- Vec3 / 3D math. LaserCAD v2 is strictly 2D.
- A separate epsilon for angles (radians). Angle tolerances are handled per-call by passing an explicit `eps` argument; the canonical `EPSILON` is for lengths and areas in mm-space.
- `serde` derives. Persistence formats (LCV-058+) decide their own serialization; deriving `Serialize`/`Deserialize` here would force a `serde` dep that LCV-001 has not unlocked.
- `Eq` / `Hash` impls. `f64` cannot satisfy them; deferring forces consumers to be explicit about epsilon comparisons.
- Removing the `MODULE` placeholder from the other 9 top-level modules. Each module removes its own placeholder when its first real-surface demand lands.

## Acceptance criteria

1. `src/geometry/vec2.rs` exists and defines `pub struct Vec2 { pub x: f64, pub y: f64 }` with `#[derive(Copy, Clone, Debug, Default, PartialEq)]` and no `Eq` / `Hash` derive.
2. `Vec2::new(3.0, 4.0).length()` returns exactly `5.0` (the (3,4,5) triple is exactly representable in f64).
3. `Vec2::new(3.0, 4.0).length_squared()` returns exactly `25.0`.
4. `Vec2::new(0.0, 0.0).normalize()` returns `None`. `Vec2::new(EPSILON / 2.0, 0.0).normalize()` returns `None` (length below `EPSILON`).
5. `Vec2::new(2.0, 0.0).normalize().unwrap().approx_eq(Vec2::new(1.0, 0.0), EPSILON)` is `true`.
6. `Vec2::new(1.0, 0.0).cross(Vec2::new(0.0, 1.0))` returns exactly `1.0`; `Vec2::new(0.0, 1.0).cross(Vec2::new(1.0, 0.0))` returns exactly `-1.0`.
7. `Vec2::new(1.0, 2.0).dot(Vec2::new(3.0, 4.0))` returns exactly `11.0`.
8. `Vec2::new(0.0, 0.0).lerp(Vec2::new(10.0, 0.0), 0.5)` equals `Vec2::new(5.0, 0.0)` (with `approx_eq` at `EPSILON`); `lerp` with `t = 0.0` returns `self`; `lerp` with `t = 1.0` returns `other`.
9. Operator overloads behave as expected: `(Vec2::new(1.0, 2.0) + Vec2::new(3.0, 4.0)) == Vec2::new(4.0, 6.0)`; `(Vec2::new(3.0, 4.0) - Vec2::new(1.0, 1.0)) == Vec2::new(2.0, 3.0)`; `(Vec2::new(2.0, 3.0) * 2.0) == Vec2::new(4.0, 6.0)`; `(2.0 * Vec2::new(2.0, 3.0)) == Vec2::new(4.0, 6.0)`; `(Vec2::new(6.0, 8.0) / 2.0) == Vec2::new(3.0, 4.0)`; `(-Vec2::new(1.0, -2.0)) == Vec2::new(-1.0, 2.0)`.
10. `Vec2::default()` equals `Vec2::new(0.0, 0.0)`.
11. `Vec2::new(1.0, 1.0).approx_eq(Vec2::new(1.0 + 1e-10, 1.0 - 1e-10), EPSILON)` is `true`; `Vec2::new(1.0, 1.0).approx_eq(Vec2::new(1.0 + 1e-8, 1.0), EPSILON)` is `false`.
12. `Vec2::new(0.0, 0.0).distance(Vec2::new(3.0, 4.0))` returns exactly `5.0`; `distance_squared` of the same pair returns exactly `25.0`.
13. `src/geometry/epsilon.rs` exists and defines `pub const EPSILON: f64 = 1e-9;` with a doc comment that mentions both "millimeter" / "mm" and "UI" (to flag that UI tolerances differ).
14. `src/geometry/mod.rs` re-exports both `Vec2` and `EPSILON` (so `use lasercad::geometry::{Vec2, EPSILON};` works from outside the module). The `pub const MODULE: &str = "geometry";` line is removed.
15. `tests/skeleton.rs` no longer references `geometry::MODULE`; it references a real public item from `geometry` (e.g., `&geometry::EPSILON`). All other module references in `skeleton.rs` are unchanged. `cargo test --all` passes.
16. `src/geometry/vec2.rs` and `src/geometry/epsilon.rs` import nothing from `egui`, `eframe`, or `rfd` (kernel purity). Each file stays under 300 LOC.
17. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: `#[cfg(test)] mod tests` block in `vec2.rs` containing a compile-time check that constructs `Vec2 { x: 0.0, y: 0.0 }` directly via the struct literal (proves fields are `pub`).
- **Unit (AC 2, 12)**: test `length_and_distance_of_345_triple` asserting `Vec2::new(3.0, 4.0).length() == 5.0`, `Vec2::new(3.0, 4.0).length_squared() == 25.0`, `Vec2::default().distance(Vec2::new(3.0, 4.0)) == 5.0`, `Vec2::default().distance_squared(Vec2::new(3.0, 4.0)) == 25.0`.
- **Unit (AC 3)**: covered by the test above (length_squared assertion).
- **Unit (AC 4)**: test `normalize_returns_none_below_epsilon` asserting `Vec2::default().normalize().is_none()` and `Vec2::new(EPSILON / 2.0, 0.0).normalize().is_none()`.
- **Unit (AC 5)**: test `normalize_unit_x` asserting the result is approximately `Vec2::new(1.0, 0.0)`.
- **Unit (AC 6)**: test `cross_basis_vectors` asserting both forward and reversed cross products.
- **Unit (AC 7)**: test `dot_product_known_pair`.
- **Unit (AC 8)**: test `lerp_endpoints_and_midpoint`.
- **Unit (AC 9)**: test `operator_overloads` covering add, sub, scalar mul (both sides), scalar div, neg.
- **Unit (AC 10)**: test `default_is_origin`.
- **Unit (AC 11)**: test `approx_eq_tolerance` covering one PASS and one FAIL case.
- **Integration (AC 14, 15)**: update `tests/skeleton.rs` (it is already an integration test) so it uses `&geometry::EPSILON` instead of `&geometry::MODULE`. The existing assertion that all 10 modules are importable continues to pass.
- **Static check (AC 16)**: `grep -nE '^use (egui|eframe|rfd)' src/geometry/vec2.rs src/geometry/epsilon.rs` returns no matches; `wc -l src/geometry/vec2.rs src/geometry/epsilon.rs` reports each file at `<= 300` lines.
- **Build gate (AC 17)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- Vec2 layout matches the contract in [`AGENTS.md`](../../AGENTS.md) § "Units and types": `Vec2 { x: f64, y: f64 }`.
- The 1e-9 epsilon is a Phase-1 commitment; if a later demand finds it too tight or too loose, that demand owns the change (and the migration of every existing call site). LCV-010 freezes the default.
- The `MODULE` placeholder in `src/geometry/mod.rs` was introduced by LCV-001 (commit `fd6a31d`) solely to satisfy `tests/skeleton.rs` until real items existed. LCV-010 retires that placeholder for the geometry module; the same pattern will repeat per-module as each Phase-1+ demand lands.
- Re-export shape: prefer `pub use vec2::Vec2;` in `mod.rs` over `pub use vec2::*;` to keep the public surface explicit.
- The `dot` / `cross` / `length` / `normalize` names match common conventions; do not rename. Downstream demands import `Vec2` by path, not by trait, so there is no trait-coherence concern.
