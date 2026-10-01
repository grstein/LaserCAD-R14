# LCV-177 — Tasks

- [x] T1 [AC5, AC6] Test: `Bezier` point/start/end/map for both degrees; tight bbox with the
  control polygon far outside the curve, an axis extremum inside (0, 1), and a near-zero leading
  coefficient; `polyline(tol)` vertices on the curve, chord deviation ≤ tol, a straight curve
  gives `n = 1` (files: tests/it/geometry/bezier_props.rs, tests/it/geometry/mod.rs)
- [x] T2 [AC5, AC6] `Bezier` and the methods of T1; derivative roots in `solve.rs` (files:
  src/geometry/bezier.rs, src/geometry/bezier/solve.rs, src/geometry/mod.rs)
- [x] T3 [AC7, AC9] Test + code: `nearest`/`distance_to_point` (S-curve, a point near an end, a
  point at an off-curve control point) and `crosses_axis_segment` (one, two and zero crossings)
  (files: src/geometry/bezier/nearest.rs, src/geometry/bezier/solve.rs, src/geometry/bezier.rs)
- [x] T4 [AC8] Test: `Transform::bezier` for rotate, mirror and scale equals `Transform::point`
  on every control point, and maps `point(t)` onto the image curve (files:
  tests/it/geometry/transform_props.rs)
- [x] T5 [AC6, AC8] `Transform::bezier` plus `Entity::Bezier` with `bbox`, `kind_name`,
  `translate`, `transformed`; schema doc line (files: src/geometry/transform.rs,
  src/document/entity.rs, src/document/schema.rs)
- [x] T6 Compile-only arms in every other exhaustive `match`: render draws nothing, pick distance
  `f64::INFINITY`, trim/extend nothing, export nothing, narration `cubic|quadratic`. Mechanical;
  later tasks replace each arm. Breaks the 1–3 files rule; the compiler forces it (files: those
  the compiler lists)
- [x] T7 [AC5] Test on painted shapes: a cubic and a quadratic at two zooms, every vertex on the
  curve, chord deviation ≤ 0.5 px, layer colour stroke, selection halo and hover paint it (files:
  tests/it/app/bezier_paint.rs, tests/it/app/mod.rs)
- [x] T8 [AC5] `polyline(0.5·mm_per_px)` in the entity, selection and dashed painters (files:
  src/render/entities.rs, src/render/selection.rs, src/render/preview.rs)
- [x] T9 [AC6, AC7] Test: a click within the aperture of the curve selects, a click on an
  off-curve control point does not; a window box tight around the curve (not its control polygon)
  selects; a crossing box over the curve selects, one over only the control polygon does not;
  ZOOM Extents frames the curve's tight bbox (files: tests/it/app/bezier_edit.rs,
  tests/it/app/mod.rs)
- [x] T10 [AC6, AC7] `Rect::{contains,crosses}_bezier` and the Bézier arms of `hit.rs` (files:
  src/geometry/rect.rs, src/tools/select/hit.rs)
- [x] T11 [AC9] Test: near a cubic, Endpoint (both ends) and Nearest (on the curve) are offered;
  Midpoint, Center, Quadrant, Intersection, Perpendicular and Tangent never, even with an anchor
  and a crossing line (files: tests/it/app/object_snaps.rs)
- [x] T12 [AC9] `SnapEntity::Bezier`, endpoint candidates, pair intersections skipped (files:
  src/geometry/snap/mod.rs, src/geometry/snap/candidates.rs, src/app/snap.rs)
- [x] T13 [AC9] Nearest for Béziers, skipped by perpendicular/tangent (files:
  src/geometry/snap/anchored.rs)
- [x] T14 [AC8] Test: MOVE, COPY, ROTATE, MIRROR and SCALE on a cubic and a quadratic through
  the tools; points equal `Transform::point` of the originals, each undoes as one step (files:
  tests/it/app/bezier_edit.rs)
- [x] T15 [AC10] Test: TRIM and EXTEND aimed at a Bézier leave document and history unchanged
  and set "Cannot trim/extend a curve"; a Bézier is never a cutter or boundary for a line (files:
  tests/it/app/bezier_edit.rs)
- [x] T16 [AC10] Pick distance and `take_message` in TRIM/EXTEND; `cut_points`/`extend_reach`
  arms (files: src/tools/trim.rs, src/tools/extend.rs, src/document/commands/trim/mod.rs)
- [x] T17 [AC11, AC12] Test: exact strings for a cubic and a quadratic on a non-default bed
  height; `GOLDEN` unchanged; the audit's `check_path` accepts `M … C …` and `M … Q …` (files:
  tests/it/io_svg/bezier.rs, tests/it/io_svg/mod.rs, tests/it/io_svg/export_audit.rs)
- [x] T18 [AC11] `encode_entity` Bézier arm; then `scripts/mutants.sh` on `export.rs` and
  `geometry/bezier*` (files: src/io/svg/export.rs)
- [x] T19 [AC1, AC2, AC3, AC4] Test through `import_svg`: `C`/`c`; `S` after `C`, after `S`,
  after `L` (current point); `Q`/`q`; `T` after `Q`, after `T`, after `C` (current point);
  implicit repetition; one `C` under `skewX(30)` gives the CTM images; a degenerate `C` and `Q`
  create nothing and report `path curve (degenerate)` (files: tests/it/io_svg/bezier.rs)
- [x] T20 [AC1, AC2, AC3] `PathData` `Cubic`/`Quad` segments and the reflection state;
  `Skipped` removed (files: src/io/svg/path_data.rs)
- [x] T21 [AC1, AC4] `path_entities` maps curve points through the CTM and `flip_y`, notes the
  degenerate label; LCV-172 unit tests expecting `path C|S|Q|T` rewritten (files:
  src/io/svg/import/path.rs, src/io/svg/import/tests.rs)
- [x] T22 [AC13] Test: a proptest round trip of cubics and quadratics on random layers within
  `FORMAT_TOL`, kind preserved; a corpus `cubic`/`quadratic` record, one fixture pair, and the
  LCV-172 fixtures that expected `ignored … path C|S|Q|T` updated (files:
  tests/it/io_svg/roundtrip_props.rs, tests/it/io_svg/corpus/expected.rs,
  tests/fixtures/svg/beziers.{svg,expected})
- [x] T23 [AC14] Test: `query_entities` lists a cubic and a quadratic with kinds `cubic` and
  `quadratic` and their points in mm, in order (files: tests/it/agent/turn.rs)
- [ ] T24 [AC14] The narration arm, a prompt line saying Béziers are read-only, and the raster
  arm (files: src/app/agent_narrate.rs, src/agent/prompt.rs, src/render/raster.rs)
- [ ] T25 Docs: `AGENTS.md` export bullet plus ADR 0016 in the list, coverage rows (files:
  AGENTS.md, docs/research/svg-spec-coverage.md)
- [ ] T26 CHANGELOG: curved SVG artwork (`C S Q T`) opens, edits, snaps and exports natively
  (files: CHANGELOG.md)
