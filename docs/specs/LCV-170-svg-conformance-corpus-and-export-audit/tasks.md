# LCV-170 — Tasks

- [ ] T1 [AC10] Test: pin today's `export_svg` bytes for one audit document (two layers, one
      Output off, a current layer, a line, a circle, an arc, a name with `& < > " '` and `é`) as a
      `const` golden string; register the module (files: tests/it/io_svg/export_audit.rs,
      tests/it/io_svg/mod.rs)
- [ ] T2 [AC1] `.expected` parser with its own tests: `bed`, `layer`, `line`/`circle`/`arc`
      (layer index, degrees, `ccw|cw`), `error <Variant>`, `#` comments; a malformed line is an
      `Err` naming line number (files: tests/it/io_svg/corpus/expected.rs, tests/it/io_svg/corpus.rs)
- [ ] T3 [AC2] [AC3] Corpus runner `check_corpus(dir) -> Vec<String>`: import each `.svg`, compare
      bed, layers (name, color, output, current, order) and entities (kind, layer, 1e-6 mm, 1e-9 rad
      modulo 2π) or the error variant; report orphan `.svg`/`.expected` and unparseable
      `.expected`. Tests: `corpus_matches_expected` over `tests/fixtures/svg/`, and
      `an_svg_without_expected_fails_naming_it` over a temp dir (files: tests/it/io_svg/corpus.rs)
- [ ] T4 [AC4] Seed fixtures, expectations written by hand: `v02-presets` (v0.2 `<g id=…>` groups,
      no `data-layer`) and `v03-mother-three-layers` (three layers, one `data-output="0"`,
      `data-current` on the second) (files: tests/fixtures/svg/v02-presets.svg,
      tests/fixtures/svg/v02-presets.expected, tests/fixtures/svg/v03-mother-three-layers.svg +
      .expected)
- [ ] T5 [AC4] Seed fixtures: `export-layers-engrave` (one `<g data-layer>`, no `data-current`)
      and `inkscape-mm` (`xmlns:sodipodi`/`xmlns:inkscape`, `<metadata>`, `<sodipodi:namedview>`,
      `<g inkscape:groupmode="layer" inkscape:label="Layer 1">` with a line, a circle and a
      circular-arc path; width/height in mm equal to the viewBox; no transform) (files:
      tests/fixtures/svg/export-layers-engrave.svg + .expected, tests/fixtures/svg/inkscape-mm.svg +
      .expected)
- [ ] T6 [AC9] Test: `check_fields` refuses `"A\u{7}B"`, `"A\tB"`, `"A\u{85}B"` with
      `LayerError::ControlChar`, and accepts `"Grav é"`; add the import fixture
      `layer-control-char` (`data-layer="A&#9;B"`, expectation `error MalformedLayer`) (files:
      src/document/layer.rs (tests section), tests/fixtures/svg/layer-control-char.svg + .expected)
- [ ] T7 [AC9] `LayerError::ControlChar` + the check in `check_fields`, before the `name_key`
      test (files: src/document/layer.rs)
- [ ] T8 [AC9] Tests: the Layers dialog, opened by typing `layer`, refuses the rename to `"A\tB"`
      with the ControlChar message, and the document is unchanged; `AddLayer`/`from_parts` refuse
      it (files: tests/it/app/layers_dialog.rs, tests/it/document/layers.rs)
- [ ] T9 [AC9] Test: an agent `create_line` naming layer `"Cut\u{7}"` is refused and adds no
      entity; no message-text assertion (files: tests/it/agent/layers.rs)
- [ ] T10 [AC5] Audit-set builder: an empty doc, each entity kind, layers with Output on and off
      plus a current layer, and names with `& < > " '` and non-ASCII. For every mother and layer
      export, assert `roxmltree` parses it and the root is `svg` in
      `http://www.w3.org/2000/svg` (files: tests/it/io_svg/export_audit.rs)
- [ ] T11 [AC6] Test: every element is one of `svg g line circle path`, and each carries only
      its contract attributes (svg: xmlns width height viewBox fill; g: data-layer stroke
      stroke-width data-output data-current; line: x1 y1 x2 y2; circle: cx cy r; path: d) (files:
      tests/it/io_svg/export_audit.rs)
- [ ] T12 [AC7] Test: every numeric value (coordinates, `r`, `stroke-width`, `width`/`height`
      without `mm`, the four `viewBox` numbers, `d` tokens) is finite and matches SVG 2 `number`,
      checked by a hand-written scanner; every `d` is `M x y A r r 0 f f x y` with `f ∈ {0,1}` and
      `r > 0` (files: tests/it/io_svg/export_audit.rs)
- [ ] T13 [AC8] Test: export → `import_svg` → `into_document` for each audit document restores
      the bed, the layers (order, name, color, output, current) and the entities within 5e-4 mm
      (files: tests/it/io_svg/export_audit.rs)
- [ ] T14 CHANGELOG `Unreleased`: layer names with control characters are refused; SVG
      conformance corpus added (files: CHANGELOG.md)
