# LCV-176 — Tasks

- [x] T1 [AC4, AC6] Test: `Ellipse` point/start/end, span sweep and containment (wrap-around,
  cw/ccw), exact bbox (rotated, partial span), quadrants inside the span, and `polyline(tol)`
  vertices on the curve with chord deviation ≤ tol (files: tests/it/geometry/ellipse_props.rs,
  tests/it/geometry/mod.rs)
- [x] T2 [AC4, AC6] `Ellipse`, `EllipseSpan` and the methods of T1; sweep and containment go
  through a unit `Arc` (files: src/geometry/ellipse.rs, src/geometry/mod.rs)
- [x] T3 [AC5, AC7] Test + code: `nearest`, `distance_to_point` (full ellipse, span foot inside or
  outside, a point at the centre) and `hits_segment` (files: src/geometry/ellipse/nearest.rs,
  src/geometry/ellipse.rs)
- [x] T4 [AC1, AC3] Test + code: `from_conjugate`. Orthogonal u, v keep rx, ry and rotation
  (rx < ry included). Skewed u, v give principal radii that match an SVD. A reflection negates the
  span and flips `ccw`. Endpoints are preserved within `EPSILON` (files:
  src/geometry/ellipse/conjugate.rs, src/geometry/ellipse.rs)
- [x] T5 [AC6] Test: `Transform::ellipse` for rotate, mirror (rotation `2θ − r`, span negated,
  `ccw` flipped, endpoints map to endpoints) and scale (files: tests/it/geometry/transform_props.rs)
- [x] T6 [AC6] `Transform::ellipse` plus `Entity::Ellipse` with `bbox`, `kind_name`, `translate`
  and `transformed` (files: src/geometry/transform.rs, src/document/entity.rs)
- [x] T7 Compile-only arms for the new variant in every other exhaustive `match`: render draws
  nothing, pick distance is `f64::INFINITY`, trim/extend return nothing, export writes nothing,
  narration writes `ellipse`. Mechanical; later tasks replace each arm. This breaks the 1–3 files
  rule; accepted, because the compiler forces it (files: those the compiler lists)
- [x] T8 [AC4] Test on painted shapes: an ellipse and an elliptical arc at two zooms. Every vertex
  is on the curve, chord deviation ≤ 0.5 px, the stroke is the layer colour, and the selection halo
  and hover paint it (files: tests/it/app/ellipse_paint.rs, tests/it/app/mod.rs)
- [x] T9 [AC4] `ellipse_polyline(e, 0.5·mm_per_px)` in the entity, selection and dashed painters
  (files: src/render/entities.rs, src/render/selection.rs, src/render/preview.rs)
- [x] T10 [AC5] Test: a click within the aperture of the curve selects, a click at the centre does
  not, and a window/crossing box selects like an arc (files: tests/it/app/ellipse_edit.rs,
  tests/it/app/mod.rs)
- [x] T11 [AC5] `Rect::{contains,crosses}_ellipse` and the ellipse arms of `hit.rs` (files:
  src/geometry/rect.rs, src/tools/select/hit.rs)
- [x] T12 [AC7] Test: near an ellipse arc, Endpoint, Center, Quadrant (only vertices inside the
  span) and Nearest are offered. Intersection, Midpoint, Perpendicular and Tangent are never
  offered, even with an anchor and a crossing line (files: tests/it/app/object_snaps.rs)
- [x] T13 [AC7] `SnapEntity::Ellipse`, endpoint/centre candidates, pair intersections skipped
  (files: src/geometry/snap/mod.rs, src/geometry/snap/candidates.rs, src/app/snap.rs)
- [x] T14 [AC7] Quadrant and Nearest for ellipses, skipped by perpendicular/tangent (files:
  src/geometry/snap/anchored.rs)
- [x] T15 [AC6] Test: MOVE, COPY, ROTATE, MIRROR and SCALE on an ellipse arc through the tools.
  Each is exact, mirror negates rotation and reverses direction, and each undoes as one step
  (files: tests/it/app/ellipse_edit.rs)
- [x] T16 [AC8] Test: TRIM and EXTEND aimed at an ellipse leave the document and history unchanged
  and set "Cannot trim/extend an ellipse". An ellipse is never a cutter or boundary for a line
  (files: tests/it/app/ellipse_edit.rs)
- [x] T17 [AC8] Pick distance and `take_message` in TRIM/EXTEND; `cut_points`/`extend_reach` arms
  (files: src/tools/trim.rs, src/tools/extend.rs, src/document/commands/trim/mod.rs)
- [x] T18 [AC9, AC10, AC11] Test: exact strings for a full ellipse with rotation 0 and 30°, and
  for arcs with each combination of `large` and `sweep` under the mirror. `GOLDEN` stays unchanged.
  The audit allowlist gains `ellipse` and `transform="rotate(a cx cy)"` (files:
  tests/it/io_svg/ellipse.rs, tests/it/io_svg/mod.rs, tests/it/io_svg/export_audit.rs)
- [x] T19 [AC9, AC10] `encode_entity` ellipse arm and `deg()`; then `scripts/mutants.sh` on the
  change (files: src/io/svg/export.rs)
- [x] T20 [AC1] Test: `A` with rx ≠ ry, for each combination of large/sweep, with φ ≠ 0, relative,
  and needing radius correction. Endpoints match within `EPSILON`. rx = ry still gives an `Arc`,
  and the `path elliptical arc` note is gone (files: tests/it/io_svg/ellipse.rs)
- [ ] T21 [AC1] `PathData::Arc` keeps `phi`; `conic.rs::{center_arc, conic_entity}`;
  `path_entities` routes the elliptical case (files: src/io/svg/path_data.rs,
  src/io/svg/import/conic.rs, src/io/svg/import/path.rs)
- [ ] T22 [AC2] Test: `<ellipse>` with rx ≠ ry, rx = ry → Circle, `auto`/absent radius, `%`
  radius, rx or ry ≤ 0 → skipped and reported `ellipse (invalid radius)` (files:
  tests/it/io_svg/ellipse.rs)
- [ ] T23 [AC2] The walk's `ellipse` arm through `conic_entity` (files: src/io/svg/import/walk.rs,
  src/io/svg/import/conic.rs)
- [ ] T24 [AC3] Test: a circle and a circular arc under `scale(2 1)`, `skewX(30)` and
  `preserveAspectRatio="none"` import as the exact ellipse; the LCV-173 labels are gone (files:
  tests/it/io_svg/transforms.rs)
- [ ] T25 [AC3] `parse_circle` and `path_entities` send a non-similar CTM to `conic_entity` (files:
  src/io/svg/import.rs, src/io/svg/import/path.rs)
- [ ] T26 [AC12] Test: a proptest round trip of ellipses and arcs on random layers within
  `FORMAT_TOL`, plus a corpus `ellipse` record and one fixture pair (files:
  tests/it/io_svg/roundtrip_props.rs, tests/it/io_svg/corpus/expected.rs,
  tests/fixtures/svg/ellipses.{svg,expected})
- [ ] T27 [AC13] Test: `query_entities` lists a full ellipse and an arc with kind `ellipse`,
  centre, rx, ry, rotation_deg and start/end/direction (files: tests/it/agent/turn.rs)
- [ ] T28 [AC13] The narration arm, a prompt line saying ellipses are read-only with parametric
  angles, and the raster arm (files: src/app/agent_narrate.rs, src/agent/prompt.rs,
  src/render/raster.rs)
- [ ] T29 Docs: `AGENTS.md` export bullets plus ADR 0015 in the list, and coverage rows (files:
  AGENTS.md, docs/research/svg-spec-coverage.md)
- [ ] T30 CHANGELOG: ellipses and elliptical arcs open, edit, snap and export natively (files:
  CHANGELOG.md)
