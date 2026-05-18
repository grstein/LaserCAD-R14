# LCV-022 — Command trait + do/undo semantics

- **Status**: Done
- **Phase**: 2
- **Depends on**: LCV-021
- **Suggested agent**: architect
- **Suggested model**: opus
- **Implementation**: implementer-rust (task #71); af28f9b

## Problem

AGENTS.md §"State and mutation (hard contract)" requires that every mutation of a `Document` goes through a `Command` trait and the history stack. Without that trait existing yet, every later demand — `CreateLine` (LCV-023), `DeleteEntities` (LCV-024), `TrimEntities` (LCV-025), the history stack (LCV-026), `SelectionCommand` (LCV-027), every drawing tool in Phase 4, every agent action in Phase 7 — would either invent its own ad-hoc mutation interface or mutate `Document` directly and violate the contract.

This demand introduces the trait, the round-trip invariant, and the smallest possible `Command` implementation (`NoOpCommand`) so downstream demands have a tested compile-time pattern to copy.

User outcome: every action the operator can perform — drawing a line, deleting a circle, trimming a curve, selecting a group — is reversible by Ctrl+Z because every action is a `Command`. Without this trait, undo is impossible.

## Scope

- New file `src/document/commands.rs` defining:
  - `pub trait Command { fn do_(&mut self, doc: &mut Document); fn undo(&mut self, doc: &mut Document); fn label(&self) -> &str; }`.
    - The method is named `do_` (with trailing underscore) because `do` is a reserved Rust keyword. Document this naming choice in the trait's doc comment so reviewers and future demands match the convention.
    - All three methods take `&mut self` so commands can capture and restore state needed for undo (e.g., the index at which an entity was inserted, the original bytes of a trimmed segment).
    - The trait is **object-safe** — no generic methods, no `Self: Sized` bounds, no associated types. The history stack (LCV-026) stores `Box<dyn Command>`.
    - `label` returns `&str` (not `String`) so commands can return string literals without allocating.
  - `pub struct NoOpCommand;` with `impl Command for NoOpCommand`:
    - `do_(&mut self, _doc: &mut Document) {}` — no work.
    - `undo(&mut self, _doc: &mut Document) {}` — no work.
    - `label(&self) -> &str { "No-op" }`.
    - Purpose: a minimal test fixture for the history stack (LCV-026) and a copy-paste template for future command authors.
  - Trait-level doc comment stating the **round-trip invariant**: `do_` followed by `undo` must restore the `Document` to a state byte-equivalent to its pre-`do_` state. The Phase-2 commands honor this; reviewers gate-check on it.
- Update `src/document/mod.rs`:
  - Add `pub mod commands;`.
  - Add `pub use commands::{Command, NoOpCommand};`.
- `src/document/commands.rs` MUST NOT import `egui`, `eframe`, or `rfd`. The file stays ≤300 LOC. It may import `crate::document::Document`.
- Per-module unit tests under `#[cfg(test)] mod tests` in `commands.rs`. No new file under `tests/`.

## Out of scope

- Real command implementations (`CreateLine`, `CreateCircle`, `CreateArc`, `DeleteEntities`, `MoveEntities`, `TrimEntity`, `ExtendEntity`, `SelectionCommand`) — owned by LCV-023, LCV-024, LCV-025, LCV-027.
- The history stack (`commit`, undo/redo, depth cap) — owned by **LCV-026**.
- Command grouping / coalescing (e.g., merging consecutive `MoveEntities` into one history entry) — deferred. No current consumer.
- Multi-command transactions (begin / commit / rollback) — deferred. The history stack treats one `Box<dyn Command>` as one undoable unit.
- Error returns from `do_` / `undo`. A `Command` is constructed only after the tool / agent has validated its inputs; runtime failure of `do_` is a panic-class invariant violation, not a recoverable error. (If a future demand needs fallible commands, it can wrap them in a `Result`-returning facade; not in this demand.)
- Serialization of commands (recording / replaying scripts) — not in v2.
- Naming conflicts: this demand picks `do_`; alternatives like `apply` were considered. Once the trait ships, downstream demands match this name.

## Acceptance criteria

1. `src/document/commands.rs` exists and defines `pub trait Command` with exactly the three methods: `fn do_(&mut self, doc: &mut Document)`, `fn undo(&mut self, doc: &mut Document)`, `fn label(&self) -> &str`.
2. The trait is **object-safe**: the test file constructs `let _: Box<dyn Command> = Box::new(NoOpCommand);` and compiles.
3. `NoOpCommand` exists, implements `Command`, and its `label()` returns the exact string `"No-op"`.
4. **Round-trip on an empty document**: build a `Document::default()`, build a `NoOpCommand`, call `cmd.do_(&mut doc)`, then `cmd.undo(&mut doc)`. `doc.entity_count() == 0` and `doc.bounds() == None` afterward.
5. **Round-trip on a one-entity document**: build a `Document` with one `Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0)))` pushed into `entities`. Apply a `NoOpCommand` do-then-undo. The document still contains exactly that one entity, and `entities[0] == Entity::Line(Line::new(...))` (bit-equal via `PartialEq`).
6. `label()` returns a non-empty string for `NoOpCommand` (`!cmd.label().is_empty()`).
7. `src/document/mod.rs` re-exports `Command` and `NoOpCommand`.
8. Kernel-purity: `grep -nE '^use (egui|eframe|rfd)' src/document/commands.rs` returns no matches.
9. Size: `wc -l src/document/commands.rs` reports `<= 300`.
10. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: test `command_trait_has_three_methods` — compile-only test that calls `do_`, `undo`, `label` on `NoOpCommand` through the `Command` trait, forcing the compiler to verify the surface.
- **Unit (AC 2)**: test `command_is_object_safe` — `let _: Box<dyn Command> = Box::new(NoOpCommand);` and `let _: &dyn Command = &NoOpCommand;`.
- **Unit (AC 3)**: test `noop_label_exact` — `assert_eq!(NoOpCommand.label(), "No-op");`.
- **Unit (AC 4)**: test `noop_roundtrip_on_empty_document`.
- **Unit (AC 5)**: test `noop_roundtrip_preserves_existing_entity` — uses `PartialEq` on `Entity` to assert bit-equality.
- **Unit (AC 6)**: test `noop_label_non_empty`.
- **Static check (AC 8)**: `grep -nE '^use (egui|eframe|rfd)' src/document/commands.rs` returns no matches.
- **Size check (AC 9)**: `wc -l src/document/commands.rs` reports `<= 300`.
- **Build gate (AC 10)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **Method naming**: `do_` with trailing underscore is the chosen convention. Rust keyword `do` cannot be a method name without `r#do`, which is uglier in call sites than `do_`. The alternative `apply` was considered but rejected because the AGENTS.md hard contract explicitly references "`do_` / `undo` semantics", and matching the documented term avoids a layer of translation between docs and code.
- **Object safety** is load-bearing: LCV-026's history stack holds `VecDeque<Box<dyn Command>>`. If a future change adds a generic method to `Command`, it breaks the history stack. Reviewers should flag any such addition.
- **Round-trip invariant** is the contract every concrete command (LCV-023+) must honor. Reviewers verify with a `do_ → undo` test on at least one representative input per command.
- `&mut self` on all three methods is deliberate: commands capture pre-state during `do_` and consume it during `undo`. A future "redo from scratch" capability is not in this demand; redo (LCV-026) calls `do_` a second time, which is why state capture must be idempotent across repeated `do_/undo` cycles.
- `label() -> &str` returns a borrowed string so commands like `"Create Line"` can be literals. The label is used by the history-aware UI (Phase 6 — `Edit > Undo Create Line`).
- Reference: v1's `document/Command.ts` exposed `execute()` / `undo()` / `label`. v2 renames `execute` to `do_` to match the AGENTS.md contract verbatim.
