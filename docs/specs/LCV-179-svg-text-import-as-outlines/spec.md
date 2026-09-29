# LCV-179 — SVG text import as outlines

- **Status**: Draft
- **Depends on**: LCV-175, LCV-177
- **Implementation**: -

## Problem

An SVG `<text>` is not imported at all, so labels and engraved lettering disappear unless the
author converted text to paths first. The user decided on 2026-09-29 that import converts text to
outlines with system fonts (`docs/research/svg-spec-coverage.md` §1). Export keeps no live text:
TEXT/DTEXT writes Hershey single-stroke lines (`text/layout.rs::layout_text`).

## Stories

- As an operator, I want text in a foreign SVG to arrive as curves in the right font, size and
  place, so that I can engrave or cut it.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Live text entities, text editing after import, exporting `<text>`.

## Open questions

- Dependencies (e.g. `fontdb`, `ttf-parser`, `rustybuzz`) and an ADR; kernel purity.
- Missing font: fallback font and report, or refuse the text element and report?
- Scope of `tspan` positioning, `textPath`, `inline-size` wrapping, vertical writing modes.
