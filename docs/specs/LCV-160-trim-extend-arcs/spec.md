# LCV-160 — TRIM and EXTEND with arcs

- **Status**: Done
- **Depends on**: none
- **Implementation**: 03aa6cc..b8e1ab5

## Problem

TRIM and EXTEND skip every pair that involves an arc (`tools/trim.rs::entities_intersect`,
`document/commands/trim/mod.rs`). An arc cannot be trimmed or extended, and it cannot act as a
cutter or a boundary. A circle cannot be trimmed by another circle either. Rounded corners and
slots therefore cannot be cleaned up before export.

## Stories

- As an operator, I want to trim and extend lines and arcs against lines, circles and arcs, so
  that I can clean up rounded shapes without redrawing them.

## Acceptance criteria

A *cut point* is a point that lies on both entities: a point on an arc's circle but outside
the arc's span does not count. TRIM keeps the piece that holds the click, as it does today
(LCV-050).

1. WHEN the operator clicks an Arc target that a Line, Circle or Arc cutter crosses, THE SYSTEM
   SHALL replace it with the sub-arc between the cut points around the click. The sub-arc keeps
   the target's center, radius and direction.
2. WHEN the operator clicks a Line target that an Arc cutter crosses, THE SYSTEM SHALL keep the
   sub-segment around the click, using only the arc's cut points.
3. WHEN the operator clicks a Circle target that a Circle or Arc cutter crosses at two cut
   points, THE SYSTEM SHALL replace it with the arc between them that holds the click.
4. IF the click finds no cut point on the target, or one cut point on a Circle target, THEN THE
   SYSTEM SHALL leave the document unchanged and add no undo entry.
5. WHEN the operator hovers in EXTEND within the pick radius of an Arc endpoint, THE SYSTEM
   SHALL preview the arc grown from that end along its own circle, away from its other end, to
   the nearest cut point on a Line, Circle or Arc boundary. On click it SHALL commit that arc.
6. WHEN the operator extends a Line, THE SYSTEM SHALL accept an Arc as boundary, using only the
   cut points of the line's extension that lie on the arc's span.
7. IF no boundary is reached before the extended arc would close into a full turn THEN THE
   SYSTEM SHALL show no preview, and a click SHALL change nothing.
8. WHEN a trim or extend from ACs 1–6 is undone, THE SYSTEM SHALL restore the original entity
   exactly. Redo SHALL reapply the same result. The result SHALL stay on the target's layer.
9. WHEN one TRIM click meets several cutters THE SYSTEM SHALL commit it as one undo step.

## Out of scope

- Fillet, chamfer and offset (product non-goals).
- Changing TRIM so it removes the picked piece (R14 style) instead of keeping it.
- Trim and extend previews and hover highlight (LCV-163).
- Screen-space pick radius (LCV-162).

## Open questions

- None.
