# LCV-178 — SVG structure reuse: use, defs, symbol, switch

- **Status**: Draft
- **Depends on**: LCV-173, LCV-175
- **Implementation**: -

## Problem

Import ignores `<use>` (`href` and the deprecated `xlink:href`), so repeated parts drawn by
reference — common in CAD exports and icon sets — are lost, while after LCV-171 the referenced
`<defs>`/`<symbol>` content is correctly not drawn on its own. `<switch>` imports every branch
instead of the first whose conditions pass. SVG 2 ch. 5 defines the shadow-tree semantics
(`x`/`y` offset, `symbol` viewport, styles inherited from the `<use>`).

## Stories

- As an operator, I want every instance placed by `<use>` to open as real geometry at its place.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Keeping instances linked (blocks/xref are not in LaserCAD's model).
- External references (`href` to another file): forbidden by the secure static mode; reported.

## Open questions

- Recursion and size limits for hostile files (deep or cyclic references).
