# LCV-172 — Full SVG path-data grammar

- **Status**: Draft
- **Depends on**: LCV-171
- **Implementation**: -

## Problem

`io/svg/import.rs::parse_path` splits `d` on whitespace and reads exactly `M x y A rx ry φ f f x y`
(11 tokens). Relative `m`/`a` are read as absolute (wrong geometry, no error); tokens after the
11th are dropped (lost subpaths); `L H V Z C S Q T`, commas, compact syntax, exponents and glued
flags make the path vanish silently. Out-of-range arc radii, which SVG 2 corrects (rx = 0 → line,
negative → absolute, λ > 1 → scale by √λ), fail the whole file with `MalformedPath`; a semicircle
reopened after `export.rs::encode_entity`'s `{:.4}` rounding is suspected to hit this. Inkscape
and most tools write everything as `<path>`.

## Stories

- As an operator, I want any valid path from Inkscape, Illustrator or LightBurn to open exactly,
  so that I can cut drawings I did not make in LaserCAD.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Transforms and units (LCV-173).
- Entities for elliptical arcs (LCV-176) and Béziers (LCV-177): until they exist, those segments
  are reported by LCV-171, never approximated.

## Open questions

- Grammar implementation: `svgtypes` (new dependency, ADR) or a hand-written tokenizer?
- Error policy: import up to the first error and report it (SVG 2 "render up to the error"), or
  refuse the file?
