# LCV-162 — Crosshair cursor and screen-space picking

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

The canvas shows the OS arrow and nothing else: no crosshair and no pickbox, so the operator
cannot see which point or which entity the next click will take. Pointer tolerances also mix
two spaces. The snap aperture is 12 pt on screen (`app/snap.rs::SNAP_TOLERANCE_PX`). The pick,
trim and extend radii are 5 mm in the world (`tools/select/hit.rs::PICK_THRESHOLD_MM`,
`tools/trim.rs::PICK_THRESHOLD_MM`, `tools/extend.rs::PICK_RADIUS_MM`), and the drag threshold is
2 mm (`tools/select/mod.rs::DRAG_THRESHOLD_MM`). Zoomed out, a 5 mm pick grabs the wrong
entity. Zoomed in, the pick needs pixel-exact aim. COPY (LCV-157) and ROTATE/MIRROR/SCALE
(LCV-158) add more picking on top of this.

## Stories

- As an operator, I want an R14 crosshair that spans the canvas, so that I can line points up
  by eye before I click.
- As an operator, I want a pickbox when a tool asks me to pick an entity, so that I know what my
  click can hit.
- As an operator, I want picking to feel the same at every zoom level.

## Direction

- The canvas draws a full-width, full-height crosshair at the cursor (resolved) position and
  hides the OS cursor over the canvas (`egui::CursorIcon::None`). The crosshair is painted last
  (after the LCV-137 layers) from a new `render/cursor.rs`.
- While a tool waits for an entity pick (Select idle, Trim, Extend, Delete), a small square
  pickbox sits at the crosshair centre. Its size is the pick tolerance.
- Every pointer tolerance — snap aperture, pick radius, drag threshold — is set in screen
  points and turned into millimetres through the camera at the moment of use.
- DESIGN.md §5 and §6 are updated in this spec's last task.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Crosshair size or colour settings, and a crosshair-as-percentage option.
- Snap tracking lines and polar tracking.
- The snap glyph's look (LCV-164).

## Open questions

- Should the crosshair follow the snapped point or the raw pointer? R14 moves the crosshair to
  the snapped point. That is only honest once feedback reflects the current frame (fast-lane
  fix F1 in DESIGN.md).
- Pickbox size: R14 uses 3 px by default. Proposal: 5 pt, the same as the pick radius.
- Should the drag threshold get its own screen value, or reuse the pick radius?
