# LCV-171 — SVG import report and never-rendered elements

- **Status**: Draft
- **Depends on**: LCV-170
- **Implementation**: -

## Problem

`io/svg/import.rs::Walk::collect` imports `<line>`, `<circle>` and `<path d="M…A…">` wherever
they sit and silently skips everything else. Geometry inside never-rendered elements (`defs`,
`symbol`, `clipPath`, `mask`, `marker`, `pattern`, gradients) is imported as cut geometry, while
`<image>`, `<text>`, unknown paths and paint-only features vanish with no word to the operator.
The SVG 2 rendering model says never-rendered content is not drawn, and the conformance target
(`docs/research/svg-spec-coverage.md`) forbids silent loss or misreading.

## Stories

- As an operator, I want Open/Import to tell me what it ignored and why, so that I never cut a
  drawing that silently lost or gained parts.
- As an operator, I want content the SVG file would not display to stay out of my drawing.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Supporting the ignored features themselves (LCV-172..179).
- `display:none` via CSS (LCV-175 owns the cascade).

## Open questions

- Where the report shows: command-line history, a dialog, or both; wording and grouping.
- Root namespace check: refuse a root `<svg>` outside the SVG namespace?
