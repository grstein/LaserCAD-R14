# LCV-165 — Command dock messages, prompts and repeat

- **Status**: Specified
- **Depends on**: LCV-167
- **Implementation**: -

## Problem

The command dock speaks in one tone and several dialects. Every message uses the same
`warn_fg_color` orange (`ui/command_line.rs`), so an error, a refusal and a DIST result look
alike. Prompts come in four formats: `LINE Specify first point:`,
`TRIM: Click on a segment to trim`, `EXTEND: Click near a line or arc endpoint to extend it`,
and Delete's bare `ERASE`. `Tool::status_text` returns `&'static str`, so no prompt can show a
default or the current option. Enter on an empty line at `Command:` does nothing, while R14
repeats the last command (the empty Enter goes to the tool, ADR 0003 §B5, and Select ignores it).
The right mouse button does nothing (`tools/manager.rs`, LCV-041 AC 4); in R14 it is Enter. A
bare number refused at a value prompt shows `NO_DIRECTION` (`app/cmdline.rs::send`), e.g.
`-1` at the SCALE factor prompt (found in LCV-182).

## Stories

- As an operator, I want errors, warnings and results to look different.
- As an operator, I want every prompt to tell me what to do, my options and the default.
- As an operator, I want Enter or right-click to repeat the last command.

## Acceptance criteria

1. WHEN the dock shows a message THE SYSTEM SHALL paint an error in `status.error`, a warning or
   refusal in the warning orange, and an info message (query results such as DIST) in
   `text.primary`.
2. THE SYSTEM SHALL let `Tool::status_text` return an owned or borrowed string (`Cow<str>`), so a
   prompt can include runtime values.
3. WHILE any drawing, modify or inquiry tool is waiting THE SYSTEM SHALL show a prompt of the form
   `VERB  Specify <thing> [Opt/Opt] <default>:` (options and default only where they exist), and
   picking tools SHALL use the same form (`TRIM  Select object to trim:`,
   `EXTEND  Select object to extend:`, `ERASE  Select objects:`).
4. WHEN the operator presses Enter on an empty line while Select is idle THE SYSTEM SHALL start
   the last CAD command word from the recall ring, as if it had been typed.
5. IF the recall ring holds no CAD command word, or its last entry is an agent prompt (`:`/`/ai`)
   THEN THE SYSTEM SHALL skip agent prompts and, with no command word left, do nothing.
6. WHEN the operator right-clicks on the canvas THE SYSTEM SHALL act exactly as Enter on an empty
   line (finish, accept or repeat) and SHALL NOT pick or place a point; middle-drag pan is
   unchanged (reverses LCV-041 AC 4).
7. IF a bare number is refused at a prompt that takes numbers as values (ROTATE angle, SCALE
   factor) THEN THE SYSTEM SHALL show that tool's refusal, not `NO_DIRECTION`.
8. THE SYSTEM SHALL record the severities, the prompt grammar, repeat and right-click in
   DESIGN.md §7 and §8, and as an amendment note in ADR 0003 (§B5 empty Enter, §C prompt type),
   in this spec's last task.

## Out of scope

- A right-click context menu; typed option letters (`[Close/Undo]`) beyond showing them.
- A scrolling command history window (rejected, LCV-139).

## Open questions

- None. Decided (self-approved per user goal): ADR 0003 gets an amendment note, not a new ADR;
  right-click = Enter reverses LCV-041 AC 4; agent prompts never repeat; the LCV-111 AC 17 prompt
  table is updated to the new grammar; `status.error` comes from LCV-167, which lands first.
