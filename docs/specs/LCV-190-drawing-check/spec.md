# LCV-190 — Drawing check

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

Nothing checks geometry before export; the agent's batch deliberately has no bed-bounds rule
(`src/agent/drawing.rs`). In an agent session (2026-09-30) an arc had to be corrected by hand-eye,
a double frame and inner details all landed on the Cut layer, and no tool could say whether the
contours were closed or whether a cut would drop loose pieces. Such defects surface only at the
laser, where they waste material.

## Stories

- As an operator, I want to check the drawing for open contours, duplicates, small gaps,
  zero-length entities and entities off the bed so that I fix them before cutting.
- As an operator, I want the agent to run the same check and report or fix what it finds.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Automatic repair.
- Consistency between orthographic views or parametric views derived from shared dimensions.
- Cut-vs-engrave semantics beyond layers (LaserGRBL decides per exported file).

## Open questions

- Gap and duplicate tolerance: fixed (epsilon) or configurable?
- Report closed contours nested inside a cut contour as possible loose pieces?
- Per layer with Output on only, or all layers?
- User command name and toolbar rail entry (every new command gets one).
