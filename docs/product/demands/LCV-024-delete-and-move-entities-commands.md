# LCV-024 — DeleteEntities + MoveEntities commands

- **Status**: Done
- **Phase**: 2
- **Depends on**: LCV-022
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: implementer-rust (task #74); c8f4e1e

## Problem

After the operator has drawn primitives, the two most common edits are "remove what I selected" (Delete / `Erase` in AutoCAD) and "shift what I selected by this offset" (`Move`). These actions back the `DeleteTool` (LCV-052) and `MoveTool` (LCV-049), and the agent (LCV-078) must invoke them too. Without these commands, the only mutation pathway is "create more", which is not a CAD application.

A working `MoveEntities` also forces the document model to grow a small `Entity::translate(delta)` method — this demand owns that change.

User outcome: with three lines selected, the operator presses Delete; all three disappear and Ctrl+Z restores them at their exact positions. With one circle selected, the operator drags by (10, 0); the circle moves; Ctrl+Z snaps it back.

## Scope

- New file `src/document/commands/edit.rs` defining:
  - `pub struct DeleteEntities { pub indices: Vec<usize>, captured: Vec<(usize, Entity)> }`.
    - Public constructor `pub fn new(indices: Vec<usize>) -> Self { Self { indices, captured: Vec::new() } }`.
    - `do_(&mut self, doc)`: sort `indices` descending (so each `Vec::remove` doesn't invalidate later indices), then for each `i` in the sorted order, push `(i, doc.entities[i])` onto `captured` and call `doc.entities.remove(i)`. The result: `captured` holds the removed entities **in the order they were removed** (highest index first).
    - `undo(&mut self, doc)`: iterate `captured` in reverse (lowest original index first) and `doc.entities.insert(i, entity)` to put each entity back at its original index. Clear `captured` afterward (so a double-undo is a no-op).
    - `label(&self) -> &str { "Delete Entities" }`.
    - Empty `indices` is a valid no-op: `do_` removes nothing, `undo` restores nothing, the document is unchanged.
    - Out-of-range indices in `do_` are an invariant violation; the implementer may `debug_assert!(i < doc.entities.len())` but must not panic in release. Documented as the caller's responsibility (tools build `indices` from selection state, which always references valid entities).
  - `pub struct MoveEntities { pub indices: Vec<usize>, pub delta: Vec2 }`.
    - Public constructor `pub fn new(indices: Vec<usize>, delta: Vec2) -> Self`.
    - `do_(&mut self, doc)`: for each `i` in `indices`, call `doc.entities[i].translate(self.delta)`.
    - `undo(&mut self, doc)`: for each `i` in `indices`, call `doc.entities[i].translate(-self.delta)`. (Pure translation is exactly invertible at floating-point precision because addition is commutative for finite f64; the round-trip test asserts within `EPSILON`.)
    - `label(&self) -> &str { "Move Entities" }`.
    - Duplicate indices in `indices`: documented as caller's responsibility to deduplicate; if present, the entity is translated twice (which double-moves it). The Phase-4 `MoveTool` builds `indices` from the selection set (`Selection::iter`), which is naturally unique.
- Update `src/document/entity.rs` (extending LCV-020):
  - Add `pub fn translate(&mut self, delta: Vec2)` on `Entity`. Dispatches to each variant:
    - `Entity::Line(l) => { l.p1 = l.p1 + delta; l.p2 = l.p2 + delta; }` (or equivalent in-place mutation; the kernel `Line` is a value type with `pub` fields).
    - `Entity::Circle(c) => { c.center = c.center + delta; }`.
    - `Entity::Arc(a) => { a.center = a.center + delta; }`.
  - This translate method is part of LCV-024's scope, not a separate demand, because no other consumer needs it yet and the cost is ~10 LOC.
- Update `src/document/commands/mod.rs`:
  - Add `pub mod edit;`.
  - Add `pub use edit::{DeleteEntities, MoveEntities};`.
- `commands/edit.rs` MUST NOT import `egui`, `eframe`, or `rfd`. The file stays ≤300 LOC.
- Per-module unit tests under `#[cfg(test)] mod tests` in `edit.rs` and (for `Entity::translate`) in `entity.rs`.

## Out of scope

- Trim / Extend commands — owned by **LCV-025**.
- Copy / rotate / scale / mirror commands — deferred to a future demand (not in Phase 2's keystone set).
- Tool wiring (DeleteTool, MoveTool) — owned by LCV-049 / LCV-052 (Phase 4).
- Selection updates as a side-effect of delete (e.g., "after delete, selection is cleared"). The Phase-4 tool issues a follow-up `SelectionCommand`. The delete command itself leaves `Document::selection` untouched.
- Entity ID stability across delete-undo cycles. Indices may shift; the captured-index approach handles single delete-undo correctly. A future demand introduces stable IDs if needed.
- Snapping the move delta to grid / ortho. The tool layer is responsible for producing a clean delta; the command is a dumb translation.
- Move of zero entities or with zero delta. These are no-ops by definition; tests cover the empty-indices case.

## Acceptance criteria

1. `src/document/commands/edit.rs` exists and defines `pub struct DeleteEntities` and `pub struct MoveEntities` with the constructors and method signatures described in Scope.
2. `commands/mod.rs` re-exports `DeleteEntities` and `MoveEntities`.
3. `Entity::translate(&mut self, delta: Vec2)` exists in `src/document/entity.rs` and updates the variant's coordinate state (line endpoints, circle center, arc center).
4. **Delete round-trip on a 3-entity document**: start with `entities = [Entity::Line(line_a), Entity::Circle(circle_b), Entity::Arc(arc_c)]`. `let mut cmd = DeleteEntities::new(vec![0, 1, 2]); cmd.do_(&mut doc);` leaves `doc.entity_count() == 0`. After `cmd.undo(&mut doc);` the document equals the starting state — `entities[0] == Entity::Line(line_a)`, `entities[1] == Entity::Circle(circle_b)`, `entities[2] == Entity::Arc(arc_c)` (bit-equal via `PartialEq`).
5. **Delete of non-contiguous indices**: with 5 entities (indices 0..4), `DeleteEntities::new(vec![1, 3])` `do_` leaves indices 0, 2, 4 in positions 0, 1, 2 of the resulting vec; `undo` restores all five entities at their original indices in the original order.
6. **Empty-indices delete is a no-op**: `DeleteEntities::new(Vec::new())` `do_` followed by `undo` leaves the document unchanged (any starting state, tested with a 2-entity document).
7. **Move round-trip on a line**: start with `entities = [Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0)))]`. `let mut cmd = MoveEntities::new(vec![0], Vec2::new(3.0, 4.0)); cmd.do_(&mut doc);` makes `entities[0] == Entity::Line(Line::new(Vec2::new(3.0, 4.0), Vec2::new(13.0, 4.0)))` (within `EPSILON`). After `cmd.undo(&mut doc);` the line is back at its starting endpoints (within `EPSILON`).
8. **Move applies to multiple entities**: with two entities at known positions, `MoveEntities::new(vec![0, 1], Vec2::new(5.0, 0.0))` shifts both by `(5, 0)`; `undo` restores both.
9. **Move with empty indices is a no-op**: `MoveEntities::new(Vec::new(), Vec2::new(99.0, 99.0))` `do_` and `undo` leave the document unchanged.
10. **Labels are exact**: `DeleteEntities::new(vec![0]).label() == "Delete Entities"`; `MoveEntities::new(vec![0], Vec2::default()).label() == "Move Entities"`.
11. **Object-safe**: `let _: Box<dyn Command> = Box::new(DeleteEntities::new(vec![0])); let _: Box<dyn Command> = Box::new(MoveEntities::new(vec![0], Vec2::default()));` both compile.
12. Kernel-purity: `grep -nE '^use (egui|eframe|rfd)' src/document/commands/edit.rs` returns no matches.
13. Size: `wc -l src/document/commands/edit.rs` reports `<= 300`.
14. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: test `edit_module_defines_delete_and_move` — instantiates each via `new(...)`.
- **Unit (AC 2)**: test `commands_re_exports_delete_and_move`.
- **Unit (AC 3, in entity.rs)**: test `entity_translate_line_circle_arc` — translates one of each and asserts new coordinates within `EPSILON`.
- **Unit (AC 4)**: test `delete_all_three_then_undo_restores_order`.
- **Unit (AC 5)**: test `delete_non_contiguous_indices_then_undo`.
- **Unit (AC 6)**: test `delete_empty_indices_is_noop`.
- **Unit (AC 7)**: test `move_line_roundtrip`.
- **Unit (AC 8)**: test `move_multiple_entities_roundtrip`.
- **Unit (AC 9)**: test `move_empty_indices_is_noop`.
- **Unit (AC 10)**: test `edit_labels_exact`.
- **Unit (AC 11)**: test `edit_commands_are_object_safe`.
- **Static check (AC 12)**: `grep -nE '^use (egui|eframe|rfd)' src/document/commands/edit.rs` returns no matches.
- **Size check (AC 13)**: `wc -l src/document/commands/edit.rs` reports `<= 300`.
- **Build gate (AC 14)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- The "sort descending then `Vec::remove`" pattern in `do_` is the canonical idiom for batch-removal-with-undo-by-original-index. Documenting it here means LCV-025 (TrimEntities, which replaces in place) does not have to relitigate the approach.
- `captured: Vec<(usize, Entity)>` is intentionally `Vec` rather than `HashMap<usize, Entity>` because (a) the ordering of removals matters for the descending-sort invariant, and (b) the count is `O(selection size)` which is small.
- `undo` iterating `captured` in reverse means re-inserting in ascending original-index order, which is the right order to use `Vec::insert(i, e)` without shifting positions of subsequent inserts. (Insert at index 1, then at index 3, etc.; if we did it the other way, index 3 would be wrong after inserting at index 1.)
- `Entity::translate` taking `&mut self` and mutating in place avoids unnecessary clones. The kernel value types are `Copy`, so the move-and-update pattern is also cheap; `&mut` is chosen for clarity.
- Negation of `Vec2`: `MoveEntities::undo` uses `-self.delta`. This relies on `Vec2` implementing `Neg` (LCV-010 ships this — confirm in code). If `Neg` is not available, the implementer constructs `Vec2::new(-self.delta.x, -self.delta.y)` manually; the AC is the round-trip behavior, not the syntax.
- AGENTS.md §"State and mutation (hard contract)" lists `Document::selection` as mutated only via `SelectionCommand`s. This demand does **not** touch selection, even though deleting an entity logically invalidates indices in the current selection. The Phase-4 `DeleteTool` issues both a `DeleteEntities` and a `SelectionCommand` to keep the model consistent.
- Reference: v1's `commands/deleteEntities.ts` used the same descending-sort idiom; v2 keeps the pattern verbatim.
