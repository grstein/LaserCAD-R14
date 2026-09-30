# LCV-190 — Drawing check

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

Nothing checks geometry before export; the agent's batch deliberately has no bed-bounds rule
(`src/agent/drawing.rs`). In an agent session (2026-09-30) an arc direction had to be corrected
after the fact, and no tool could say whether contours were closed, duplicated or off the bed.
Such defects show up only at the laser, where they waste material and time.

## Stories

- As an operator, I want to check the drawing for open contours, duplicates, small gaps,
  degenerate entities and entities off the bed so that I fix them before cutting.
- As an operator, I want the agent to run the same check and act on its report.

## Acceptance criteria

The check covers entities on layers with Output on. Two endpoints *meet* when they are within the
geometry epsilon; a *gap* is two endpoints closer than 0.5 mm that do not meet.

1. WHEN the operator runs `CHECK` (command line, or its toolbar rail button), THE SYSTEM SHALL
   print one summary line per finding kind with its count, then one line per finding with the
   entity indices and the location in mm.
2. WHEN a line or arc endpoint meets no other endpoint, THE SYSTEM SHALL report it as an open end.
3. WHEN two endpoints form a gap, THE SYSTEM SHALL report it as a gap (not as two open ends).
4. WHEN two entities have the same geometry within epsilon (a line in either direction; a circle;
   an arc with the same span), THE SYSTEM SHALL report them as duplicates.
5. WHEN a line has zero length or an arc a zero span, THE SYSTEM SHALL report it as degenerate.
6. WHEN any part of an entity lies outside the bed, THE SYSTEM SHALL report it as off-bed.
7. WHEN no finding exists, THE SYSTEM SHALL print `CHECK: no problems found.`
8. WHEN the check runs, THE SYSTEM SHALL not change the document, the selection or the undo stack.
9. WHEN the agent calls `check_drawing`, THE SYSTEM SHALL return the same report as text, as one
   read-only step that does not trip the fence.

## Out of scope

- Automatic repair; closed contours nested in a cut contour (loose pieces) — revisit later.
- Consistency between orthographic views, or views derived from shared dimensions.
- Cut-vs-engrave semantics beyond layers (LaserGRBL decides per exported file).

## Open questions

- None; tolerances are fixed (epsilon, 0.5 mm). Rail icon follows LCV-183 if it lands first.
