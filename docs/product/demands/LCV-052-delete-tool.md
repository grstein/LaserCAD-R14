# LCV-052 — DeleteTool / Delete key

- **Status**: Done
- **Implementation**: c976c36 — feat(LCV-052): DeleteTool (ERASE) + Delete key in SelectTool
- **Phase**: 4
- **Depends on**: LCV-041 (Done), LCV-024 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

The user has no way to remove entities from the drawing short of starting over.
`DeleteEntities` (LCV-024) and `SelectionCommand` (LCV-027) are already
implemented, but there is no tool that wires them to user input.  The
AutoCAD R14 workflow is: pick entities with `SELECT`, press **Delete** (or
invoke `ERASE`) → entities are gone, undo-able with a single Ctrl+Z.
Currently neither the Delete key nor a dedicated ERASE tool exists in the
application, blocking every laser-cutting workflow that requires correcting
mistakes.

## Scope

- `src/tools/delete.rs` — new file, `DeleteTool` implementing `Tool`.
- `src/tools/mod.rs` — register `DeleteTool` (pub module + re-export).
- `src/app.rs` (`App::update`) — route `Key::Delete` and `Key::Backspace`
  through `tool_manager.handle_key` using the same `mem::take` pattern
  already used for `Key::Escape`.
- `src/tools/select/mod.rs` (`SelectTool::on_key`) — add a
  `Key::Delete | Key::Backspace` arm that deletes the current selection and
  clears it (primary UX path: user stays in SelectTool and presses Delete).

## Out of scope

- Atomic / grouped undo: delete and selection-clear are two separate history
  entries (two Ctrl+Z presses to fully reverse).  A grouped-undo facility is
  not part of v0.1.0.
- Any interactive "pick entities to delete" loop à la classic AutoCAD.
  `DeleteTool` operates on the already-established selection only.
- Toolbar button or menu item for ERASE.
- Confirmation dialog.

## Acceptance criteria

1. `DeleteTool::name()` returns exactly `"ERASE"`.

2. `DeleteTool` is stateless (no internal state machine fields).  `cancel()`
   and `preview()` are trivial no-ops.

3. **`on_pointer_down`** — when `doc.selection` is non-empty, commits
   `DeleteEntities::new(sorted_desc_indices)` then
   `SelectionCommand::new(vec![])` to `history` (two separate commits, in
   that order); when `doc.selection.is_empty()`, does nothing.
   `on_pointer_move` and `on_pointer_up` are no-ops.

4. **`on_key`** — for `egui::Key::Delete` and `egui::Key::Backspace`: same
   delete-then-clear-selection behavior as AC#3, reading selection from
   `app.document.selection` and writing through `app.history`.  For
   `egui::Key::Escape`: no-op.  All other keys: no-op.

5. **After a delete**: `doc.selection.is_empty()` is `true`.
   `doc.entities` no longer contains the deleted entities.
   The deleted entities are recoverable by calling `history.undo(&mut doc)`
   twice (first restores the selection, second restores the entities).

6. **Empty selection guard**: `on_pointer_down` and `on_key(Delete)` on an
   empty selection must not push any command onto the history stack.
   `history.can_undo()` remains unchanged.

7. **`App::update`** — a `Key::Delete` or `Key::Backspace` press anywhere in
   the viewport is forwarded to `tool_manager.handle_key(key, self)` via the
   same `mem::take`-and-restore pattern used for Escape (line ~183 of
   `src/app.rs`).

8. **`SelectTool::on_key`** gains a `Key::Delete | Key::Backspace` arm: if
   `app.document.selection` is non-empty, commits `DeleteEntities` then
   `SelectionCommand::new(vec![])` in that order; if selection is empty, does
   nothing.  This is the primary "select-then-Delete-key" workflow.

9. `DeleteTool` is object-safe and can be boxed as `Box<dyn Tool>`.

10. All of `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, and
    `cargo test --all` pass with no regressions.

## Expected tests

- **Unit — `DeleteTool::name_is_erase`**: `assert_eq!(DeleteTool.name(), "ERASE")`.

- **Unit — `delete_tool_is_stateless`**: `DeleteTool` implements `Default`;
  `preview()` returns empty vec; `cancel()` leaves tool callable.

- **Unit — `delete_tool_is_object_safe`**: `let _: Box<dyn Tool> = Box::new(DeleteTool)`.

- **Unit — `on_pointer_down_deletes_selection`**: build a 3-entity `Document`,
  set selection to `{0, 2}`, call `on_pointer_down`; assert `entity_count()`
  drops to 1, `selection.is_empty()`, and `history.can_undo()` is true.

- **Unit — `on_pointer_down_noop_on_empty_selection`**: empty document,
  empty selection, call `on_pointer_down`; assert history unchanged and
  entity count unchanged.

- **Unit — `on_key_delete_deletes_selection`**: same setup as
  `on_pointer_down_deletes_selection` but driven through `on_key(Key::Delete, app)`.

- **Unit — `on_key_backspace_deletes_selection`**: same with `Key::Backspace`.

- **Unit — `on_key_empty_selection_no_commit`**: call `on_key(Key::Delete)`
  when selection is empty; assert `history.can_undo()` is `false`.

- **Unit — `delete_undo_restores_entities`**: delete entities through
  `on_key`; undo twice; assert `entity_count()` and entity values match
  original state.

- **Unit — `select_tool_delete_key_deletes_selection`**: construct a
  `SelectTool`, add entities to doc, set selection, call
  `on_key(Key::Delete, app)`; assert entities removed and selection cleared.

- **Unit — `select_tool_delete_key_empty_noop`**: `SelectTool::on_key(Delete)`
  with empty selection must not push to history.

- **Compile-check — `app_routes_delete_key`**: adding a
  `ctx.input(|i| i.key_pressed(egui::Key::Delete))` guard that calls
  `tm.handle_key(egui::Key::Delete, self)` in `App::update` compiles cleanly.

## Open questions

*(none — demand is Ready)*

## Notes

- `DeleteEntities::new` accepts a `Vec<usize>` and sorts indices descending
  internally — the caller need not pre-sort.  See
  `src/document/commands/edit.rs` for the existing implementation.
- `SelectionCommand::new(vec![])` is the established pattern for clearing
  selection (see `SelectTool::on_key` Escape arm in
  `src/tools/select/mod.rs` line 128–134).
- The `mem::take`-and-restore pattern for key dispatch in `App::update`
  exists at line ~182–186 of `src/app.rs` and must be replicated for
  `Key::Delete` / `Key::Backspace`.
- `DeleteTool` is retrieved by the command line via its `name()` string
  `"ERASE"` once the command-line dispatch demand (Phase 5) lands.  The
  `name()` string is therefore load-bearing; do not change it.
- Per AGENTS.md purity rule: `src/document/*` must not import `egui`.
  `DeleteTool` lives in `src/tools/` and may import `egui` freely.
- File must stay ≤ 300 LOC (AGENTS.md hard cap).
