# LCV-196 — Batch drawing primitives

- **Status**: Done
- **Depends on**: LCV-185, LCV-188, LCV-192
- **Implementation**: 75120e9..2c23133, 3e2c499, e4e3ae3

## Problem

`create_drawing` (ADR 0010) accepts only line, circle and arc. A rounded plate, a hexagon, a row of
holes or a label must be spelled out entity by entity, so the model does the geometry in its head
and payloads grow. Benchmark-leading CAD agents write *code* (CadQuery, bpy) for this
expressiveness. LaserCAD runs no code; a few declarative primitives give most of the gain while
staying inspectable and bounded.

## Stories

- As an operator, I want the agent to draw polylines, rectangles, polygons, text and arrays in one
  batch so that complex parts come out right the first time.

## Acceptance criteria

New items expand into lines, circles and arcs in batch order; the 1000-entity cap applies to the
expanded count. LCV-185's rule holds: another type's key is tolerated only when `null`.

1. WHEN an item is `polyline {points, closed}` with 2–1000 `{x, y}` points, THE SYSTEM SHALL create
   one line per segment, plus the closing segment when `closed` is true (3+ points).
2. WHEN an item is `rect {x, y, width, height, corner_radius}` with `(x, y)` its lower-left corner,
   THE SYSTEM SHALL create four lines, or with `corner_radius` > 0 four CCW quarter arcs and the
   straight sides that have length.
3. WHEN an item is `polygon {cx, cy, r, sides, start_deg}` with 3–64 sides, THE SYSTEM SHALL create
   the regular polygon inscribed in radius `r`, its first vertex at `start_deg`.
4. WHEN an item is `text {x, y, height, text}`, THE SYSTEM SHALL create exactly the geometry the
   `TEXT` command creates for that insertion point, height and string.
5. WHEN an item is `linear_array {of, count, dx, dy}`, THE SYSTEM SHALL add `count − 1` copies of
   the listed earlier items, copy k moved by (k·dx, k·dy).
6. WHEN an item is `polar_array {of, count, cx, cy, step_deg}`, THE SYSTEM SHALL add `count − 1`
   copies, copy k rotated by k·`step_deg` (CCW) about (cx, cy).
7. WHEN an array's `of` lists another array, THE SYSTEM SHALL repeat that array's whole output
   (a grid); an array listed by such an array SHALL itself list no array.
8. IF a value is invalid — a repeated consecutive point, `corner_radius` above half the shorter
   side, `count` outside 2–1000, `of` naming a later item, itself or a nested-too-deep array, a
   string empty, over 256 characters or with control characters — or the expanded count exceeds
   1000, THEN THE SYSTEM SHALL refuse the whole batch in the LCV-192 shape naming the path.
9. WHEN a batch is accepted, THE SYSTEM SHALL commit it as one step and one undo, and report the
   expanded count and index range, as today.
10. WHEN the schema and the prompt describe `create_drawing`, THE SYSTEM SHALL list every new type
    and key, still without `oneOf`/`anyOf`/`allOf`/`const`/`additionalProperties`.

## Out of scope

- Running code or expressions (Python, Rhai, arithmetic strings).
- New entity kinds: a polyline stays exploded lines; rotated text; per-item layers.

## Decisions (self-approved per user goal, 2026-09-30)

- Amends ADR 0010 (new item types, expansion before the cap; `version` stays 1 because every
  existing payload stays valid); architect in `/design`.
- An oversize `corner_radius` is refused, not clamped; a radius of exactly half the short side
  omits the zero-length sides. `corner_radius` and `start_deg` may be omitted or `null` (0).
- Arrays reference earlier items by batch position (`of`), keeping the schema flat; a grid is an
  array of an array, two levels at most.
- `polar_array` rotates each copy (like ROTATE), it does not only move it.

## Open questions

- None.
