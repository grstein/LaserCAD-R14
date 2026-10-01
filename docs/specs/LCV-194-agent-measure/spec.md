# LCV-194 — Agent measure tool

- **Status**: Planned
- **Depends on**: LCV-188, LCV-190, LCV-192
- **Implementation**: -

## Problem

The agent can list entities (`query_entities`) and look at the canvas, but it cannot measure: it
cannot confirm that a hole is 30 mm from an edge, that two parts keep apart, or that a part has the
requested size. It does the arithmetic in text, where dimension mistakes slip through. CAD agents
that lead image-to-CAD benchmarks render, **measure** and iterate before answering (BenchCAD,
arXiv 2605.10865). The operator has `DIST` (LCV-159); the agent has nothing equivalent.

## Stories

- As an operator, I want the agent to measure what it drew so that its "done" means the
  dimensions I asked for.

## Acceptance criteria

`measure {query, points, indices | ids}`: `points` is a list of `{x, y}` in mm; entities are given
by `indices` or `ids` (LCV-188), never both. Values are mm and degrees to 3 decimals, computed by
`src/geometry/`; a line, arc or circle is the bounded entity, never its extension.

1. WHEN `query` is `distance` with two operands (two points, a point and an entity, or two
   entities), THE SYSTEM SHALL return the minimum distance, dx, dy and the two closest points;
   entities that touch, cross or overlap give 0.
2. WHEN `query` is `length` with one entity, THE SYSTEM SHALL return a line's length, an arc's
   arc length or a circle's circumference.
3. WHEN `query` is `bbox` with any number of entities, or none for the whole drawing, THE SYSTEM
   SHALL return min, max, width and height of their exact extents (arc bulges included).
4. WHEN `query` is `bbox` on an empty drawing, THE SYSTEM SHALL answer `bbox: the drawing is empty`.
5. WHEN `query` is `intersections` with two entities, THE SYSTEM SHALL return every intersection
   point, `none`, or `overlap` when they share a segment or an arc span.
6. WHEN `query` is `angle` with two lines, THE SYSTEM SHALL return the counter-clockwise angle from
   the first line's direction (start to end) to the second's in [0, 360) and the angle between the
   lines in [0, 90].
7. WHEN `measure` runs, THE SYSTEM SHALL count one step that changes neither the document, the
   selection nor the undo stack, and does not trip the fence (as `query_entities`).
8. IF the operand count or kinds do not fit the query, an index or id is unknown, or a point is not
   finite, THEN THE SYSTEM SHALL refuse in the LCV-192 shape naming the field and the accepted form.
9. WHEN the built-in prompt lists the tools, THE SYSTEM SHALL describe `measure` and every query
   (`tests/it/agent/default_prompt.rs`).

## Out of scope

- Area and perimeter of closed contours (needs contour chaining after LCV-190).
- Dimension entities or any annotation in the drawing; a new operator command beyond `DIST`.

## Decisions (self-approved per user goal, 2026-09-30)

- One tool with a `query` enum and a flat provider-safe schema (ADR 0010 §2), always advertised.
- Operands are typed lists (`points`, `indices`/`ids`), not a union field, so the schema needs no
  `oneOf`; each query states its operand count and the validator enforces it.
- `angle` returns both the directed and the undirected angle so polygon corners and parallelism are
  both checkable without the model re-deriving directions.
- `bbox` covers all layers (unlike `CHECK`, which covers Output layers only).

## Open questions

- None.
