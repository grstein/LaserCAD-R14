# LCV-176 — Ellipse and elliptical-arc entities

- **Status**: Specified
- **Depends on**: LCV-172, LCV-173
- **Implementation**: -

## Problem

The document holds `Entity::{Line, Circle, Arc}` only (`document/entity.rs`). SVG `<ellipse>`,
elliptical `A` segments (rx ≠ ry), and circles under non-uniform scale or skew cannot be
represented, so import must refuse or report them. The user chose native entities on 2026-09-29
(`docs/research/svg-spec-coverage.md` §1).

## Stories

- As an operator, I want ellipses and elliptical arcs to open, display, select, snap, move and
  export exactly, so that a round trip through LaserCAD changes nothing.

## Acceptance criteria

1. WHEN a path `A` segment has rx ≠ ry after radius correction, THE SYSTEM SHALL import one
   elliptical arc with the segment's endpoints (within `EPSILON`), rotation, side and direction.
2. WHEN an `<ellipse>` is imported, THE SYSTEM SHALL create a full ellipse (a circle if rx = ry; a
   missing or `auto` radius equals the other); IF rx or ry ≤ 0 THEN skip it and report it.
3. WHEN a circle or circular arc is under a non-uniform scale or skew, THE SYSTEM SHALL import the
   exact ellipse or elliptical arc (replacing the LCV-173 report).
4. WHEN an ellipse is painted, THE SYSTEM SHALL draw it with every painted vertex on the curve and
   a chord deviation ≤ 0.5 px, styled like any entity of its layer and selection state.
5. WHEN the operator clicks within the pick aperture of an ellipse's curve, or a selection box
   covers it, THE SYSTEM SHALL select it as it would an arc.
6. WHEN MOVE, COPY, ROTATE, MIRROR or SCALE acts on an ellipse, THE SYSTEM SHALL transform it
   exactly (mirror negates its rotation and reverses an arc's direction), undoable as one step.
7. WHEN snapping near an ellipse, THE SYSTEM SHALL offer Endpoint (arc ends), Center, Quadrant (the
   four axis vertices lying within the span) and Nearest; no other snap kind.
8. IF TRIM or EXTEND is given an ellipse as target, THEN THE SYSTEM SHALL leave the document
   unchanged and say "Cannot trim/extend an ellipse" in the status bar; ellipses are no boundary.
9. WHEN a full ellipse is exported, THE SYSTEM SHALL write `<ellipse cx cy rx ry/>` in SVG
   coordinates, adding `transform="rotate(a cx cy)"` with `a = −rotation` in degrees only when
   the rotation is not 0.
10. WHEN an elliptical arc is exported, THE SYSTEM SHALL write
    `<path d="M sx sy A rx ry φ large sweep ex ey"/>` with `φ = −rotation` in degrees, `sweep`
    inverted by the mirror as for circular arcs and `large` unchanged.
11. WHEN a document without ellipses is exported, THE SYSTEM SHALL write bytes identical to today.
12. WHEN a document with ellipses is saved and reopened, THE SYSTEM SHALL restore each center,
    radius, rotation and end point within `EPSILON` and each on its layer.
13. WHEN the agent calls `query_entities`, THE SYSTEM SHALL list each ellipse with kind `ellipse`,
    `cx`, `cy`, `rx`, `ry`, `rotation_deg` and, for an arc, `start_deg`, `end_deg`, `ccw`.

## Out of scope

- TRIM/EXTEND/intersections, Perpendicular/Tangent/Midpoint snaps on ellipses (follow-up).
- An ELLIPSE drawing command; agent creation of ellipses (`create_drawing` unchanged).

## Open questions

- None. Decided (self-approved per user goal): one ellipse entity kind covering full ellipses and
  arcs; export `<ellipse>` (+ `rotate` transform only when rotated) and `A rx ry φ`, an additive
  change to the AGENTS.md export contract for the new kind only; the `<ellipse>` element is
  imported here, not in LCV-174; the agent sees ellipses read-only.
