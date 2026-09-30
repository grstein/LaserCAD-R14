# LCV-179 — SVG text import as outlines

- **Status**: Specified
- **Depends on**: LCV-173, LCV-175, LCV-177
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

1. WHEN a `<text>` names an installed `font-family`, THE SYSTEM SHALL import each glyph's outline
   as closed contours of line, quadratic and cubic entities, the first glyph's origin at (`x`,`y`),
   scaled by `font-size` and under the element's transform.
2. WHEN `font-family` lists several families or a generic one (`serif`, `sans-serif`,
   `monospace`), THE SYSTEM SHALL use the first installed family, the generic mapped to the font
   database default; `font-weight` and `font-style` pick the nearest face.
3. IF no listed family is installed, THEN THE SYSTEM SHALL use the default sans-serif face and
   report the substitution; IF no font is installed at all THEN skip the text and report it.
4. WHEN glyphs follow each other, THE SYSTEM SHALL advance by each glyph's horizontal advance,
   without shaping, ligatures or kerning.
5. WHEN `text-anchor` is `middle` or `end`, THE SYSTEM SHALL shift each text chunk left by half or
   all of its advance width.
6. WHEN a `<text>` or `<tspan>` gives `x`, `y`, `dx` or `dy` lists, THE SYSTEM SHALL apply the
   i-th value to the i-th character, later characters continuing from the advance.
7. WHEN text holds runs of whitespace, THE SYSTEM SHALL collapse each run to one space and trim
   the ends, unless `xml:space="preserve"`, which keeps every space (newlines and tabs as spaces).
8. IF a character has no glyph in the chosen face, THEN THE SYSTEM SHALL advance by the `.notdef`
   advance, draw nothing and count it in the import report.
9. IF text uses `textPath` or a vertical `writing-mode`, THEN THE SYSTEM SHALL skip that element
   and report it; IF it uses `inline-size`, `rotate`, `letter-spacing` or `word-spacing` THEN lay
   it out without them and report it.
10. WHEN text outlines are imported, THE SYSTEM SHALL place them by the LCV-175 layer rules (the
    fill color standing in for a missing stroke) as ordinary entities, with no live text.
11. WHEN a document holding imported text outlines is exported, THE SYSTEM SHALL write them as the
    LCV-177 and line forms and never write `<text>`.
12. WHEN the same file is opened twice with the same font database, THE SYSTEM SHALL produce
    identical entities; tests load a bundled OFL font file, never the system fonts.

## Out of scope

- Live text entities, text editing after import, exporting `<text>`.
- Shaping (complex scripts, ligatures, kerning), `@font-face`/web fonts, `textLength`.

## Open questions

- None. Decided (self-approved per user goal): `fontdb` + `ttf-parser`, no `rustybuzz`, under a
  new ADR, with font code kernel-pure (no `egui`); a missing family falls back to the default
  sans-serif and is reported; per-character `x`/`y`/`dx`/`dy` supported; `textPath` and vertical
  text skipped and reported; `inline-size` wrapping ignored and reported.
