# LCV-179 — Tasks

- [x] T1 [AC12] Bundle the test font: subset Liberation Sans Regular and Bold to ASCII 0x20–0x7E
  with `pyftsubset`, add `OFL.txt`; add `fontdb`/`ttf-parser` to `Cargo.toml`
  (files: `tests/fixtures/fonts/*`, `Cargo.toml`)
- [x] T2 [AC2, AC3] Test first: `FontBook::from_files` resolves `"Nope, Liberation Sans"`,
  `sans-serif`, bold/italic nearest face, unknown family → `substituted`, `empty()` → `None`;
  then `FontBook` (files: `src/text/fonts.rs`, `src/text/mod.rs`)
- [x] T3 [AC1, AC4, AC8] Test first: glyph `l` of the bundled font is closed contours of lines,
  `o` holds quadratics, advances match `hmtx`, an unmapped char (`é`) is `missing` with glyph 0's
  advance; then `outline.rs` (files: `src/text/outline.rs`, `src/text/mod.rs`)
- [x] T4 [AC1] Test first: `<text x="10" y="20" font-size="10">l</text>` under a `translate` and
  `scale` imports closed contours whose bbox matches the glyph scaled by size/upem with origin at
  (10,20) through the CTM; then `import_text`, the walk arm and `import_svg_with`
  (files: `src/io/svg/import/text.rs`, `src/io/svg/import/walk.rs`, `src/io/svg/import.rs`)
- [x] T5 [AC1, AC3] Test first: `App::default().fonts` is empty (opening a text file reports
  `text (no font)`); then `App::fonts` (`system()` in `new`) and `open_content` calling
  `import_svg_with` (files: `src/app/mod.rs`, `src/io/file_actions.rs`, `tests/it/app/…`)
- [x] T6 [AC4] Test first: `"ll"` places the second glyph exactly one advance right of the first;
  `"AV"` gets no kerning (files: `src/io/svg/import/text.rs`)
- [x] T7 [AC5] Test first: `text-anchor` `middle`/`end` shift each chunk by w/2 / w, a tspan with
  its own `x` anchoring separately; then chunk shift (files: `src/io/svg/import/text.rs`)
- [x] T8 [AC6] Test first: `x="0 10 20"` places three chars at 0/10/20, the 4th continues by
  advance; `dy` list on a tspan; inner list overrides outer; then positions
  (files: `src/io/svg/import/text.rs`)
- [x] T9 [AC7] Test first: `"  a \n  b  "` lays out as `"a b"`, spanning a tspan boundary;
  `xml:space="preserve"` keeps every space and turns newline/tab into spaces; then whitespace
  (files: `src/io/svg/import/text.rs`)
- [x] T10 [AC3, AC8] Test first: unknown family → `text (font substituted)` once; `é` →
  `text (missing glyph)` 1 and the next glyph still advances; `empty()` book → no entities and
  `text (no font)`; drop `text` from the unsupported list and rewrite the LCV-171 report test
  (files: `src/io/svg/import/report.rs`, `src/io/svg/import/text.rs`, `src/io/svg/import/tests.rs`)
- [x] T11 [AC9] Test first: `textPath` and `writing-mode="tb"` skip the text with their labels;
  `rotate`, `inline-size`, `letter-spacing`, `word-spacing` lay out unchanged and are reported
  once each (files: `src/io/svg/import/text.rs`)
- [x] T12 [AC10] Test first: fill-only red text lands on the red layer; a tspan with
  `stroke="blue"` lands on the blue layer; a `data-layer` group wins; no stroke and no fill
  imports nothing (files: `src/io/svg/import/text.rs`)
- [x] T13 [AC11] Test first: import a text fixture with the bundled font, export, assert no
  `<text`, `Q`/`C`/`L` path data present and a reimport yields the same entity count
  (files: `tests/it/io_svg/text.rs`, `tests/fixtures/svg/text-outlines.svg`)
- [ ] T14 [AC12] Test first: the same file through two fresh `FontBook::from_files` imports
  identical entities (`==` on the vectors) (files: `tests/it/io_svg/text.rs`)
- [ ] T15 Mutation run on `src/text/{fonts,outline}.rs` and `import/text.rs`; kill or justify
  survivors in the review notes (files: tests above as needed)
- [ ] T16 CHANGELOG: "Open SVG: text becomes cuttable outlines in the named installed font;
  substitutions and unsupported text features appear in the import report" (files: `CHANGELOG.md`)
