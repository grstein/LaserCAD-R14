# LCV-177 — Cubic and quadratic Bézier entities

- **Status**: Specified
- **Depends on**: LCV-172
- **Implementation**: -

## Problem

SVG `C`/`S`/`Q`/`T` path segments — the bulk of logos, traced art and text converted to paths —
have no representation: `io/svg/import.rs::parse_path` skips them silently (LCV-172 reports them
until this spec), and the `AGENTS.md` export contract says "never béziers". The user chose native
Bézier entities on 2026-09-29 (`docs/research/svg-spec-coverage.md` §1).

## Stories

- As an operator, I want curved artwork from any SVG to open, display, select, snap to endpoints,
  move and export exactly, so that LaserGRBL receives the original curves.

## Acceptance criteria

1. WHEN a path has a `C` segment, THE SYSTEM SHALL import one cubic Bézier entity whose four
   points equal the segment's absolute points (Y mirrored).
2. WHEN a path has an `S` segment, THE SYSTEM SHALL import a cubic whose first control point is
   the reflection of the previous `C`/`S` second control point, or the current point otherwise.
3. WHEN a path has a `Q` segment, THE SYSTEM SHALL import one quadratic Bézier entity with its
   three points; WHEN it has a `T` segment, the control point is the reflection of the previous
   `Q`/`T` control point, or the current point otherwise.
4. IF every point of a Bézier segment coincides within `EPSILON`, THEN THE SYSTEM SHALL create no
   entity and count it in the import report.
5. WHEN a Bézier is painted, THE SYSTEM SHALL draw it with every painted vertex on the curve and a
   chord deviation ≤ 0.5 px, styled like any entity of its layer and selection state.
6. WHEN ZOOM Extents or a selection box uses a Bézier's extent, THE SYSTEM SHALL use the curve's
   tight bounding box, not its control polygon's.
7. WHEN the operator clicks within the pick aperture of the curve, or a selection box covers it,
   THE SYSTEM SHALL select it as it would a line.
8. WHEN MOVE, COPY, ROTATE, MIRROR or SCALE acts on a Bézier, THE SYSTEM SHALL transform its
   points exactly, undoable as one step.
9. WHEN snapping near a Bézier, THE SYSTEM SHALL offer Endpoint and Nearest; no other snap kind.
10. IF TRIM or EXTEND is given a Bézier as target, THEN THE SYSTEM SHALL leave the document
    unchanged and say "Cannot trim/extend a curve" in the status bar; Béziers are no boundary.
11. WHEN a cubic is exported, THE SYSTEM SHALL write `<path d="M x0 y0 C x1 y1 x2 y2 x3 y3"/>`,
    and a quadratic `<path d="M x0 y0 Q x1 y1 x2 y2"/>`, in SVG coordinates, 4 decimals.
12. WHEN a document without Béziers is exported, THE SYSTEM SHALL write bytes identical to today.
13. WHEN a document with Béziers is saved and reopened, THE SYSTEM SHALL restore each entity's
    kind, points within `EPSILON` and layer.
14. WHEN the agent calls `query_entities`, THE SYSTEM SHALL list each Bézier with kind `cubic` or
    `quadratic` and its points in mm.

## Out of scope

- TRIM/EXTEND/intersections, Midpoint/Perpendicular/Tangent snaps on Béziers (follow-up).
- SPLINE drawing, agent creation of Béziers, a joined path entity, flattening to lines.

## Open questions

- None. Decided (self-approved per user goal): quadratics stay quadratics (byte-exact round trip);
  one entity per segment, no path entity; export `C`/`Q` paths for Bézier entities only while
  lines, circles and circular arcs keep their current form — an additive change to the AGENTS.md
  export contract ("never béziers" becomes "béziers only for Bézier entities"); the agent sees
  Béziers read-only.
