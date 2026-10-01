# LCV-165 — Plan

## Approach

Four small changes behind the dock, after LCV-184, LCV-164 and LCV-167 have landed:
- **Severity.** `command_feedback` stays a `String`, so its 90 readers are untouched. A sibling
  `command_feedback_severity: Severity` (`Error | Warning | Info`) is written only through one
  helper, `App::say(severity, text)`, in a new `app/feedback.rs`. The dock picks the colour
  from the severity. That replaces LCV-167's interim `! ` prefix rule; the text keeps its `! `.
- **Prompts.** `Tool::status_text` returns `Cow<'_, str>`, and every prompt follows
  `VERB  Specify <thing> [Opt/Opt] <default>:`. LCV-184's `prompt_spans` already splits the
  verb at a space and colours `[…]` and `<…>`, so it needs no change.
- **Repeat.** An empty Enter while Select is at rest starts `CommandHistory::last_tool()`: the
  newest ring entry that parses to `CommandInput::Tool`. Agent lines (`:`/`/ai`), points,
  numbers, toggles and unknown words never parse to a tool, so AC 5 falls out of `parse`. A
  right press on the canvas calls `app::cmdline::submit(app, "")`, the exact empty-Enter path.
- **AC 7.** SCALE and ROTATE leave a refusal line in `take_message`. `send` drains it as a
  Warning before it falls back to `NO_DIRECTION`.

## Touches

- `src/app/cmdline/dispatch.rs` (new) — `send`, `direct_distance`, `NO_BASE_POINT`,
  `NO_DIRECTION` move out of `app/cmdline.rs` first (271 → ~235), with no behaviour change.
- `src/app/feedback.rs` (new) — `Severity`, `App::say`; `app/mod.rs` field, `app/init.rs` default.
- Writers switch to `say`: `app/cmdline.rs` (Unknown → Warning, toggles/echo → Info,
  unavailable → Error, empty/busy → Warning), `dispatch.rs` (refusals → Warning),
  `app/viewport.rs::poll_successor` (tool result → Info), `io/export_layers.rs` (not saved /
  nothing to export → Warning, written → Info).
- `src/ui/command_line.rs::draw_context_row` — colour = `error_fg_color` / `warn_fg_color` /
  `theme::TEXT_PRIMARY` by severity.
- `src/tools/tool.rs::Tool::status_text` → `Cow<'_, str>` (default `Borrowed(self.name())`);
  `tools/manager.rs::active_status_text` follows; every tool's prompt text (table below).
- `src/cmdline/history.rs::CommandHistory::last_tool` (kernel-pure: `parse` only).
- `src/tools/tool.rs::Tool::at_rest` (default `false`), `select/mod.rs` = `state == Idle`,
  `manager.rs::at_rest`; `app/cmdline.rs` `Empty` arm: at rest → repeat, else Enter to the tool.
- `src/app/viewport.rs::handle_hover` — the secondary press submits `""`;
  `tools/pointer_event.rs` doc: Secondary no longer "reserved".
- `src/tools/scale.rs`, `rotate.rs` — a refusal message; `dispatch.rs::send` drains it.
- `DESIGN.md` §7, §8; ADR 0003 amendment note (§B5 empty Enter, §C prompt type); `CHANGELOG.md`.

## Prompt table (replaces LCV-111 AC 17; that demand is pre-SDD, so DESIGN.md §7 holds it)

`LINE`/`PLINE  Specify first point:` · `…  Specify next point <Enter to finish>:` ·
`RECT  Specify first corner:` / `opposite corner:` · `CIRCLE  Specify center point:` / `radius:` ·
`ARC  Specify start point:` / `point on arc:` / `end point:` · `TEXT  Specify start point:` /
`TEXT  Specify text:` / `TEXT  Specify height <5>:` (formatted from `DEFAULT_TEXT_HEIGHT_MM`,
so it is `Cow::Owned`; the refused-height variant keeps its sentence before `Specify`) ·
`DIST`/`MOVE`/`COPY`/`ROTATE`/`SCALE  Specify base point:` (DIST: first/second point) ·
`MOVE  Specify destination point:` · `COPY  Specify second point:` ·
`ROTATE  Specify rotation angle:` · `SCALE  Specify scale factor:` · MIRROR's three prompts,
with the verb padded to two spaces · `TRIM  Select object to trim:` ·
`EXTEND  Select object to extend:` · `ERASE  Select objects:` · Select `Command:` unchanged.

## Decisions (self-approved per user goal)

- Error = the operation or configuration failed (AI unavailable). Warning = input refused
  (unknown word, no base point, no direction, tool refusal, AI empty/busy, export not possible).
  Info = results and acknowledgements (DIST, `SNAP on`, `→ AI: "…"`, `Exported layers: …`).
- Options are shown only where the tool already acts on them (MIRROR `[Yes/No] <N>`). No
  `[Undo]`/`[Close]` is invented. `<Enter to finish>` is the default of the next-point prompts.
- Repeat does not push to the ring; Up still recalls what was typed. Only tool words repeat.
- Right-click ignores any text already in the field. It is Enter on an *empty* line, as the AC says.
- "Select at rest" = `SelectState::Idle`; the press/drag states never repeat.

## Test approach

Painted feedback colours as in `visual_refresh.rs::text_colours` (AC 1); a prompt table driven
by typed input plus a grammar check (AC 2/3); `last_tool` units and an empty-Enter flow (AC 4/5);
a harness secondary press while LINE waits (AC 6); SCALE `-1` with no cursor (AC 7).

## Risks

- LOC cap: `tools/manager.rs` 269 → ~273 (seam: delegations to `manager/delegate.rs`);
  `tools/scale.rs` 267 → ~272 (seam: `scale/factor.rs`); `app/mod.rs` ~293 → ~295 after
  LCV-167 (seam: its `AutosaveState`); `app/cmdline.rs` drops to ~235 in T1.
- T7 (`Cow`) touches every tool file at once: the one task over 3 files, mechanical `.into()`.
- ~10 integration test files match old prompts; each prompt task updates its own (`grep`).
- Mutation testing: no (no `src/agent/`, export or `History` change).
