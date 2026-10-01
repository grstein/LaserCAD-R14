# LCV-174 — Plan

## Approach

Every basic shape becomes path data and goes through the existing path pipeline. A new
`import/shapes.rs` reads a shape's attributes into a `PathData` (LCV-172) and hands it to
`import/path.rs::path_entities(&PathData, &Ctx, bed_h)`. That function already maps through the CTM
(LCV-173), flips Y, drops zero-length lines and sends circular arcs to `Arc` and the rest to
`import/conic.rs` (LCV-176). So there is no new geometry code:

- `<rect>` → the SVG 2 §10.2 equivalent path: `M x+rx,y H x+w−rx A rx ry 0 0 1 x+w,y+ry V …`
  then three more `H/V`s, `A`s and `Z`. Sharp corners (`rx = 0` or `ry = 0`) → four `L`s.
- `<polyline>`/`<polygon>` → `M p0 L p1 …`, plus a closing `L` for a polygon.
- `<line>`, `<circle>` and LCV-176's `<ellipse>` keep their own builders but share the new
  attribute reader.

One tri-state reader, `Attr::{Missing, Value(f64), Invalid}`, on top of LCV-173's `attr_len`,
turns SVG 2's error rules into three outcomes: import, skip silently (AC 2), or skip and report
`<element> (invalid attribute)` (AC 3). The file never fails on a shape, so `MalformedAttribute`
has no producer left and is removed, as LCV-172 removed `MalformedPath`.

## Touches

- `src/io/svg/import/shapes.rs` (new, ~190): `Attr`, `attr(node, name, axis, ctx)`,
  `resolve_radii(rx, ry, w, h) -> (f64, f64)`, `rect_path`, `points_path`, and `Shape { entities,
  notes }`. `parse_line` and `parse_circle` move here from `import.rs`, and the ellipse centre read
  moves here from `conic.rs`/`walk.rs`. If the file passes 200, the `points` parser moves to
  `shapes/points.rs`.
- `src/io/svg/import/walk.rs`: `classify` moves `rect`, `polyline` and `polygon` to `Import`
  (AC 10). The import arm calls `shapes::import_shape(name, node, &ctx)` and pushes its entities
  and notes. It shrinks, because the per-element arms leave.
- `src/io/svg/import.rs`: the `MalformedAttribute` variant, `malformed` and `attr_f64` go, and
  the module doc lists the new shapes.
- `src/io/svg/path_data.rs` and `path_data/lexer.rs`: `PathData`/segment constructors and the
  number reader become `pub(in crate::io::svg)`, so `shapes.rs` can build segments and parse
  `points` with the same number grammar (no second lexer).
- Tests: `tests/it/io_svg/{shapes.rs, mod.rs}`. `tests/it/io_svg/import.rs` AC 12/13 (retired
  LCV-057: `r<0` and `x1="abc"` failed the file) are rewritten to "skipped and reported".
  `src/io/svg/import/tests.rs` swaps its "ignored `rect`" examples for `<image>`.
  `tests/it/io_svg/{corpus.rs, corpus_expected.rs}` drop the `MalformedAttribute` kind. There are
  three corpus pairs: `shapes-rect`, `shapes-poly` and `shapes-degenerate`.
- Docs: `svg-spec-coverage.md` §4, an LCV-171 AC 5 amendment note, and `CHANGELOG.md`.
- ADRs: none (import only, no new dependency or boundary).

## Export contract changes

None. `export.rs` is not touched, and the RECTANGLE tool still writes four lines. LCV-170's
`GOLDEN` stays green.

## Decisions (self-approved per user goal)

- "Zero effective radii" (AC 4) means that either resolved radius is 0, per SVG 2: `rx = 0` or
  `ry = 0` gives a sharp rect.
- `auto` is matched ASCII case-insensitively before the length parse. A `%` radius resolves on
  the X axis for `rx` and the Y axis for `ry` (LCV-173 `Axis`).
- Entities follow the equivalent-path order, starting at the top-left corner and going clockwise
  in SVG space. So a `<rect>` imports exactly like its equivalent `<path>`, which the AC 6 test
  asserts entity by entity.
- A rounded rect whose side has length 0 (`rx = w/2`) emits no line there, because
  `path_entities` already drops zero-length lines. A fully round rect gives four arcs.
- Polyline "distinct points" means distinct after mapping, within `EPSILON` (LCV-172's rule). A
  single point, or none, imports nothing and is not reported. A polygon whose last point equals
  its first gets no closing line.
- `points` uses the path number grammar (comma or whitespace, glued signs). An odd count reports
  `<el> (data error)` and keeps the complete pairs before it, as AC 8 says.
- `<ellipse>` radius errors reuse this spec's rules: missing or 0 skips silently, and negative or
  unparseable reports `ellipse (invalid attribute)`. This replaces LCV-176's `ellipse (invalid
  radius)` label, for one vocabulary.
- One report note per bad element, even if several of its attributes are bad.

## Risks

- LOC cap: `shapes.rs` ~190 (seam `shapes/points.rs`), `walk.rs` ~200 → ~180, `import.rs` ~195
  → ~170. No file nears 270.
- Mutation testing: no (not `export.rs`, `agent/` or `History`). Every radius case is a unit
  test.
- Public API: removing `SvgImportError::MalformedAttribute` breaks only tests, and T4 updates
  them. `app/` only formats the error through `Display`.
- Old-test rewrites: two retired-spec tests change from an error to a skip. The spec's Problem
  section asks for this, and T2 records it in the test docs.
