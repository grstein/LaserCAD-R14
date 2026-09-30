# LCV-191 — Agent moves entities between layers

- **Status**: Planned
- **Depends on**: LCV-186
- **Implementation**: -

## Problem

The agent can create entities on an existing layer (`layer` argument), but cannot move an existing
entity to another layer; `copy_entity` and `mirror_entity` keep the source layer. In an agent
session (2026-09-30) everything, including inner details and a decorative double frame, stayed on
the Cut layer; the only fix was to delete and redraw. Layers are how the operator separates cut
from engrave output (ADR 0012).

## Stories

- As an operator, I want the agent to move entities to an existing layer so that cut and engrave
  work end up in the right exported file.

## Acceptance criteria

1. WHEN the agent calls `set_layer` with `indices` (as in LCV-186) and an existing `layer` name,
   THE SYSTEM SHALL move every listed entity to that layer as one action and one step.
2. WHEN a listed entity is already on that layer, THE SYSTEM SHALL leave it and still succeed.
3. IF the layer does not exist THEN THE SYSTEM SHALL refuse with today's `unknown layer "X" (the
   layers are: …)` text and change nothing.
4. IF `indices` is invalid (LCV-186 AC 5) THEN THE SYSTEM SHALL refuse and change nothing.
5. WHEN the move is undone after the turn, THE SYSTEM SHALL restore every entity's layer with the
   rest of the turn in one undo.
6. WHEN the built-in prompt lists the tools, THE SYSTEM SHALL describe `set_layer`.

## Out of scope

- Creating, renaming or deleting layers, or changing colour and Output, from the agent.
- Loose-piece and contour warnings (LCV-190).

## Open questions

- None.
