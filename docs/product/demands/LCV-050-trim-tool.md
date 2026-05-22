# LCV-050 — TrimTool

- **Status**: Ready
- **Phase**: 4
- **Depends on**: LCV-041 (Done), LCV-025 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

After drawing a laser-cutting layout — parallel guides, circles overlapping
corner lines, a rectangular frame with diagonal braces — the operator needs to
clip individual line and circle segments to the boundaries they intersect.
Without TRIM, they must delete an entity and redraw the shortened version with
exact coordinates: slow and error-prone.

`TrimEntity` (LCV-025) and the intersection routines (LCV-014) are already
fully implemented. The pointer-input pipeline (LCV-041) delivers world-space mm
coordinates. This demand wires those pieces into a single-click `TrimTool`: the
operator activates TRIM, clicks on the part of a segment they want to
**remove**, and the entity is immediately shortened at every intersecting entity
— all in one undoable step per cutter.

## Scope

- **New file `src/tools/trim.rs`** — `TrimTool` struct implementing `Tool`.
  - Stateless unit struct: `#[derive(Debug, Default)] pub struct TrimTool;`
  - Two private helper functions (not `pub`) defined in the same file:
    - `fn pick_entity(pos: Vec2, entities: &[Entity]) -> Option<usize>` — finds
      the index of the entity whose stroke is nearest to `pos` within
      `PICK_THRESHOLD_MM`. Uses `Line::distance_to_point`, `Circle::distance_to_point().abs()`,
      and the arc-distance formula (same as `select/hit.rs`). Returns the index
      of the closest entity if within threshold, `None` otherwise.
    - `fn entities_intersect(a: Entity, b: Entity) -> bool` — returns `true`
      when `a` and `b` have at least one segment-level intersection:
      - `Line × Line` → `crate::geometry::intersect::line_line(&l1, &l2).is_some()`
      - `Line × Circle` or `Circle × Line` →
        `!crate::geometry::intersect::line_circle(&l, &c).is_empty()`
      - `Circle × Circle` →
        `!crate::geometry::intersect::circle_circle(&c1, &c2).is_empty()`
      - Any pair involving `Arc` → `false` (Arc trim is out of scope; the
        command layer guards against it too).
  - `pub const PICK_THRESHOLD_MM: f64 = 5.0;` — pick tolerance in mm.
    Same value as `select/hit.rs`; defined here independently (the two files
    do not share it).

- **`Tool` trait implementation** for `TrimTool`:
  - `name()` → `"TRIM"`.
  - `status_text()` → `"TRIM: Click on a segment to trim"`.
  - `on_pointer_down(pos, _shift, doc, history)`:
    1. Call `pick_entity(pos, &doc.entities)`. If `None`, return immediately
       (no-op; no command committed).
    2. Let `target_idx` be the returned index.
    3. Collect `cutter_indices: Vec<usize>` — all indices `i ≠ target_idx` such
       that `entities_intersect(doc.entities[target_idx], doc.entities[i])` is
       `true`.
    4. If `cutter_indices` is empty, return immediately (no-op; no command
       committed).
    5. For each `cutter_idx` in `cutter_indices` (any order), commit:
       `history.commit(Box::new(TrimEntity::new(target_idx, cutter_idx, pos)), doc)`.
       Each call is a separate undo step (see Notes on undo granularity).
  - `on_pointer_move(_pos, _doc)` — no-op.
  - `on_pointer_up(_pos, _shift, _doc, _history)` — no-op.
  - `on_key(key, app)`:
    - `egui::Key::Escape` → `self.cancel()` (already stateless; kept for
      trait consistency).
    - All other keys → no-op.
  - `preview()` → returns empty `Vec<Entity>` (no hover preview in this
    demand; see Out of scope).
  - `cancel()` — no-op (stateless tool; nothing to reset).

- **Update `src/tools/mod.rs`**:
  - Add `pub mod trim;`.
  - Add `pub use trim::TrimTool;`.

## Out of scope

- **Arc targets**: the intersection routines (LCV-014) do not cover arc-line
  or arc-circle; `entities_intersect` returns `false` for any pair involving
  `Arc`. A future demand adds arc trim when arc-intersection lands.
- **Live hover preview** (amber ghost of trimmed entity): requires calling
  TrimEntity logic from `on_pointer_move`; added in a later demand if the
  intersection overhead is acceptable.
- **Cutting-edge pre-selection** (AutoCAD's first-phase "select boundary"
  prompt): simplified to "all other entities are boundaries". A future demand
  may add an explicit boundary selection phase.
- **Undo grouping** (single Ctrl+Z reverting all cutters at once): requires a
  compound-command infrastructure not yet in scope. Each cutter is its own
  undo step.
- **`ExtendTool`** — owned by LCV-051.
- **Toolbar button or keyboard shortcut** for TRIM — owned by LCV-066 / LCV-070.
- **Snap integration** for the click position — LCV-054.
- **Arc-arc / arc-line intersections** — out of scope across the entire Phase 4
  until LCV-014 is extended.

## Acceptance criteria

1. `src/tools/trim.rs` exists and defines `pub struct TrimTool` that derives
   `Debug` and `Default`. `src/tools/mod.rs` exports `TrimTool` via
   `pub use trim::TrimTool`.

2. `TrimTool::name()` returns exactly `"TRIM"`.

3. `TrimTool::status_text()` returns exactly
   `"TRIM: Click on a segment to trim"`.

4. **No-op when click misses all entities**: doc with one line
   `(0.0, 0.0)→(10.0, 0.0)`, click at `Vec2::new(0.0, 20.0)` (> 5 mm away).
   `history.can_undo()` is `false` after `on_pointer_down`.

5. **No-op when target has no intersecting entities**: doc with two parallel
   horizontal lines `(0.0, 0.0)→(10.0, 0.0)` and `(0.0, 5.0)→(10.0, 5.0)`.
   Click at `Vec2::new(5.0, 0.1)` (near first line). `history.can_undo()` is
   `false` (no intersection between the two parallel lines).

6. **Basic trim — Line × Line, keep left side**: doc with
   `entities = [Line((0.0, 0.0)→(10.0, 0.0)), Line((5.0, -5.0)→(5.0, 5.0))]`.
   Click at `Vec2::new(2.0, 0.0)` (left of intersection at `(5.0, 0.0)`).
   After `on_pointer_down`, `entities[0]` is
   `Entity::Line(Line { p1 ≈ (0.0, 0.0), p2 ≈ (5.0, 0.0) })` within
   `EPSILON`; `entities[1]` is unchanged; `history.can_undo()` is `true`.

7. **Basic trim — Line × Line, keep right side**: same doc, click at
   `Vec2::new(8.0, 0.0)` (right of intersection). `entities[0]` becomes
   `Entity::Line(Line { p1 ≈ (5.0, 0.0), p2 ≈ (10.0, 0.0) })` within
   `EPSILON`.

8. **Trim with two cutters — middle segment removed**: doc with
   `entities = [Line((0.0,0.0)→(20.0,0.0)), Line((5.0,-5.0)→(5.0,5.0)),
   Line((15.0,-5.0)→(15.0,5.0))]`. Click at `Vec2::new(10.0, 0.0)` (between
   the two cutters). After `on_pointer_down`:
   - `entities[0]` is `Entity::Line` with endpoints within `EPSILON` of
     `(5.0, 0.0)` and `(15.0, 0.0)`.
   - `entities[1]` and `entities[2]` are unchanged.
   - Exactly **two** TrimEntity commands were committed (one per cutter), so
     `history.can_undo()` is `true` and a second `history.undo(doc)` followed
     by a third `history.undo(doc)` fully restores `entities[0]` to
     `(0.0, 0.0)→(20.0, 0.0)`.

9. **Undo restores step by step** (continuation of AC#8): after the two-cutter
   trim, calling `history.undo(&mut doc)` once restores `entities[0]` to an
   intermediate state `(5.0, 0.0)→(20.0, 0.0)` or `(0.0, 0.0)→(15.0, 0.0)`
   (depending on which cutter was applied last). Calling `history.undo(&mut doc)`
   a second time restores `entities[0]` to `(0.0, 0.0)→(20.0, 0.0)`.

10. **Circle × Line trim produces an Arc**: doc with
    `entities = [Circle(center=(0.0,0.0), r=5.0), Line((-6.0, 0.0)→(6.0, 0.0))]`.
    The cutter-line intersects the circle at `(-5.0, 0.0)` and `(5.0, 0.0)`.
    Click at `Vec2::new(0.0, 4.0)` (near the upper half of the circle stroke;
    distance to circle = `|(0.0,4.0) - (0.0,0.0)|.length() - 5.0 = 4.0 - 5.0| = 1.0`,
    which is within `PICK_THRESHOLD_MM = 5.0`).
    After `on_pointer_down`, `entities[0]` is an `Entity::Arc` variant (not
    `Entity::Circle`). `entities[1]` is unchanged.

11. **`on_pointer_up` is always a no-op**: call with any `pos`; assert
    `history.can_undo()` remains `false` on a fresh doc.

12. **`preview()` always returns an empty `Vec<Entity>`**.

13. **`cancel()` is a no-op**: call on a fresh `TrimTool`; no panic; state
    unchanged.

14. **Object-safe**:
    `let _: Box<dyn Tool> = Box::new(TrimTool::default());` compiles.

15. **Kernel purity**:
    `grep -nE '^use (eframe|rfd)' src/tools/trim.rs` returns no matches.
    (`egui` is allowed in `src/tools/` per AGENTS.md; `eframe` and `rfd`
    are not.)

16. **Size**: `wc -l src/tools/trim.rs` reports `≤ 300`.

17. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0 with no regressions.

## Expected tests

All tests live in `#[cfg(test)] mod tests` inside `src/tools/trim.rs`.
Each test constructs its own `Document`, `History`, and `TrimTool` directly
(no App required for the pure path).

- **Unit (AC 1)**: `trim_tool_struct_constructs` — `TrimTool::default()` and
  `let _: TrimTool = Default::default()` both compile; confirms `Default` derive.

- **Unit (AC 2)**: `name_is_trim` —
  `assert_eq!(TrimTool::default().name(), "TRIM")`.

- **Unit (AC 3)**: `status_text_is_constant` —
  `assert_eq!(TrimTool::default().status_text(), "TRIM: Click on a segment to trim")`.

- **Unit (AC 4)**: `pointer_down_miss_is_noop` — doc with one line, click 20 mm
  away; assert `hist.can_undo() == false`.

- **Unit (AC 5)**: `pointer_down_no_intersect_is_noop` — two parallel lines,
  click on one; assert `hist.can_undo() == false`.

- **Unit (AC 6)**: `trim_line_line_keep_left` — horizontal + vertical line,
  click left of intersection; assert `entities[0].p2 ≈ (5.0, 0.0)` within
  `EPSILON` and `entities[1]` unchanged.

- **Unit (AC 7)**: `trim_line_line_keep_right` — same setup, click right;
  assert `entities[0].p1 ≈ (5.0, 0.0)` within `EPSILON`.

- **Unit (AC 8)**: `trim_two_cutters_middle_removed` — line + two vertical
  cutters, click in the middle; assert `entities[0]` spans
  `(5.0, 0.0)→(15.0, 0.0)` within `EPSILON`.

- **Unit (AC 9)**: `trim_two_cutters_undo_step_by_step` — continuation of the
  two-cutter test; two successive `hist.undo` calls restore the entity to its
  original extent.

- **Unit (AC 10)**: `trim_circle_line_yields_arc` — circle at origin r=5 +
  horizontal cutter line crossing the circle, click near the upper arc stroke;
  assert `entities[0]` is `Entity::Arc(_)`.

- **Unit (AC 11)**: `pointer_up_is_noop` — fresh doc; call `on_pointer_up`;
  assert no command pushed.

- **Unit (AC 12)**: `preview_always_empty` —
  `assert!(TrimTool::default().preview().is_empty())`.

- **Unit (AC 13)**: `cancel_is_noop` — `TrimTool::default().cancel()` does not
  panic.

- **Unit (AC 14)**: `object_safe` —
  `let _: Box<dyn Tool> = Box::new(TrimTool::default());` compiles.

- **Static check (AC 15)**: `grep -nE '^use (eframe|rfd)' src/tools/trim.rs`
  returns no matches.

- **Size check (AC 16)**: `wc -l src/tools/trim.rs` ≤ 300.

- **Build gate (AC 17)**:
  `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all`
  exits 0.

## Open questions

*(none — demand is Ready)*

## Notes

- **`PICK_THRESHOLD_MM = 5.0`**: same value as `select/hit::PICK_THRESHOLD_MM`
  (LCV-042). The two constants are independent; if SelectTool's threshold
  changes, TrimTool's does not automatically follow. A future demand can
  consolidate them into a shared `src/tools/pick.rs` helper if consistency
  becomes important.

- **Undo granularity**: each `TrimEntity` command pushed by a single click is
  its own undo step. For the most common case (one intersecting cutter) this is
  one Ctrl+Z. For N cutters it is N Ctrl+Zs. A `MacroCommand` wrapper is not
  in scope for Phase 4; users trim boundary-by-boundary or undo all N steps.
  This matches v1 behavior.

- **`entities_intersect` uses segment-level tests**: `line_line` tests segment
  overlap (not infinite lines); `line_circle` and `circle_circle` also test the
  segment/circle pair as implemented in LCV-014. Pairs that only cross on
  infinite projections are not picked as cutters — matching AutoCAD's "only
  real intersections matter" rule.

- **`keep_side_point = pos`** (the raw click position): the click is already in
  world-space mm (delivered by LCV-041's snap pipeline). Since the user clicked
  near the entity they want to *remove*, `pos` is always on the segment of the
  target they intend to trim away. TrimEntity's closest-point disambiguation
  (LCV-025 Scope §"Trim keep-side rule") uses the foot of the perpendicular
  from `pos`, so the point does not need to lie exactly on the entity stroke.

- **Order of cutter iteration**: `cutter_indices` is collected from
  `(0..doc.entities.len())` in index order. The final result is the same
  regardless of iteration order because `keep_side_point` is the fixed click
  position and each TrimEntity re-reads the (already shortened) entity from
  `doc.entities[target_idx]` at `do_` time.

- **Arc targets**: `entities_intersect` returns `false` for any pair involving
  `Entity::Arc`. The TrimEntity command would be a no-op for arc targets
  anyway (it falls through to the `_ => None` arm); the pre-filter in TrimTool
  just avoids committing a visible no-op to history.

- **No state, no `take_successor`**: unlike `MoveTool` (LCV-049), TrimTool
  stays active after a successful trim so the operator can click again without
  re-activating the tool. `take_successor` (LCV-049 trait extension) defaults
  to `None` and need not be overridden.

- **AGENTS.md purity rule**: `src/tools/trim.rs` may import `egui` (for
  `egui::Key` in `on_key`). It must NOT import `eframe` or `rfd`.

- **Reference**: v1's `tools/trim.ts` followed the same stateless click
  pattern: `mouseDown → pickEntity → collectCutters → for each cutter: commitTrimEntity`.
  v2 keeps that shape and replaces the TypeScript `Command` bus with the Rust
  `history.commit` call.
