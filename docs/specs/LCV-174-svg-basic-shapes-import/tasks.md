# LCV-174 — Tasks

- [x] T1 [AC1] Refactor: create `import/shapes.rs` with `Attr`/`attr`. Move `parse_line`,
  `parse_circle` and the ellipse centre read into it, and route walk's `line|circle|ellipse` arms
  through `shapes::import_shape`. No behavior change; gate green.
  (files: src/io/svg/import/shapes.rs, src/io/svg/import/walk.rs, src/io/svg/import.rs)
- [x] T2 [AC1, AC2, AC3] Test: `tests/it/io_svg/shapes.rs`.
  - Missing `x1 y1 x2 y2 cx cy x y` default to 0.
  - `r="0"` and a missing `r` import nothing and add no report entry.
  - `r="-1"` and `x1="abc"` (rewritten from the retired LCV-057 AC 12/13) and `<ellipse rx="-2">`
    skip and report `<el> (invalid attribute)`, and the file still opens.
  (files: tests/it/io_svg/shapes.rs, tests/it/io_svg/mod.rs, tests/it/io_svg/import.rs)
- [x] T3 [AC1, AC2, AC3] Impl: the tri-state rules in `import_shape` for line, circle and ellipse.
  Ellipse radius errors use `(invalid attribute)` instead of LCV-176's `(invalid radius)`.
  (files: src/io/svg/import/shapes.rs, src/io/svg/import/conic.rs)
- [x] T4 [AC3] Refactor: remove `SvgImportError::MalformedAttribute`, `malformed` and `attr_f64`,
  and drop the kind from the corpus error table.
  (files: src/io/svg/import.rs, tests/it/io_svg/corpus.rs, tests/it/io_svg/corpus_expected.rs)
- [x] T5 [AC4, AC10] Test:
  - A sharp `<rect>` gives four lines in equivalent-path order (top-left, clockwise in SVG space).
  - `width`/`height` 0 or missing imports nothing, unreported. `width="-1"` is reported.
  - `rect` is no longer in the ignored report. Rewrite the LCV-171 `[("rect",1)]` unit
    assertions to use `<image>`.
  (files: tests/it/io_svg/shapes.rs, src/io/svg/import/tests.rs)
- [x] T6 [AC2, AC3, AC4, AC10] Impl: `rect_path` (sharp) feeds `path_entities`; `classify`
  marks `rect` as `Import`. Widen the `PathData` constructors to `pub(in crate::io::svg)`.
  (files: src/io/svg/import/shapes.rs, src/io/svg/import/walk.rs, src/io/svg/path_data.rs)
- [x] T7 [AC5] Test: `resolve_radii` unit table covering only-rx, only-ry, `auto`, `AUTO`, both
  missing, both `auto`, clamp to w/2 and h/2, `%` radius, and a negative rx reported.
  (files: src/io/svg/import/shapes.rs)
- [ ] T8 [AC5] Impl: `resolve_radii`, with `auto` matched before `attr_len`.
  (files: src/io/svg/import/shapes.rs)
- [ ] T9 [AC6] Test:
  - Equal radii give four quarter `Arc`s plus sides.
  - `rx = w/2` gives no zero-length top or bottom line.
  - Unequal radii give four quarter elliptical arcs (LCV-176 entity).
  - A `<rect>` matches its equivalent `<path>` entity by entity.
  (files: tests/it/io_svg/shapes.rs)
- [ ] T10 [AC6] Impl: rounded `rect_path`, with the four `A` corners per SVG 2 §10.2.
  (files: src/io/svg/import/shapes.rs)
- [ ] T11 [AC7, AC8, AC10] Test:
  - A polyline gives one line per distinct pair. Duplicate points are skipped, and one point
    imports nothing.
  - A polygon gets a closing line, but not when it is already closed.
  - An odd count or a bad token keeps the earlier pairs and reports `(data error)`.
  - Neither element is reported as ignored.
  (files: tests/it/io_svg/shapes.rs)
- [ ] T12 [AC7, AC8, AC10] Impl: `points_path` reuses the path-data number reader (widened);
  `classify` marks `polyline|polygon` as `Import`. If `shapes.rs` passes 200 LOC, split it into
  `shapes/points.rs`.
  (files: src/io/svg/import/shapes.rs, src/io/svg/path_data/lexer.rs, src/io/svg/import/walk.rs)
- [ ] T13 [AC9] Test + fix: `<g transform>` around a rect, polyline and polygon; `scale(2,1)` on a
  rounded rect gives elliptical corners, `rotate(30)` keeps circular arcs. Expected to pass with
  no code change, otherwise fix in `shapes.rs`.
  (files: tests/it/io_svg/shapes.rs, src/io/svg/import/shapes.rs)
- [ ] T14 [AC11] Corpus `shapes-rect`: sharp, rounded, `auto` and clamped rects.
  (files: tests/fixtures/svg/shapes-rect.svg, tests/fixtures/svg/shapes-rect.expected)
- [ ] T15 [AC11] Corpus `shapes-poly`: a polyline and a polygon, open and closed.
  (files: tests/fixtures/svg/shapes-poly.svg, tests/fixtures/svg/shapes-poly.expected)
- [ ] T16 [AC11] Corpus `shapes-degenerate`: zero sizes, a negative radius and odd `points`, with
  their report lines.
  (files: tests/fixtures/svg/shapes-degenerate.svg, tests/fixtures/svg/shapes-degenerate.expected)
- [ ] T17 [AC10] Docs: `svg-spec-coverage.md` §4 shapes rows, and the LCV-171 AC 5 amendment noted
  there.
  (files: docs/research/svg-spec-coverage.md)
- [ ] T18 [all] CHANGELOG: under Unreleased, "SVG import: `<rect>` (rounded corners as arcs),
  `<polyline>` and `<polygon>`; invalid shapes are skipped and reported instead of failing the
  file".
  (files: CHANGELOG.md)
