# LCV-165 — Command dock messages, prompts and repeat

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

The command dock is the main surface of a keyboard-first CAD, but it speaks in one tone and
several dialects:

- **Messages.** Every message uses the same `warn_fg_color` orange
  (`ui/command_line.rs`). An error looks like a refusal, and DIST (LCV-159) would look alike too.
- **Prompts.** Tools prompt in four formats. Most use `LINE Specify first point:`. Trim and
  Extend use `TRIM: Click on a segment to trim` and `EXTEND: Click to extend  |  Esc to cancel`.
  Delete shows only the name `ERASE`. `Tool::status_text` returns `&'static str`, so a prompt
  cannot show a default value or the current option.
- **Repeat.** Enter on an empty line at the idle `Command:` prompt does nothing, while R14
  repeats the last command. The empty Enter goes to the active tool (ADR 0003 §B5), and Select
  ignores it.
- **Right button.** The right mouse button does nothing
  (`tools/pointer_event.rs::PointerButton::Secondary`, LCV-041 AC 4). In R14 it is Enter.

## Stories

- As an operator, I want errors, warnings and results to look different, so that I notice the
  ones that need action.
- As an operator, I want every prompt to tell me what to do, which options I have, and the default.
- As an operator, I want Enter or right-click to repeat the last command, so that I can draw
  several lines in a row.

## Direction

- Three message severities, each with its own chrome token: error (`status.error`, see
  LCV-167), warning (today's orange) and info (`text.primary`). Query results such as DIST
  (LCV-159) are info.
- One prompt grammar: `VERB  Specify <thing> [Option/Option] <default>:`. Picking tools use the
  same grammar (`TRIM  Select object to trim:`). `status_text` returns a `String` or a `Cow`.
- Enter on an empty line with Select idle repeats the last command word from the recall ring.
- Right-click on the canvas acts exactly like Enter on an empty line: it finishes, accepts or
  repeats. Middle-drag pan stays as it is.
- DESIGN.md §7 and §8 are updated in this spec's last task.

## Acceptance criteria

To be written by /specify.

## Out of scope

- A context menu on right-click (R14's default is Enter; a menu would need a new surface).
- Command options typed as letters (`[Close/Undo]` choices) beyond showing them in the prompt.
- A scrolling command history window (rejected, LCV-139).

## Open questions

- ✱ ADR 0003 §B5 routes an empty Enter to the tool, and §C keeps `status_text` `&'static str`.
  Repeat and runtime prompts amend both: an amendment note, or a new ADR?
- ✱ LCV-041 AC 4 makes `Secondary` a no-op. Right-click = Enter reverses it.
- Does an agent prompt (`:`/`/ai`) repeat? Proposal: no, only CAD command words.
- Should the prompt text change for the Done specs' tests (LCV-111 AC 17 prompt table)?
