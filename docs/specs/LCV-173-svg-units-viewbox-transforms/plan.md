# LCV-173 — Plan

## Approach

Import carries one context down the walk: `Ctx { ctm: Matrix, viewport: [f64; 2] }`. `ctm` maps
the current user units to **bed millimetres, Y-down**, and `viewport` is the nearest viewport's
size in current user units. `flip_y` stays the last step, so the world mirror is unchanged.

1. **Lengths** (`io/svg/length.rs`): `parse_length(raw) -> Option<Length>` reads a number and a
   unit (`mm cm Q in pt pc px`, none, `%`, `em`, `ex`, all ASCII case-insensitive). `to_user(len,
   ref_len)` gives user units: 1 px = 1 user unit, 96 px = 25.4 mm, `%` × `ref_len`.
2. **Matrix** (`io/svg/matrix.rs`): `Matrix { a b c d e f }` in f64, with `then`, `apply(Vec2)`,
   `det`, `similarity_scale() -> Option<f64>`, and `parse_transform(raw) -> Option<Matrix>` for
   the six functions (comma or space separated, list composed left to right).
3. **Viewport** (`io/svg/viewport.rs`): `par(raw)` for `preserveAspectRatio`, and
   `view_box_map(vb, rect, par) -> Matrix` (SVG 2 §8.2 algorithm, all nine aligns, meet, slice,
   none). `nested(node, &Ctx) -> Ctx` applies `transform · translate(x y) · viewBox map`.
4. **Root** (`header.rs::parse_root -> Root { bed_mm, ctx }`): the bed comes from absolute
   `width`/`height`, otherwise from the viewBox size read as px, otherwise the default bed. The
   range check is kept (AC 3). The root `ctx.ctm` maps the viewBox **directly onto the bed rect
   in mm**. For LaserCAD's own files the scale is `W / W = 1.0` exactly and the offset is 0, so
   AC 11 holds bit for bit and never goes through a `96 / 25.4` product.
5. **Walk** (`import/walk.rs`): every imported or descended element composes its own `transform`
   into the child `ctx`. A nested `<svg>` takes `viewport::nested`. Lines map both points,
   circles need `similarity_scale`, and path segments are mapped in SVG space before LCV-172's
   converter (arcs: endpoints mapped, `rx·s`, `sweep ^= det < 0`).

## Touches

- `src/io/svg/{length.rs, matrix.rs, viewport.rs}` (new, kernel-pure, private in `mod.rs`).
- `src/io/svg/header.rs`: `parse_bed` becomes `parse_root`. The unit and viewBox restrictions and
  their module doc go; its unit tests are rewritten to the SVG 2 rules (spec decision).
- `src/io/svg/import.rs`: `import_svg` uses `parse_root`. `parse_line` and `parse_circle` take
  `&Ctx`, and `attr_f64` becomes `attr_len(n, el, attr, Axis)` (Axis X, Y or Diag picks `%`'s
  reference). The module doc gains the unit/viewBox/transform paragraph.
- `src/io/svg/import/walk.rs`: the `Ctx` argument, the transform per element, the nested `svg`
  arm, and the report labels `transform (invalid)`, `svg (not clipped)` and `circle|arc
  (non-uniform transform)`.
- `import/path.rs::path_entities(&PathData, &Ctx, bed_h)` maps segments first;
  `import/report.rs`: `transform` leaves `REPORTED_PROPERTIES` (amends LCV-171 AC 7).
- Tests: unit tests in the three new files, `tests/it/io_svg/{units_viewbox.rs, transforms.rs}`,
  six corpus pairs, `import/tests.rs` rewrites. Four fixtures outside `io_svg` with unitless
  `width="200"` (`agent/memory.rs`, `agent/turn_group.rs`, `app/document_title_and_file_feedback.rs`,
  `ui/discard_dialog_pointer_click.rs`) become `200mm`: they stand for LaserCAD files.
- Docs: `svg-spec-coverage.md` §3/§4, LCV-171 AC 7 amendment note, `CHANGELOG.md`. ADRs: none.

## Export contract changes

None. `export.rs` is not touched. AC 11 is proven by `roundtrip_props.rs`, the LCV-170 corpus
seeds and a new bitwise test that reopens `export_svg` output with an offset-free viewBox.

## Non-uniform scale and skew before LCV-176

A circle or arc imports only when `similarity_scale` is `Some(s)`: columns of equal length and
orthogonal, within a 1e-9 relative tolerance. Otherwise it imports nothing, the current point
still advances (for paths), and the report counts `circle (non-uniform transform)` or
`arc (non-uniform transform)`. This includes `preserveAspectRatio="none"` with unequal scales.
Lines and polyline segments always import, because an affine map keeps them straight.

## Decisions (self-approved per user goal)

- `em` = 16 px, `ex` = 8 px (CSS initial `font-size`) in geometry; on the root they are relative.
- AC 2: if either root side is relative or absent, both sides come from the viewBox.
- A root viewBox that is not four finite numbers with positive size stays `MalformedBedDimension
  { attr: "viewBox" }`; a bad nested viewBox counts as absent (SVG 2).
- A bad `preserveAspectRatio` means `xMidYMid meet`; `defer` is ignored.
- The root `transform` applies inside the viewBox, like a group's; a nested `<svg>`'s applies in
  the parent space, before `x y`. Nested `width`/`height` default to `100%`, `x`/`y` to 0.
- A singular matrix (`|det|` < 1e-12 of the norm) imports nothing for the element and reports
  `transform (singular)` (SVG 2: "not rendered").

## Risks

- LOC cap: `header.rs` ~105 → ~90 impl lines, `walk.rs` ~120 → ~170, `import.rs` ~205; if
  `matrix.rs` passes 200 the parser moves to `matrix/parse.rs`.
- AC 11 exactness: the identity map is `x·1 + y·0 + 0`, which is exact in f64. T6 pins
  `f64::to_bits` equality on the reopened export entities.
- Mutation testing: no (not `export.rs`/`agent/`/`History`); every unit and align has a test.
- Rebase: only the four fixture edits leave `svg`-branch files. T16's six pairs (12 files) break
  the 1–3 files rule; accepted, data only.
