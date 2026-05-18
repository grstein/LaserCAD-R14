# LCV-026 — History stack (200-deep, undo/redo)

- **Status**: Done
- **Phase**: 2
- **Depends on**: LCV-022
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: implementer-rust (task #70); 2fd3960

## Problem

CAD operators trust Ctrl+Z. Without it, every accidental click — a wrong line, a misplaced circle, a delete of the wrong selection — is permanent, and operators stop trusting the tool. AGENTS.md fixes the depth at 200 commands (matching v1) and locks the rule that every entity mutation goes through `commit(cmd)`. This demand lands the data structure and the three methods (`commit` / `undo` / `redo`) that enforce that contract.

The `App` struct (Phase 3+) will own a `History` next to its `Document` and wire Ctrl+Z / Ctrl+Y to `undo` / `redo`. Tools (Phase 4) and the agent (Phase 7) construct `Box<dyn Command>` and call `App::commit`, which delegates here.

User outcome: the operator places 250 lines, decides they made a mistake on line 51, and presses Ctrl+Z. The most recent 200 actions undo cleanly; the first 50 are gone forever (the oldest 50 were evicted to honor the depth cap). Ctrl+Y replays. A new action after some undo's clears the redo chain — the classic single-branch history model that every CAD operator already understands.

## Scope

- New file `src/document/history.rs` defining:
  - `pub const HISTORY_DEPTH: usize = 200;` — matches AGENTS.md and v1.
  - `pub struct History { undo_stack: VecDeque<Box<dyn Command>>, redo_stack: Vec<Box<dyn Command>>, max_depth: usize }`.
    - Derives: `Debug` is **not** derived because `Box<dyn Command>` is not `Debug`. The implementer may add a manual `impl Debug for History` that prints `History { undo: <len>, redo: <len> }`. Optional; tests do not assert `Debug`.
    - No `Default` derive; provide `pub fn new() -> Self { Self { undo_stack: VecDeque::new(), redo_stack: Vec::new(), max_depth: HISTORY_DEPTH } }` and `impl Default for History { fn default() -> Self { Self::new() } }`.
    - `max_depth` is stored on the struct (rather than hard-coded in methods) so future tests / tools can construct a smaller-depth `History` via `pub fn with_depth(depth: usize) -> Self` (handy for AC 5's overflow test). `with_depth` is part of this demand's API.
  - Public methods:
    - `pub fn commit(&mut self, mut cmd: Box<dyn Command>, doc: &mut Document)`:
      1. Call `cmd.do_(doc)`.
      2. Push `cmd` onto `undo_stack` (`push_back`).
      3. If `undo_stack.len() > max_depth`, `pop_front()` and drop the oldest command.
      4. Clear `redo_stack` (committing new work invalidates any pending redo).
    - `pub fn undo(&mut self, doc: &mut Document) -> bool`:
      - If `undo_stack` is empty, return `false`.
      - `pop_back()` to get the most recent command, call `cmd.undo(doc)`, push it onto `redo_stack`. Return `true`.
    - `pub fn redo(&mut self, doc: &mut Document) -> bool`:
      - If `redo_stack` is empty, return `false`.
      - `pop()` (Vec's last) to get the next-to-redo command, call `cmd.do_(doc)`, push it onto `undo_stack`. Honor `max_depth` again here (drop oldest if overflowed — symmetric with `commit`). Return `true`.
    - `pub fn can_undo(&self) -> bool { !self.undo_stack.is_empty() }`.
    - `pub fn can_redo(&self) -> bool { !self.redo_stack.is_empty() }`.
- Update `src/document/mod.rs`:
  - Add `pub mod history;`.
  - Add `pub use history::{History, HISTORY_DEPTH};`.
- `src/document/history.rs` MUST NOT import `egui`, `eframe`, or `rfd`. The file stays ≤300 LOC. It may import from `crate::document::{Document, Command}` and `std::collections::VecDeque`.
- Per-module unit tests under `#[cfg(test)] mod tests` in `history.rs`. Tests use `NoOpCommand` (LCV-022) and `CreateLine` (LCV-023) — both are real commands at the time this demand lands. (LCV-023 is **not** a declared dependency because LCV-026's API can be tested with `NoOpCommand` alone; if the implementer wants integration coverage with a state-changing command, they import `CreateLine`. Cycle-free: LCV-023 ships before LCV-026 per the dependency graph.)

## Out of scope

- **Command grouping / coalescing** (merging consecutive `MoveEntities` calls into one history entry to keep the depth cap effective). Tracked as future work; not in Phase 2.
- **Branched history** (multiple redo paths). Single-branch is the v1 behavior and the AutoCAD R14 default.
- **Persistence of history across sessions**. Closing and reopening the document clears history. Autosave (LCV-059) snapshots the `Document`, not the history.
- **History introspection UI** ("History panel" listing the last N labels). The Edit menu (Phase 6) shows `Undo <label>` / `Redo <label>` via `History::peek` if needed; that helper can be added then.
- **Concurrent mutation** while a command is running. `commit` borrows `&mut self` and `&mut Document`; no thread-safety needed.
- **Validation that a command is undoable** (i.e., that the round-trip invariant holds). Reviewers gate-check the invariant per command in LCV-022 / LCV-023 / LCV-024 / LCV-025. The history stack trusts the commands it's handed.
- **App-side wiring** (binding Ctrl+Z / Ctrl+Y to `undo` / `redo`). Owned by LCV-070 (keyboard shortcuts).

## Acceptance criteria

1. `src/document/history.rs` exists and defines `pub const HISTORY_DEPTH: usize = 200;` and `pub struct History` with the methods listed in Scope. `History::default()` and `History::new()` both produce an empty history with `max_depth == HISTORY_DEPTH`. `History::with_depth(d)` produces an empty history with `max_depth == d`.
2. `can_undo()` returns `false` and `can_redo()` returns `false` on a freshly-constructed `History`.
3. **Commit calls do_ and pushes to undo stack**: with a fresh `Document::default()` and `History::new()`, `history.commit(Box::new(CreateLine::new(line_a)), &mut doc);` leaves `doc.entities.len() == 1` and `history.can_undo() == true`, `history.can_redo() == false`.
4. **Undo restores document and shifts to redo**: continuing from AC 3, `assert!(history.undo(&mut doc));` leaves `doc.entities.is_empty()`, `history.can_undo() == false`, `history.can_redo() == true`.
5. **Redo replays**: continuing from AC 4, `assert!(history.redo(&mut doc));` leaves `doc.entities.len() == 1` (same line restored), `history.can_undo() == true`, `history.can_redo() == false`.
6. **Undo on empty returns false**: with a fresh `History`, `history.undo(&mut doc) == false` and the document is unchanged. Same for `redo` on empty.
7. **New commit clears redo**: with a sequence `commit(A); undo; commit(B);` — after the second `commit`, `history.can_redo() == false`. Specifically: commit a `CreateLine(line_a)`, undo (so `redo_stack` has one item), then commit a `CreateLine(line_b)`. `history.can_redo()` is `false` and `doc.entities[0] == Entity::Line(line_b)`.
8. **Depth cap evicts oldest**: with `History::with_depth(3)`, commit four `NoOpCommand`s. After the fourth commit, `history.undo_stack.len() == 3` (the oldest was dropped) and three calls to `undo` return `true`; the fourth returns `false`. (Use a `pub(crate) fn undo_len(&self) -> usize` test-only accessor or expose `undo_stack.len()` via a `pub fn len(&self) -> usize` — implementer's choice, document in the file.)
9. **Default depth is 200**: `History::default().max_depth == HISTORY_DEPTH`. Implementer exposes `max_depth` via either a public field, a `pub fn max_depth(&self) -> usize` accessor, or `#[cfg(test)]` visibility — pick one and document.
10. **Commit / undo / redo on real state**: with `CreateLine` and `DeleteEntities` (both from LCV-023 / LCV-024), build a 2-line document via two `commit`s, then `delete entities[0]` via a third commit, then undo three times. After all three undos, `doc.entity_count() == 0` and `history.can_redo() == true` (three commands queued for redo). (This AC integrates with LCV-023 / LCV-024; if implemented before those land, substitute `NoOpCommand` and skip the document-state assertions, keeping the can_undo / can_redo counts. Project-manager schedules LCV-026 after LCV-023 / LCV-024 per the dependency graph.)
11. `src/document/mod.rs` re-exports `History` and `HISTORY_DEPTH`.
12. Kernel-purity: `grep -nE '^use (egui|eframe|rfd)' src/document/history.rs` returns no matches.
13. Size: `wc -l src/document/history.rs` reports `<= 300`.
14. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: test `history_constructors_match_depth_constant` — `assert_eq!(HISTORY_DEPTH, 200);` and asserts `History::new().max_depth() == 200`, `History::with_depth(7).max_depth() == 7`.
- **Unit (AC 2)**: test `fresh_history_cannot_undo_or_redo`.
- **Unit (AC 3)**: test `commit_runs_do_and_enables_undo`.
- **Unit (AC 4)**: test `undo_reverses_and_enables_redo`.
- **Unit (AC 5)**: test `redo_replays_and_re_enables_undo`.
- **Unit (AC 6)**: test `undo_and_redo_on_empty_return_false`.
- **Unit (AC 7)**: test `new_commit_after_undo_clears_redo_stack`.
- **Unit (AC 8)**: test `depth_cap_evicts_oldest` using `History::with_depth(3)` and four `NoOpCommand`s.
- **Unit (AC 9)**: test `default_history_depth_is_200`.
- **Unit (AC 10)**: test `commit_undo_redo_chain_three_commands` using `CreateLine` × 2 + `DeleteEntities` × 1 (skip if LCV-023 / LCV-024 not yet landed — see Notes).
- **Static check (AC 12)**: `grep -nE '^use (egui|eframe|rfd)' src/document/history.rs` returns no matches.
- **Size check (AC 13)**: `wc -l src/document/history.rs` reports `<= 300`.
- **Build gate (AC 14)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **`Box<dyn Command>` is necessary because `Command` is object-safe by design** (see LCV-022). Storing commands by value (`enum`) is not feasible: every future command type would have to be enumerated centrally, which violates KISS and prevents the agent registry (LCV-078) from adding commands at the boundary.
- **VecDeque for undo, Vec for redo**: undo is FIFO-evicted from the front when over depth and LIFO-popped from the back on `undo`, so `VecDeque` is the natural pick. Redo is pure LIFO (never depth-capped from the front — see "depth-cap symmetry" below), so `Vec` is sufficient. The asymmetry is intentional.
- **Depth-cap symmetry**: when `redo` pushes onto `undo_stack`, the cap is re-checked. This matters because a long sequence of commits followed by a long sequence of redos could otherwise re-grow `undo_stack` past `max_depth`. The expected tests do not exercise this exhaustively; the AC pins the symmetry behavior.
- **Why 200**: matches v1. AGENTS.md §"State and mutation" hard-codes it. Coalescing (e.g., merging consecutive moves into one history entry) is the right answer if 200 feels too small — that's a future demand.
- **No `peek` method in this demand**: the Edit menu (LCV-065) will want `Undo <label>` / `Redo <label>`. Adding `peek_undo_label()` / `peek_redo_label()` is a tiny extension, but it has no Phase-2 consumer. The implementer may add it preemptively only if it lands within the 300-LOC budget; if it does, document the addition.
- **No `clear()` method**: clearing history is "new document" semantics; that's owned by Phase-5 file I/O (LCV-062). The history struct is replaced wholesale rather than mutated.
- AGENTS.md §"State and mutation (hard contract)" mandates the `App::commit(Box<dyn Command>)` entry point. This demand provides the underlying `History::commit`; Phase 3's `App` (LCV-030) wires `App::commit` to call this.
- Reference: v1's `document/History.ts` used the same single-branch model with `undoStack` / `redoStack` and a depth cap; v2 keeps the shape.
