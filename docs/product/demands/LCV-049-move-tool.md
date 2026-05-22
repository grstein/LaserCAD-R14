# LCV-049 — MoveTool

- **Status**: Ready
- **Phase**: 4
- **Depends on**: LCV-041 (Done), LCV-024 (Done), LCV-042 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

After selecting entities with `SelectTool` (LCV-042), users cannot reposition
them without deleting and redrawing — a fundamental gap for any laser-cutting
layout task (aligning cut paths, spacing parts on the bed, adjusting geometry
relative to the origin).  `MoveEntities` (LCV-024) already exists as an
undoable command; this demand wires it to a two-click UI tool that follows
the classic AutoCAD R14 MOVE workflow: select entities, invoke MOVE, pick base
point, pick destination, entities translate.

## Scope

- `src/tools/move_.rs` — new file, `MoveTool` implementing `Tool`.
- `src/tools/tool.rs` — add one default method `fn take_successor(&mut self) -> Option<Box<dyn Tool>>` (returns `None`), enabling any tool to hand control to a successor after a pointer event.
- `src/tools/mod.rs` — declare `pub mod move_`, re-export `MoveTool`.
- `src/tools/manager.rs` — add `ToolManager::take_successor(&mut self) -> Option<Box<dyn Tool>>` forwarding method.
- `src/app.rs` (`App::update`) — after each call to `tool_manager.on_pointer_event(...)`, call `tool_manager.take_successor()` and, if `Some(t)`, call `tool_manager.set_tool(t)`.

## Out of scope

- Ortho lock on the delta vector (LCV-053).
- Snap integration for base or destination point (LCV-054).
- Interactive entity-picking inside MOVE: the selection must exist **before**
  the tool activates.
- Clearing the selection after a successful move (moved entities remain
  selected, matching AutoCAD R14 behavior).
- Numeric distance/angle entry via the command line.
- Copy-while-move (AutoCAD's `C` modifier).
- Toolbar button or menu item for MOVE.

## Acceptance criteria

1. `MoveTool::name()` returns exactly `"MOVE"`.

2. **`status_text()`** returns context-sensitive prompts:
   - `"MOVE: Click to set base point"` in `Idle` state.
   - `"MOVE: Click destination  |  Esc to cancel"` in `WaitingDest` state.

3. **First click — `on_pointer_down` in `Idle`**:
   - If `doc.selection.is_empty()`: no-op; state stays `Idle`; no command
     pushed to history.
   - If `doc.selection` is non-empty: snapshot each selected entity by
     collecting `doc.entities[i].clone()` for every `i` in
     `doc.selection.iter()` (snapshot stored on the tool); enter
     `WaitingDest { base: pos, cursor: pos }`.

4. **Cursor move — `on_pointer_move` in `WaitingDest`**:
   - Update the stored `cursor` to `pos`; preview geometry updates on the
     next frame.
   - `on_pointer_move` in `Idle` is a no-op.

5. **Second click — `on_pointer_down` in `WaitingDest`**:
   - Compute `delta = pos − base` (`Vec2`, mm).
   - If `delta.length() <= EPSILON`: discard click, stay in `WaitingDest`
     (zero-delta guard — degenerate move not committed).
   - Otherwise: collect `indices: Vec<usize>` from `doc.selection.iter()`
     (any order); commit exactly one
     `MoveEntities::new(indices, delta)` via `history.commit`; transition
     to `Idle`; set the pending-successor flag so that `take_successor()`
     returns `Some(Box::new(SelectTool::default()))` on the very next call.

6. **`preview()` in `WaitingDest`**:
   - For each entity snapshot cached in step AC#3, clone it, call
     `translate(cursor − base)` on the clone, and collect into a
     `Vec<Entity>`.
   - Returns that translated snapshot vec.  Does **not** access `doc` or
     mutate any state.

7. **`preview()` in `Idle`**: returns an empty `Vec<Entity>`.

8. **`on_pointer_up`**: always a no-op (for all states).

9. **Escape — `on_key(Key::Escape, app)`**: call `self.cancel()`.
   `cancel()` transitions the tool to `Idle`, clears the entity snapshot
   cache, and clears the pending-successor flag.  After `cancel()`,
   `preview()` is empty and the active tool in `app.tool_manager` is still
   `"MOVE"` (Escape does **not** switch tools).

10. **`take_successor` semantics** (single-shot): after the second valid
    click commits a `MoveEntities` command, the first call to
    `take_successor()` returns `Some(Box<dyn Tool>)` whose `name()` is
    `"Select"`.  All subsequent calls (until the next successful commit)
    return `None`.  The flag is consumed on read.

11. **`Tool` trait extension** — `fn take_successor(&mut self) -> Option<Box<dyn Tool>>` in `src/tools/tool.rs`:
    - Default impl returns `None`.
    - All existing tools (`LineTool`, `SelectTool`, `CircleTool`, `ArcTool`,
      `DeleteTool`) do **not** need to override it and must compile without
      modification.
    - The trait remains object-safe: `Box<dyn Tool>` compiles.

12. **`ToolManager::take_successor`**: a forwarding method that calls and
    returns `self.active.take_successor()`.

13. **App polling** (`App::update`): the `take_successor` poll must happen
    inside the same frame as the pointer event, after
    `tool_manager.on_pointer_event(...)` returns and before the next
    repaint.  Use the existing `mem::take`-and-restore pattern for
    `tool_manager` borrowing if needed.

14. `MoveTool` is object-safe: `let _: Box<dyn Tool> = Box::new(MoveTool::default())` compiles.

15. All of `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` pass with no regressions.

## Expected tests

- **Unit — `name_is_move`**: `assert_eq!(MoveTool::default().name(), "MOVE")`.

- **Unit — `idle_status_text`**: fresh `MoveTool` `status_text()` contains
  `"base point"`.

- **Unit — `waiting_dest_status_text`**: enter `WaitingDest` (select one
  entity, first click); assert `status_text()` contains `"destination"`.

- **Unit — `idle_empty_selection_no_commit`**: empty doc, empty selection,
  call `on_pointer_down`; assert `hist.can_undo()` is `false` and tool is
  in `Idle`.

- **Unit — `idle_nonzero_selection_enters_waiting`**: doc with one line,
  selection `{0}`, call `on_pointer_down(any_pos)`; assert tool is in
  `WaitingDest`.

- **Unit — `preview_empty_in_idle`**: `MoveTool::default().preview().is_empty()` is `true`.

- **Unit — `preview_ghost_in_waiting_dest`**: add one line
  `(0.0, 0.0) → (10.0, 0.0)` to doc, select `{0}`, first click at
  `Vec2::new(0.0, 0.0)`, move cursor to `Vec2::new(5.0, 5.0)` via
  `on_pointer_move`; assert `preview()` returns one `Entity::Line` with
  `p1.approx_eq(Vec2::new(5.0, 5.0), EPSILON)` and
  `p2.approx_eq(Vec2::new(15.0, 5.0), EPSILON)`.

- **Unit — `second_click_commits_move`**: doc with one line at
  `(0,0)→(10,0)`, select `{0}`, first click at `(0,0)`, second click at
  `(3,4)`; assert `hist.can_undo()` is `true`, entity endpoint
  `p1.approx_eq(Vec2::new(3.0, 4.0), EPSILON)`, tool is `Idle`.

- **Unit — `zero_delta_stays_waiting`**: first click at `(5,5)`, second
  click at `(5,5)`; assert no command committed (`hist.can_undo()` is
  `false`), tool stays `WaitingDest`.

- **Unit — `cancel_clears_preview`**: enter `WaitingDest`, call `cancel()`;
  assert `preview().is_empty()` and state is `Idle`.

- **Unit — `escape_key_stays_in_move_tool`**: build a full `App`, activate
  `MoveTool`, enter `WaitingDest`, call `on_key(Key::Escape, &mut app)`;
  assert `app.tool_manager.active_tool_name() == "MOVE"`.

- **Unit — `take_successor_after_commit_returns_select`**: first click then
  second valid click; call `tool.take_successor()`; assert `Some(t)` where
  `t.name() == "Select"`.

- **Unit — `take_successor_is_single_shot`**: call `take_successor()` twice
  after a commit; first call `Some`, second call `None`.

- **Unit — `take_successor_fresh_is_none`**: `MoveTool::default().take_successor()` returns `None`.

- **Unit — `object_safe`**: `let _: Box<dyn Tool> = Box::new(MoveTool::default())` compiles.

- **Unit — `undo_restores_position`**: commit a move via second click; call
  `hist.undo(&mut doc)`; assert entity coordinates match pre-move values
  within `EPSILON`.

- **Unit — `tool_trait_default_take_successor`**: create a `SelectTool`
  (which does not override `take_successor`); call `take_successor()`; assert
  `None` — confirms default impl and no regressions on existing tools.

## Open questions

*(none — demand is Ready)*

## Notes

- `MoveEntities::new(indices, delta)` is in `src/document/commands/edit.rs`.
  `delta` is a `Vec2` in mm.  `undo` translates by `-delta`; round-trip is
  bit-stable within `EPSILON`.
- **Entity snapshot cache**: captured inside `on_pointer_down` when
  transitioning to `WaitingDest`, where `&mut Document` is available.
  `preview()` operates on this cache — it does not touch `doc`.  A single
  `Vec<Entity>` field on `MoveTool` is sufficient.
- **`take_successor` polling in `App::update`**: add the poll immediately
  after the block that calls `tool_manager.on_pointer_event(...)`.  The
  existing `mem::take`-and-restore pattern (line ~182–186 of `src/app.rs`)
  is the precedent for borrowing `tool_manager` while passing `&mut self`.
  For the successor poll, a simple `if let Some(t) = self.tool_manager.take_successor() { self.tool_manager.set_tool(t); }` suffices.
- `doc.selection` is **not** modified by `MoveTool` on commit — selected
  indices remain valid post-move (entity positions change; indices do not).
- `EPSILON` lives in `src/geometry/mod.rs` (re-export from
  `src/geometry/epsilon.rs`).
- Per AGENTS.md purity rule: `src/document/*` and `src/geometry/*` must not
  import `egui`.  `MoveTool` lives in `src/tools/` and may import `egui`
  freely.
- File must stay ≤ 300 LOC (AGENTS.md hard cap).
