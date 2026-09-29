# LCV-170 — SVG conformance corpus and export audit

- **Status**: Draft
- **Depends on**: LCV-156
- **Implementation**: -

## Problem

The user set the SVG target on 2026-09-29 (`docs/research/svg-spec-coverage.md` §1): export is an
SVG 2 *Conforming SVG Generator*, import a *Conforming SVG Interpreter* in secure static mode.
Nothing proves either today. The export tests (`src/io/svg/export/tests.rs`) pin LaserCAD's own
shape, not SVG conformance. A layer name with a C0 control character is accepted by
`document/layer.rs::check_fields` and written raw by `io/svg/layers.rs::xml_escape`, which may
produce a file that is not well-formed XML and that LaserCAD itself cannot reopen. Later SVG specs
(LCV-171..179) need a shared corpus of foreign files to test against.

## Stories

- As an operator, I want every SVG LaserCAD writes to be valid SVG 2, so that any viewer or laser
  tool opens it.
- As a maintainer, I want a corpus of real-world SVG files with expected geometry, so that each
  import change is measured against the spec, not against our own exporter.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Changing what the importer accepts (LCV-171..179).
- `.svgz` reading, unless /specify decides it belongs here.

## Open questions

- Oracle: `usvg` as a dev-dependency (flatten and compare within a tolerance) or hand-written
  expected geometry? New dev-dependency needs an ADR.
- Corpus sources and licences (resvg tests, W3C SVG 1.1 suite, own Inkscape/Illustrator samples).
- Control characters in layer names: refuse at `check_fields`, or escape as character references?
