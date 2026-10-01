# LCV-196 — Batch drawing primitives

- **Status**: Draft
- **Depends on**: LCV-185
- **Implementation**: -

## Problem

`create_drawing` (ADR 0010) accepts only line, circle and arc. A rounded plate, a hexagon, a row
of holes or a label must be spelled out entity by entity, so the model does the geometry in its
head, and payloads grow. Benchmark-leading CAD agents write *code* (CadQuery, bpy) for exactly
this expressiveness: loops, parameters and higher-level shapes. LaserCAD rejects running code;
a few declarative primitives give most of the gain while staying inspectable and bounded.

## Stories

- As an operator, I want the agent to draw rectangles, rounded rectangles, polygons, polylines,
  text and patterns in one declarative batch so that complex parts come out right the first time.

## Acceptance criteria

Every new item expands into existing entity kinds (line, circle, arc) before validation; the
1000-entity cap applies to the expanded count.

1. WHEN a batch has a `polyline` item with points and `closed`, THE SYSTEM SHALL create one line
   per segment, closing it when asked.
2. WHEN a batch has a `rect` item with corner, width, height and optional `corner_radius`, THE
   SYSTEM SHALL create four lines, or four lines and four quarter arcs when the radius is > 0.
3. WHEN a batch has a `polygon` item with centre, circumradius, side count (3–64) and start angle,
   THE SYSTEM SHALL create the regular polygon.
4. WHEN a batch has a `text` item with position, height and string, THE SYSTEM SHALL create the
   same geometry as the `TEXT` command with the Hershey font.
5. WHEN a batch has a `linear_array` or `polar_array` item wrapping other items, THE SYSTEM SHALL
   create the copies at the given step or angle, the count included.
6. IF an item is invalid, or the expanded count exceeds 1000, THEN THE SYSTEM SHALL refuse the
   whole batch naming the failing path, as today.
7. WHEN a batch is accepted, THE SYSTEM SHALL commit it as one step and one undo, as today.

## Out of scope

- Running any code or expression language (Python, Rhai, arithmetic strings).
- New entity kinds (polylines stay exploded lines and arcs).
- Nested arrays beyond one level.

## Open questions

- Needs an amendment to ADR 0010 (new item kinds, expansion before validation).
- Rounded-rect radius larger than half the short side: refuse or clamp? (Default: refuse.)
