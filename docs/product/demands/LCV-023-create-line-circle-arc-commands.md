# LCV-023 — CreateLine / CreateCircle / CreateArc commands

- **Status**: Done
- **Phase**: 2
- **Depends on**: LCV-022
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: implementer-rust (task #75); e9f3f82

## Problem

The three primitive-drawing tools — `LineTool` (LCV-043), `CircleTool` (LCV-046), `ArcTool` (LCV-047) — share one shape: collect a small parameter set from pointer clicks, build a command, and commit it. The agent's CAD tool registry (LCV-078) needs to invoke the same commands from the LLM side. Without these concrete `Command` implementations, every drawing tool would invent its own mutation path, breaking AGENTS.md's "all mutation through `Command`" contract.

This demand ships exactly three "append one entity" commands, each carrying the captured index needed to undo cleanly.

User outcome: when the operator finishes drawing a line, the action is recorded in history; Ctrl+Z removes that exact line and leaves the rest of the drawing intact.

## Scope

- New file `src/document/commands/create.rs`. (To make `commands` a sub-folder, rename the existing `src/document/commands.rs` from LCV-022 into `src/document/commands/mod.rs` and move `Command` + `NoOpCommand` there. The implementer of this demand executes that move and updates the `pub mod commands;` declaration in `document/mod.rs` accordingly.)
- `commands/create.rs` defines:
  - `pub struct CreateLine { pub line: Line, captured_index: Option<usize> }`.
  - `pub struct CreateCircle { pub circle: Circle, captured_index: Option<usize> }`.
  - `pub struct CreateArc { pub arc: Arc, captured_index: Option<usize> }`.
  - Each derives `Debug`. None derive `Copy` (history holds `Box<dyn Command>` and ownership is straightforward).
  - Public constructors: `pub fn new(line: Line) -> Self { Self { line, captured_index: None } }` (and equivalents for circle / arc). The `captured_index` field is module-private and starts `None`.
  - `impl Command for CreateLine`:
    - `fn do_(&mut self, doc: &mut Document) { doc.entities.push(Entity::Line(self.line)); self.captured_index = Some(doc.entities.len() - 1); }`.
    - `fn undo(&mut self, doc: &mut Document) { if let Some(i) = self.captured_index.take() { doc.entities.remove(i); } }`.
    - `fn label(&self) -> &str { "Create Line" }`.
  - `impl Command for CreateCircle` — same shape, label `"Create Circle"`, variant `Entity::Circle`.
  - `impl Command for CreateArc` — same shape, label `"Create Arc"`, variant `Entity::Arc`.
- Update `src/document/commands/mod.rs`:
  - Add `pub mod create;`.
  - Add `pub use create::{CreateLine, CreateCircle, CreateArc};`.
- `src/document/mod.rs`: no API change beyond the existing `pub use commands::*` re-exports.
- `commands/create.rs` MUST NOT import `egui`, `eframe`, or `rfd`. The file stays ≤300 LOC.
- Per-module unit tests under `#[cfg(test)] mod tests` in `create.rs`. No new file under `tests/`.

## Out of scope

- Tool wiring (constructing a `CreateLine` from pointer events) — owned by **LCV-043** / LCV-046 / LCV-047 (Phase 4).
- Multi-entity / batch create commands (e.g., create-polyline-as-N-lines) — deferred; PolylineTool (LCV-044) decides whether it emits one batch command or N individual ones.
- Input validation (degenerate lines, zero-radius circles, zero-sweep arcs). The kernel value types accept these without panic; rejecting them is the tool / agent layer's responsibility, not the command's.
- Entity IDs (a stable identifier surviving across history operations). Commands address entities by index in `Document::entities`; this is sufficient for Phase 2.
- Selection side-effects (e.g., "newly-created entity becomes selected"). If a tool wants that, it commits a `SelectionCommand` (LCV-027) after the create. The create commands themselves leave selection untouched.
- Error returns. `do_` and `undo` never fail at runtime; constructing the command is the only validation point.
- Coalescing repeated creates into a single history entry.

## Acceptance criteria

1. `src/document/commands/create.rs` exists. `src/document/commands.rs` has been moved to `src/document/commands/mod.rs` (preserving `Command` and `NoOpCommand`), and `commands/mod.rs` declares `pub mod create;` and re-exports `CreateLine`, `CreateCircle`, `CreateArc`.
2. `CreateLine`, `CreateCircle`, `CreateArc` exist with `pub` constructors `CreateLine::new(line)`, `CreateCircle::new(circle)`, `CreateArc::new(arc)`. Each starts with `captured_index: None`.
3. **CreateLine round-trip**: starting from `Document::default()`, `let mut cmd = CreateLine::new(Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0))); cmd.do_(&mut doc);` leaves `doc.entities.len() == 1` and `doc.entities[0] == Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0)))`. After `cmd.undo(&mut doc);`, `doc.entities.is_empty()` and `doc.entity_count() == 0`.
4. **CreateCircle round-trip**: same pattern with `Circle::new(Vec2::new(5.0, 5.0), 3.0)`. After `do_`, the document has exactly that circle; after `undo`, the document is empty.
5. **CreateArc round-trip**: same pattern with a quarter arc `Arc::new(Vec2::default(), 1.0, 0.0, std::f64::consts::FRAC_PI_2, true)`. After `do_`, the document has exactly that arc; after `undo`, the document is empty.
6. **Two creates in a row**: `cmd_a = CreateLine::new(line_a); cmd_b = CreateLine::new(line_b); cmd_a.do_(&mut doc); cmd_b.do_(&mut doc);` leaves `doc.entities.len() == 2`, with `entities[0] == Entity::Line(line_a)` and `entities[1] == Entity::Line(line_b)`. Calling `cmd_b.undo(&mut doc); cmd_a.undo(&mut doc);` (LIFO) restores the empty document.
7. **Labels are exact**: `CreateLine::new(...).label() == "Create Line"`, `CreateCircle::new(...).label() == "Create Circle"`, `CreateArc::new(...).label() == "Create Arc"`.
8. **Object-safe**: `let _: Box<dyn Command> = Box::new(CreateLine::new(Line::new(Vec2::default(), Vec2::default()))); let _: Box<dyn Command> = Box::new(CreateCircle::new(Circle::new(Vec2::default(), 1.0))); let _: Box<dyn Command> = Box::new(CreateArc::new(Arc::new(Vec2::default(), 1.0, 0.0, 1.0, true)));` all compile.
9. Kernel-purity: `grep -nE '^use (egui|eframe|rfd)' src/document/commands/create.rs src/document/commands/mod.rs` returns no matches.
10. Size: `wc -l src/document/commands/create.rs` reports `<= 300`; `wc -l src/document/commands/mod.rs` reports `<= 300`.
11. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: test `create_module_re_exports_three_commands` — `use crate::document::commands::{CreateLine, CreateCircle, CreateArc};` compiles.
- **Unit (AC 2)**: test `create_constructors_capture_index_none` — inspects the public surface by constructing and immediately running `do_` then asserting the post-state.
- **Unit (AC 3)**: test `create_line_roundtrip`.
- **Unit (AC 4)**: test `create_circle_roundtrip`.
- **Unit (AC 5)**: test `create_arc_roundtrip`.
- **Unit (AC 6)**: test `two_creates_then_lifo_undo` — two `CreateLine`s, undo in reverse order, document empty.
- **Unit (AC 7)**: test `create_labels_exact` — covers all three exact label strings.
- **Unit (AC 8)**: test `create_commands_are_object_safe` — `Box<dyn Command>` construction for each.
- **Static check (AC 9)**: `grep -nE '^use (egui|eframe|rfd)' src/document/commands/create.rs src/document/commands/mod.rs` returns no matches.
- **Size check (AC 10)**: `wc -l src/document/commands/create.rs` and `wc -l src/document/commands/mod.rs` both report `<= 300`.
- **Build gate (AC 11)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **File move**: this demand turns `src/document/commands.rs` (from LCV-022) into a folder with a `mod.rs`. The implementer does this move atomically — `git mv src/document/commands.rs src/document/commands/mod.rs` then add `create.rs`. The `Command` trait and `NoOpCommand` stay in `mod.rs`. Re-exports in `src/document/mod.rs` stay unchanged.
- `captured_index` is `Option<usize>` so an un-`do_`ed command (or an already-undone one) cannot accidentally remove the wrong entity. `take()` in `undo` makes a double-undo a no-op rather than a corruption.
- The captured index works because `Document::entities` is a `Vec` and `do_` always pushes to the end. If a future demand inserts entities at arbitrary positions, that demand updates the capture logic.
- The commands intentionally do not touch `Document::selection` — see Out of scope. Tools that want "create then select" issue two commands.
- Reference: v1's `commands/createLine.ts` (and siblings) captured `entityId` rather than index; v2 uses index because no entity-ID concept exists yet. Behavior is equivalent for Phase 2 where indices are stable across the single command.
- LCV-023 pairs with LCV-026 (history stack): the captured-index pattern enables the redo path to push at the same end-of-vec position the do_ originally used (because undo removed it, redo's do_ pushes a new one — the index is captured fresh on each `do_`).
