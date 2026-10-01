# LCV-194 — Agent measure tool

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

The agent can list entities (`query_entities`) and, with vision on, look at the canvas, but it
cannot measure: it has no way to confirm that a hole is 30 mm from an edge, that two parts do not
overlap, or that a contour has the requested area. CAD agents that top image-to-CAD benchmarks do
so "with tools": a sandbox to render, **measure** and iterate before submitting (BenchCAD,
<https://benchcad.com>, arXiv 2605.10865). The operator already has `DIST` (LCV-159); the agent
has nothing equivalent, so it verifies by reading raw coordinates and doing arithmetic in text,
which is where dimension mistakes slip through.

## Stories

- As an operator, I want the agent to measure what it drew so that its "done" means the
  dimensions I asked for.

## Acceptance criteria

All values are in mm and degrees, rounded to 0.001, computed by `src/geometry/`.

1. WHEN the agent calls `measure` with `distance` between two points, THE SYSTEM SHALL return the
   distance, dx and dy.
2. WHEN the agent calls `measure` with `distance` between a point and an entity index, or between
   two entity indices, THE SYSTEM SHALL return the minimum distance and the closest points.
3. WHEN the agent calls `measure` with `length` on a line or arc, THE SYSTEM SHALL return its
   length (a circle returns its circumference).
4. WHEN the agent calls `measure` with `bbox` on a set of indices (or none for the whole drawing),
   THE SYSTEM SHALL return min, max, width and height.
5. WHEN the agent calls `measure` with `intersections` on two indices, THE SYSTEM SHALL return
   every intersection point, or "none".
6. WHEN the agent calls `measure` with `angle` on two lines, THE SYSTEM SHALL return the angle
   between them in degrees.
7. WHEN `measure` runs, THE SYSTEM SHALL count one read-only step that does not change the
   document, the selection or the undo stack and does not trip the fence.
8. IF an index is out of range or the operand kinds do not fit the query, THEN THE SYSTEM SHALL
   refuse naming the field and the accepted form.

## Out of scope

- Area of closed contours (needs contour chaining; revisit with LCV-190's contour work).
- Dimension entities or any drawing annotation.
- A user `MEASURE` command beyond the existing `DIST`.

## Open questions

- One `measure {query, …}` tool or one tool per query? (Default: one tool, `query` enum, flat
  provider-safe schema per LCV-185.)
