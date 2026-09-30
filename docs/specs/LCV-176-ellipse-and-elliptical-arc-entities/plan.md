# LCV-176 — Plan

## Approach

Add `Entity::Ellipse(Ellipse)` per ADR 0015: one kernel type with an optional parametric span.
All geometry lives in `geometry/ellipse*`: point, sweep/containment (delegated to a unit `Arc`),
bbox, quadrants, `polyline(tol)`, `nearest`, `hits_segment` and `from_conjugate`. Every consumer
gets one small `match` arm that calls it. Import builds each elliptical case through
`from_conjugate`, after LCV-173 maps the conjugate semi-axes through the CTM and the Y flip.
Export adds one `encode_entity` arm. Ellipses stay out of intersections, TRIM and EXTEND. This
covers the ACs with no new tool, command or dependency.

## Touches

- `src/geometry/ellipse.rs` (new, ~150): `Ellipse`, `EllipseSpan`, `point`, `start_point`,
  `end_point`, `sweep`, `contains_param`, `bbox`, `quadrants`, `polyline`.
  `geometry/ellipse/nearest.rs` (~90): `nearest`, `distance_to_point`, `hits_segment`.
  `geometry/ellipse/conjugate.rs` (~60): `from_conjugate`. `geometry/mod.rs` re-exports.
- `geometry/transform.rs::Transform::ellipse`; `geometry/rect.rs::{contains,crosses}_ellipse`.
- `document/entity.rs`: variant, `bbox`, `kind_name`, `translate`, `transformed`.
- `geometry/snap/{mod.rs::SnapEntity, candidates.rs, anchored.rs}`: Endpoint, Center, Quadrant,
  Nearest; `app/snap.rs::to_snap_entity`.
- `render/{entities.rs, selection.rs, preview.rs, raster.rs}`: `ellipse_polyline(e, tol)`,
  `tol = 0.5·mm_per_px`. The raster uses half its own pixel.
- `tools/select/hit.rs`, `tools/trim.rs` (pick distance, message), `tools/extend.rs` (message);
  `document/commands/trim/{mod.rs, removed.rs}`: empty cut points, `None` reach.
- `io/svg/path_data.rs`: `Arc` segments keep `phi`. `import/path.rs`: non-circular or
  non-similar arcs go to `import/conic.rs` (new, ~120): `center_arc` (§F.6.5/F.6.6 with rx≠ry,
  in user space) and `conic_entity(ctx, c, u, v, span, bed_h)` (map, flip, `from_conjugate`,
  collapse to Circle/Arc when `|rx−ry| ≤ EPSILON`). The circular similar path stays byte for byte.
- `import/walk.rs`: an `ellipse` arm, which leaves LCV-171's "other" row. `import.rs::parse_circle`:
  a non-similar CTM goes to `conic_entity`. The `circle|arc (non-uniform transform)` and
  `path elliptical arc` labels go.
- `io/svg/export.rs::encode_entity`: the ellipse arm and a `deg()` helper.
- `app/agent_narrate.rs::{kind, geometry}`: the ellipse line (AC 13).
- Tests: `tests/it/geometry/ellipse_props.rs`, `tests/it/io_svg/ellipse.rs`,
  `tests/it/app/ellipse_edit.rs`, `export_audit.rs` allowlist, corpus `expected.rs` record.
- Docs: `AGENTS.md` (export bullets and ADR 0015 in the list), `svg-spec-coverage.md`, `CHANGELOG.md`.
- ADRs: **0015** (new, this design).

## Export contract changes

Additive, for the new kind only (ADR 0015 §5):
- `<ellipse cx cy rx ry/>` plus `transform="rotate(a cx cy)"` only when `a ≠ 0`.
- `<path d="M sx sy A rx ry φ large sweep ex ey"/>`.
- `a = φ = −rotation°`, normalized into (−180, 180], `{:.6}`. Coordinates use `{:.4}`.
- `sweep = 0` for CCW; `large` iff the parametric sweep > π.

Lines, circles and arcs are unchanged. The LCV-170 `GOLDEN` pins AC 11. The audit allowlist gains
`ellipse` (`cx cy rx ry`, optional `transform` of the form `rotate(a cx cy)` only) and `A` paths
with rx ≠ ry or φ ≠ 0.

## Decisions (self-approved per user goal)

- Span angles are parametric, and `query_entities` reports `start_deg`/`end_deg` in the same
  parameter. The prompt line says so.
- AC 12's "within EPSILON" is read as the file format's tolerance (`FORMAT_TOL` = 5e-5 mm on
  points and radii, 1e-6° on rotation), as for arcs today. An arc's centre is compared only away
  from a half turn.
- AC 13 follows the current narration format. The labelled values are `ellipse center (cx, cy) mm,
  rx = …, ry = … mm, rotation_deg = …[, s°→e° ccw|cw]`.
- EXTEND: a click with no extendable endpoint in reach, whose pick lands on an ellipse, posts the
  message. A hover on an ellipse arc endpoint is not offered as an extension.
- `<ellipse>` with `rx`/`ry` of `auto` or absent takes the other value; both absent or either ≤ 0 →
  skipped, reported `ellipse (invalid radius)`. A `%` radius resolves like LCV-173 lengths.
- An imported ellipse whose principal radii differ by ≤ `EPSILON` becomes a Circle/Arc (AC 2).

## Risks

- LOC cap: `geometry/snap/mod.rs` is 258 → ~262 (seam: move `SnapEntity`/`SnapResult` to
  `snap/types.rs` if it passes 270). `raster.rs` 203, `trim/mod.rs` 214, `trim.rs` 181 and
  `export.rs` 167 each gain less than 20 lines. `walk.rs` is ~200 after LCV-175: the `ellipse` arm
  calls `conic.rs` (seam: element parsers go to `import/shapes.rs`, which LCV-174 creates anyway).
- Mutation testing: **yes** on `src/io/svg/export.rs` and `src/geometry/ellipse*`.
- Nearest-point exactness holds only inside the curvature radius (ADR 0015 §7). The pick and snap
  tests use a thin ellipse (rx/ry = 10) at a working zoom.
- Rebase: the four LCV-173/175 files outside `io_svg` are not touched. `agent_narrate.rs` also
  changes on `agent-harness` (LCV-185+), with a one-arm conflict at most.
