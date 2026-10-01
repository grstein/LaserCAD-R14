# LCV-195 — Agent feedback after mutation

- **Status**: Done
- **Depends on**: LCV-187, LCV-189, LCV-190
- **Implementation**: 7268247..2ffde58

## Problem

After each change the agent gets one sentence ("Created … N entities") and must spend extra steps
on `query_entities`, `capture_canvas` or `check_drawing` to see the result, which it often skips.
The most used FreeCAD MCP server returns a screenshot with every model-changing call by default
(`include_screenshot`, <https://github.com/neka-nat/freecad-mcp>); agents that see the effect of
each action correct themselves earlier. LaserCAD's observation is opt-in and manual.

## Stories

- As an operator, I want the agent to see the effect of each change without spending a step on it,
  so that it catches its own mistakes during the turn.

## Acceptance criteria

A reply *mutated* the drawing when at least one of its tool calls was applied and moved the revision.

1. WHILE the setting `Feedback after changes` is on, WHEN the tool calls of a reply that mutated the
   drawing have run, THE SYSTEM SHALL append to the last tool result, before LCV-189's
   `Steps left` line, `Drawing now: <n> entities, X <x0>..<x1> mm, Y <y0>..<y1> mm. <check>`,
   where `<check>` is the LCV-190 summary lines joined with `; `, or `CHECK: no problems found.`
2. WHEN the drawing is empty after the reply, THE SYSTEM SHALL write `Drawing now: 0 entities.`
   followed by the check text.
3. WHILE the setting and both canvas-capture opt-ins (ADR 0011) are on, WHEN feedback is appended
   to a non-empty drawing, THE SYSTEM SHALL also attach one `frame: "drawing"` capture (LCV-187),
   under the same upload authorization and with the same per-image transcript note, using the id
   of the reply's last tool call.
4. WHEN a reply did not mutate the drawing, or its calls were stopped by the fence, THE SYSTEM
   SHALL append nothing and attach nothing.
5. WHEN feedback or its capture is added, THE SYSTEM SHALL count no extra step.
6. WHILE the setting is off (the default, and for settings files without it), THE SYSTEM SHALL
   behave exactly as today.
7. WHEN Agent Settings is open, THE SYSTEM SHALL show the checkbox `Feedback after changes` with
   the hint `After each reply that changes the drawing, tell the agent its size and CHECK result.`

## Out of scope

- Feedback after each call inside one reply (once per reply keeps tokens bounded).
- Colour or multi-view captures; feedback that is not derived from the live document.

## Decisions (self-approved per user goal, 2026-09-30)

- Default off, like canvas capture; LCV-200 measures whether it should become on.
- One setting drives both the text and the capture; the capture still needs both ADR 0011 opt-ins.
- The summary is computed on the UI thread from the live document after the reply's calls; the
  worker still holds no document state (ADR 0007 §D1).
- The `Steps left` line stays last, so LCV-189's contract is unchanged.

## Open questions

- None.
