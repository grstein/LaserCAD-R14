# LCV-170 — SVG conformance corpus and export audit

- **Status**: Done
- **Depends on**: LCV-156
- **Implementation**: db5bd4d..8619e8f

## Problem

The 2026-09-29 target (`docs/research/svg-spec-coverage.md` §1) makes export an SVG 2
*Conforming SVG Generator* and import a *Conforming Interpreter* in secure static mode. No test
proves either: export tests pin LaserCAD's own output, not SVG. A layer name with a control
character passes `document/layer.rs::check_fields` and `io/svg/layers.rs::xml_escape` writes it
raw, giving a file that is not well-formed XML 1.0 and cannot be reopened. LCV-171..179 need a
shared corpus.

## Stories

- As an operator, I want every SVG LaserCAD writes to be valid SVG 2, so any tool opens it.
- As a maintainer, I want a corpus with hand-checked expected geometry, so each import change is
  measured against SVG 2 rather than against our own exporter.

## Acceptance criteria

1. THE SYSTEM SHALL keep a corpus in `tests/fixtures/svg/`, each `<name>.svg` paired with a
   plain-text `<name>.expected` listing either the bed, layers (name, color, output, current)
   and entities (kind, world-mm parameters), or the expected import error kind.
2. WHEN the corpus test runs THE SYSTEM SHALL import every `.svg` with `import_svg` and match
   its `.expected` within 1e-6 mm and 1e-9 rad.
3. IF a corpus `.svg` has no parseable `.expected` THEN THE SYSTEM SHALL fail the corpus test and
   name the file.
4. THE SYSTEM SHALL seed the corpus with self-authored files: a v0.2 file with no layers; a v0.3
   mother file with three layers, one with Output off; a `File > Export layers` output file; and
   an Inkscape-style mm file (`sodipodi`/`inkscape` namespaces, `<metadata>`, layer `<g>`) holding
   only line, circle and circular-arc geometry.
5. WHEN `export_svg` or `export_layer_svg` writes an audit-set document THE SYSTEM SHALL produce
   text that `roxmltree` parses, with root `svg` in namespace `http://www.w3.org/2000/svg`. The
   audit set: an empty document; each entity kind; layers with Output on and off and a current
   layer; names with `& < > " '` and non-ASCII characters.
6. WHEN an audit-set export is inspected THE SYSTEM SHALL find only the elements `svg`, `g`,
   `line`, `circle`, `path`, and only the attributes of the `AGENTS.md` export contract.
7. WHEN an audit-set export is inspected THE SYSTEM SHALL find every numeric value finite and in
   SVG 2 `number` syntax, and every `d` matching `M x y A r r 0 f f x y` with `r > 0`.
8. WHEN an audit-set document is exported and reopened THE SYSTEM SHALL restore its bed, its
   layers (order, name, color, output, current) and its entities within 5e-4 mm.
9. IF a layer name contains a character for which `char::is_control` is true THEN
   `check_fields` SHALL refuse it with a `LayerError` naming the problem, for the Layers dialog,
   the command line, the agent and import (`MalformedLayer`).
10. WHEN a document without control characters in layer names is exported THE SYSTEM SHALL write
    output byte-identical to today's; the existing export tests pass unchanged.

## Out of scope

- Changing what the importer accepts (LCV-171..179); `.svgz`; third-party corpus files.

## Open questions

- None. Decided (self-approved per user goal): the oracle is hand-written `.expected` files (no
  `usvg`, no new dependency, no ADR); the corpus is self-authored under the project licence;
  control characters are refused rather than escaped, since escaping them is still invalid XML
  1.0 and would not round-trip.
