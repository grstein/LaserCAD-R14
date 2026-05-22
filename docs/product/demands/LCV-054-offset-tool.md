# LCV-054 — OffsetTool — Offset a Line or Arc by Distance

- **Status**: Ready
- **Phase**: 4
- **Depends on**: LCV-041 (Done), LCV-023 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

When producing laser-cutting geometry, operators frequently need a parallel copy
of an edge at a precise standoff distance — an outer allowance 0.2 mm outside a
cut line for kerf compensation, a concentric ring 3 mm outside an arc for a
stacked-layer part, or an inner reference line for a fold. Without an OFFSET
command the operator must derive the offset coordinates by hand, open a drawing
tool, enter typed absolute coordinates, and then measure the result to verify —
a multi-step, error-prone workflow that interrupts the CAD-to-laser pipeline.
`CreateLine` (LCV-023) and `CreateArc` (LCV-023) are already in place; this
demand wires them to a two-click offset tool following the AutoCAD R14 `OFFSET`
interaction model.

## Scope

- **New file `src/tools/offset.rs`** — `OffsetTool` implementing `Tool`.
  File must stay ≤ 300 LOC (AGENTS.md hard cap).
- **Tool name**: `name()` returns exactly `"OFFSET"`.
- **Distance property**: `pub distance_mm: f64` on `OffsetTool`, initialized to
  `1.0` mm. Persists across uses in the same session. Exposed `pub` so the
  Phase-6 command-line wiring demand can write it without downcasting.
- **State machine** with two states:
  - `Idle`: waiting for entity selection click.
  - `WaitingSide { entity_index: usize, entity: Entity, cursor: Vec2 }`: an
    entity has been picked; cursor is tracked for the live preview; waiting for
    the side-pick click.
- **`on_pointer_down` in `Idle`** (first click — entity pick):
  - Uses `entity_distance_to_point` (currently `pub(super)` in
    `src/tools/select/hit.rs`; see Notes) to find the closest entity within
    `PICK_THRESHOLD_MM` (5.0 mm).
  - Only `Entity::Line` and `Entity::Arc` are eligible candidates;
    `Entity::Circle` is skipped.
  - If an eligible entity is found: transition to
    `WaitingSide { entity_index, entity: doc.entities[entity_index].clone(), cursor: pos }`.
  - If no eligible entity is found within the threshold: stay in `Idle`; no
    command is committed.
- **`on_pointer_move` in `WaitingSide`**: update `cursor` for the live preview.
  No-op in `Idle`.
- **`on_pointer_down` in `WaitingSide`** (second click — side pick):
  - Compute the offset entity using the geometry rules below.
  - If the result is **valid**: commit one command (`CreateLine::new(…)` or
    `CreateArc::new(…)`) via `history.commit(cmd, doc)`; reset to `Idle`
    (preserving `distance_mm`).
  - If the result is **degenerate or ambiguous** (see geometry rules): do
    nothing — stay in `WaitingSide` so the operator can try a different click
    position.
- **`on_pointer_up`**: no-op in all states.
- **`on_key`**:
  - `Key::Escape` in any state: `self.cancel()`.
  - `Key::Enter` in `WaitingSide`: same effect as a pointer-down at `cursor`
    (commit if valid, otherwise no-op). Allows keyboard confirmation once the
    side is visually obvious.
  - All other keys: no-op.
- **`preview()`**: in `WaitingSide`, compute the offset entity using `cursor`
  with the same geometry rules as `on_pointer_down`; if valid, return
  `vec![offset_entity]`; otherwise return `vec![]`. Return `vec![]` in `Idle`.
- **`cancel()`**: reset state to `Idle`; leave `distance_mm` unchanged.
- **`status_text()`**:
  - `Idle`: `"OFFSET: Click entity to offset"`.
  - `WaitingSide`: `"OFFSET: Click side to offset toward  |  Esc to cancel"`.
- **Register in `src/tools/mod.rs`**: add `pub mod offset;` and
  `pub use offset::OffsetTool;`.

### Offset geometry

#### Line offset (`Entity::Line(l)`)

1. **Degenerate source guard**: if `l.length() ≤ EPSILON`, the source is
   degenerate — result is **invalid**.
2. Compute the signed area of the triangle `(l.p1, l.p2, pos)`:
   ```
   signed = (l.p2.x - l.p1.x) * (pos.y - l.p1.y)
           - (l.p2.y - l.p1.y) * (pos.x - l.p1.x)
   ```
3. **Ambiguous side guard**: if `|signed| ≤ EPSILON`, `pos` lies on the
   line — result is **invalid**.
4. Compute the unit direction `dir` via `l.direction().unwrap()` (safe after
   step 1) and the perpendicular toward `pos`:
   - `signed > 0` (pos is left of `p1→p2`): `perp = Vec2::new(-dir.y, dir.x)`.
   - `signed < 0` (pos is right of `p1→p2`): `perp = Vec2::new(dir.y, -dir.x)`.
5. Result line:
   ```
   Line { p1: l.p1 + perp * distance_mm, p2: l.p2 + perp * distance_mm }
   ```
6. Commit `CreateLine::new(result_line)`.

#### Arc offset (`Entity::Arc(a)`)

1. Compute `click_dist = (pos - a.center).length()`.
2. **Ambiguous side guard**: if `(click_dist - a.r).abs() ≤ EPSILON`, `pos`
   lies on the arc stroke — result is **invalid**.
3. Determine new radius:
   - `click_dist > a.r` (pos is outside): `new_r = a.r + distance_mm`.
   - `click_dist < a.r` (pos is inside):  `new_r = a.r - distance_mm`.
4. **Degenerate result guard**: if `new_r ≤ EPSILON`, result is **invalid**.
5. Result arc:
   ```
   Arc { center: a.center, r: new_r,
         start_angle: a.start_angle, end_angle: a.end_angle, ccw: a.ccw }
   ```
6. Commit `CreateArc::new(result_arc)`.

## Out of scope

- **`Entity::Circle` offset**: circles have no directional "side" resolvable
  from a single click in this demand. A future demand may add circle offset if
  the interaction model is defined.
- **Distance input via keyboard or command line**: `distance_mm` defaults to
  1.0 mm. Setting it from the command-line widget is deferred to the Phase-6
  command-line integration demand (which will call `tool.distance_mm = d` on
  the active tool). No digit-key input is wired in this demand; no changes to
  `App::update` key forwarding are required.
- **Looping / auto-reselect after commit**: after one successful offset the
  tool resets to `Idle`. AutoCAD's loop-back pattern (auto-prompt for next
  entity) is not implemented.
- **Undo grouping**: the commit is one `CreateLine` / `CreateArc` history entry.
  No grouped undo with the entity pick.
- **Polyline offset**: polylines are stored as individual `Line` entities;
  operators offset each segment independently.
- **Offset of multi-segment paths or splines**: not supported in v2.
- **Variable / tapered offset**: the offset distance is uniform.
- **Kerf-compensation presets / material database**: offset geometry is
  general-purpose; no preset logic.
- **Toolbar button or menu entry for OFFSET**: Phase-6 UI work.
- **Command alias registration** in a Phase-6 dispatch table: out of scope
  for this demand.

## Acceptance criteria

1. `src/tools/offset.rs` exists; `src/tools/mod.rs` declares `pub mod offset`
   and re-exports `pub use offset::OffsetTool`.

2. `OffsetTool::name()` returns exactly `"OFFSET"`.

3. `OffsetTool` implements `Default`; the default value has `distance_mm == 1.0`
   and `state == Idle`.

4. **Entity pick — Line**: given a `Document` with one `Entity::Line` and a
   click within `PICK_THRESHOLD_MM` (5.0 mm) of it, `on_pointer_down` in `Idle`
   transitions the tool to `WaitingSide`. `status_text()` contains `"side"`.
   No command is committed (`history.can_undo() == false`).

5. **Entity pick — Arc**: same as AC#4 but for `Entity::Arc`. The tool
   transitions to `WaitingSide`.

6. **Entity pick — miss**: a click farther than `PICK_THRESHOLD_MM` from all
   entities leaves the tool in `Idle`. `history.can_undo() == false`.
   `preview()` returns an empty vec.

7. **Entity pick — Circle skipped**: if the only entity within
   `PICK_THRESHOLD_MM` is `Entity::Circle`, `on_pointer_down` stays in `Idle`;
   no command is committed.

8. **Side pick — Line commits `CreateLine`**: from `WaitingSide` (Line entity
   selected, `distance_mm = 5.0`), a click with `|signed| > EPSILON` commits
   exactly one `CreateLine` command. After the call: `doc.entity_count()` has
   increased by one, `history.can_undo() == true`, tool is back in `Idle`.

9. **Parallel line geometry — left side**: source line
   `Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0))`, `distance_mm = 5.0`,
   click at `Vec2::new(5.0, 3.0)` (above the line → `signed > 0`).
   Committed line: `p1 ≈ (0.0, 5.0)`, `p2 ≈ (10.0, 5.0)` within `EPSILON`.

10. **Parallel line geometry — right side**: same source line, click at
    `Vec2::new(5.0, -3.0)` (below → `signed < 0`).
    Committed line: `p1 ≈ (0.0, -5.0)`, `p2 ≈ (10.0, -5.0)` within `EPSILON`.

11. **Side pick — Arc commits `CreateArc`**: from `WaitingSide` (Arc entity
    selected, `distance_mm = 2.0`), a click clearly outside the arc radius
    commits exactly one `CreateArc` command. `doc.entity_count()` increases by
    one; `history.can_undo() == true`; tool resets to `Idle`.

12. **Arc outward offset geometry**: source arc centered at origin with `r = 5.0`,
    `distance_mm = 2.0`, click at distance `7.0` from center (outside).
    Committed arc: `center` unchanged, `r ≈ 7.0`, `start_angle`, `end_angle`,
    and `ccw` identical to the source, all within `EPSILON`.

13. **Arc inward offset geometry**: same arc, click at distance `3.0` from
    center (inside). Committed arc: `r ≈ 3.0`; all other fields unchanged.

14. **Degenerate inward arc rejected**: source arc with `r = 2.0`,
    `distance_mm = 3.0`, click inside the arc radius (where `new_r = -1.0 ≤ EPSILON`).
    No command is committed; `history.can_undo() == false`; tool stays in
    `WaitingSide` (operator can try a valid side).

15. **Ambiguous side — Line rejected**: click on the line
    (`|signed| ≤ EPSILON`): no command is committed; tool stays in
    `WaitingSide`.

16. **Ambiguous side — Arc rejected**: click exactly on the arc stroke
    (`|click_dist - r| ≤ EPSILON`): no command is committed; tool stays in
    `WaitingSide`.

17. **Degenerate source Line rejected**: source `Line::new(p, p)` (zero length),
    `on_pointer_down` in `WaitingSide` does not commit any command and does not
    panic. (The `Idle` pick step will generally not select a degenerate line
    since `entity_distance_to_point` degenerates to a point test, but the
    guard in the geometry path is required for robustness.)

18. **`preview()` returns offset entity**: after `on_pointer_move` to a clearly
    off-side cursor position, `preview()` returns a `Vec<Entity>` of length 1
    whose geometry matches the entity that `on_pointer_down` at the same
    position would commit (same field values within `EPSILON`).

19. **`preview()` returns empty on invalid cursor**: with `cursor` at the exact
    midpoint of the selected line (ambiguous side), `preview()` returns `vec![]`.

20. **`distance_mm` persists across commits**: set `tool.distance_mm = 3.0`,
    execute a full two-click offset (pick entity, pick side). After reset to
    `Idle`, `tool.distance_mm == 3.0`.

21. **Escape from `WaitingSide`**: call `on_key(Key::Escape, app)` while in
    `WaitingSide`; tool transitions to `Idle`; `preview()` returns empty;
    `distance_mm` is unchanged; `history.can_undo() == false`.

22. **Enter in `WaitingSide` commits at current cursor**: `on_pointer_move` to
    a valid off-side position; `on_key(Key::Enter, app)` commits the same entity
    that `on_pointer_down` at `cursor` would have committed.

23. **`cancel()` resets to `Idle`**: calling `cancel()` from `WaitingSide`
    resets the tool; `preview()` returns empty; `distance_mm` is unchanged.

24. **Object-safe**: `let _: Box<dyn Tool> = Box::new(OffsetTool::default())`
    compiles.

25. **Kernel-purity**: `grep -nE '^use (eframe|rfd)' src/tools/offset.rs`
    returns no matches.

26. `wc -l src/tools/offset.rs` reports ≤ 300.

27. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0 with no regressions to existing tests.

## Expected tests

All tests live in `#[cfg(test)] mod tests` inside `src/tools/offset.rs`.

- **Unit (AC 2)**: `offset_name_is_offset` —
  `assert_eq!(OffsetTool::default().name(), "OFFSET")`.

- **Unit (AC 3)**: `offset_default_distance_is_1mm` —
  `assert_eq!(OffsetTool::default().distance_mm, 1.0)`.

- **Unit (AC 4)**: `idle_pick_line_transitions_to_waiting_side` — build
  `Document` with one horizontal `Line`, call `on_pointer_down` near midpoint;
  assert `status_text()` contains `"side"` and `history.can_undo() == false`.

- **Unit (AC 5)**: `idle_pick_arc_transitions_to_waiting_side` — same with
  `Entity::Arc`.

- **Unit (AC 6)**: `idle_miss_stays_idle` — call `on_pointer_down` far from all
  entities; assert `preview().is_empty()`, `history.can_undo() == false`,
  `status_text()` contains `"entity"`.

- **Unit (AC 7)**: `idle_circle_ignored` — document with one `Circle`;
  `on_pointer_down` near it; assert tool stays in `Idle`.

- **Unit (AC 8)**: `side_pick_commits_create_line` — pick a line, then click
  off-side; assert `doc.entity_count() == 2`, `history.can_undo() == true`,
  `preview().is_empty()` (tool in `Idle`).

- **Unit (AC 9)**: `parallel_line_left_side` — horizontal line `(0,0)→(10,0)`,
  distance 5, click at `(5, 3)`; assert committed line has
  `p1 ≈ (0, 5)`, `p2 ≈ (10, 5)`.

- **Unit (AC 10)**: `parallel_line_right_side` — same line, click at `(5, -3)`;
  assert `p1 ≈ (0, -5)`, `p2 ≈ (10, -5)`.

- **Unit (AC 11)**: `side_pick_commits_create_arc` — pick an arc, click outside;
  assert `doc.entity_count() == 2`, `history.can_undo() == true`.

- **Unit (AC 12)**: `arc_outward_offset_geometry` — arc r=5, distance 2, click
  at dist 7 from center; assert committed arc has r≈7, same center, same angles.

- **Unit (AC 13)**: `arc_inward_offset_geometry` — arc r=5, distance 2, click
  at dist 3 from center; assert committed arc has r≈3.

- **Unit (AC 14)**: `arc_inward_degenerate_no_commit` — arc r=2, distance 3,
  click inside; assert `doc.entity_count() == 1`, `history.can_undo() == false`,
  tool stays in `WaitingSide` (inferred from non-empty `status_text` containing
  `"side"`).

- **Unit (AC 15)**: `ambiguous_line_side_no_commit` — click exactly on the
  line (construct `pos` so `signed ≈ 0`); assert no commit, tool stays in
  `WaitingSide`.

- **Unit (AC 16)**: `ambiguous_arc_side_no_commit` — click at distance equal
  to `a.r` from center; assert no commit.

- **Unit (AC 17)**: `degenerate_source_line_no_panic` — `WaitingSide` with
  `Entity::Line(Line::new(p, p))`; call `on_pointer_down` at any position;
  assert no panic, no commit.

- **Unit (AC 18)**: `preview_returns_offset_entity` — pick line, move cursor
  to clear off-side position; assert `preview().len() == 1`; the preview
  geometry matches the expected offset line within `EPSILON`.

- **Unit (AC 19)**: `preview_empty_on_ambiguous_cursor` — move cursor to exact
  midpoint of source line; assert `preview().is_empty()`.

- **Unit (AC 20)**: `distance_persists_after_commit` — set
  `tool.distance_mm = 3.0`, do a full two-click offset; assert
  `tool.distance_mm == 3.0`.

- **Unit (AC 21)**: `escape_from_waiting_side_resets_to_idle` — pick entity,
  send `Key::Escape`; assert `preview().is_empty()`, `distance_mm` unchanged.

- **Unit (AC 22)**: `enter_in_waiting_side_commits` — pick line, move cursor
  to clear off-side position, call `on_key(Key::Enter, app)`; assert
  `doc.entity_count() == 2`, same geometry as a pointer-down would produce.

- **Unit (AC 23)**: `cancel_resets_to_idle` — pick entity, call `cancel()`;
  assert `preview().is_empty()`, `distance_mm` unchanged.

- **Unit (AC 24)**: `object_safe` —
  `let _: Box<dyn Tool> = Box::new(OffsetTool::default())`.

- **Static check (AC 25)**: `grep -nE '^use (eframe|rfd)' src/tools/offset.rs`
  returns no matches.

- **Static check (AC 26)**: `wc -l src/tools/offset.rs` ≤ 300.

- **Build gate (AC 27)**: `cargo fmt --all -- --check &&
  cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

- **Manual smoke**: activate OFFSET (set `tool.distance_mm = 5.0` in a test
  harness or via Rust test). Draw a diagonal line; first click near the line;
  status bar changes to "OFFSET: Click side…"; second click to one side →
  a parallel line appears 5 mm away; Ctrl+Z removes it. Repeat with an arc:
  outward click → concentric arc with larger radius added.

## Open questions

*(none — demand is Ready)*

## Notes

- **`pick_closest` and `entity_distance_to_point` visibility**: both helpers
  live in `src/tools/select/hit.rs` as `pub(super)`. The implementer must
  choose one of: (a) relax their visibility to `pub(crate)` — preferred,
  minimal churn, single source of truth; (b) extract them to a new
  `src/tools/hit.rs` shared module; (c) inline the distance logic in
  `offset.rs`. Option (a) is recommended; the demand does not mandate the
  choice.

- **`PICK_THRESHOLD_MM = 5.0`** is the same constant used by `SelectTool`. If
  made `pub(crate)`, `OffsetTool` reuses it. Otherwise define a local
  `const PICK_THRESHOLD_MM: f64 = 5.0` in `offset.rs`.

- **Line direction guard**: `Line::direction()` returns `Option<Vec2>` and is
  `None` for degenerate segments (length ≤ EPSILON). The geometry path must
  handle this via the degenerate-source guard (AC#17) before calling
  `.unwrap()`.

- **Arc offset is angular-extent-preserving**: the `start_angle`, `end_angle`,
  and `ccw` fields are copied verbatim from the source arc. Only `r` changes.
  The result arc is concentric and co-angular with the source.

- **`distance_mm` and Phase-6 command-line wiring**: the field is `pub` so the
  Phase-6 command-line demand can write it directly via something like
  `if let Some(t) = tool_manager.active_as_mut::<OffsetTool>() { t.distance_mm = d; }`.
  The exact dispatch mechanism is defined by the command-line demand.

- **`on_key(Key::Enter, …)` in `WaitingSide`**: the implementation may delegate
  to `on_pointer_down(self.cursor, …)` internally, or share the geometry helper,
  to avoid duplicating logic.

- **State representation**: `WaitingSide` stores a clone of `Entity` to avoid
  holding a borrow on `Document` across calls. The `entity_index` is also
  stored so that a future demand that wants to highlight the source entity during
  the `WaitingSide` phase can read it.

- **ID note (for `demand-manager` and `project-manager`)**: the PLAN.md slot
  for LCV-054 was previously "Snap integration into all drawing tools" and is
  referenced by name in LCV-041's Out-of-scope note. Reassigning this slot to
  OffsetTool means the snap-integration feature needs a new demand ID. Additionally,
  `docs/product/README.md` listed "No … offset" as a v0.1.0 non-goal; that line
  should be removed or updated now that this demand is accepted. Both changes
  are `demand-manager` / `project-manager` scope, not `implementer-rust` scope.

- **Reference**: AutoCAD R14 `OFFSET` command: distance prompt → entity pick →
  side click → loops back to entity pick. This demand implements a single
  pass (pick → side → idle) which is the minimal useful subset.
