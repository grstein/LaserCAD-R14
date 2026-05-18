# LCV-025 — TrimEntities + ExtendEntities commands

- **Status**: Ready
- **Phase**: 2
- **Depends on**: LCV-023, LCV-024
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: <to be filled by demand-manager>

## Problem

`Trim` and `Extend` are the two AutoCAD modify commands every CAD operator relies on to clean up intersecting geometry: "shorten this line at the cutter" / "stretch this line to the boundary". The Phase-4 `TrimTool` (LCV-050) and `ExtendTool` (LCV-051) need a command-layer implementation to wrap.

Both operations are surgical: they replace a single entity in place with a modified version, capturing the original so undo is exact. The math reuses LCV-014's intersection routines and LCV-011 / LCV-013's helpers; the command's job is bookkeeping.

User outcome: the operator clicks a vertical line as the cutter and a horizontal line on the side they want to keep; Trim shortens the horizontal line to the intersection. Ctrl+Z restores the original full-length horizontal line.

## Scope

- New file `src/document/commands/trim.rs` defining:
  - `pub struct TrimEntity { pub target_idx: usize, pub cutter_idx: usize, pub keep_side_point: Vec2, captured: Option<Entity> }`.
    - Constructor `pub fn new(target_idx: usize, cutter_idx: usize, keep_side_point: Vec2) -> Self`.
    - `do_(&mut self, doc)`:
      1. Read `target = doc.entities[target_idx]` and `cutter = doc.entities[cutter_idx]` (capture by `Copy`).
      2. Compute the intersection(s) between target and cutter using `crate::geometry::intersect::{line_line, line_circle, circle_circle}` per the supported pair (see below).
      3. If there are zero intersections, the trim is a no-op: `captured` stays `None` and `do_` returns without mutation. (Documented behavior; the Phase-4 tool prevents this case by only invoking when the cursor is over a real intersection.)
      4. If there are one or more intersections, choose the segment / arc of the target that **contains** `keep_side_point` (per the geometry rules below), build the trimmed entity, store the original in `captured = Some(target)`, and write the trimmed entity back to `doc.entities[target_idx]`.
    - `undo(&mut self, doc)`: if `captured.is_some()`, restore `doc.entities[target_idx] = captured.take().unwrap()`.
    - `label(&self) -> &str { "Trim" }`.
  - `pub struct ExtendEntity { pub target_idx: usize, pub boundary_idx: usize, pub extend_endpoint: u8, captured: Option<Entity> }`.
    - `extend_endpoint` is `0` (extend `p1`) or `1` (extend `p2`). Values outside `{0, 1}` are an invariant violation — the implementer may `debug_assert!`. The Phase-4 `ExtendTool` picks the endpoint based on which side of the target the cursor is over.
    - Constructor `pub fn new(target_idx: usize, boundary_idx: usize, extend_endpoint: u8) -> Self`.
    - `do_(&mut self, doc)`:
      1. Read `target` and `boundary`.
      2. Compute the intersection of the infinite-line projection of the target line with the boundary (line or circle) using `line_line_infinite` or `line_circle` (extended-line variant — the implementer chooses whether to use the infinite or segment variant; the AC pins the behavior).
      3. If there is no intersection, `do_` is a no-op (`captured` stays `None`). If there are multiple (e.g., line-circle), pick the intersection nearest to the endpoint being extended.
      4. Build the extended entity (the target line with `p1` or `p2` replaced by the intersection point), store the original in `captured`, write the extended entity back.
    - `undo`: same restore pattern as `TrimEntity`.
    - `label(&self) -> &str { "Extend" }`.
- Supported entity pairs (target × cutter or target × boundary):
  - **Trim**: target Line × cutter Line; target Line × cutter Circle; target Circle × cutter Line. Arc targets are out of scope (see Out of scope).
  - **Extend**: target Line × boundary Line; target Line × boundary Circle. Circle targets are out of scope.
- Trim "keep side" rule:
  - **Target Line, single cutter intersection at point `X`**: the target splits into two sub-segments: `[p1, X]` and `[X, p2]`. The result is whichever sub-segment contains `keep_side_point` in its closed segment (use `Line::closest_point(keep_side_point)` and check whether the foot is on that sub-segment within `EPSILON`). If `keep_side_point` is ambiguous (equidistant from both sub-segments, e.g., it coincides with `X`), the implementer keeps `[p1, X]` deterministically (documented choice).
  - **Target Line, two cutter intersections (cutter Circle)**: the target splits into three sub-segments. Pick the one containing `keep_side_point`. If `keep_side_point` is in the middle segment, replace the target with that middle segment (the cutter "carves out" the middle); otherwise replace with whichever outer segment contains `keep_side_point`.
  - **Target Circle, cutter Line** (one or two intersections): for two intersections, the target splits into two arcs (CCW from one intersection to the other and back). The result is a new `Entity::Arc` representing the arc on the same side as `keep_side_point` (closest-point on circle test). For one intersection (tangent), trim is a no-op (matches AutoCAD behavior — cannot trim a circle at a tangent because there is no closed split).
- Extend rule:
  - The extended endpoint moves to the chosen intersection point. The other endpoint stays. If the intersection is "behind" the current endpoint (i.e., extending would shorten the line instead of lengthening it), `do_` is a no-op. ("Behind" = the parameter `t` of the intersection point projected onto the infinite line is on the same side of `[0, 1]` as the unmoved endpoint.)
- Update `src/document/commands/mod.rs`:
  - Add `pub mod trim;`.
  - Add `pub use trim::{TrimEntity, ExtendEntity};`.
- `commands/trim.rs` MUST NOT import `egui`, `eframe`, or `rfd`. The file stays ≤300 LOC. It may import from `crate::geometry::*` and `crate::document::{Document, Entity, Command}`.
- Per-module unit tests under `#[cfg(test)] mod tests` in `trim.rs`.

## Out of scope

- **Arc targets** for either Trim or Extend. Arc-involving intersection routines do not exist yet (LCV-014 covers line-line / line-circle / circle-circle only). A future demand adds arc trim/extend when arc intersection lands.
- **Circle targets for Extend**. A circle is closed; "extending" it has no defined CAD meaning. AutoCAD does not extend circles either.
- **Chain trim / multi-cutter** ("trim using all selected entities as cutters at once"). The Phase-4 `TrimTool` issues one `TrimEntity` per click; chaining is the tool's job, not the command's.
- **Tool wiring** — owned by LCV-050 / LCV-051 (Phase 4).
- **Snapping the `keep_side_point` or `boundary` intersection to grid / ortho**. Pre-snapped points are the tool's responsibility.
- **Modifying both target and cutter** (e.g., "Fillet" creates a connecting arc and modifies both lines). Fillet is not in v2 Phase 2.
- **Trim semantics for coincident / parallel pairs**: if target and cutter are coincident or parallel (no intersection), `do_` is a no-op. The Phase-4 tool prevents this case by validating before commit.

## Acceptance criteria

1. `src/document/commands/trim.rs` exists and defines `pub struct TrimEntity` and `pub struct ExtendEntity` with the constructors and method signatures in Scope. `commands/mod.rs` re-exports both.
2. **Trim Line × Line — basic**: `entities = [Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0))), Entity::Line(Line::new(Vec2::new(5.0, -1.0), Vec2::new(5.0, 1.0)))]`. `TrimEntity::new(0, 1, Vec2::new(3.0, 0.0))` after `do_` makes `entities[0] == Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(5.0, 0.0)))` (within `EPSILON`); `entities[1]` is unchanged. `undo` restores `entities[0]` to its original.
3. **Trim Line × Line — keep other side**: with the same setup but `keep_side_point = Vec2::new(7.0, 0.0)`, `do_` makes `entities[0] == Entity::Line(Line::new(Vec2::new(5.0, 0.0), Vec2::new(10.0, 0.0)))` (within `EPSILON`).
4. **Trim Line × Circle — keep middle**: `entities = [Entity::Line(Line::new(Vec2::new(-5.0, 0.0), Vec2::new(5.0, 0.0))), Entity::Circle(Circle::new(Vec2::default(), 2.0))]`. The cutter intersects the target at `(-2, 0)` and `(2, 0)`. `TrimEntity::new(0, 1, Vec2::new(0.0, 0.0))` (middle) after `do_` makes `entities[0] == Entity::Line(Line::new(Vec2::new(-2.0, 0.0), Vec2::new(2.0, 0.0)))` (within `EPSILON`); `undo` restores.
5. **Trim Line × Circle — keep one outer side**: same entities but `keep_side_point = Vec2::new(-4.0, 0.0)`. `do_` makes `entities[0] == Entity::Line(Line::new(Vec2::new(-5.0, 0.0), Vec2::new(-2.0, 0.0)))` (within `EPSILON`); `undo` restores.
6. **Trim Circle × Line**: `entities = [Entity::Circle(Circle::new(Vec2::default(), 1.0)), Entity::Line(Line::new(Vec2::new(-2.0, 0.0), Vec2::new(2.0, 0.0)))]`. The cutter intersects the circle at `(-1, 0)` and `(1, 0)`. `TrimEntity::new(0, 1, Vec2::new(0.0, 1.0))` (upper half) after `do_` replaces `entities[0]` with `Entity::Arc(_)` representing the upper semicircle (start_angle `0`, end_angle `π`, ccw `true`, center origin, radius 1, within `EPSILON` on angles). `undo` restores the original `Entity::Circle`.
7. **Trim with no intersection is a no-op**: `entities = [Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0))), Entity::Line(Line::new(Vec2::new(20.0, 20.0), Vec2::new(30.0, 30.0)))]`. `TrimEntity::new(0, 1, Vec2::new(5.0, 0.0))` `do_` leaves the document unchanged and `captured` remains `None`. `undo` is a safe no-op.
8. **Extend Line × Line — basic**: `entities = [Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(5.0, 0.0))), Entity::Line(Line::new(Vec2::new(10.0, -1.0), Vec2::new(10.0, 1.0)))]`. `ExtendEntity::new(0, 1, 1)` (extend `p2`) after `do_` makes `entities[0] == Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0)))` (within `EPSILON`); `undo` restores.
9. **Extend Line × Circle — nearest intersection wins**: `entities = [Entity::Line(Line::new(Vec2::new(-5.0, 0.0), Vec2::new(-3.0, 0.0))), Entity::Circle(Circle::new(Vec2::default(), 2.0))]`. The infinite-line projection crosses the circle at `(-2, 0)` and `(2, 0)`. `ExtendEntity::new(0, 1, 1)` (extend `p2`, which is the closer endpoint to the circle) after `do_` makes `entities[0].p2 == Vec2::new(-2.0, 0.0)` (within `EPSILON`) — the nearest intersection. `undo` restores.
10. **Extend with intersection "behind" the endpoint is a no-op**: `entities = [Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0))), Entity::Line(Line::new(Vec2::new(-5.0, -1.0), Vec2::new(-5.0, 1.0)))]`. `ExtendEntity::new(0, 1, 1)` (try to extend `p2` to `x=-5`, which is behind `p2 = (10, 0)`) `do_` leaves the document unchanged and `captured` remains `None`.
11. **Labels are exact**: `TrimEntity::new(0, 1, Vec2::default()).label() == "Trim"`; `ExtendEntity::new(0, 1, 0).label() == "Extend"`.
12. **Object-safe**: `let _: Box<dyn Command> = Box::new(TrimEntity::new(0, 1, Vec2::default())); let _: Box<dyn Command> = Box::new(ExtendEntity::new(0, 1, 0));` both compile.
13. Kernel-purity: `grep -nE '^use (egui|eframe|rfd)' src/document/commands/trim.rs` returns no matches.
14. Size: `wc -l src/document/commands/trim.rs` reports `<= 300`. If implementation pressure pushes past 300, split into `commands/trim_line.rs` (Line targets) and `commands/trim_circle.rs` (Circle targets) and update `commands/mod.rs` accordingly; describe the split in the commit message.
15. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: test `trim_module_defines_trim_and_extend` — instantiates each via `new(...)`.
- **Unit (AC 2)**: test `trim_line_line_keep_left_side`.
- **Unit (AC 3)**: test `trim_line_line_keep_right_side`.
- **Unit (AC 4)**: test `trim_line_circle_keep_middle`.
- **Unit (AC 5)**: test `trim_line_circle_keep_outer_side`.
- **Unit (AC 6)**: test `trim_circle_line_yields_arc` — asserts `Entity::Arc` variant and angles within `EPSILON`.
- **Unit (AC 7)**: test `trim_no_intersection_is_noop` — also asserts `undo` is safe.
- **Unit (AC 8)**: test `extend_line_line_to_intersection`.
- **Unit (AC 9)**: test `extend_line_circle_picks_nearest_intersection`.
- **Unit (AC 10)**: test `extend_behind_endpoint_is_noop`.
- **Unit (AC 11)**: test `trim_extend_labels_exact`.
- **Unit (AC 12)**: test `trim_extend_commands_are_object_safe`.
- **Static check (AC 13)**: `grep -nE '^use (egui|eframe|rfd)' src/document/commands/trim.rs` returns no matches.
- **Size check (AC 14)**: `wc -l src/document/commands/trim.rs` reports `<= 300` (or the split files all do).
- **Build gate (AC 15)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- The trim-keep-side disambiguation for the "exactly at intersection" case is deliberately specified (deterministic: keep `[p1, X]`) so tests are not flaky. This is a corner the Phase-4 tool prevents in practice by snapping `keep_side_point` to the segment interior.
- Trim Circle × Line yielding an Arc is the canonical AutoCAD behavior. The arc orientation is **CCW from the first intersection (smaller angle from the circle center) to the second**, and the choice of which arc is "on `keep_side_point`'s side" is made by the closest-point-on-circle test. Implementers verify by examining the AC 6 test: keeping `(0, 1)` over a unit circle cut by the x-axis must yield the upper arc with `start_angle = 0`, `end_angle = π`, `ccw = true`.
- Extend's "behind" no-op rule prevents the surprising case where the user clicks "extend" on a line whose infinite projection would shorten the segment. This matches AutoCAD's behavior of silently doing nothing in that case.
- Three intersection routines exist (LCV-014): `line_line`, `line_circle`, `circle_circle`. For Extend Line × Line, use `line_line_infinite` (extended-line variant) because the target's endpoint can be beyond either current line. For Extend Line × Circle, use `line_circle` with an infinite-line interpretation — the implementer chooses the cleanest API.
- The 300-LOC cap may bite if the four trim cases (Line × Line, Line × Circle, Circle × Line, plus Extend's two cases) each take ~50 LOC. The Scope explicitly authorizes a split if needed; the AC pins the behavior, not the file layout.
- AGENTS.md §"State and mutation": tools never mutate `Document` directly. `TrimTool` (LCV-050) builds a `TrimEntity` and calls `App::commit`; this demand makes that pathway possible.
- Reference: v1's `commands/trimEntity.ts` and `commands/extendEntity.ts` used the same single-entity-replacement pattern with `captured` rollback. v2 keeps the shape but supports the same three target/cutter pairs from day one.
- The captured-Entity-as-Option pattern from LCV-023's CreateLine generalizes: any "modify or replace one entity in place" command captures the original `Entity` value (cheap because `Entity` is `Copy`) and writes it back on `undo`.
