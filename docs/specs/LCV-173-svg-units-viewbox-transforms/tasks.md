# LCV-173 — Tasks

- [x] T1 [AC1] Test + code: `parse_length`/`to_user` for `mm cm Q in pt pc px`, unitless = px,
  `%`, `em`/`ex`, case-insensitive, junk → `None` (files: src/io/svg/length.rs, src/io/svg/mod.rs)
- [x] T2 [AC5] Test + code: `Matrix` (`then`, `apply`, `det`, `similarity_scale`) and
  `parse_transform` for the six functions, comma/space separators, composition order, invalid →
  `None` (files: src/io/svg/matrix.rs, src/io/svg/mod.rs)
- [x] T3 [AC4] Test + code: `par` parser and `view_box_map` for all nine aligns × meet/slice, plus
  `none`, with an offset origin (files: src/io/svg/viewport.rs, src/io/svg/mod.rs)
- [x] T4 [AC2, AC3] Test: root bed rules (absolute pair, relative/absent → viewBox as px, default,
  out-of-range refusal); rewrite `header.rs` unit tests to SVG 2 units (files:
  src/io/svg/header.rs)
- [x] T5 [AC2, AC3, AC4] `header.rs::parse_root -> Root { bed_mm, ctx }`, viewBox mapped onto the
  bed rect in mm; `import_svg` uses it (files: src/io/svg/header.rs, src/io/svg/import.rs)
- [x] T6 [AC11] Test: reopen `export_svg` output for a mixed document and compare entities by
  `f64::to_bits`; fix the four unitless fixtures outside `io_svg` to `200mm` (files:
  tests/it/io_svg/units_viewbox.rs, tests/it/io_svg/mod.rs, tests/it/agent/memory.rs)
- [x] T7 [AC11] Same fixture edit in the remaining three files (files: tests/it/agent/turn_group.rs,
  tests/it/app/document_title_and_file_feedback.rs, tests/it/ui/discard_dialog_pointer_click.rs)
- [x] T8 [AC10] Test: `x1 y1 x2 y2 cx cy r` with units and `%` (x by width, y by height, r by
  normalized diagonal), `em` (files: tests/it/io_svg/units_viewbox.rs)
- [ ] T9 [AC10] `attr_len` with `Axis`, `parse_line`/`parse_circle` take `&Ctx` and map through
  `ctm`; rewrite `import/tests.rs` unitless-root cases (files: src/io/svg/import.rs,
  src/io/svg/import/tests.rs)
- [ ] T10 [AC5, AC8] Test: nested transformed groups on lines, circles and paths; an invalid
  `transform` is ignored and reported `transform (invalid)`; `transform` is no longer reported
  as a property (files: tests/it/io_svg/transforms.rs, tests/it/io_svg/mod.rs)
- [ ] T11 [AC5, AC8] Walk: compose each element's `transform` into the child `Ctx`, the invalid
  and singular labels, drop `transform` from `REPORTED_PROPERTIES` (files:
  src/io/svg/import/walk.rs, src/io/svg/import/report.rs)
- [ ] T12 [AC6, AC7] Test: circle and arc under rotate/uniform scale/reflection (radius scaled,
  `ccw` inverted on reflection) and under non-uniform scale/skew (nothing imported, report
  `circle|arc (non-uniform transform)`) (files: tests/it/io_svg/transforms.rs)
- [ ] T13 [AC6, AC7] Circle similarity check in `parse_circle`; `path_entities` maps segments
  through `ctm` (`rx·s`, `sweep ^= det < 0`, non-similar arc → report, point advances) (files:
  src/io/svg/import.rs, src/io/svg/import/path.rs, src/io/svg/import/walk.rs)
- [ ] T14 [AC9] Test: nested `<svg>` with `x y width height viewBox preserveAspectRatio` and a
  transform, content imported unclipped, report `svg (not clipped)` (files:
  tests/it/io_svg/transforms.rs)
- [ ] T15 [AC9] `viewport::nested` and the walk's nested `svg` arm (files:
  src/io/svg/viewport.rs, src/io/svg/import/walk.rs)
- [ ] T16 [AC12] Corpus fixtures: `inkscape-px`, `units-pt-in`, `viewbox-offset`,
  `par-slice-xmaxymin`, `nested-transforms` and `nested-svg`, with hand-written `.expected` files
  (files: tests/fixtures/svg/*.svg, tests/fixtures/svg/*.expected)
- [ ] T17 [AC5] Extend `import_fuzz.rs` with random `transform` and `viewBox` strings (never
  panics) (files: tests/it/io_svg/import_fuzz.rs)
- [ ] T18 Docs: the coverage note §3/§4 rows and the LCV-171 AC 7 amendment note (files:
  docs/research/svg-spec-coverage.md, docs/specs/LCV-171-svg-import-report-and-never-rendered/spec.md)
- [ ] T19 CHANGELOG: px/pt/in files, scaled and offset viewBoxes, transforms and nested `<svg>`
  now open at their true size (files: CHANGELOG.md)
