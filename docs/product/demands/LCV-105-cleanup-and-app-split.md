# LCV-105 — Cleanup: orphan files, Select-All through a Command, `app.rs` split, MODULE retirement

- **Status**: Ready
- **Phase**: 10
- **Depends on**: LCV-100, LCV-101, LCV-102, LCV-103, LCV-104 (all must be Done — this demand moves the code they edit)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

Marco 0 exists to make "Done" true. Five small items stand between the repo and
that claim, and each one is a rule the project already wrote down and then broke:

- Two files are compiled by nobody. `src/tools/move_tool.rs` (59 lines) and
  `src/app/tests.rs` (133 lines, landed with the LCV-053 merge `d8d7a01`) are
  declared by no `mod` statement. `src/app/tests.rs` has **never** compiled and
  duplicates the inline test module in `src/app.rs` — a reviewer reading it
  would believe assertions are running that are not.
- `src/ui/menubar.rs:124` (`do_select_all`) writes `app.document.selection.set(…)`
  directly, bypassing `SelectionCommand`. That violates the AGENTS.md hard
  contract ("Selection is part of `Document`, mutated via `SelectionCommand`s")
  and means Edit > Select All cannot be undone — the operator selects 400
  entities by accident and has no way back.
- `SelectTool` commits an empty `SelectionCommand` on every Escape
  (`src/tools/select/mod.rs:129-135`), even when nothing is selected. Escape is
  the most-pressed key in a CAD session, so the undo stack fills with no-ops and
  Ctrl+Z stops feeling like undo.
- `src/app.rs` is 482 implementation lines against a 300-line hard cap
  (AGENTS.md §"Module tree"). It is the single file every Marco 0 demand edits,
  and it is the reason those edits keep colliding.
- `pub const MODULE` placeholders from LCV-001 still sit in `io/`, `agent/`,
  `tools/` and `util/`, and `tests/skeleton.rs` still asserts on them. They were
  meant to retire "as each module's owning demand lands"; every one of those
  demands has landed.

## Scope

### 1. Delete the orphan files

- `git rm src/tools/move_tool.rs` — the live tool is `src/tools/move_.rs`
  (declared `pub mod move_;`, re-exported as `MoveTool`).
- `git rm src/app/tests.rs` — deleting is the default. Its only test not already
  present in the live inline module is `app_default_ortho_is_false`, and that
  assertion already runs in `src/app.rs` (`assert!(!app.ortho_enabled)` inside
  the default-state tests). If, on inspection, any assertion in the file covers
  behaviour the live suite does not, that assertion moves into the live test
  module instead of being lost.

### 2. Select All through the command stack

- `do_select_all` builds `SelectionCommand::new((0..n).collect::<Vec<usize>>())`
  and commits it through the history stack, so Ctrl+Z restores the previous
  selection.
- On an empty document (`entity_count() == 0`) it commits nothing.

### 3. No empty selection commits from Escape

- `SelectTool::on_key(Key::Escape, …)` cancels the in-progress state
  unconditionally, and commits the clearing `SelectionCommand` **only** when
  `!app.document.selection.is_empty()`.

### 4. Split `src/app.rs`

Per ADR 0002 §"The 300-LOC cap and `src/app.rs`". `src/app.rs` becomes
`src/app/mod.rs`, matching the repo's existing directory-module convention:

| file | contents |
|---|---|
| `src/app/mod.rs` | module docs, `struct App`, `impl Default`, `impl App` (`new`, `default`, `commit`, `action_*`, `mark_clean`, `sync_dirty`), `update_ui` as a ~20-line orchestrator, `impl eframe::App`, the re-exports |
| `src/app/input.rs` | the `wants_keyboard_input` gate and all key/text routing (created by LCV-103; moves unchanged) |
| `src/app/viewport.rs` | the `CentralPanel` body: camera sync, painter setup, render calls, pointer events, wheel zoom, middle-drag pan, plus `handle_wheel_zoom` / `handle_pan` / `handle_zoom_extents` |
| `src/app/panels.rs` | menubar, statusbar, command line, toolbar, agent panel, and the three dialogs |
| `src/app/autosave.rs` | `AUTOSAVE_DEBOUNCE`, `autosave_due`, the flush check |
| `src/app/ortho.rs`, `snap.rs`, `agent_poll.rs` | unchanged |

`src/app/mod.rs` must re-export `suppress_snap_if_disabled`,
`handle_wheel_zoom`, `handle_pan`, `handle_zoom_extents`, `apply_ortho`,
`resolve_snap`, `poll_agent_rx` (and `autosave_due`, once LCV-102 adds it) so
the `lasercad::app::*` paths used by `tests/lcv070.rs` keep resolving. No
deep-path imports from outside the module.

### 5. Retire the MODULE placeholders

- Remove `pub const MODULE` from `src/io/mod.rs:9`, `src/agent/mod.rs:13`,
  `src/tools/mod.rs:41`, `src/util/mod.rs:6` (the full list — confirm with
  `grep -rn "pub const MODULE" src/`).
- `src/util/` contains **nothing but** that constant and is imported by no code
  (`grep -rn "util" src/ tests/` finds only `src/lib.rs:16` and
  `tests/skeleton.rs`). Delete `src/util/mod.rs` and drop `pub mod util;` from
  `src/lib.rs`; git history keeps it, and the module gets re-created when a real
  utility needs a home. If a future demand has landed code in `src/util/` before
  this one runs, keep the module and witness it with that item instead.
- Rewrite `tests/skeleton.rs::module_tree_is_wired` to witness every remaining
  module with a real public item, e.g. `io::load_autosave`, `agent::AgentPanelMsg`
  (or `agent::tool_definitions`), `tools::ToolManager`, alongside the existing
  `app::App`, `document::SCHEMA_VERSION`, `geometry::EPSILON`, `render::Camera`,
  `text::CAP_HEIGHT_HERSHEY`, `ui::CANVAS_BG`.
- Fix the stale module doc in `src/lib.rs:4-5`, which lists
  `agent::classifier` (a file that does not exist) as a kernel module, and drops
  `util` with it.

## Out of scope

- **Any behaviour change outside the four listed fixes.** The split is a pure
  move: no renamed public item, no changed signature, no new field.
- **Refactoring tools, renderers, io, or the agent.** Only `src/app.rs`,
  `src/ui/menubar.rs`, `src/tools/select/mod.rs`, `src/lib.rs`, the four
  `mod.rs` placeholders and `tests/skeleton.rs` are touched.
- **`src/text/hershey_data.rs` (976 lines).** It is a generated glyph table with
  no logic; the 300-line cap is not applied to it in this demand.
- **New tests for pre-existing behaviour** beyond the criteria below.
- **Removing `OffsetTool`** — that is LCV-106, even though it also touches
  `src/tools/mod.rs`.
- **Changing the AGENTS.md module tree** to match the `util/` removal — that is
  a doc edit owned by LCV-108.
- **`PLAN.md`, `backlog.md`, `CHANGELOG.md`, demand `Status:` lines.**

## Acceptance criteria

1. `git ls-files src/tools/move_tool.rs src/app/tests.rs` returns nothing, and
   `grep -rn "move_tool" src/ tests/` returns no match. `MoveTool` is still
   re-exported from `src/tools/mod.rs` and still works.

2. Every assertion that existed only in `src/app/tests.rs` is either already
   covered by the live suite or has been moved into it. Specifically,
   `cargo test --all` still contains a test asserting
   `App::default().ortho_enabled == false`.

3. `grep -n "selection.set" src/ui/menubar.rs` returns no match; `do_select_all`
   goes through `SelectionCommand` and the history stack.

4. **Select All is undoable**: with 3 entities and entity `1` selected, calling
   `do_select_all` selects all 3; one `history.undo` restores exactly the
   previous selection `{1}`.

5. **Select All on an empty document commits nothing**: `history.can_undo()`
   is `false` after calling `do_select_all` on a fresh `App::default()`.

6. **Escape with an empty selection commits nothing**: with `SelectTool` active
   and `document.selection.is_empty()`, `on_key(Key::Escape, &mut app)` leaves
   `history.can_undo() == false`.

7. **Escape with a non-empty selection still clears it, once**: after selecting
   2 entities, one Escape leaves the selection empty and adds exactly one
   command to the history (one `undo` restores the 2-entity selection).

8. `src/app.rs` no longer exists; `src/app/mod.rs` does. The files
   `src/app/input.rs`, `src/app/viewport.rs`, `src/app/panels.rs`,
   `src/app/autosave.rs` exist.

9. **Every `.rs` file under `src/app/` is ≤ 300 implementation lines**, where
   implementation lines are the lines before the first `#[cfg(test)]` (whole
   file if there is none). Verified with:
   ```
   for f in src/app/*.rs; do t=$(grep -n '^#\[cfg(test)\]' "$f" | head -1 | cut -d: -f1); \
     echo "$([ -n "$t" ] && echo $((t-1)) || wc -l < "$f") $f"; done
   ```
   Every reported count is ≤ 300.

10. `App::update_ui` is an orchestrator of ≤ 25 implementation lines that calls
    one function per phase; `impl eframe::App for App` still contains exactly
    `self.update_ui(ctx);`.

11. `lasercad::app::{suppress_snap_if_disabled, handle_wheel_zoom, handle_pan,
    handle_zoom_extents, apply_ortho, resolve_snap, poll_agent_rx}` all still
    resolve from outside the crate; `tests/lcv070.rs` compiles and passes with
    no edit.

12. `grep -rn "pub const MODULE" src/` returns no match.

13. `src/util/` is gone and `src/lib.rs` no longer declares `pub mod util;`
    (or, if code landed in `src/util/` first, the module survives with a real
    public item and no `MODULE` constant).

14. `tests/skeleton.rs::module_tree_is_wired` compiles and passes, witnessing
    each remaining top-level module with a real public item — no placeholder
    constants.

15. `src/lib.rs`'s module doc no longer names `agent::classifier` or `util`.

16. The kernel purity rule still holds: no `egui` / `eframe` / `rfd` import in
    `src/geometry/*`, `src/document/*`, `src/io/svg/*`, `src/text/*`.

17. No public API outside `src/util` was renamed, removed or re-signed by this
    demand: `cargo doc` builds and `cargo test --all` passes with the same test
    count as before, minus none.

18. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0.

## Expected tests

- **(AC 1, 2)**: `cargo test --all` green after deletion; static check
  `git ls-files` + `grep`. Keep/port `app_default_ortho_is_false` coverage.
- **(AC 3)**: static `grep` on `src/ui/menubar.rs`.
- **(AC 4)**: unit test in `src/ui/menubar.rs` —
  `select_all_is_undoable` — 3 entities, select `{1}` via `SelectionCommand`,
  `do_select_all`, assert `selection.len() == 3`, `history.undo`, assert the
  selection is exactly `{1}`.
- **(AC 5)**: `select_all_on_empty_document_commits_nothing`.
- **(AC 6)**: unit test in `src/tools/select/mod.rs` —
  `escape_with_empty_selection_commits_nothing`.
- **(AC 7)**: `escape_with_selection_clears_it_once` — assert one undo restores
  the two selected indices.
- **(AC 8, 9, 13)**: static checks (`ls`, the implementation-line loop above,
  `git ls-files src/util`).
- **(AC 10)**: structural review plus the line count from AC 9.
- **(AC 11)**: `cargo test --test lcv070` passes with no diff to the file.
- **(AC 12, 15)**: static `grep`.
- **(AC 14)**: `cargo test --test skeleton`.
- **(AC 16)**: `grep -rnE '^use (egui|eframe|rfd)' src/geometry src/document
  src/io/svg src/text` returns no match.
- **(AC 17, 18)**: the three build gates; compare the `cargo test --all` summary
  line before and after.
- **Manual smoke**: `cargo run`. Draw three lines, Edit > Select All → all three
  highlight; Ctrl+Z → the previous selection comes back. Press Escape on an
  empty selection five times, then Ctrl+Z → the last drawn line is undone (not
  five no-op selection commits). Every tool, menu, dialog and the agent panel
  still open and behave as before the split.

## Risks

- **Merge collisions.** This demand moves the exact code LCV-100..104 edit. It
  is last in the Marco 0 code sequence for that reason; starting it before those
  are `Done` will produce conflicts that no test catches. The `Depends on` line
  is a hard gate, not a hint.
- **A silent behaviour change hidden inside a large move.** The split touches
  ~480 lines. Mitigation: no logic edit is allowed in the same commit as a move
  — the four fixes land as separate commits from the split, and AC 17 compares
  the test summary before and after.
- **Re-export drift.** Missing one re-export in `src/app/mod.rs` breaks
  `tests/lcv070.rs` at compile time, which is the cheap failure mode. AC 11
  makes it explicit.
- **Deleting `src/util/`** narrows the module tree documented in AGENTS.md. That
  doc edit is deliberately deferred to LCV-108 so this demand stays code-only;
  the two must both land before Marco 0 closes.
- **`src/app/tests.rs` may contain a genuinely unique assertion** beyond the one
  identified. AC 2 forces the implementer to check rather than assume.

## Open questions

*(none — demand is Ready)*

## Notes

- ADR 0002 (`docs/adr/0002-headless-input-tests-and-dirty-tracking.md`),
  §"The 300-LOC cap and `src/app.rs`", prescribes the target file shape and
  states the cap counts **implementation** LOC, not inline `#[cfg(test)]`
  modules. Only two files in `src/` exceed 300 implementation lines today:
  `src/app.rs` (482) and the generated `src/text/hershey_data.rs` (976).
- `src/app.rs` + `src/app/` already coexist (Rust 2018 module rules), so the
  rename to `src/app/mod.rs` is mechanical: `git mv src/app.rs src/app/mod.rs`,
  then move blocks out one at a time, running `cargo test --all` between moves.
- AGENTS.md mutation contract: "All entity mutation goes through `Command` trait
  + the history stack" and "Selection is part of `Document`, mutated via
  `SelectionCommand`s". Items 2 and 3 are that contract being enforced, not a
  new rule.
- Millimeter canonicity and the LaserGRBL SVG export rules are untouched by this
  demand — no coordinate or export path is moved.
