# LCV-177 — Plan

## Approach

Add `Entity::Bezier(Bezier)` per ADR 0016, after LCV-176 (ADR 0015) set the pattern: one kernel
type `Bezier { Quadratic([Vec2; 3]), Cubic([Vec2; 4]) }` holding all curve geometry (point, tight
bbox, `polyline(tol)`, `nearest`, rect-edge crossing), and one small `match` arm per consumer.
Import turns LCV-172's skipped `C S Q T` into segments, reflecting `S`/`T` control points in the
resolver and mapping points through the CTM and `flip_y` in the converter. Export adds one
`encode_entity` arm. Béziers stay out of intersections, TRIM and EXTEND. No new tool, command or
dependency.

## Touches

- `src/geometry/bezier.rs` (new, ~130): `Bezier`, `point`, `start`, `end`, `points`, `map`,
  `bbox`, `polyline`. `geometry/bezier/solve.rs` (~80): axis derivative roots, monotone pieces,
  `crosses_axis_segment`. `geometry/bezier/nearest.rs` (~60): `nearest`, `distance_to_point`.
  `geometry/mod.rs` re-exports.
- `geometry/transform.rs::Transform::bezier`; `geometry/rect.rs::{contains,crosses}_bezier`.
- `document/entity.rs`: variant, `bbox`, `kind_name`, `translate`, `transformed`;
  `document/schema.rs` doc line.
- `geometry/snap/{mod.rs::SnapEntity, candidates.rs, anchored.rs}`: Endpoint and Nearest only;
  `app/snap.rs::to_snap_entity`.
- `render/{entities.rs, selection.rs, preview.rs, raster.rs}`: `b.polyline(0.5·mm_per_px)`; the
  raster uses half its own pixel.
- `tools/select/hit.rs`, `tools/trim.rs` (pick distance, message), `tools/extend.rs` (message);
  `document/commands/trim/{mod.rs, removed.rs}`: empty cut points, `None` reach.
- `io/svg/path_data.rs`: `Cubic`/`Quad` segments, `S`/`T` reflection state (last `c2`, last `c`,
  cleared by any other command); `Skipped` removed. `import/path.rs::path_entities`: CTM, flip,
  degenerate note `path curve (degenerate)`. LCV-172 tests and fixtures expecting `path C|S|Q|T`
  are rewritten to the imported curves.
- `io/svg/export.rs::encode_entity`: the Bézier arm.
- `app/agent_narrate.rs::{kind, geometry}`, `agent/prompt.rs` (read-only line).
- Tests: `tests/it/geometry/bezier_props.rs`, `tests/it/io_svg/bezier.rs`,
  `tests/it/app/{bezier_paint.rs, bezier_edit.rs}`, `object_snaps.rs`, `transform_props.rs`,
  `export_audit.rs` allowlist, `roundtrip_props.rs`, corpus record + one fixture pair,
  `tests/it/agent/turn.rs`.
- Docs: `AGENTS.md` (export bullet, ADR 0016 in the list), `svg-spec-coverage.md`, `CHANGELOG.md`.
- ADRs: **0016** (new, this design).

## Export contract changes

Additive, for Bézier entities only (ADR 0016 §6):
- cubic `<path d="M x0 y0 C x1 y1 x2 y2 x3 y3"/>`, quadratic `<path d="M x0 y0 Q x1 y1 x2 y2"/>`;
- every `y` via `flip_y(y, bed_height)`, `{:.4}`, single spaces; no flag to invert.

Lines, circles, arcs and ellipses are unchanged; arcs are never written as béziers. The LCV-170
`GOLDEN` pins AC 12. The audit's `check_path` gains the two `M … C …` / `M … Q …` shapes.
`AGENTS.md`: "Arcs as `<path … A …>`, never béziers" keeps its rule and gains "Bézier entities,
and only they, as `C`/`Q` paths".

## Decisions (self-approved per user goal)

- One `Entity::Bezier` with two degrees, not two variants: one arm per consumer.
- AC 4 "every point coincides": all points within `EPSILON` of the first, after CTM and flip.
  A curve with equal endpoints but distinct controls (a loop) is imported.
- AC 13 "within EPSILON" is read as the file format's `FORMAT_TOL` (5e-5 mm), as LCV-176 AC 12.
- AC 5 chord bound: Wang's formula, `n` clamped to [1, 4096] (ADR 0016 §4).
- AC 14 narration: `cubic (x0, y0) → (x1, y1) → (x2, y2) → (x3, y3) mm`, points in path order.
- A crossing box counts only the curve, never the control polygon (AC 6/7).

## Risks

- LOC cap: `geometry/snap/mod.rs` is 258, ~262 after LCV-176, ~266 here. Seam: move
  `SnapEntity`/`SnapResult` to `snap/types.rs` if it passes 270. `path_data.rs` ~170 → ~205
  (seam: segment enum to `path_data/segment.rs`, as LCV-172 names). `export.rs` ~185 → ~200,
  `rect.rs` ~200 → ~212, `raster.rs`, `trim/mod.rs` each gain < 10.
- Mutation testing: **yes** on `src/io/svg/export.rs` and `src/geometry/bezier*`.
- `nearest` is sampled then refined (ADR 0016 §4); pick/snap tests use a clear S-curve at a
  working zoom, plus a click on an off-curve control point that must not select.
- Rebase: builds on LCV-176's arms in the same files (one more arm each). `agent_narrate.rs` also
  changes on `agent-harness` (LCV-185+): a one-arm conflict at most.
