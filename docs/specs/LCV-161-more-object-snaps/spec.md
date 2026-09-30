# LCV-161 — Quadrant, perpendicular, tangent and nearest snaps

- **Status**: Specified
- **Depends on**: none
- **Implementation**: -

## Problem

Object snap covers endpoint, midpoint, center and intersection only
(`geometry/snap/mod.rs::SnapKind`), all on or all off with F3. Drawing a tangent line, a
perpendicular or a point on a circle's quadrant needs construction geometry that is later deleted.

## Stories

- As an operator, I want quadrant, perpendicular, tangent and nearest snaps so that I can draw
  precise geometry without helpers.

## Acceptance criteria

1. WHEN the cursor is within the snap aperture of a circle's point at 0°, 90°, 180° or 270°
   (world axes) THE SYSTEM SHALL snap to it as Quadrant; for an arc, only quadrant angles inside
   its sweep count.
2. WHILE the active tool reports an anchor (`Tool::anchor`) and the foot of the perpendicular
   from the anchor onto a line segment, circle or arc lies on that entity and within the aperture
   of the cursor, THE SYSTEM SHALL snap to the foot as Perpendicular (a circle or arc gives up to
   two feet, on the line through its centre and the anchor).
3. WHILE the active tool reports an anchor outside a circle or arc and a tangent point from the
   anchor lies on that entity and within the aperture of the cursor, THE SYSTEM SHALL snap to it
   as Tangent.
4. IF no anchor exists, or the anchor is on or inside the circle (tangent), or coincides with
   the centre (perpendicular to a circle or arc), THEN THE SYSTEM SHALL offer no candidate of
   that kind.
5. WHEN the Nearest kind is on and no other candidate lies within the aperture THE SYSTEM SHALL
   snap to the closest point of the nearest line, circle or arc within the aperture as Nearest.
6. WHEN several candidates lie within the aperture THE SYSTEM SHALL pick the closest one and break
   distance ties by Endpoint > Intersection > Midpoint > Center > Quadrant > Perpendicular >
   Tangent.
7. THE SYSTEM SHALL draw each new kind with its R14 glyph in the `snap` token: Quadrant diamond,
   Perpendicular right-angle mark, Tangent circle with a tangent bar, Nearest hourglass.
8. THE SYSTEM SHALL offer one on/off checkbox per snap kind under View > Object snap, persisted in
   the settings file; the defaults are all kinds on except Nearest, and F3 stays the master switch.
9. WHILE a kind is off THE SYSTEM SHALL offer no candidate of that kind.
10. IF the settings file predates this spec THEN THE SYSTEM SHALL load it with the default kinds.

## Out of scope

- Snap tracking and polar tracking.
- Deferred tangent/perpendicular for a first point (tan-tan and perpendicular-from lines).
- One-shot snap overrides typed at a point prompt (`END`, `PER`, …).
- Intersections involving arcs (`candidates.rs` still skips them; see LCV-160).
- Foot or tangent points on the extension of a segment or arc.

## Open questions

- None.
