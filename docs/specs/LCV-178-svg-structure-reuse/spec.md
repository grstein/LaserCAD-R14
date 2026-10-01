# LCV-178 — SVG structure reuse: use, defs, symbol, switch

- **Status**: Done
- **Depends on**: LCV-173, LCV-175
- **Implementation**: 0ab2608..3455629, c48683e..61187fb, 1e9f04b, 760b9d4, cc5c4ea

## Problem

Import ignores `<use>` (`href` and the deprecated `xlink:href`), so repeated parts drawn by
reference — common in CAD exports and icon sets — are lost, while after LCV-171 the referenced
`<defs>`/`<symbol>` content is correctly not drawn on its own. `<switch>` imports every branch
instead of the first whose conditions pass. SVG 2 ch. 5 defines the shadow-tree semantics
(`x`/`y` offset, `symbol` viewport, styles inherited from the `<use>`).

## Stories

- As an operator, I want every instance placed by `<use>` to open as real geometry at its place.

## Acceptance criteria

1. WHEN a `<use>` references `#id` of a graphics element or group, THE SYSTEM SHALL import a copy
   of its geometry under the `<use>`'s transform followed by `translate(x, y)`.
2. WHEN a `<use>` has both `href` and `xlink:href`, THE SYSTEM SHALL follow `href`.
3. WHEN a `<use>` references a `<symbol>`, THE SYSTEM SHALL map the symbol's `viewBox` into the
   viewport of the `<use>`'s `width`/`height` (else the symbol's, else 100%) with the LCV-173
   `preserveAspectRatio` rules.
4. WHEN an instance is imported, THE SYSTEM SHALL resolve inherited properties from the `<use>`,
   and the referenced elements' own `style`, attributes and matching `<style>` rules as in place.
5. WHEN an instance is imported, THE SYSTEM SHALL place it on the layer the `<use>` itself would
   land on (its `<g data-layer>`, or the LCV-175 color → layer rule).
6. WHEN referenced content contains another `<use>`, THE SYSTEM SHALL expand it recursively.
7. IF a `<use>` would re-enter an element it is already expanding (a cycle), THEN THE SYSTEM SHALL
   skip that `<use>` and count it in the import report.
8. IF `href` is missing, names no element in the file, or is not a same-document `#id`, THEN THE
   SYSTEM SHALL skip the `<use>`, fetch nothing and count it in the import report.
9. IF `<use>` nesting exceeds depth 32 or expansion would create more than 100 000 entities, THEN
   THE SYSTEM SHALL refuse the file with an error naming the limit and leave the open document
   unchanged.
   *Amended by the LCV-178 review fix:* the limit counts instanced elements: every `<use>`
   expansion and every entity it creates, so a fan-out that draws nothing is bounded too; more
   than 100 000 instanced elements refuse the file.
10. WHEN a `<switch>` is imported, THE SYSTEM SHALL import only its first direct child whose
    conditions pass and count the others in the import report.
11. WHEN evaluating conditions on any element, THE SYSTEM SHALL fail a non-empty
    `requiredExtensions`, pass `systemLanguage` only if it lists `en` or an `en-*` tag, ignore
    `requiredFeatures`, and not import an element whose conditions fail (counted in the report).
12. WHEN instances are imported, THE SYSTEM SHALL create independent entities: editing one changes
    no other.

## Out of scope

- Keeping instances linked (blocks/xref are not in LaserCAD's model); any export change.
- External references (`href` to another file or URL): forbidden by secure static mode.

## Open questions

- None. Decided (self-approved per user goal): cycles are skipped and reported; depth > 32 or
  more than 100 000 generated entities refuse the whole file (no partial import);
  `systemLanguage` matches English, the UI language, for deterministic results.
