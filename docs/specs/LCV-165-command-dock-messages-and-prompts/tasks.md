# LCV-165 — Tasks

- [ ] T1 Refactor, no behaviour change: move `send`, `direct_distance`, `NO_BASE_POINT`,
  `NO_DIRECTION` into `app/cmdline/dispatch.rs`; `scripts/check.sh cmdline` green (files:
  src/app/cmdline.rs, src/app/cmdline/dispatch.rs)
- [ ] T2 [AC1] Test first: painted feedback colour — `hello` ⏎ → `STATUS_WARNING`, `grid` ⏎ →
  `TEXT_PRIMARY`, `:x` ⏎ with no API key → `STATUS_ERROR`; `App::command_feedback_severity`
  asserted for DIST result (Info), `@1,0` with no anchor (Warning), export not saved (Warning)
  (files: tests/it/cmdline/dock_messages.rs, tests/it/cmdline/mod.rs)
- [ ] T3 [AC1] `Severity` + `App::say` in `app/feedback.rs`; field and default (files:
  src/app/feedback.rs, src/app/mod.rs, src/app/init.rs)
- [ ] T4 [AC1] Writers use `say` with the plan's severities (files: src/app/cmdline.rs,
  src/app/cmdline/dispatch.rs, src/app/viewport.rs)
- [ ] T5 [AC1] Export-layers writers use `say`; the dock colours by severity (the `! ` prefix
  rule goes); T2 green (files: src/io/export_layers.rs, src/ui/command_line.rs)
- [ ] T6 [AC2] Test first: the TEXT height prompt is `TEXT  Specify height <5>:` and is
  `Cow::Owned`; the manager hands the owned prompt through unchanged (files:
  src/tools/text/tests.rs, src/tools/manager.rs)
- [ ] T7 [AC2] `Tool::status_text -> Cow<'_, str>` (default `Borrowed(name)`), manager and dock
  follow, every impl wraps its literal with `.into()`; TEXT height formats its default
  (mechanical; the one task over 3 files: src/tools/*.rs, src/ui/command_line.rs)
- [ ] T8 [AC3] Test first: `tests/it/cmdline/prompt_grammar.rs` — the plan's prompt table,
  driven by typed input per tool state, plus the grammar check on every non-`Command:` prompt
  (files: tests/it/cmdline/prompt_grammar.rs, tests/it/cmdline/mod.rs)
- [ ] T9 [AC3] LINE, PLINE, RECT prompts; tests keyed on the old strings follow (files:
  src/tools/line.rs, src/tools/polyline.rs, src/tools/rect.rs)
- [ ] T10 [AC3] CIRCLE, ARC, TEXT prompts; tests follow (files: src/tools/circle.rs,
  src/tools/arc.rs, src/tools/text.rs)
- [ ] T11 [AC3] DIST, MOVE, COPY prompts; tests follow (files: src/tools/dist.rs,
  src/tools/move_.rs, src/tools/copy.rs)
- [ ] T12 [AC3] ROTATE, SCALE, MIRROR prompts; tests follow (files: src/tools/rotate.rs,
  src/tools/scale.rs, src/tools/mirror.rs)
- [ ] T13 [AC3] TRIM, EXTEND, ERASE prompts (ERASE overrides `status_text`); T8 green (files:
  src/tools/trim.rs, src/tools/extend.rs, src/tools/delete.rs)
- [ ] T14 [AC4] [AC5] Test first: `last_tool` over `[l, 0,0, 10]` → Line, `[c, :draw box, /ai hi]`
  → Circle, `[:draw]` → None, `[grid, hello]` → None; integration: `l`, Esc, empty ⏎ → LINE
  prompt; LINE mid-command empty ⏎ finishes, no repeat; ring unchanged after a repeat (files:
  src/cmdline/history.rs, tests/it/cmdline/repeat.rs, tests/it/cmdline/mod.rs)
- [ ] T15 [AC4] [AC5] `CommandHistory::last_tool` (files: src/cmdline/history.rs)
- [ ] T16 [AC4] `Tool::at_rest` (default false), Select overrides, `ToolManager::at_rest`
  (files: src/tools/tool.rs, src/tools/select/mod.rs, src/tools/manager.rs)
- [ ] T17 [AC4] The `Empty` arm repeats `last_tool` when at rest, else routes Enter; T14 green
  (files: src/app/cmdline.rs)
- [ ] T18 [AC6] Test first: a secondary press on the canvas — LINE waiting for its next point
  goes back to its first-point prompt and the press adds no entity; at `Command:` after `c` ⏎
  Esc → CIRCLE; the selection is unchanged; middle-drag still pans (files:
  tests/it/app/right_click.rs, tests/it/app/mod.rs)
- [ ] T19 [AC6] `handle_hover`: the secondary press → `cmdline::submit(app, "")`;
  `PointerButton::Secondary` doc updated; T18 green (files: src/app/viewport.rs,
  src/tools/pointer_event.rs)
- [ ] T20 [AC7] Test first: `s`, a base point, `-1` ⏎ and `0` ⏎ with the pointer never on the
  canvas → `Scale factor must be greater than 0.`, Warning, never `NO_DIRECTION`; ROTATE unit
  test: a non-finite angle leaves its refusal message (files:
  tests/it/cmdline/transform_commands.rs, src/tools/rotate.rs)
- [ ] T21 [AC7] SCALE/ROTATE set the refusal message; `send` drains `take_message` as a
  Warning on refusal; T20 green (files: src/tools/scale.rs, src/tools/rotate.rs,
  src/app/cmdline/dispatch.rs)
- [ ] T22 [AC8] DESIGN.md §7 (severities, prompt grammar and table), §8 (repeat, right-click =
  Enter, replacing the LCV-041 line); ADR 0003 amendment note (§B5 empty Enter, §C `Cow`
  prompt) (files: DESIGN.md, docs/adr/0003-command-line-input-contract.md)
- [ ] T23 CHANGELOG line (files: CHANGELOG.md)
