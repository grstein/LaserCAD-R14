# LCV-179 — Plan

## Approach

On top of LCV-173 (`Ctx`, `length.rs`), LCV-175 (`Style`, `Slot`) and LCV-177 (`Entity::Bezier`).
Font lookup and glyph outlines live in `src/text/` (kernel-pure, ADR 0017); SVG text layout lives
in `src/io/svg/import/text.rs`; the walk gains one thin `text` arm.

1. **Fonts** (`text/fonts.rs`): `FontBook` over a lazy `fontdb::Database`;
   `face(families, weight, style) -> Option<(face_id, substituted)>` tries each listed family
   (generics mapped at load), else the `sans-serif` default with `substituted = true`; `None`
   only for an empty database (AC 2, 3).
2. **Outlines** (`text/outline.rs`): `glyph(face, char) -> Glyph { advance, contours, missing }`
   in font units via `OutlineBuilder`; a char without a glyph gives glyph 0's advance, no
   contours, `missing = true` (AC 1, 4, 8).
3. **Layout** (`import/text.rs`): flatten `<text>` and nested `<tspan>` into characters, each
   with its style (`Style::child` per tspan), font, size and slot. Whitespace per AC 7 across
   tspan boundaries. Position lists: a global character index; a list on an element applies from
   that element's first character, inner lists override outer (AC 6). Each char with an absolute
   `x` or `y` starts a chunk; the pen advances by `advance · size / units_per_em` (AC 4); after a
   chunk is laid out it shifts by `−w/2` or `−w` for `text-anchor` `middle`/`end` of its first
   char (AC 5). Points map `(pen + p.x·s, baseline − p.y·s)` then `ctx.ctm`, then into the
   existing world conversion, so the first glyph's origin is (`x`,`y`) (AC 1).
4. **Unsupported** (AC 9): a `textPath` descendant or `writing-mode` starting with `tb`/`vertical`
   skips the whole `<text>` → `text (textPath)` / `text (vertical)`; `rotate`, `inline-size`,
   `letter-spacing`, `word-spacing` (non-`normal`/`0`) are ignored, each noted once per `<text>`
   as `text (<name>)`.
5. **Style** (AC 10): the stroke color, else the fill color, picks the slot by LCV-175 rules; no
   stroke and no fill → the text is invisible and imports nothing (as for shapes).

## Touches

- `Cargo.toml` — `fontdb` (no default features; `std`, `fs`) and `ttf-parser` direct.
- `src/text/fonts.rs` (new, ~110) — `FontBook::{empty, system, from_files, face, with_face}`.
- `src/text/outline.rs` (new, ~90) — `Glyph`, `glyph`, `Collector: OutlineBuilder`.
- `src/text/mod.rs` — re-export `FontBook`.
- `src/io/svg/import/text.rs` (new, ~230) — `import_text`, `chars`, `positions`, `chunks`.
- `src/io/svg/import/walk.rs` — `Kind::Text` arm calling `text::import_text`; `tspan`/`textPath`
  silent outside `<text>` as before.
- `src/io/svg/import.rs` — new `import_svg_with(src, &FontBook)`; `import_svg(src)` keeps its
  signature and calls it with `FontBook::empty()`; the book is threaded into `Walk`.
- `src/io/svg/import/report.rs` — `text` leaves the unsupported-element list; new labels.
- `src/app/mod.rs::App` — field `fonts: FontBook`; `App::new` → `system()`, `default` → `empty()`.
- `src/io/file_actions.rs::open_content` — calls `import_svg_with(content, &app.fonts)`.
- `tests/fixtures/fonts/` — `LiberationSans-subset.ttf` (ASCII 0x20–0x7E, regular + bold) and
  `OFL.txt`.
- ADRs: ADR 0017 (new). Export contract: **no change** (`export.rs` untouched).

## Decisions (self-approved per user goal)

- `fontdb` without `fontconfig`/`memmap`: smallest dependency set, no `unsafe` in the default
  path; fontconfig-only font dirs are a known gap (ADR 0017 consequences).
- System fonts load lazily on the first `<text>`; `App::default()` uses an empty book, so headless
  tests see `text (no font)` unless they inject the bundled font.
- Font-size: `length.rs` (default 16 user units; `em`/`%` of the inherited size; keywords
  `xx-small`…`xx-large` → 9/10/13/16/18/24/32; `larger`/`smaller` → ×1.2 / ÷1.2). Weight:
  number, `normal` 400, `bold` 700, `bolder` 700, `lighter` 400. Style: `italic`/`oblique` →
  italic.
- Report labels: `text (no font)`, `text (font substituted)` (per `<text>`), `text (missing
  glyph)` (per char), `text (textPath)`, `text (vertical)`, `text (rotate|inline-size|
  letter-spacing|word-spacing)`. The old `text` unsupported-element label disappears.
- Zero-length segments (coincident ends) are dropped; contours closed with a line.
- Inside `<text>`, an `<a>` is laid out like a `<tspan>`; a `<tref>` is skipped and reported as
  `text (tref)`; a `<tspan>` outside `<text>` stays silent as before.

## Risks

- LOC cap: `walk.rs` ~240 after LCV-178; the text arm must stay ≤10 lines. If `text.rs` passes
  270, positions/chunks move to `import/text/layout.rs`.
- Mutation testing: yes — new kernel layout arithmetic (positions, anchor shift, whitespace).
  Run `scripts/mutants.sh` on `src/text/{fonts,outline}.rs` and `import/text.rs`.
- LCV-171 test `tests.rs` expects `entry("text", 2)`: rewrite it to the new labels (amends
  LCV-171 AC wording only through this spec; note it in the commit).
- Determinism (AC 12): faces are picked by family name, generics by a fixed list, never by
  directory order; the test imports the same file twice from two fresh `FontBook`s.
- Binary size and first-scan latency: noted once in the T5 commit message; no test.
