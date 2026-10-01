# LCV-175 — Tasks

- [x] T1 [AC3] [AC4] Test: `css.rs` unit tests — type, `*`, `.a`, `#b`, `g.a#b` compounds and a
      comma list parse with their specificity; `g path`, `g>path`, `a:hover`, `[x]` drop the rule
      and note `style rule (unsupported selector)`; `@import url(x);` and `@media print { … }`
      are skipped and noted `style @import` / `style @media`; comments stripped; an unterminated
      block notes `style rule (malformed)` (files: src/io/svg/css.rs, src/io/svg/mod.rs)
- [x] T2 [AC3] [AC4] `css::parse_sheet`, `Rule`, `Selector`, `Specificity`, `declarations`
      (`!important`, ASCII case-insensitive names) (files: src/io/svg/css.rs)
- [x] T3 `report::style_decls` delegates to `css::declarations`; no behaviour change, LCV-171
      tests stay green (files: src/io/svg/import/report.rs)
- [x] T4 [AC1] [AC2] [AC5] [AC6] [AC11] Test: `import/style.rs` unit tests — attribute < rule <
      `style`; `!important` rule beats `style`; id > class > type > `*`, later rule on a tie;
      `stroke`/`fill`/`color` inherit, `inherit` keyword; `currentColor` takes `color`; an invalid
      `stroke` falls back to the next candidate and notes `stroke (invalid color)`
      (files: src/io/svg/import/style.rs)
- [x] T5 [AC1] [AC2] [AC5] [AC6] [AC11] `Style`, `Style::root`, `Style::child`, `collect_sheet`
      (files: src/io/svg/import/style.rs, src/io/svg/import.rs)
- [x] T6 [AC7] Test: `display:none` on a `<g>` hides its subtree (one `hidden (display:none)`);
      `visibility:hidden` on a `<g>` hides its lines but a `visibility:visible` child imports;
      `collapse` = `hidden`; a `display:none` `<g data-layer>` still declares its layer
      (files: tests/it/io_svg/styling.rs, tests/it/io_svg/mod.rs)
- [x] T7 [AC7] Walk carries `Style`; display/visibility gates; `<style>` silent, `<defs>` holding
      only `<style>` unreported; `display`/`visibility` leave `REPORTED_PROPERTIES` (files:
      src/io/svg/import/walk.rs, src/io/svg/import/report.rs)
- [x] T8 [AC1]–[AC6] [AC11] Test: end-to-end through `import_svg` — an Illustrator-style
      `<defs><style>.cls-1{stroke:#f00}</style></defs>` file, `style` over `.cls` over attribute,
      `currentColor` via an ancestor `color`, invalid color report entry
      (files: tests/it/io_svg/styling.rs)
- [x] T9 [AC8] [AC9] [AC10] Test: stray red line reuses a declared `#ff0000` layer; two stray
      colors append `#0000ff`, `#00aa00` in first-appearance order with Output on; `stroke:none;
      fill:blue` goes to `#0000ff`; an unstyled line goes to the first layer; a file with no layer
      and all colored geometry has no `Cut`; a mixed one has `Cut` first and a red stray on it;
      name clash gives `#ff0000 2` (files: tests/it/io_svg/color_layers.rs, tests/it/io_svg/mod.rs)
- [x] T10 [AC8] [AC9] [AC10] `Slot`, `LayerReader::finish(slots)`, walk pushes slots, `STRAY_LAYER`
      removed (files: src/io/svg/layers.rs, src/io/svg/import/walk.rs, src/io/svg/import.rs)
- [x] T11 [AC12] Test: every `export_svg`/`export_layer_svg` output and the
      `v03-mother-three-layers` seed reopen with identical layers, colors, output, current and
      memberships; a sheet rule `g{stroke:blue}` does not recolor a `<g data-layer>`
      (files: tests/it/io_svg/color_layers.rs)
- [x] T12 [AC8] [AC10] Follow the ADR 0012 §4 amendment: `layers_roundtrip.rs::
      stray_geometry_goes_to_the_first_layer` (v0.2 part now `#0000ff`) and
      `preset_roundtrip.rs` (one layer per preset color) (files: tests/it/io_svg/layers_roundtrip.rs,
      tests/it/io_svg/preset_roundtrip.rs)
- [x] T13 [AC8] Corpus: hand-written expectations — `inkscape-mm.expected` (layer `#000000`),
      `v02-presets.expected` (three hex layers) (files: tests/fixtures/svg/inkscape-mm.expected,
      tests/fixtures/svg/v02-presets.expected)
- [ ] T14 [AC3] [AC7] [AC8] Corpus: new pairs `illustrator-classes` (`<style>` classes, one rule
      with a combinator) and `inkscape-hidden-layer` (`style="display:none"` layer)
      (files: tests/fixtures/svg/illustrator-classes.{svg,expected},
      tests/fixtures/svg/inkscape-hidden-layer.{svg,expected})
- [ ] T15 ADR 0012 §4 amendment note (stray colored geometry → color layer, LCV-175); LCV-171
      AC 7 note (`display`/`visibility` now applied) (files: docs/adr/0012-document-layers-and-per-layer-export.md,
      docs/specs/LCV-171-svg-import-report-and-never-rendered/spec.md)
- [ ] T16 Run the stray-`#ff0000` fixtures outside `io_svg` (`scripts/check.sh memory turn_group
      document_title discard_dialog`); fix any layer assertion (files: as needed, ≤3)
- [ ] T17 `svg-spec-coverage.md` §5 rows → ✅; CHANGELOG line (files:
      docs/research/svg-spec-coverage.md, CHANGELOG.md)
