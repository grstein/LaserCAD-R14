# LCV-171 — Tasks

- [x] T1 Refactor, no behaviour change: move `Walk` and `collect` into `import/walk.rs` (`mod walk;`
      in `import.rs`; `parse_*` reached through `super::`); the existing tests stay green (files:
      src/io/svg/import.rs, src/io/svg/import/walk.rs)
- [x] T2 [AC1] Test: `no_svg_root_returns_error` also refuses `<svg/>` without `xmlns` and
      `<svg xmlns="http://example.com/x"/>` with `NoSvgRoot`, and accepts a prefixed
      `<s:svg xmlns:s="http://www.w3.org/2000/svg">` (files: src/io/svg/import/tests.rs)
- [x] T3 [AC1] `SVG_NS` const and the namespace check in `import_svg`; `NoSvgRoot` doc names the
      namespace (files: src/io/svg/import.rs)
- [x] T4 [AC8] Test: `Report::note` keeps first-occurrence order and merges repeats
      (`a, b, a` → `[(a, 2), (b, 1)]`); `import_svg` of an empty `<svg>` has `report == []`
      (files: src/io/svg/import/report.rs, src/io/svg/import/tests.rs)
- [x] T5 [AC8] `Report` builder, `ImportedSvg::report: Vec<(String, usize)>` with doc comment,
      `Walk` holds a `Report` (files: src/io/svg/import/report.rs, src/io/svg/import.rs,
      src/io/svg/import/walk.rs)
- [x] T6 [AC2] [AC4] Test: geometry inside `<a>` and a nested `<svg>` imports; `<title>`, `<desc>`,
      `<metadata>` holding a `<line>`, and `<sodipodi:namedview>`/`<foo:line>` (foreign namespace)
      import nothing and leave the report empty; `<svg:line>` with the SVG prefix imports (files:
      src/io/svg/import/tests.rs)
- [ ] T7 [AC3] Test: a `<line>`/`<circle>` inside each of the nine never-rendered elements imports
      nothing and reports that element's name; `<defs/>` and `<clipPath></clipPath>` (no element
      child) report nothing; `<linearGradient><stop/></linearGradient>` reports
      `linearGradient` (files: src/io/svg/import/tests.rs)
- [ ] T8 [AC5] [AC6] Test: `image`, `text` (with a `line` child), `use`, `switch`, `rect`, `style`,
      `script`, `foreignObject` are skipped and reported by name with counts; rewrite
      `unknown_elements_silently_skipped` (`<rect/>` → `[("rect", 1)]`) and
      `non_arc_path_silently_skipped` (→ `path (unsupported data)`), plus a `<path>` without `d`
      (files: src/io/svg/import/tests.rs)
- [ ] T9 [AC2]–[AC6] Classification in `collect` per the plan's table; `LayerReader::enter` is
      called only for descended elements; module doc of `import.rs` replaces "silently skips"
      with the table (files: src/io/svg/import/walk.rs, src/io/svg/import.rs)
- [ ] T10 [AC7] Test: each of the eleven properties is reported as an attribute and as a `style`
      declaration on `svg`, `g`, `a`, `line`, `circle` and an imported `path`; `fill="none"` and
      `style="fill: NONE"` are not reported, `fill="red"` is; `!important` and case are ignored; a
      `transform` on a skipped `<image>` or inside `<defs>` is not reported; a repeat counts twice
      under one label (files: src/io/svg/import/tests.rs)
- [ ] T11 [AC7] `REPORTED_PROPERTIES`, `style_decls`, `note_properties` in `report.rs`, called by
      `collect` on imported and descended elements; `layers.rs::own_stroke` reuses `style_decls`
      (files: src/io/svg/import/report.rs, src/io/svg/import/walk.rs, src/io/svg/layers.rs)
- [ ] T12 [AC10] Test: `export_svg` and every `export_layer_svg` output of a document with a line,
      a circle, a CCW and a CW arc on two layers (one Output off, one current) reopen with
      `report == []` (files: tests/it/io_svg/import_report.rs, tests/it/io_svg/mod.rs)
- [ ] T13 [AC9] Test: `action_open_path` on a file with two `<image>` and a `transform` on a `<g>`
      sets `command_feedback` to `Ignored: 2 image, 1 transform`; on a clean export it clears a
      pre-set feedback; a failed open leaves the feedback unchanged (files:
      src/io/file_actions/tests.rs)
- [ ] T14 [AC9] `open_content(app, path, &content)` shared by `action_open` and
      `action_open_path`; rewrite the `action_open` source scan to assert the `open_content(` call
      and `None => return,` (files: src/io/file_actions.rs, src/io/file_actions/tests.rs)
- [ ] T15 [AC11] `.expected` gains `ignored <count> <label…>` lines (count first, label to end of
      line); the runner compares the report in order, and no `ignored` line means an empty report;
      parser tests for both (files: tests/it/io_svg/corpus/expected.rs, tests/it/io_svg/corpus.rs)
- [ ] T16 [AC11] [AC3] Fixture `inkscape-defs`: Inkscape namespaces, `<sodipodi:namedview>`,
      `<metadata>`, `<defs>` holding a `<path>` and a `<linearGradient><stop/>`, a layer `<g>` with
      `transform="translate(0,0)"`, a line and a circle-arc path using
      `style="fill:none;stroke:#000"`, and an `<image>`. Expectation by hand: two entities on the
      default layer, `ignored 1 defs`, `ignored 1 transform`, `ignored 1 image` (files:
      tests/fixtures/svg/inkscape-defs.svg, tests/fixtures/svg/inkscape-defs.expected)
- [ ] T17 Coverage doc §4 rows for never-rendered geometry and silent loss marked done by LCV-171;
      CHANGELOG `Changed`: Open refuses non-SVG-namespace files and reports what it ignored on the
      command line (files: docs/research/svg-spec-coverage.md, CHANGELOG.md)

- Review note (LCV-170): add an assertion in the corpus test that the four LCV-170 seed stems exist in `tests/fixtures/svg/`.
