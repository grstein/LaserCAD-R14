# LCV-191 — Agent moves entities between layers

- **Status**: Draft
- **Depends on**: LCV-186
- **Implementation**: -

## Problem

The agent can create entities on an existing layer (`layer` argument), but cannot move an existing
entity to another layer; `copy_entity` and `mirror_entity` keep the source layer. In an agent
session (2026-09-30) everything, including inner details and a decorative double frame, stayed on
the Cut layer, and the only fix would have been to delete and redraw. Layers are how the operator
separates cut from engrave output (ADR 0012).

## Stories

- As an operator, I want the agent to move entities to an existing layer so that cut and engrave
  work end up in the right exported file.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Loose-piece and contour warnings (LCV-190).
- Changing layer properties (colour, Output) from the agent.

## Open questions

- May the agent also create a layer, or only use existing ones?
- One set-level tool reusing LCV-186's entity-set argument?
