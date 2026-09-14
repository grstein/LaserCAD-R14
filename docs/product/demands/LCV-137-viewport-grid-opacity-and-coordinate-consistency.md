# LCV-137 - Visible grid and consistent viewport coordinates

- **Status**: Draft
- **Phase**: 11
- **Depends on**: LCV-032, LCV-033, LCV-034, LCV-120
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: -

## Problem

The opaque bed fill covers the grid inside the drawing area. Wheel zoom and
grid bounds mix application-global and viewport-local coordinates, while
`Camera::pan` subtracts screen Y despite the Y-up world projection.

## Scope

Correct the existing rendering order and coordinate boundaries, preserving
grid spacing, exterior dimming, geometry, preview, snap and navigation gestures.

## Out of scope

New grid/theme controls, rulers, automatic camera fitting, SVG changes, a
rendering rewrite or new repaint sites.

## Acceptance criteria

1. Paint canvas background, bed background, enabled grid, bed border/exterior overlay, entities, selection, preview and snap in that order.
2. Minor/major grid lines remain visible inside the bed; GRID off removes them without removing the bed. Preserve outside-bed dimming.
3. Pointer unprojection and wheel anchoring subtract the viewport origin exactly once; projection adds it exactly once. Grid bounds come from local viewport corners.
4. Wheel zoom preserves the world point beneath the pointer within 0.5 logical point after reprojection, including nonzero viewport X/Y origins and an open agent panel.
5. Grid intersections align with geometry at known millimeter coordinates within 0.5 logical point across tested zoom levels.
6. Middle-drag follows both screen axes: world-center delta is `(-dx * mm_per_px, +dy * mm_per_px)`. Positive, negative and diagonal drags agree.
7. Cursor readout, preview, snap and committed endpoint agree after navigation. Navigation does not mutate document/history or change SVG bytes or repaint guards.

## Expected tests

- AC 1-2: real-frame paint ordering and GRID on/off, with a control that fails when the opaque fill is moved above the grid.
- AC 3-5: wheel and grid/entity alignment through the real App with nonzero viewport origin and resized panels.
- AC 6: actual pointer drag sequences and projected displacement, not X-only camera-state assertions.
- AC 7: hover/snap/click after navigation, history/export comparison, and existing LCV-120 idle regressions.
- Manual: inside/outside bed visibility, diagonal pan and pointer-anchored zoom.

## Open questions

None at product level; investigate each boundary rather than making two wrong
conversions compensate for one another.

## Notes

Primary files: `src/app/viewport.rs`, `src/render/camera.rs`,
`src/render/grid.rs`, `src/render/bed.rs`, `src/render/mod.rs`.
The existing pan comment is not evidence that its Y sign is correct.
