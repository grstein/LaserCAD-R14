# LCV-175 — SVG styling cascade, CSS colors and color → layer

- **Status**: Specified
- **Depends on**: LCV-171, LCV-156
- **Implementation**: -

## Problem

Import reads `stroke` only (the `style` attribute and the presentation attribute) and only to
color LaserCAD's own `<g data-layer>` (`io/svg/layers.rs::layer_stroke`); LCV-156 already accepts
any CSS color there (`io/svg/css_color.rs`). `<style>` sheets, `class`, `fill`, `currentColor` and
`display`/`visibility` are ignored, and geometry outside a layer group lands on the first layer.
Inkscape and Illustrator files thus lose their cut/engrave separation and import hidden content.
SVG 2 ch. 6 and 13 define the cascade and CSS `<color>` syntax.

## Stories

- As an operator, I want a foreign SVG's stroke colors to become layers, LightBurn-style, so that
  its cut and engrave parts arrive separated, and hidden content left out and reported.

## Acceptance criteria

1. WHEN a property is set by a presentation attribute, a `<style>` rule and a `style` attribute,
   THE SYSTEM SHALL apply `style` over the rule over the attribute, and `!important` over all.
2. WHEN two `<style>` rules match an element, THE SYSTEM SHALL apply the higher specificity
   (id > class > type > `*`), and the later rule on a tie.
3. WHEN a `<style>` selector is a type, `.class`, `#id`, `*`, a compound of these or a comma list,
   THE SYSTEM SHALL match it; IF it uses a combinator, pseudo-class or attribute selector THEN THE
   SYSTEM SHALL ignore that rule and count it in the import report.
4. WHEN a `<style>` holds `@import`, `@media` or any other at-rule, THE SYSTEM SHALL ignore it,
   fetch nothing and count it in the import report.
5. WHEN an element sets no `stroke`, `fill` or `color`, THE SYSTEM SHALL inherit it from the
   nearest ancestor that does.
6. WHEN `stroke` or `fill` is `currentColor`, THE SYSTEM SHALL use the element's resolved `color`.
7. IF an element or ancestor has `display:none`, or the element resolves `visibility` `hidden` or
   `collapse` (inherited, so a `visible` descendant still imports), THEN THE SYSTEM SHALL import
   none of its geometry and count it in the import report.
8. WHEN geometry outside any `<g data-layer>` resolves a stroke color, THE SYSTEM SHALL place it on
   the first layer with that exact `#rrggbb` color, else on a new layer named `#rrggbb` with that
   color and Output on, appended in order of first appearance.
9. WHEN such geometry has `stroke` none but a `fill` color, THE SYSTEM SHALL use the fill color
   for rule 8 and import the outline only.
10. WHEN such geometry has neither, THE SYSTEM SHALL place it on the first layer, as today; IF the
    file declares no layer and all geometry has a color THEN no empty default `Cut` layer is made.
11. IF a `stroke`, `fill` or `color` value is not a supported CSS color, THEN THE SYSTEM SHALL drop
    that declaration, fall back per rules 1–5 and count it in the import report.
12. WHEN a LaserCAD mother SVG is opened, THE SYSTEM SHALL restore its layers exactly as
    ADR 0012 does today (round trip unchanged).

## Out of scope

- Hatching, dashes, opacity, `stroke-width` (report only); `inkscape:label`; any export change.

## Open questions

- None. Decided (self-approved per user goal): CSS subset = simple/compound selectors and comma
  lists, at-rules ignored and never fetched; color → layer reuses an existing layer of the same
  color first, else one new layer per distinct color named by its hex; fill color stands in for a
  missing stroke.
