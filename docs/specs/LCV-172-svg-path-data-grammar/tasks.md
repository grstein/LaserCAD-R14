# LCV-172 — Tasks

- [x] T1 [AC1] Test (unit, `lexer.rs`): `M1-2.5.5` → `M`, 1, −2.5, 0.5; commas and whitespace in
      any mix; `1e3`, `-1.5E-2`, `.5`, `+2`; flags `1110` → 1, 1, then number 10; errors on `.`,
      `1e`, `-`, `2` as a flag, and `1e999` (non-finite) (files: src/io/svg/path_data/lexer.rs)
- [x] T2 [AC1] Lexer: cursor, `command()`, `number()`, `flag()`, separator skipping (files:
      src/io/svg/path_data/lexer.rs, src/io/svg/path_data.rs, src/io/svg/mod.rs)
- [x] T3 [AC1] [AC2] Test (unit, `path_data.rs`): absolute and relative `M L H V Z A` resolve to
      the expected absolute segments; implicit repetition (`L 1 1 2 2`, `M 0 0 1 1` → move + line,
      `m 1 1 2 2` → relative line); several subpaths; `Z` then `l` starts at the subpath start;
      `C S Q T c s q t` advance the current point to their endpoint and emit `Skipped` with the
      uppercase label (files: src/io/svg/path_data.rs)
- [x] T4 [AC8] Test (unit): `M 0 0 L 10 0 L 5` keeps the first line and sets `error`; `L 1 1`
      (no leading `M`) and `M 0 0 Z 5` are errors; empty and whitespace-only `d` give no segments
      and no error (files: src/io/svg/path_data.rs)
- [x] T5 [AC2] [AC8] `parse_path_data` and the segment enum: command loop, current point, subpath
      start, emitting a segment only once it has fully parsed (files: src/io/svg/path_data.rs)
- [x] T6 [AC9] Test: two exported arc `d` strings joined into one `d` import as the same two
      arcs as when imported separately (the existing golden-path and half-turn tests already pin
      single arcs and stay unchanged) (files: src/io/svg/import/tests.rs)
- [x] T7 [AC3] [AC4] [AC5] [AC6] [AC7] Test (unit, `import/path.rs`): zero-length `L`/`H`/`V`
      yield nothing; `Z` adds the closing line only when the current point differs from the start;
      `rx = ry` with φ = 30° gives the same arc as φ = 0; equal endpoints omit the arc; `rx = 0`
      gives a line; `A -10 -10 …` equals `A 10 10 …`; `λ > 1` gives `r = chord/2`; `rx ≠ ry`
      imports nothing and returns `path elliptical arc` (files: src/io/svg/import/path.rs)
- [x] T8 [AC3]–[AC7] [AC9] `path_entities`: move the arc rebuild from `import.rs::parse_path`
      unchanged, add the §F.6.6 cases and line/Z handling (files: src/io/svg/import/path.rs,
      src/io/svg/import.rs)
- [x] T9 [AC2] [AC7] [AC8] Wire `walk.rs`'s `path` arm to `parse_path_data` + `path_entities`:
      every entity goes on the current layer and every label is noted, with `path (data error)` on
      error. Remove `parse_path`, `tok_f64` and `MalformedPath`, and update the module doc (files:
      src/io/svg/import/walk.rs, src/io/svg/import.rs)
- [x] T10 [AC2] [AC7] [AC8] Rewrite the old tests: the non-numeric `A` imports nothing and reports
      `path (data error)`; `A 0 0 …` imports a line; `M 0 0 L 10 10` imports a line. Also drop
      `MalformedPath` from the corpus kind list (files: src/io/svg/import/tests.rs,
      tests/it/io_svg/import.rs, tests/it/io_svg/corpus/expected.rs)
- [x] T11 [AC7] Test (integration): a file with one path mixing `L`, `C`, `q`, `T` and an
      elliptical `A` imports the lines at the right world points and reports `path C`, `path Q`,
      `path T` and `path elliptical arc` once each, in order (files: tests/it/io_svg/path_grammar.rs,
      tests/it/io_svg/mod.rs)
- [x] T12 [AC1] Fuzz: an arbitrary `d` string inside a `<path>` never panics `import_svg` (files:
      tests/it/io_svg/import_fuzz.rs)
- [x] T13 [AC10] Fixtures with `.expected` written by hand from the SVG text: `path-relative`
      (`m l h v a` resolve), `path-compact` (`M1-2.5.5`, glued flags, exponents),
      `path-subpaths` (three `M` in one `d`, two closed with `Z`) (files:
      tests/fixtures/svg/path-relative.{svg,expected}, tests/fixtures/svg/path-compact.{svg,expected},
      tests/fixtures/svg/path-subpaths.{svg,expected})
- [ ] T14 [AC10] Fixtures: `path-radii` (negative radii, `rx = 0`, `λ > 1`, equal endpoints,
      rotated circular arc), `path-error` (mid-path error: segments before it plus
      `ignored 1 path (data error)`), `path-curves` (a Bézier and an elliptical arc reported)
      (files: tests/fixtures/svg/path-radii.{svg,expected}, tests/fixtures/svg/path-error.{svg,expected},
      tests/fixtures/svg/path-curves.{svg,expected})
- [ ] T15 Coverage doc path-data rows marked done by LCV-172, with Béziers and ellipses pointing at
      LCV-176/177. CHANGELOG `Changed`: Open reads every path command and reports the curves it
      can't import yet (files: docs/research/svg-spec-coverage.md, CHANGELOG.md)
