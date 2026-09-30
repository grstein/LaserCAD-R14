# LCV-172 — Full SVG path-data grammar

- **Status**: Planned
- **Depends on**: LCV-171
- **Implementation**: -

## Problem

`io/svg/import.rs::parse_path` splits `d` on whitespace and reads exactly
`M x y A rx ry φ f f x y`. Relative `m`/`a` are read as absolute (wrong geometry, no error), and
tokens after the 11th are dropped (lost subpaths). `L H V Z`, commas, compact syntax, exponents
and glued flags make the path vanish. An arc with rx ≠ ry, or with any rotation, fails the whole
file with `MalformedPath`, even when rx = ry, where φ is meaningless. Inkscape and most tools
write all geometry as `<path>`.

## Stories

- As an operator, I want any valid path from Inkscape, Illustrator or LightBurn to open exactly,
  so I can cut drawings I did not make in LaserCAD.

## Acceptance criteria

1. WHEN `d` is read THE SYSTEM SHALL tokenize it per the SVG 2 path-data grammar: comma or
   whitespace separators; a sign or a second `.` starts a new number (`M1-2.5.5` is `1 -2.5 .5`);
   exponents; single-character flags, glued allowed (`a5 5 0 1110 10`); implicit repetition of
   the previous command, extra `M`/`m` pairs read as `L`/`l`.
2. WHEN a path uses `M L H V Z A` in absolute or relative form THE SYSTEM SHALL resolve each
   segment against the current point and import every subpath of the `d`.
3. WHEN an `L`, `H` or `V` segment has non-zero length THE SYSTEM SHALL import it as a line; a
   zero-length segment yields nothing.
4. WHEN `Z` closes a subpath whose current point differs from its start THE SYSTEM SHALL import
   the closing line and move the current point to the subpath start.
5. WHEN an `A` segment has `|rx| = |ry|` within `EPSILON` THE SYSTEM SHALL import a circular arc,
   ignoring `φ`, with today's centre and sweep rules.
6. WHEN an arc's radii are out of range THE SYSTEM SHALL apply SVG 2 §F.6.6: equal endpoints omit
   the segment; `rx = 0` or `ry = 0` gives a line; negative radii take their absolute value;
   `λ > 1` scales both radii by `√λ`.
7. WHEN a segment is `C S Q T`, or an `A` with `|rx| ≠ |ry|`, THE SYSTEM SHALL import nothing for
   it, advance the current point to its endpoint, and add `path C|S|Q|T|elliptical arc` to the
   LCV-171 report.
8. IF `d` has a syntax error THEN THE SYSTEM SHALL import the segments before the command holding
   the error, add `path (data error)` to the report, and still open the file.
9. WHEN LaserCAD's own export is reopened THE SYSTEM SHALL produce the same entities as before
   this change, the half-turn cases of `tests/it/io_svg/roundtrip_props.rs` included.
10. WHEN the LCV-170 corpus runs THE SYSTEM SHALL pass new fixtures with hand-checked `.expected`
    files for relative commands, compact syntax, several subpaths, `Z`, out-of-range radii and a
    mid-path error.

## Out of scope

- Transforms and units (LCV-173).
- Elliptical-arc and Bézier entities (LCV-176, LCV-177): until then they are reported, never
  approximated.

## Open questions

- None. Decided (self-approved per user goal): the tokenizer is hand-written in a new kernel file
  under `src/io/svg/` (no `svgtypes`, no new dependency, no ADR); a path error imports up to the
  error and reports it (SVG 2 "render up to the error") instead of failing the file.
