# LCV-173 — SVG lengths, units, viewBox and transforms

- **Status**: Draft
- **Depends on**: LCV-172
- **Implementation**: -

## Problem

`io/svg/header.rs::parse_bed` accepts only `mm` or unitless `width`/`height` and a `viewBox` of
`0 0 W H`; `px`, `pt`, `in`, `cm`, `%` are refused and the viewBox is never a scale.
`preserveAspectRatio`, nested `<svg>` viewports and every `transform`
(`matrix/translate/scale/rotate/skewX/skewY`, on any element or group) are ignored silently, so a
typical Inkscape file (layer groups with `transform="translate(…)"`) opens misplaced. SVG 2 ch. 8
defines all of these (96 px = 1 in).

## Stories

- As an operator, I want a file drawn in px, pt or with a scaled viewBox and transformed groups to
  open at its true size and position in mm.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Non-uniform scale or skew applied to a circle or arc yields an ellipse: represented after
  LCV-176; until then reported, never approximated.

## Open questions

- Bed size from a foreign file whose viewport is larger than the 2000 mm clamp, or in `%`.
- Clip content outside a nested `<svg>` viewport, or import it whole and report?
