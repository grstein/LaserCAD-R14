# LCV-016 — Snap engine (endpoint / midpoint / center / intersection)

- **Status**: Done
- **Phase**: 1
- **Depends on**: LCV-014
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: ccadb5e — feat(LCV-016): add snap engine with endpoint/midpoint/center/intersection kinds

## Problem

When a CAD operator draws a new segment that should start at the endpoint of an existing one, they need the cursor to "lock" onto the meaningful geometric feature instead of trying to hit a single floating-point pixel. The four features that matter for v2's Phase-1 primitives are:

- **Endpoint** — `Line.p1`, `Line.p2`, `Arc.start_point`, `Arc.end_point`.
- **Midpoint** — `Line.midpoint`.
- **Center** — `Circle.center`, `Arc.center`.
- **Intersection** — any two entities whose intersection lies within snap range.

Without a kernel snap engine, every drawing tool would reinvent the math, and the renderer's snap-marker code (LCV-038) would have nothing to display. This demand ships the routine that takes a world-space cursor position plus the document's entities and returns the best snap (if any) within the tolerance.

User outcome: while drawing a line whose endpoint should land exactly on a circle's center, the operator moves the cursor near the center and the next click commits the line endpoint at the exact center coordinate — no eyeballing.

## Scope

- New file `src/geometry/snap.rs` defining:
  - `pub enum SnapKind { Endpoint, Midpoint, Center, Intersection }` with `#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]` (the enum is discrete, so `Eq`/`Hash` are valid here).
  - `pub enum SnapEntity { Line(Line), Circle(Circle), Arc(Arc) }` with `#[derive(Copy, Clone, Debug, PartialEq)]`. **Transitional**: this enum is a stripped-down stand-in for the `document::Entity` enum that will arrive with LCV-020. It is documented as such in the file's `//!` header. When LCV-020 lands, a follow-up demand will replace `SnapEntity` with the document `Entity` and remove the transitional enum.
  - `pub struct SnapResult { pub point: Vec2, pub kind: SnapKind, pub primary_idx: usize, pub secondary_idx: Option<usize> }` with `#[derive(Copy, Clone, Debug, PartialEq)]` (no `Eq`/`Hash` because `point: Vec2`). `primary_idx` is the index into the input slice of the entity that produced the snap (the entity carrying the endpoint/midpoint/center; for intersections, one of the two entities). `secondary_idx` is `Some(idx)` only for `SnapKind::Intersection`, identifying the other participant; `None` for all other kinds.
  - `pub fn snap(world: Vec2, tolerance: f64, entities: &[SnapEntity]) -> Option<SnapResult>` — returns the best snap candidate within `tolerance` of `world`, or `None` if no feature is within range. `tolerance` is in the same units as `world` (millimeters); callers convert from screen-pixel snap radius to world tolerance via the renderer's camera (LCV-035, future).
  - Candidate enumeration:
    - **Endpoint** candidates: for each `SnapEntity::Line(l)`, add `l.p1` and `l.p2`. For each `SnapEntity::Arc(a)`, add `a.start_point()` and `a.end_point()`. Circles do not contribute endpoint candidates.
    - **Midpoint** candidates: for each `SnapEntity::Line(l)`, add `l.midpoint()`. Arcs and circles do not contribute midpoint candidates in this demand (arc midpoint at half the sweep is deferred).
    - **Center** candidates: for each `SnapEntity::Circle(c)`, add `c.center`. For each `SnapEntity::Arc(a)`, add `a.center`.
    - **Intersection** candidates: for each unordered pair of entities, compute intersections via LCV-014 (`line_line`, `line_circle`, `circle_circle`). Arc-involving pairs are skipped in this demand (no arc intersection routines exist yet — documented limitation). Each intersection point becomes one candidate with `primary_idx = lower index`, `secondary_idx = Some(higher index)`.
  - Selection rule: among all candidates within `tolerance` of `world`, pick the one with the **smallest distance to `world`**. Break ties on equal distance (within `EPSILON`) by `SnapKind` priority `Endpoint > Intersection > Midpoint > Center`. Break further ties on `primary_idx` (lower wins). The selection is deterministic.
- Update `src/geometry/mod.rs`:
  - Add `pub mod snap;`.
  - Add `pub use snap::{SnapKind, SnapEntity, SnapResult, snap};`.
- Per-module unit tests under `#[cfg(test)] mod tests` in `snap.rs`. No new file under `tests/`.

## Out of scope

- Rendering snap markers (the visible square / triangle / diamond / X glyphs at the snapped point) — owned by **LCV-038** (Phase 3).
- Tool integration (binding the snap engine to the active drawing tool so that the next click commits at the snapped point) — owned by **LCV-054** (Phase 4).
- Replacing the transitional `SnapEntity` with the document `Entity` from LCV-020 — owned by a follow-up demand once LCV-020 ships.
- Arc-arc, arc-line, and arc-circle intersection candidates — deferred. The demand explicitly documents that arc-involving intersection pairs are skipped.
- Arc midpoint (point at half the sweep) — deferred. No current consumer; can be added later without breaking the API.
- Snap toggles (per-kind enable/disable). This demand always considers all four kinds; the toggle UI (LCV-068, future) gates the call site, not the kernel.
- Performance optimization (spatial indexing). The expected entity count for a v2 document is ≤10,000; linear scan is fine.
- Tracking / construction-line snaps (extension, perpendicular, tangent). Not in v2 Phase 1.

## Acceptance criteria

1. `src/geometry/snap.rs` exists and defines `SnapKind`, `SnapEntity`, `SnapResult`, and `pub fn snap(world: Vec2, tolerance: f64, entities: &[SnapEntity]) -> Option<SnapResult>` with exactly the signatures listed in Scope.
2. **No-candidate case**: `snap(Vec2(0, 0), 1.0, &[])` returns `None`.
3. **Out-of-tolerance case**: with `entities = [SnapEntity::Line(Line::new(Vec2(10, 10), Vec2(20, 10)))]`, `snap(Vec2(0, 0), 1.0, &entities)` returns `None`.
4. **Endpoint snap**: with the same entities, `snap(Vec2(10.1, 10.1), 1.0, &entities)` returns `Some(SnapResult { point: Vec2(10, 10), kind: SnapKind::Endpoint, primary_idx: 0, secondary_idx: None })` (point compared within `EPSILON`).
5. **Midpoint snap**: with `entities = [SnapEntity::Line(Line::new(Vec2(0, 0), Vec2(10, 0)))]`, `snap(Vec2(5.1, 0.1), 1.0, &entities)` returns `Some(SnapResult { point: Vec2(5, 0), kind: SnapKind::Midpoint, primary_idx: 0, secondary_idx: None })`.
6. **Center snap**: with `entities = [SnapEntity::Circle(Circle::new(Vec2(5, 5), 3.0))]`, `snap(Vec2(5.2, 4.9), 1.0, &entities)` returns `Some(SnapResult { point: Vec2(5, 5), kind: SnapKind::Center, primary_idx: 0, secondary_idx: None })`.
7. **Intersection snap (line-line)**: with `entities = [SnapEntity::Line(Line::new(Vec2(0, 0), Vec2(10, 0))), SnapEntity::Line(Line::new(Vec2(5, -5), Vec2(5, 5)))]`, `snap(Vec2(5.1, 0.1), 1.0, &entities)` returns `Some(SnapResult { point: Vec2(5, 0), kind: SnapKind::Intersection, primary_idx: 0, secondary_idx: Some(1) })`.
8. **Tie-break by priority — Endpoint wins over Midpoint**: with `entities = [SnapEntity::Line(Line::new(Vec2(0, 0), Vec2(10, 0))), SnapEntity::Line(Line::new(Vec2(5, 0), Vec2(5, 10)))]`, the cursor at `Vec2(5, 0)` is equidistant (distance 0) from both `entities[1].p1` (endpoint) and `entities[0].midpoint` (midpoint). The result has `kind == SnapKind::Endpoint` and `primary_idx == 1`.
9. **Tie-break by priority — Intersection wins over Midpoint**: with `entities = [SnapEntity::Line(Line::new(Vec2(0, 0), Vec2(10, 0))), SnapEntity::Line(Line::new(Vec2(5, -5), Vec2(5, 5)))]`, the cursor at `Vec2(5, 0)` is exactly on both the midpoint of `entities[0]` and the line-line intersection. The result has `kind == SnapKind::Intersection`.
10. **Tie-break by priority — Endpoint wins over Intersection**: at `Vec2(0, 0)` with `entities = [SnapEntity::Line(Line::new(Vec2(0, 0), Vec2(10, 0))), SnapEntity::Line(Line::new(Vec2(0, -5), Vec2(0, 5)))]` — the cursor coincides with `entities[0].p1` (endpoint, distance 0) and with the line-line intersection (distance 0). The result has `kind == SnapKind::Endpoint`.
11. **Closer non-priority candidate wins over farther priority candidate**: when a Midpoint is at distance 0.1 and an Endpoint is at distance 0.5 (both within tolerance 1.0), the Midpoint wins. The priority tie-break only applies when distances are equal within `EPSILON`.
12. **Determinism on equal-priority equal-distance ties**: when two endpoints are at identical distance (within `EPSILON`), the candidate with the lower `primary_idx` is returned. (Tested with two lines whose shared endpoint coincides with the cursor.)
13. **Arc endpoint snap**: with `entities = [SnapEntity::Arc(Arc { center: Vec2::default(), r: 1.0, start_angle: 0.0, end_angle: PI/2, ccw: true })]`, `snap(Vec2(1.05, 0.05), 1.0, &entities)` returns `Some(SnapResult { point: Vec2(1, 0), kind: SnapKind::Endpoint, primary_idx: 0, secondary_idx: None })`.
14. **Arc center snap**: with the same arc, `snap(Vec2(0.1, 0.1), 1.0, &entities)` returns `Some(SnapResult { point: Vec2(0, 0), kind: SnapKind::Center, primary_idx: 0, secondary_idx: None })`.
15. **Arc-involving intersection pairs are skipped**: with `entities = [SnapEntity::Arc(...), SnapEntity::Line(...)]` where the arc and line visibly cross, `snap` does **not** return a `SnapKind::Intersection` for that pair. It may still return Endpoint/Midpoint/Center for either entity. (This is the documented limitation.)
16. `src/geometry/mod.rs` re-exports `SnapKind`, `SnapEntity`, `SnapResult`, and `snap`.
17. `src/geometry/snap.rs` imports nothing from `egui`, `eframe`, or `rfd`. The file stays under 300 LOC. The file may import from `crate::geometry::{Vec2, Line, Circle, Arc, EPSILON, intersect}`.
18. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: a test that constructs each of `SnapKind::Endpoint`, `SnapEntity::Line(...)`, and `SnapResult { ... }` via literal syntax (proves the public surface is as declared).
- **Unit (AC 2)**: test `snap_empty_entities_returns_none`.
- **Unit (AC 3)**: test `snap_outside_tolerance_returns_none`.
- **Unit (AC 4)**: test `snap_endpoint_of_line`.
- **Unit (AC 5)**: test `snap_midpoint_of_line`.
- **Unit (AC 6)**: test `snap_center_of_circle`.
- **Unit (AC 7)**: test `snap_intersection_of_two_lines` (also asserts `secondary_idx == Some(1)`).
- **Unit (AC 8)**: test `tie_endpoint_beats_midpoint`.
- **Unit (AC 9)**: test `tie_intersection_beats_midpoint`.
- **Unit (AC 10)**: test `tie_endpoint_beats_intersection`.
- **Unit (AC 11)**: test `closer_non_priority_wins_over_farther_priority`.
- **Unit (AC 12)**: test `equal_priority_equal_distance_returns_lower_index`.
- **Unit (AC 13)**: test `snap_arc_endpoint`.
- **Unit (AC 14)**: test `snap_arc_center`.
- **Unit (AC 15)**: test `arc_involving_intersection_pairs_skipped` — asserts the returned `SnapResult.kind != SnapKind::Intersection` when the only feature near the cursor is the arc-line crossing.
- **Static check (AC 17)**: `grep -nE '^use (egui|eframe|rfd)' src/geometry/snap.rs` returns no matches; `wc -l src/geometry/snap.rs` reports `<= 300`.
- **Build gate (AC 18)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- The transitional `SnapEntity` enum is **deliberate technical debt** introduced because LCV-020 (`document::Entity`) does not yet exist. The follow-up work is tracked here: once LCV-020 ships, a future Phase-2 demand will (a) change the `snap` function signature to accept `&[document::Entity]`, (b) delete the `SnapEntity` enum, (c) update LCV-038 (snap markers) and LCV-054 (tool integration) accordingly. Estimated migration: ~50 LOC changed across snap.rs and tests. Flag this in `Notes` so `project-manager` schedules a `Replace transitional SnapEntity` demand after LCV-020 lands.
- Tie-break priority (`Endpoint > Intersection > Midpoint > Center`) matches AutoCAD's default OSNAP precedence so muscle memory carries over.
- `tolerance` is in mm (world units). The renderer multiplies a pixel snap radius (e.g., 8 px) by the inverse of the camera's pixels-per-mm scale to derive the world tolerance — that conversion lives in the render module, not here.
- The intersection enumeration is `O(n^2)`. For the v2 document size cap (~10k entities) this is acceptable; the snap call only runs at pointer-event rate, not per-frame. Profiling can revisit if it becomes a bottleneck.
- Determinism (AC 12) matters for reproducible bug reports — two operators with the same document should see the same snap.
- Reference: v1's `tools/snap.ts` had `EndpointSnap`, `MidpointSnap`, `CenterSnap`, `IntersectionSnap` classes; v2 collapses them into a single `snap()` function returning a tagged result, which is easier to test and call.
