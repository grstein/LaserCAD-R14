# LCV-162 — Crosshair cursor and screen-space picking

- **Status**: Specified
- **Depends on**: none
- **Implementation**: -

## Problem

The canvas shows the OS arrow and nothing else, so the operator cannot see which point or which
entity the next click will take. Pointer tolerances mix two spaces: the snap aperture is 12 pt
(`app/snap.rs::SNAP_TOLERANCE_PX`), but the entity picks of Select, TRIM and EXTEND are 5 mm in the
world (`tools/select/hit.rs::PICK_THRESHOLD_MM`, `tools/trim.rs::PICK_THRESHOLD_MM`,
`tools/extend.rs::PICK_RADIUS_MM`) and the box-drag threshold is 2 mm
(`tools/select/mod.rs::DRAG_THRESHOLD_MM`). Zoomed out, a 5 mm pick grabs the wrong entity; zoomed
in, it needs pixel-exact aim. The canvas also paints before it handles input
(`app/viewport.rs::draw`, DESIGN.md F1), so any cursor feedback would trail by a frame. COPY,
MOVE, ROTATE, MIRROR, SCALE and DIST pick points only (snap aperture); ERASE acts on the
selection. DESIGN.md §5 and §6 carry the gap tags this spec closes.

## Stories

- As an operator, I want an R14 crosshair across the canvas, so that I can line points up by eye.
- As an operator, I want a pickbox when a tool asks me to pick an entity, so that I know what my
  click can hit.
- As an operator, I want picking to feel the same at every zoom level.

## Acceptance criteria

1. WHILE the pointer is over the canvas THE SYSTEM SHALL set the OS cursor icon to none.
2. WHILE the pointer is over the canvas THE SYSTEM SHALL paint one horizontal and one vertical
   1 pt line spanning the full canvas rect through the crosshair point, in a `cursor` colour
   with ≥3:1 contrast on the bed fill, as the last canvas shapes (after the snap glyph).
3. WHEN a snap or Ortho resolves the cursor THE SYSTEM SHALL place the crosshair point at the
   resolved point; otherwise at the pointer.
4. WHEN the pointer moves THE SYSTEM SHALL paint the crosshair and the snap glyph from that
   frame's pointer position (input handled before painting).
5. WHILE the active tool waits for an entity pick (Select idle, TRIM, EXTEND) THE SYSTEM SHALL
   paint a hollow square pickbox of side 2 × the pick aperture, centred on the crosshair point.
6. WHILE the active tool waits for a point (e.g. LINE, COPY, MOVE, ROTATE, MIRROR, SCALE, DIST)
   or a Select box drag is in progress THE SYSTEM SHALL paint no pickbox.
7. WHEN the operator clicks in Select at a screen distance ≤ the pick aperture (5 pt) from an
   entity THE SYSTEM SHALL pick it, and SHALL NOT pick one farther away, at 0.05 and 20 mm/pt.
8. WHEN the operator clicks in TRIM or EXTEND THE SYSTEM SHALL apply the same pick aperture in
   points as AC 7, at the same two zoom levels.
9. WHEN a Select press moves more than the drag threshold (2 pt) on screen THE SYSTEM SHALL start
   a box drag, and SHALL NOT before, at the same two zoom levels.
10. WHEN the pointer leaves the canvas THE SYSTEM SHALL paint no crosshair or pickbox and restore
    the default OS cursor.

## Out of scope

- Crosshair size or colour settings; a crosshair-as-percentage option.
- Snap/polar tracking; the snap glyph's look (LCV-164); pick-target hover highlight (LCV-163).

## Open questions

- None.
