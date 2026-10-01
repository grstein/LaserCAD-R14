# LCV-195 — Agent feedback after mutation

- **Status**: Draft
- **Depends on**: LCV-187, LCV-190
- **Implementation**: -

## Problem

After each change the agent gets one sentence ("Line created … N entities") and must spend extra
steps on `query_entities`, `capture_canvas` or a check to see the result, which it often skips.
The most used FreeCAD MCP server returns a screenshot with every model-changing call by default
(`include_screenshot`, <https://github.com/neka-nat/freecad-mcp>); agents that see the effect of
each action correct themselves earlier. LaserCAD's observation is opt-in and manual.

## Stories

- As an operator, I want the agent to see the effect of each change without spending a step on
  it, so that it catches its own mistakes during the turn.

## Acceptance criteria

1. WHILE the setting "Agent feedback after changes" is on, WHEN a model reply that mutated the
   drawing finishes, THE SYSTEM SHALL append to the last tool result a short summary: entity
   count, drawing bounding box, and the per-kind counts of the drawing check (LCV-190).
2. WHILE that setting and canvas capture (ADR 0011) are both on, THE SYSTEM SHALL also attach
   one framed capture of the drawing (LCV-187), subject to the same upload authorization.
3. WHEN a reply did not mutate the drawing, THE SYSTEM SHALL append nothing.
4. WHEN feedback is appended, THE SYSTEM SHALL not count an extra step.
5. WHILE the setting is off (the default), THE SYSTEM SHALL behave as today.

## Out of scope

- Feedback after each individual call inside one reply (once per reply keeps tokens bounded).
- Colour or multi-view captures.

## Open questions

- Default on or off? (Default: off, like canvas capture; LCV-200 decides with data.)
