# LCV-174 — SVG basic shapes on import

- **Status**: Draft
- **Depends on**: LCV-173, LCV-176
- **Implementation**: -

## Problem

Import reads `<line>` and `<circle>` only. `<rect>` (with `rx`/`ry`, including SVG 2 `auto`),
`<polyline>`, `<polygon>` and `<ellipse>` are skipped silently. `io/svg/import.rs::parse_circle`
refuses `r = 0` and a missing geometry attribute is an error, while SVG 2 ch. 10 says `r = 0`
disables rendering and missing attributes default to 0.

## Stories

- As an operator, I want rectangles, rounded rectangles, polylines, polygons and ellipses from any
  SVG to open as exact geometry.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Exporting these shapes as SVG shape elements; LaserCAD keeps writing its own entities.

## Open questions

- A rectangle as four lines, or as a closed group the operator can select as one?
