# ADR 0017 — SVG `<text>` imports as outlines of system fonts via `fontdb` + `ttf-parser`

- **Status**: Accepted
- **Date**: 2026-09-30
- **Deciders**: /design for LCV-179 (self-approved per user goal; library choice by the user,
  2026-09-29, recorded in the LCV-179 spec)

## Context

Artwork from Inkscape and Illustrator often keeps lettering as live `<text>`. LCV-171 reports and
drops it. LaserGRBL cuts only geometry, so LCV-179 turns each glyph into contours of the line,
quadratic and cubic entities of ADR 0016, laid out in the font the file names. That needs real
font outlines: the built-in Hershey font (`src/text/`) is a single-stroke engraving font and
cannot stand in for a named family (AC 1, AC 2). Constraints: kernel purity (no `egui`), no
`unsafe` in our code, ADR 0006 (`App::default()` touches nothing on disk), deterministic import
(AC 12), single binary, Linux first.

## Decision

1. **Dependency**: `fontdb` (0.23, the line paired with `ttf-parser` 0.25, already in
   `Cargo.lock` through `egui`'s `ab_glyph`), `default-features = false, features = ["std",
   "fs"]` — no `memmap`, no `fontconfig`: system fonts are found in fontdb's fixed directory
   list (`/usr/share/fonts`, `/usr/local/share/fonts`, `~/.local/share/fonts`, and the Windows /
   macOS equivalents). `ttf-parser` becomes a direct dependency for `OutlineBuilder`. No
   `rustybuzz`: no shaping, ligatures or kerning (spec out of scope).
2. **Kernel-pure font book** (`src/text/fonts.rs`): `FontBook` owns a lazily built
   `fontdb::Database` (`OnceCell`). Constructors: `FontBook::empty()` (no faces),
   `FontBook::system()` (scans system directories on first query, not at construction) and
   `FontBook::from_files(&[PathBuf])`. Generic families are set at load: `sans-serif` →
   the first installed of Liberation Sans, DejaVu Sans, Noto Sans, Arial, Helvetica, else the
   alphabetically first family; `serif` and `monospace` likewise from short lists, else the
   sans-serif choice. Face matching (nearest weight/style) is fontdb's CSS algorithm.
3. **Outlines** (`src/text/outline.rs`): `ttf-parser`'s `OutlineBuilder` emits `Line` and
   `Bezier::{Quadratic, Cubic}` in font units; scaling, the y-axis flip of the glyph and the
   element CTM are applied by the caller. Every contour is closed with a line when its end is not
   on its start. No glyph is cached between imports.
4. **Injection** (ADR 0006): `import_svg` takes `&FontBook`. `App::new()` stores
   `FontBook::system()`; `App::default()` stores `FontBook::empty()`, so headless tests never read
   system fonts and any `<text>` is skipped and reported as `text (no font)`. Tests load a bundled
   OFL subset under `tests/fixtures/fonts/` through `from_files`.
5. **No live text**: imported glyphs are ordinary entities; the export contract is unchanged
   (`<text>` is never written; Béziers use the ADR 0016 forms).

## Consequences

- Binary grows by `fontdb` (+ `slotmap`, `tinyvec`, `log`); `ttf-parser` is shared with `egui`.
- The first open of a file with `<text>` pays one font-directory scan (hundreds of ms on a
  desktop with many fonts); later opens reuse it. Files without `<text>` never scan.
- Output depends on the installed fonts: the same file can import differently on two machines.
  The import report names every substitution, so the user sees it.
- Fonts in `fontconfig`-only locations are not found; adding the `fontconfig` feature later is a
  one-line change and does not amend this ADR's interfaces.
