# LCV-027 — Selection model + SelectionCommand

- **Status**: Done
- **Phase**: 2
- **Depends on**: LCV-022
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: implementer-rust (task #73); d7429cf

## Problem

The Phase-4 `SelectTool` (LCV-042) needs to track which entities are highlighted; the Phase-3 selection renderer (LCV-036) draws them; the Phase-4 `MoveTool` / `DeleteTool` / `TrimTool` consume the set as their target list; the agent (LCV-078) reads it. Right now `Document::selection` is the placeholder type LCV-021 declared with no fields. This demand makes `Selection` concrete and ships a `SelectionCommand` so selection changes participate in the undo stack (selecting two entities and then pressing Ctrl+Z restores the previous selection, matching AutoCAD R14).

User outcome: the operator window-picks five lines, decides they meant to pick four, presses Ctrl+Z, and gets back the previous selection state. The next click goes through a `SelectionCommand` again, so the model stays consistent.

## Scope

- Replace the placeholder `Selection` body in `src/document/state.rs` (from LCV-021) with the real definition. Either:
  - Move `Selection` to a new file `src/document/selection.rs` and re-export from `mod.rs` (preferred — keeps `state.rs` focused on `Document`).
  - Or expand `Selection` in place in `state.rs` if the total LOC stays comfortably under 300.
- `src/document/selection.rs` (preferred location) defines:
  - `pub struct Selection { indices: HashSet<usize> }`. (`HashSet<usize>` over `Vec<bool>` because membership tests and add/remove are O(1) and the typical selection size is tiny.)
    - Derives: `Debug`, `Clone`, `Default`, `PartialEq`. `Eq` is fine because `usize` is `Eq`; `Hash` is not derived (no consumer needs hash-of-selection).
  - Public methods:
    - `pub fn is_selected(&self, idx: usize) -> bool`.
    - `pub fn iter(&self) -> impl Iterator<Item = usize> + '_` — yields each selected index. Order is **not guaranteed** (it's a `HashSet`); consumers that need order sort it themselves.
    - `pub fn len(&self) -> usize`.
    - `pub fn is_empty(&self) -> bool`.
    - `pub fn clear(&mut self)`.
    - `pub fn add(&mut self, idx: usize)` — idempotent (adding an already-selected index is a no-op).
    - `pub fn remove(&mut self, idx: usize)` — idempotent (removing a non-selected index is a no-op).
    - `pub fn set(&mut self, indices: impl IntoIterator<Item = usize>)` — replaces the current set with the supplied indices (clear + extend).
    - `pub fn contains_all(&self, indices: &[usize]) -> bool` — convenience for tests and the renderer. Optional; add only if it fits within the LOC budget and a test exercises it. If omitted, callers use `iter` + match.
- `src/document/commands/select.rs` defines:
  - `pub struct SelectionCommand { new_indices: HashSet<usize>, captured_old: HashSet<usize> }`.
    - Constructor `pub fn new(new_indices: impl IntoIterator<Item = usize>) -> Self { Self { new_indices: new_indices.into_iter().collect(), captured_old: HashSet::new() } }`.
    - `impl Command for SelectionCommand`:
      - `do_(&mut self, doc)`: capture `captured_old = doc.selection.indices.clone()`, then `doc.selection.indices = self.new_indices.clone()`. (Cloning is acceptable because selection sizes are tiny.)
      - `undo(&mut self, doc)`: `doc.selection.indices = self.captured_old.clone()`. Don't `take()` — a redo needs to capture again from the post-undo state, and the simplest correct behavior is to always re-capture on `do_`. (Implementer's choice: either re-capture on every `do_` or `take` and re-capture. The AC pins the round-trip; the implementation is open.)
      - `label(&self) -> &str { "Select" }`.
    - To allow `SelectionCommand` to write `doc.selection.indices`, `Selection` exposes a `pub(crate) fn replace_indices(&mut self, new: HashSet<usize>)` setter, or the `indices` field is `pub(crate)`. Either choice keeps `indices` out of the public surface so external callers must go through `add`/`remove`/`set`/`SelectionCommand`.
- Update `src/document/mod.rs`:
  - Replace the placeholder `pub use state::{Document, Selection};` chain so `Selection` is exported from `selection.rs` if that's where it lives (`pub mod selection; pub use selection::Selection;`).
- Update `src/document/commands/mod.rs`:
  - Add `pub mod select;` and `pub use select::SelectionCommand;`.
- Files MUST NOT import `egui`, `eframe`, or `rfd`. Each file stays ≤300 LOC.
- Per-module unit tests under `#[cfg(test)] mod tests` in both `selection.rs` and `commands/select.rs`.

## Out of scope

- **SelectTool** (window-pick, crossing-box, single-click pick) — owned by **LCV-042** (Phase 4).
- **Selection-highlight rendering** (drawing the dashed bounding box, the colored entity highlight) — owned by **LCV-036** (Phase 3).
- **Selection filters by entity type** ("select all lines"). The renderer / tool layer filters; the selection set is a plain index set.
- **Selection persistence across file open/close**. Selection is in-memory only; loading a document resets it.
- **Multi-document selection** (selection across multiple open drawings). v2 has one active document.
- **Selection consistency after `DeleteEntities`**. When entities are deleted, the indices in `Selection` may now refer to different entities (the vec shifted). LCV-052 (DeleteTool) is responsible for issuing a `SelectionCommand` to clean up, just like LCV-024 documented. This demand does **not** auto-clean.
- **Named selection sets** (AutoCAD's "named selection set" feature). Not in v2.
- **Selection ordering** (a "pick order" that some commands rely on). Phase 4 demands can layer this if needed; LCV-027 ships an unordered set.

## Acceptance criteria

1. `src/document/selection.rs` exists and defines `pub struct Selection` whose internal storage is a `HashSet<usize>`. Derives include `Debug`, `Clone`, `Default`, `PartialEq`.
2. The `Selection` placeholder declared in LCV-021 is replaced: `Selection::default()` constructs an empty selection (`is_empty() == true`, `len() == 0`, `iter().count() == 0`).
3. `add`, `remove`, `is_selected` round-trip: `sel.add(3); assert!(sel.is_selected(3)); sel.remove(3); assert!(!sel.is_selected(3));`.
4. `add` is idempotent: `sel.add(3); sel.add(3); assert_eq!(sel.len(), 1);`.
5. `remove` of a non-selected index is a no-op: starting from an empty `Selection`, `sel.remove(99); assert!(sel.is_empty());`.
6. `set` replaces the current set: `sel.add(1); sel.add(2); sel.set([3, 4, 5]); assert_eq!(sel.len(), 3); assert!(!sel.is_selected(1)); assert!(sel.is_selected(4));`.
7. `iter` yields each selected index exactly once. (Order is not asserted; tests collect into a sorted `Vec<usize>` and compare.)
8. `clear` empties the selection: `sel.add(7); sel.clear(); assert!(sel.is_empty());`.
9. **SelectionCommand round-trip on empty start**: with `Document::default()`, `let mut cmd = SelectionCommand::new([2, 4, 6]); cmd.do_(&mut doc);` makes `doc.selection.iter()` yield exactly `{2, 4, 6}` (set-equal). After `cmd.undo(&mut doc);`, `doc.selection.is_empty()` is `true`.
10. **SelectionCommand round-trip on non-empty start**: with a `Document` whose selection starts as `{0, 1}`, `let mut cmd = SelectionCommand::new([5, 7]); cmd.do_(&mut doc);` makes the selection equal `{5, 7}`. After `undo`, the selection is back to `{0, 1}` (set-equal).
11. **Empty SelectionCommand clears**: `let mut cmd = SelectionCommand::new(Vec::<usize>::new()); cmd.do_(&mut doc);` clears the selection; `undo` restores the prior state.
12. **SelectionCommand label**: `SelectionCommand::new([0]).label() == "Select"`.
13. **Object-safe**: `let _: Box<dyn Command> = Box::new(SelectionCommand::new([0, 1]));` compiles.
14. `src/document/mod.rs` re-exports `Selection`; `src/document/commands/mod.rs` re-exports `SelectionCommand`.
15. Kernel-purity: `grep -nE '^use (egui|eframe|rfd)' src/document/selection.rs src/document/commands/select.rs` returns no matches.
16. Size: `wc -l src/document/selection.rs` reports `<= 300`; `wc -l src/document/commands/select.rs` reports `<= 300`.
17. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: test `selection_struct_constructs_via_default` — `let _ = Selection::default();` and verifies `Debug` (format string is non-empty).
- **Unit (AC 2)**: test `selection_default_is_empty`.
- **Unit (AC 3)**: test `selection_add_remove_is_selected_roundtrip`.
- **Unit (AC 4)**: test `selection_add_is_idempotent`.
- **Unit (AC 5)**: test `selection_remove_non_member_is_noop`.
- **Unit (AC 6)**: test `selection_set_replaces_existing`.
- **Unit (AC 7)**: test `selection_iter_yields_each_member_once` — collects to a sorted `Vec<usize>` and compares to a fixture.
- **Unit (AC 8)**: test `selection_clear_empties`.
- **Unit (AC 9)**: test `selection_command_roundtrip_from_empty_start`.
- **Unit (AC 10)**: test `selection_command_roundtrip_from_non_empty_start`.
- **Unit (AC 11)**: test `empty_selection_command_clears_then_undo_restores`.
- **Unit (AC 12)**: test `selection_command_label_is_select`.
- **Unit (AC 13)**: test `selection_command_is_object_safe`.
- **Static check (AC 15)**: `grep -nE '^use (egui|eframe|rfd)' src/document/selection.rs src/document/commands/select.rs` returns no matches.
- **Size check (AC 16)**: `wc -l src/document/selection.rs` and `wc -l src/document/commands/select.rs` both report `<= 300`.
- **Build gate (AC 17)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **HashSet over Vec<bool>**: a `Vec<bool>` would be tighter for densely-selected documents (every entity flagged), but Phase-2 selection is sparse (typical: a handful of entities out of hundreds). `HashSet<usize>` has cheaper add/remove and a smaller default footprint. If a future profiling pass shows hot-spot overhead, swap is one-line internal.
- **Index stability**: `Selection` stores `usize` indices into `Document::entities`. After a `DeleteEntities` operation those indices shift. LCV-024 documented that the Phase-4 `DeleteTool` is responsible for issuing a follow-up `SelectionCommand` to clear or remap. This demand does **not** auto-clean; doing so would couple `Selection` to the entity vec mutation and break the "one responsibility per command" rule.
- **`SelectionCommand` clones the set**: selection sizes are small (typically <100 indices); cloning is O(n) and negligible. Avoiding the clone would require taking ownership of `new_indices` on `do_` and reconstructing on `redo`, which is more code for no benefit.
- **`do_` always re-captures**: this makes `do_` idempotent and safe to call after a `take`-style undo. The simpler implementation, but slightly less efficient than capturing once on construction. The AC pins behavior, not implementation.
- **Encapsulating `indices`**: `pub(crate)` on the field, or a `pub(crate) fn replace_indices` setter, keeps the public surface narrow. External callers must use `add`/`remove`/`set` or commit a `SelectionCommand`. The contract: outside `src/document/commands/select.rs`, no one mutates `selection.indices` directly.
- **Why not put `SelectionCommand` in `selection.rs`?** Keeping commands under `commands/` keeps the trait-implementation surface in one folder, which makes the agent registry (LCV-078) easier to write.
- AGENTS.md §"State and mutation (hard contract)" lists "Selection is part of `Document`, mutated via `SelectionCommand`s." This demand satisfies the second half of that contract.
- Reference: v1's `document/Selection.ts` used a `Set<EntityId>` (string IDs); v2 uses `HashSet<usize>` because no IDs exist yet. The Phase-4 tools translate pointer events into index sets; the contract remains the same.
