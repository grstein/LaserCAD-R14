# LCV-174 — SVG basic shapes on import

- **Status**: Specified
- **Depends on**: LCV-173, LCV-176
- **Implementation**: -

## Problem

Import reads `<line>` and `<circle>` only; `<rect>` (with `rx`/`ry`, including SVG 2 `auto`),
`<polyline>` and `<polygon>` are skipped (reported since LCV-171). `io/svg/import.rs::parse_circle`
refuses `r = 0`, and a missing or unparseable geometry attribute fails the whole file with
`MalformedAttribute`, while SVG 2 ch. 10 says missing attributes default to 0, a zero size
disables rendering, and a negative size is an error for that element only.

## Stories

- As an operator, I want rectangles, rounded rectangles, polylines and polygons from any SVG to
  open as exact geometry, and one bad shape not to block the whole file.

## Acceptance criteria

1. WHEN a `<line>`, `<circle>`, `<rect>` or `<ellipse>` lacks a position attribute (`x1 y1 x2 y2
   cx cy x y`) THE SYSTEM SHALL use 0 for it.
2. WHEN a `<circle>` has `r` missing or 0, or a `<rect>` has `width` or `height` missing or 0, THE
   SYSTEM SHALL import nothing for it and add no report entry.
3. IF a geometry attribute is negative where SVG 2 forbids it (`r width height rx ry`) or does not
   parse THEN THE SYSTEM SHALL import nothing for that element, add `<element> (invalid
   attribute)` to the report and still open the file.
4. WHEN a `<rect>` has zero effective corner radii THE SYSTEM SHALL import four lines along its
   edges.
5. WHEN a `<rect>` has `rx`/`ry` THE SYSTEM SHALL resolve them per SVG 2: a missing or `auto`
   radius equals the other; both `auto` or missing give 0; each is clamped to half the width or
   height.
6. WHEN the resolved radii are equal and positive THE SYSTEM SHALL import four quarter circular
   arcs and the straight sides of non-zero length; WHEN they differ THE SYSTEM SHALL import four
   quarter elliptical arcs (LCV-176) and those sides.
7. WHEN a `<polyline>` has two or more points THE SYSTEM SHALL import one line per consecutive
   pair of distinct points; a `<polygon>` also gets a closing line when its last point differs
   from its first.
8. IF `points` holds an odd count of numbers or a syntax error THEN THE SYSTEM SHALL use the
   points before the error, report `<polyline|polygon> (data error)` and still open the file.
9. WHEN any of these shapes is under a `transform` THE SYSTEM SHALL map it per LCV-173, rounded
   corners becoming elliptical arcs under a non-similarity.
10. THE SYSTEM SHALL no longer report `rect`, `polyline` or `polygon` as ignored elements
    (amends LCV-171 AC 5).
11. WHEN the LCV-170 corpus runs THE SYSTEM SHALL pass new fixtures for a sharp and a rounded
    rect, `auto` and clamped radii, a polyline, a polygon, zero sizes and a negative radius.

## Out of scope

- `<ellipse>` import (LCV-176 owns it); exporting shapes as SVG shape elements (the RECTANGLE
  tool keeps writing four lines, export is unchanged); percentages beyond LCV-173 AC 10.

## Open questions

- None. Decided (self-approved per user goal): a rect imports as four loose lines (plus corner
  arcs), no group, matching the RECTANGLE tool; an invalid or negative shape is skipped and
  reported rather than failing the file.
