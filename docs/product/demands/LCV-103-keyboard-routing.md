# LCV-103 — Keyboard routing: single gate, no double dispatch, Enter reaches the tool

- **Status**: Done
- **Phase**: 10
- **Depends on**: none (ADR 0002 is Accepted). Sequencing with LCV-102 — see Notes.
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: 88bc4c9 — fix(LCV-103): route keyboard input through one gated dispatcher and deliver Enter to tools

## Problem

LaserCAD is keyboard-first by product principle #1, and today the keyboard is
the least trustworthy surface in the app. Three defects compound:

- **D4** — F8 and Ctrl+Z / Ctrl+Y are read twice in the same frame, once in
  `src/ui/shortcuts.rs` (~76-81, ~109-112) and again inside the `CentralPanel`
  closure in `src/app.rs` (~376-387). The operator presses F8 to lock ortho and
  nothing happens (it toggles twice); the operator presses Ctrl+Z once and
  loses two commands.
- **D8** — `F`, `Ctrl+0`, `Delete`, `Backspace` (`src/app.rs:347,365,370`) and
  the `Event::Text` → `ToolManager::on_text_input` forward (~`src/app.rs:390`)
  read raw input with no `ctx.wants_keyboard_input()` guard. Typing "f" in the
  agent chat box zooms the drawing to extents; pressing Backspace to correct a
  typo in a text field deletes the selected entities from the document.
- **D5** — `src/app.rs:358` routes only `Escape` to the active tool, so
  `Key::Enter` never reaches it. `TextTool::on_key(Key::Enter, …)`
  (`src/tools/text.rs:98-106`) is unreachable, which means the TEXT command —
  an explicit v0.1.0 scope target — cannot commit a single character of
  engravable geometry. PolylineTool's "Enter to finish" is dead for the same
  reason.

All three are invisible to the current test suite because nothing drives the
real frame body: `tests/lcv070.rs` calls `dispatch_shortcuts` with synthetic
arguments and proves only that the dispatch table is *correct*, never that it
is *reachable*. ADR 0002 decides the fix shape and the regression-test harness;
this demand implements decision **A** of that ADR.

## Scope

- **ADR 0002 A1 — extract the frame body.** `src/app.rs` gains
  `pub fn update_ui(&mut self, ctx: &egui::Context)` carrying the whole former
  `update` body; `impl eframe::App for App` keeps nothing but
  `fn update(&mut self, ctx, _frame) { self.update_ui(ctx); }`.
- **ADR 0002 A3 — the headless harness.** New `tests/harness/mod.rs` exposing
  exactly the five items listed in the ADR (`SCREEN`, `key_events`, `raw_input`,
  `frame`, `tap`), with `#![allow(dead_code)]` at the top.
- **ADR 0002 A2 — doc the constructor rule.** Doc comments on `App::new` ("boot
  only — reads the real platform data directory; never call from a test") and
  `App::default` ("the test constructor — touches no filesystem").
- **D4 fix.** The duplicate key blocks inside the `CentralPanel` closure (F8,
  Ctrl+Z, Ctrl+Y) are **deleted, not guarded**. `dispatch_shortcuts` in
  `src/ui/shortcuts.rs` becomes the sole reader of key presses, with its current
  signature unchanged.
- **D8 + D5 fix.** New file `src/app/input.rs` (declared `mod input;` from
  `src/app.rs`) owning every key/text route that is not a `dispatch_shortcuts`
  entry: `Escape` / `Enter` / `Delete` / `Backspace` to the active tool,
  `F` / `Ctrl+0` zoom-extents, and the `Event::Text` → `on_text_input` forward —
  all behind one `ctx.wants_keyboard_input()` read, implementing the ADR gate
  table.
- **Escape de-duplication.** `src/ui/command_line.rs` stops calling
  `ToolManager::handle_key` and `ui.input_mut(|i| i.consume_key(…))`; it keeps
  only "clear my own buffer". Cancelling the tool is the gate's job — otherwise
  Escape stays double-handled, which is the very defect this demand closes.
- **Zero-area viewport guard.** `handle_zoom_extents` no-ops when the viewport
  size is zero-area (frame 0 has `camera.viewport_size_px == [0.0, 0.0]`,
  `src/render/camera.rs:56`).
- **Regression tests** in `tests/lcv103.rs`, driving the real update path via
  `ctx.run(raw_input(...), |ctx| app.update_ui(ctx))`.

### The gate table (ADR 0002 A6 — the contract)

| class | keys | fires while a text widget has focus |
|---|---|---|
| global commands | `Ctrl+Z/Y/N/O/S`, `Ctrl+Shift+S` | yes |
| view toggles | `F3`, `F7`, `F8` | yes |
| cancel | `Escape` | yes |
| view actions | `F`, `Ctrl+0` | **no** |
| tool activation | `L P R C A M E T X` | **no** |
| tool key routing | `Enter`, `Delete`, `Backspace` | **no** |
| typed characters | `Event::Text` | **no** |

## Out of scope

- **Autosave / dirty tracking** (ADR 0002 decision B) — that is LCV-102. This
  demand must not touch `dirty_since`, `AUTOSAVE_DEBOUNCE`, or
  `History::revision`.
- **Splitting `src/app.rs` into `src/app/mod.rs` + submodules** — that is
  LCV-105. This demand adds exactly one new submodule (`src/app/input.rs`) and
  leaves `src/app.rs` over the 300-line cap; a reviewer must not block on that.
- **New shortcuts or new bindings.** No key gains a meaning it does not have
  today. The TEXT binding and the Tools menu are LCV-104.
- **Command-line alias dispatch** (typing `LINE` / `TEXT` into the command line
  to activate a tool). Not implemented today, not implemented here.
- **Changing the `dispatch_shortcuts` signature** — `tests/lcv070.rs` must keep
  compiling and passing unchanged.
- **`egui_kittest` or any egui upgrade.** egui stays pinned at 0.29.1.
- **Painting / visual assertions.** Headless tests assert on `App` state only.
- **Statusbar dirty indicator.**

## Acceptance criteria

1. `App::update_ui(&mut self, ctx: &egui::Context)` is public and contains the
   frame body. The `impl eframe::App for App` block contains exactly one
   statement, `self.update_ui(ctx);`, and no other logic.

2. `tests/harness/mod.rs` exists, starts with `#![allow(dead_code)]`, and
   exposes exactly `SCREEN`, `key_events`, `raw_input`, `frame`, `tap` with the
   signatures given in ADR 0002 A3. `key_events` returns **two** events (press
   then release, both `repeat: false`). No single-event helper is exported.

3. `App::new` and `App::default` each carry a doc comment stating the boot-only
   / test-only rule (ADR 0002 A2).

4. **D4 — no key is read outside the single gate.** Neither `src/app.rs` nor any
   file under `src/app/` reads `Key::F8`, `Key::F3`, `Key::F7`, or a
   `modifiers.ctrl && key_pressed(Key::Z | Key::Y | Key::N | Key::O | Key::S)`
   combination. Verified by:
   `grep -rnE 'Key::(F3|F7|F8)' src/ --include=*.rs | grep -v 'src/ui/shortcuts.rs'`
   returning no matches outside `#[cfg(test)]` code.

5. **F8 toggles ortho exactly once per press.** Starting from
   `App::default()` (`ortho_enabled == false`), one frame carrying one F8 tap
   leaves `ortho_enabled == true`; a second tap leaves it `false`; a third
   leaves it `true`.

6. **Ctrl+Z undoes exactly one command.** With three committed `CreateLine`
   commands (`document.entity_count() == 3`), one frame carrying one Ctrl+Z tap
   leaves `entity_count() == 2`. One Ctrl+Y tap then leaves it `3`.

7. **Enter reaches the active tool.** With `TextTool` active, an anchor set by a
   viewport click, and the characters `H`, `I` delivered as `Event::Text`, one
   frame carrying an Enter tap (no text widget focused) commits geometry:
   `history.can_undo() == true` and `document.entity_count() > 0`.

8. **Enter is gated.** While a `TextEdit` has focus, an Enter tap does **not**
   reach `ToolManager::handle_key`: with `TextTool` holding pending text, the
   tool's `preview()` is unchanged after the frame and no command is committed
   (`history.can_undo() == false`). A single Enter press never both submits the
   command line and commits the tool.

9. **`F` is gated.** With a `TextEdit` focused, a frame carrying the character
   `f` leaves `camera.mm_per_px` and `camera.center_world` bit-identical to
   their values before the frame. With nothing focused and a non-empty
   document, an `F` tap changes the camera (zoom-extents fires).

10. **`Backspace` / `Delete` are gated.** With a `TextEdit` focused, a
    selection of two entities, and `SelectTool` active, a Backspace tap leaves
    `document.entity_count()` unchanged and `history.can_undo() == false`. With
    nothing focused, a Delete tap deletes the selection.

11. **`Event::Text` is gated.** With `TextTool` active and a `TextEdit` focused,
    a frame carrying `Event::Text("a")` does not change the tool's `preview()`.
    With nothing focused, the same frame appends the character.

12. **`Escape` is not gated and is handled exactly once.**
    `grep -nE 'handle_key|consume_key' src/ui/command_line.rs` returns no
    matches. With `TextTool` holding pending text and the command line focused,
    one Escape tap both clears `app.command_line_input` and cancels the tool
    (`preview().is_empty()`), in a single frame.

13. **Global commands stay ungated.** With a `TextEdit` focused, a Ctrl+Z tap
    still undoes one command, and an F8 tap still toggles ortho — the "yes" rows
    of the gate table.

14. **Zero-area viewport guard.** `handle_zoom_extents(&mut camera, &doc,
    [0.0, 0.0])` leaves `camera.mm_per_px` and `camera.center_world` unchanged.

15. `src/app/input.rs` exists, is ≤ 300 implementation lines, and imports
    neither `rfd` nor `eframe`.

16. `tests/lcv070.rs` compiles and passes **unchanged** (no edit to that file is
    part of this demand).

17. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0.

## Expected tests

All headless tests live in `tests/lcv103.rs` and use `mod harness;`.

- **(AC 1, 2, 3)**: `update_ui_is_the_frame_body` — the test file compiles and
  calls `app.update_ui(ctx)` through `harness::frame`; a `grep`-style structural
  review covers the `eframe::App` one-liner and the two doc comments.
- **(AC 4)**: static check —
  `grep -rnE 'Key::(F3|F7|F8)' src/ --include=*.rs | grep -v shortcuts.rs`
  produces no non-test match.
- **(AC 5)**: `f8_toggles_ortho_exactly_once_per_press` — the ADR A5 reference
  test, verbatim, retargeted to `tests/lcv103.rs`.
- **(AC 6)**: `ctrl_z_undoes_exactly_one_command` and
  `ctrl_y_redoes_exactly_one_command` — assert `entity_count()` 3 → 2 → 3.
- **(AC 7)**: `enter_commits_text_tool` — warm-up frame with `PointerMoved`,
  then a frame with `PointerButton` to set the anchor, then a frame with
  `Event::Text`, then a frame with an Enter tap; assert
  `document.entity_count() > 0`.
- **(AC 8)**: `enter_is_suppressed_while_text_widget_focused` — click the
  command-line `TextEdit` (warm-up frame + press frame), then an Enter tap;
  assert no commit and unchanged tool preview.
- **(AC 9)**: `f_does_not_zoom_while_text_widget_focused` and
  `f_zooms_when_nothing_focused` — assert camera fields.
- **(AC 10)**: `backspace_does_not_delete_while_focused` and
  `delete_removes_selection_when_unfocused`.
- **(AC 11)**: `text_event_gated_by_focus`.
- **(AC 12)**: `escape_cancels_tool_and_clears_command_line_once` plus the
  `grep` static check on `src/ui/command_line.rs`.
- **(AC 13)**: `global_commands_fire_while_focused` — Ctrl+Z and F8 with focus.
- **(AC 14)**: unit test in `src/app/viewport`-owning module (or wherever
  `handle_zoom_extents` lives): `zoom_extents_noop_on_zero_area_viewport`.
- **(AC 15)**: static checks — `wc -l` up to the first `#[cfg(test)]` line, and
  `grep -nE '^use (eframe|rfd)' src/app/input.rs` with no matches.
- **(AC 16)**: `cargo test --test lcv070` passes with no diff to the file.
- **(AC 17)**: the three build gates.
- **Manual smoke**: launch `cargo run`. Press F8 → the status bar ortho
  indicator flips on one press. Draw two lines, press Ctrl+Z once → exactly one
  line disappears. Click the agent chat box, type "for a 40 mm square" → the
  drawing does **not** zoom and no entity is deleted. Click the canvas, activate
  TEXT, click an anchor, type `HI`, press Enter → Hershey geometry appears.

## Risks

- **The repeat trap (ADR 0002).** egui rewrites `repeat` across frames: sending
  the same key as `pressed: true, repeat: false` in two consecutive frames on
  one `Context` without an intervening release makes egui mark the second event
  `repeat: true`, and `src/ui/shortcuts.rs:29` filters those out. A test written
  with a hand-rolled single `Event::Key` sees a phantom no-op. **Always** use
  `harness::key_events` / `harness::tap`, which emit press **and** release.
- **The pointer warm-up trap (ADR 0002).** With `PointerMoved` + `PointerButton`
  in one frame, `response.hovered()` is `false` and `response.hover_pos()` is
  `None` — the widget rect is not yet registered for hit-testing, and the
  viewport handler sits behind `if response.hovered()`. Any pointer-driven test
  (AC 7, AC 8, AC 10) **must** run a warm-up frame with `PointerMoved` alone
  first. Keyboard-only tests are single-frame.
- **Dialog hang.** Never send `Ctrl+O`, `Ctrl+S`, or `Ctrl+Shift+S` from a test:
  they reach `src/io/file_actions.rs` and open a blocking native `rfd` dialog
  that will hang CI. `Ctrl+N` and `Ctrl+Z/Y` are safe.
- **Real-filesystem autosave.** Never let the autosave debounce elapse inside a
  test; a fired autosave writes to the user's real data directory.
- **Focus is one frame behind.** `ctx.wants_keyboard_input()` read at the top of
  frame N reports focus established in frame N-1. Tests that need focus must
  click the widget in an earlier frame. This is verified behaviour, not a bug.
- **Escape ordering.** `process_shortcuts` runs before any panel, so the gate
  cancels the tool before `command_line.rs` sees the key. That ordering is why
  AC 12 removes the widget's own `handle_key` call rather than reordering
  panels.

## Open questions

*(none — demand is Ready)*

## Notes

- Authority for every design decision here: `docs/adr/0002-headless-input-tests-and-dirty-tracking.md`,
  decision **A** (A1-A6). Read it before starting; it records what was verified
  against the vendored egui 0.29.1 source, so nothing in it needs re-deriving.
- ADR 0002 A5 labels the reference test "the F8 case (LCV-101)" and names the
  file `tests/lcv101.rs`. That is a slip in the ADR: F8 double dispatch is
  defect D4, which is **this** demand. The test file is `tests/lcv103.rs`.
- **Sequencing with LCV-101 / LCV-102.** This demand **owns** the `App::update_ui`
  extraction and `tests/harness/mod.rs`. LCV-102 (autosave, ADR decision B)
  states in its own Notes that the harness is *not* required to ship it and
  takes no dependency on LCV-103; LCV-101 only extends the `App::new` /
  `App::default` doc comments (ADR A2) if they are not already there. So there
  is no ownership conflict — only line drift in `src/app.rs` if those land
  first. Merge order is the `project-manager`'s call.
- `src/app.rs` and `src/app/` coexist today (`src/app/ortho.rs`,
  `snap.rs`, `agent_poll.rs`), so adding `src/app/input.rs` needs only
  `mod input;` in `src/app.rs`. LCV-105 later turns `src/app.rs` into
  `src/app/mod.rs`; keep `update_ui` a thin orchestrator so that split stays
  mechanical.
- Millimeter canonicity is untouched by this demand: every coordinate that
  crosses the gate is already in mm at the `ToolManager` boundary.
