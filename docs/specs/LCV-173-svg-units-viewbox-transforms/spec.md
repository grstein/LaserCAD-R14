# LCV-173 — SVG lengths, units, viewBox and transforms

- **Status**: Done
- **Depends on**: LCV-172
- **Implementation**: 03e25bc..2b0c992

## Problem

`io/svg/header.rs::parse_bed` reads unitless `width`/`height` as mm, refuses `px pt in cm %`,
and needs a `viewBox` of `0 0 W H` that is never a scale. `preserveAspectRatio`, nested `<svg>`
and every `transform` are ignored (LCV-171 only reports them), so a typical Inkscape file opens
misplaced. SVG 2 ch. 8 defines all of these (96 px = 1 in).

## Stories

- As an operator, I want a px/pt/in file with a scaled viewBox and transformed groups to open
  at its true size and position in mm.

## Acceptance criteria

1. WHEN a length carries `mm cm Q in pt pc px` or no unit THE SYSTEM SHALL convert it to mm at
   96 px = 1 in; a unitless length is px.
2. WHEN the root has `width` and `height` in absolute units THE SYSTEM SHALL take them as the bed;
   IF either is `%`, `em` or `ex` or absent THEN THE SYSTEM SHALL use the viewBox size as px, else
   the default bed.
3. IF the resulting bed side is outside 1..=2000 mm THEN THE SYSTEM SHALL refuse the file with
   `MalformedBedDimension` (LCV-114 AC 9 kept).
4. WHEN the root has a `viewBox` with any origin and positive size THE SYSTEM SHALL map user units
   into the bed per `preserveAspectRatio` (all align values, `meet`, `slice`, `none`; default
   `xMidYMid meet`); without a viewBox one user unit is 1 px.
5. WHEN an element or ancestor carries `transform` THE SYSTEM SHALL apply the accumulated matrix
   (`matrix translate scale rotate(a[ cx cy]) skewX skewY`, comma or space separated) in f64
   before the world Y mirror, and SHALL no longer report `transform` (amends LCV-171 AC 7).
6. WHEN a circle or circular arc is under a matrix that is a similarity (uniform scale, rotation,
   translation, reflection) THE SYSTEM SHALL import it with mapped centre, radius scaled and, for
   a reflection, `ccw` inverted.
7. WHEN a circle or arc is under a non-similarity matrix THE SYSTEM SHALL import nothing for it
   and report `circle|arc (non-uniform transform)` until LCV-176 represents the ellipse.
8. IF a `transform` value does not parse THEN THE SYSTEM SHALL treat it as absent and report
   `transform (invalid)`.
9. WHEN a nested `<svg>` is met THE SYSTEM SHALL apply its `x y width height viewBox
   preserveAspectRatio` as a further mapping, import its content unclipped, report `svg (not clipped)`.
10. WHEN a geometry attribute (`x1 y1 x2 y2 cx cy r`) carries a unit or `%` THE SYSTEM SHALL
    convert it, `%` resolved against the nearest viewport (x by width, y by height, r by the
    normalized diagonal).
11. WHEN LaserCAD's own export (`width="Wmm" viewBox="0 0 W H"`) is reopened THE SYSTEM SHALL
    produce exactly today's entities and bed.
12. WHEN the LCV-170 corpus runs THE SYSTEM SHALL pass new fixtures for px/pt/in files, an offset
    viewBox, `preserveAspectRatio`, nested transformed groups and a nested `<svg>`.

## Out of scope

- Ellipses from non-uniform transforms (LCV-176); clipping to nested viewports.

## Open questions

- None. Decided (self-approved per user goal): unitless is px per SVG 2, and unit tests pinning
  "unitless = mm" or "`px` refused" are rewritten; an out-of-range bed stays an error, never
  clamped; a relative root size falls back to the viewBox; nested `<svg>` content is imported
  whole and reported, not clipped.
