# LCV-175 — SVG styling cascade, CSS colors and color → layer

- **Status**: Draft
- **Depends on**: LCV-171, LCV-156
- **Implementation**: -

## Problem

Import ignores `style="…"`, `<style>` sheets, `class`, inheritance and `currentColor`; it reads a
stroke color only on LaserCAD's own `<g data-layer>` and only as `#rrggbb`
(`io/svg/layers.rs::LayerReader::enter`, ADR 0012 §4). Inkscape writes colors in `style`,
Illustrator in `<style>` classes, and hidden layers as `display:none`, so a foreign file loses its
cut/engrave separation and hidden content is imported. SVG 2 ch. 6 and 13 define the cascade and
CSS `<color>` syntax. The user asked on 2026-09-29 that LCV-156 accept any CSS color on layer
groups; if LCV-156 closes without it, this spec supersedes the ADR 0012 §4 color rule.

## Stories

- As an operator, I want a foreign SVG's stroke colors to become layers, LightBurn-style, so that
  its cut and engrave parts arrive separated.
- As an operator, I want hidden content (`display:none`, `visibility:hidden`) left out and
  reported.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Fill-based operations (hatching), dashes, opacity: reported only.
- Inkscape-specific layer attributes (`inkscape:label`) unless /specify adds them.

## Open questions

- CSS subset: which selectors and at-rules (`@media`, `@import` must not fetch)?
- Color → layer mapping: new layer per distinct color, or match existing layers by color first?
