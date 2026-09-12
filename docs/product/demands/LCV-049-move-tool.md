# LCV-049 — MoveTool — move selected entities

- **Status**: Done
- **Phase**: 4
- **Depends on**: LCV-041 (Done), LCV-024 (Done), LCV-042 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: 498eb0d — feat(LCV-049): MoveTool — two-click move selected entities

## Problem

After selecting entities with `SelectTool` (LCV-042), the user cannot
reposition them without deleting and redrawing — a fundamental gap for any
laser-cutting layout task (aligning cut paths, spacing parts on the bed,
adjusting geometry relative to the origin).  `MoveEntities` (LCV-024) already
exists as an undoable command; this demand wires it to a two-click UI tool
that follows the classic AutoCAD R14 MOVE workflow: select entities, invoke
MOVE, pick base point, pick destination, entities translate.

## Scope

- `src/tools/move_.rs` — new file, `MoveTool` implementing `Tool`.
- `src/tools/tool.rs` — add one default method
  `fn take_successor(&mut self) -> Option<Box<dyn Tool>>` (default returns
  `None`), enabling a tool to hand control to a successor after committing.
  Must preserve object-safety of `Box<dyn Tool>`.
- `src/tools/mod.rs` — declare `pub mod move_`, re-export `MoveTool`.
- `src/tools/manager.rs` — add
  `pub fn take_successor(&mut self) -> Option<Box<dyn Tool>>` forwarding
  method.
- `src/app.rs` (`App::update`) — inside the existing `mem::take`–restore
  block for `tool_manager`, poll `tm.take_successor()` immediately after
  `tm.on_pointer_event(...)` and before restoring; if `Some(t)`, call
  `tm.set_tool(t)`.

## Out of scope

- Ortho lock on the delta vector (LCV-053).
- Snap integration for base or destination point (LCV-054).
- Interactive entity-picking inside MOVE: the selection must exist **before**
  the tool activates.
- Clearing the selection after a successful move (moved entities remain
  selected, matching AutoCAD R14 behavior).
- Numeric distance/angle entry via the command line.
- Copy-while-move (AutoCAD's `C` modifier).
- Toolbar button or keyboard shortcut for MOVE (LCV-066, LCV-070).

## Acceptance criteria

1. `MoveTool::name()` returns exactly `"MOVE"`.

2. **`status_text()`** returns context-sensitive prompts based on the tool's
   internal state and cached `has_selection` flag:
   - `"MOVE: Select objects first"` when in `Idle` state **and**
     `has_selection == false`.
   - `"MOVE: Click to set base point"` when in `Idle` state **and**
     `has_selection == true`.
   - `"MOVE: Click destination  |  Esc to cancel"` when in `WaitingDest`
     state (regardless of `has_selection`).

3. **`on_pointer_move(pos, doc)`**:
   - In both `Idle` and `WaitingDest`: updates
     `self.has_selection = !doc.selection.is_empty()`.
   - In `Idle`: no other state change; no command committed.
   - In `WaitingDest`: additionally updates the stored `cursor` field to
     `pos`; preview geometry updates on the next frame.

4. **First click — `on_pointer_down(pos, _shift, doc, history)` in `Idle`**:
   - Updates `self.has_selection = !doc.selection.is_empty()`.
   - If `doc.selection.is_empty()`: no-op; state stays `Idle`; no command
     pushed to history.
   - If `doc.selection` is non-empty: for every index `i` in
     `doc.selection.iter()`, copy `doc.entities[i]` (Entity is `Copy`) into
     `self.snapshot`; enter `WaitingDest { base: pos, cursor: pos }`.

5. **Second click — `on_pointer_down(pos, _shift, doc, history)` in
   `WaitingDest`**:
   - Compute `delta = pos − base` (`Vec2`, mm).
   - If `delta.length() <= EPSILON`: discard; stay in `WaitingDest`
     (zero-delta guard — degenerate move not committed).
   - Otherwise: collect `indices: Vec<usize>` from `doc.selection.iter()`
     (any order); call
     `history.commit(Box::new(MoveEntities::new(indices, delta)), doc)`;
     clear `self.snapshot`; transition to `Idle`; set
     `self.pending_successor = true`.

6. **`preview()` in `WaitingDest`**: for each `Entity` in `self.snapshot`,
   copy it, call `translate(cursor − base)` on the copy, collect into a
   `Vec<Entity>`.  Returns that translated vec.  Does **not** access `doc`
   or mutate any state.

7. **`preview()` in `Idle`**: returns an empty `Vec<Entity>`.

8. **`on_pointer_up(_pos, _shift, _doc, _history)`**: always a no-op in all
   states.

9. **Escape — `on_key(Key::Escape, app)`**: calls `self.cancel()`.
   `cancel()` transitions the tool to `Idle`, clears `self.snapshot`, clears
   `self.pending_successor`.  After `cancel()`, `preview()` is empty and the
   active tool in `app.tool_manager` is still `"MOVE"` (Escape does **not**
   switch tools).

10. **`take_successor` semantics — single-shot**: immediately after a
    successful second-click commit (AC#5), the first call to
    `take_successor()` returns `Some(Box<dyn Tool>)` whose `name()` is
    `"Select"`.  All subsequent calls until the next successful commit return
    `None`.  The flag is consumed on the first read.

11. **`Tool` trait extension** — `fn take_successor(&mut self) -> Option<Box<dyn Tool>>`
    added to `src/tools/tool.rs`:
    - Default impl returns `None`.
    - All existing tools (`LineTool`, `SelectTool`, `CircleTool`, `ArcTool`,
      `DeleteTool`, `PolylineTool`, `RectTool`) do **not** need to override
      it and must compile without modification.
    - The trait remains object-safe: `let _: Box<dyn Tool> = Box::new(MoveTool::default())` compiles.

12. **`ToolManager::take_successor`**:
    `pub fn take_successor(&mut self) -> Option<Box<dyn Tool>>` forwarding to
    `self.active.take_successor()`.

13. **App polling** (`App::update`): the successor poll is inside the same
    `std::mem::take`–restore block as `on_pointer_event`:

    ```rust
    let mut tm = std::mem::take(&mut self.tool_manager);
    tm.on_pointer_event(&event, &mut self.document, &mut self.history);
    if let Some(t) = tm.take_successor() {
        tm.set_tool(t);
    }
    self.tool_manager = tm;
    ```

    No `unsafe` introduced.  The poll happens before the next repaint.

14. `MoveTool` is object-safe: `let _: Box<dyn Tool> = Box::new(MoveTool::default())` compiles.

15. All of `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` pass with no regressions.

## Expected tests

- **Unit — `name_is_move`**: `assert_eq!(MoveTool::default().name(), "MOVE")`.

- **Unit — `idle_no_selection_status_text`**: fresh `MoveTool::default()`,
  assert `status_text()` contains `"Select objects first"`.

- **Unit — `idle_with_selection_status_text`**: doc with one line, selection
  `{0}`, call `tool.on_pointer_move(any_pos, &mut doc)`; assert
  `status_text()` contains `"base point"`.

- **Unit — `waiting_dest_status_text`**: doc with one line, selection `{0}`,
  first click at any pos; assert `status_text()` contains `"destination"`.

- **Unit — `idle_empty_selection_no_commit`**: empty doc, empty selection,
  call `on_pointer_down(any_pos, false, &mut doc, &mut hist)`; assert
  `hist.can_undo()` is `false` and `preview().is_empty()`.

- **Unit — `idle_nonzero_selection_enters_waiting`**: doc with one line,
  selection `{0}`, call `on_pointer_down(any_pos, false, &mut doc, &mut hist)`;
  assert `status_text()` contains `"destination"`.

- **Unit — `preview_empty_in_idle`**:
  `MoveTool::default().preview().is_empty()` is `true`.

- **Unit — `preview_ghost_in_waiting_dest`**: doc with
  `Entity::Line((0.0,0.0)→(10.0,0.0))`, selection `{0}`, first click at
  `(0.0, 0.0)`, call `on_pointer_move(Vec2::new(5.0, 5.0), &mut doc)`;
  assert `preview()` has one `Entity::Line` with
  `p1.approx_eq(Vec2::new(5.0, 5.0), EPSILON)` and
  `p2.approx_eq(Vec2::new(15.0, 5.0), EPSILON)`.

- **Unit — `second_click_commits_move`**: doc with one line at
  `(0,0)→(10,0)`, selection `{0}`, first click at `(0,0)`, second click at
  `(3,4)`; assert `hist.can_undo()` is `true`, entity
  `p1.approx_eq(Vec2::new(3.0, 4.0), EPSILON)`, `preview().is_empty()`.

- **Unit — `zero_delta_stays_waiting`**: first click at `(5,5)`, second
  click at `(5,5)`; assert `hist.can_undo()` is `false`, tool stays in
  `WaitingDest` (status_text still contains `"destination"`).

- **Unit — `cancel_clears_preview`**: enter `WaitingDest`, call `cancel()`;
  assert `preview().is_empty()` and `status_text()` no longer contains
  `"destination"`.

- **Unit — `escape_key_stays_in_move_tool`**: build a full `App`, activate
  `MoveTool`, enter `WaitingDest`, call `on_key(Key::Escape, &mut app)`;
  assert `app.tool_manager.active_tool_name() == "MOVE"`.

- **Unit — `take_successor_after_commit_returns_select`**: after a valid
  second click commit, call `tool.take_successor()`; assert `Some(t)` where
  `t.name() == "Select"`.

- **Unit — `take_successor_is_single_shot`**: call `take_successor()` twice
  after a commit; first call `Some`, second call `None`.

- **Unit — `take_successor_fresh_is_none`**:
  `MoveTool::default().take_successor()` returns `None`.

- **Unit — `object_safe`**:
  `let _: Box<dyn Tool> = Box::new(MoveTool::default())` compiles.

- **Unit — `undo_restores_position`**: commit a move via second click; call
  `hist.undo(&mut doc)`; assert entity coordinates match pre-move values
  within `EPSILON`.

- **Unit — `tool_trait_default_take_successor`**: create a `SelectTool`
  (which does not override `take_successor`); call `take_successor()`; assert
  `None` — confirms default impl and no regression on existing tools.

## Open questions

*(none — demand is Ready)*

## Notes

- **Method signatures**: pointer event methods follow the actual LCV-041
  `Tool` trait:
  `on_pointer_down(pos: Vec2, shift: bool, doc: &mut Document, history: &mut History)`,
  `on_pointer_move(pos: Vec2, doc: &mut Document)`,
  `on_pointer_up(pos: Vec2, shift: bool, doc: &mut Document, history: &mut History)`,
  `on_key(key: egui::Key, app: &mut App)`.
  The `shift` parameter is ignored (`_shift`) — MOVE has no shift-modifier
  behaviour.

- **`has_selection` initialization**: `MoveTool::default()` initialises
  `has_selection = false`, so the first status text is
  `"MOVE: Select objects first"`.  `on_pointer_move` fires on every cursor
  movement over the viewport and corrects the flag within that frame.  If
  the user activates MOVE and immediately clicks without first moving the
  mouse, `on_pointer_down` also updates `has_selection` before acting, so
  the click logic is always correct even if the status text lagged by one
  frame.

- **Entity snapshot**: `Entity` is `Copy` (derives `Copy, Clone` from
  LCV-020).  The snapshot is `Vec<Entity>`; no explicit `.clone()` calls
  are needed — `doc.entities[i]` copies by value.  `preview()` works off
  `self.snapshot` and never reads `doc`.

- **`take_successor` object safety**: `fn take_successor(&mut self) -> Option<Box<dyn Tool>>`
  is object-safe: no generic type parameters, no bare `Self` return (only
  `Box<dyn Tool>`), `&mut self` receiver.  All existing tools compile
  against the trait with no override.

- **`take_successor` polling in `App::update`**: add the three-line poll
  immediately after the existing `tm.on_pointer_event(...)` call, before
  `self.tool_manager = tm`.  `src/app.rs` is already at 373 LOC (over the
  300-line AGENTS.md cap — pre-existing violation).  The three new lines do
  not worsen this; a future `architect`-routed demand can split the file.

- **`ToolManager::on_pointer_event` call site**: the actual manager signature
  is `on_pointer_event(&mut self, event: &PointerEvent, doc: &mut Document, history: &mut History)`.
  The `mem::take`–restore pattern (already in `App::update` from LCV-041)
  handles the borrow split.

- **Selection not cleared on commit**: `doc.selection` is **not** modified by
  `MoveTool`.  Selected indices remain valid post-move (entity positions
  change; indices do not).  AutoCAD R14 leaves the selection set intact after
  MOVE; v2 follows suit.

- **`MoveEntities::new(indices, delta)`** is in
  `src/document/commands/edit.rs`.  `delta` is a `Vec2` in mm.  `undo`
  translates by `−delta`; round-trip is exact within `EPSILON`.

- **`EPSILON`**: re-exported from `src/geometry/epsilon.rs` via
  `src/geometry/mod.rs`.

- **Purity rule**: `src/document/*` and `src/geometry/*` must not import
  `egui`.  `MoveTool` lives in `src/tools/` and may import `egui` freely.

- **File size**: `src/tools/move_.rs` must stay ≤ 300 LOC (AGENTS.md hard
  cap).
