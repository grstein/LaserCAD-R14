# LCV-160 — Plan

## Approach

Add three span-aware intersection routines to the geometry kernel (`line_arc`, `circle_arc`,
`arc_arc`). Each one runs the existing circle routine on the arc's parent circle and then keeps
only the points where `Arc::contains_angle` holds. That makes them the *cut points* of the spec,
and LCV-161 can reuse them later for arc intersection snaps.

The trim commands keep their pairwise shape, one target and one cutter. Each helper now takes the
cut points instead of solving them:
- `trim_line_at_points` is the old `trim_line_by_circle` body.
- `trim_circle_at_points` needs exactly 2 points.
- `trim_arc_at_points` keeps the target's centre, radius and `ccw`.

A pure dispatcher in `trim/mod.rs` maps any (target, cutter) pair to those helpers. The commands
and both tools call it, so the tools stop duplicating geometry.

For AC 9, `TrimTool` folds the cutters over a copy of the target. It commits nothing if the
entity did not change. One changing cutter gives a bare `TrimEntity`; two or more give one
`CompositeCommand` labelled `Trim`. It does not use `History::begin_group`, because that group is
the agent fence (ADR 0007 §D14) and must stay agent-only.

EXTEND grows an Arc endpoint along its parent circle. The nearest boundary is the one with the
smallest angular travel in the growth direction. A travel of `TAU - sweep` or more, which would
close the arc into a full turn, is rejected (AC 7).

## Touches

- `src/geometry/intersect/arc.rs` (new) — `line_arc`, `circle_arc`, `arc_arc` plus unit tests;
  re-exported from `intersect/mod.rs` and `geometry/mod.rs`.
- `src/document/commands/trim/line.rs` — `trim_line_at_points` (generalises
  `trim_line_by_circle`); `extend_line_to_points`, used by the circle and arc boundaries.
- `src/document/commands/trim/circle.rs` — `trim_circle_at_points` (exactly two points, else
  `None`).
- `src/document/commands/trim/arc.rs` (new) — `trim_arc_at_points` and `extend_arc`. Cut points
  at the arc's own endpoints are ignored. The travel is measured from the endpoint, away from the
  other end, and must be below `TAU - sweep`.
- `src/document/commands/trim/mod.rs`:
  - `cut_points(target, cutter) -> Vec<Vec2>`.
  - `trim_step(target, cutter, keep) -> Option<Entity>`.
  - `extend_reach(target, boundary, ep) -> Option<(Entity, f64)>`, where the `f64` is the travel
    in mm.
  - `TrimEntity::do_` and `ExtendEntity::do_` delegate to these. ExtendEntity accepts an `Arc`
    target, with `ep` 0 for start and 1 for end.
  - Update the module docs' supported-pairs table.
- `src/tools/trim.rs` — pick and cutter filtering via `cut_points`; fold and commit once (AC 4, 9).
  Also update the unit test `trim_two_cutters_undo_step_by_step`: LCV-050 AC#9 (two undos) is
  superseded by LCV-160 AC 9.
- `src/tools/extend.rs` — endpoint pick over Line and Arc endpoints. The boundary is the minimum
  `extend_reach` travel across all other entities. The hover state keeps an `Entity` preview.
  Drop the local `pt`/`chits` copies. The status text reads "line or arc endpoint".
- `tests/it/document/trim_extend_arcs.rs` (new, registered in `tests/it/document/mod.rs`) — one
  test per AC, driving `TrimTool` and `ExtendTool` on a `Document` + `History`.
- ADRs: none. No module-boundary change: new files live inside existing kernel modules, and
  the public additions are three `geometry::` functions.

## Risks

- LOC cap: every touched file stays well under 270 impl LOC (`trim/mod.rs` 170 → ~215,
  `trim/line.rs` 139 → ~160, `tools/extend.rs` 125 → ~120). The new files are ~90 and ~120.
- Mutation testing: no. It does not touch `src/agent/`, the SVG export or `History`.
- Angle wrap and direction (CW arcs, arcs crossing ±π). The AC tests use a CW arc and an arc
  spanning the −X axis. Unit tests pin endpoint-coincident cut points, which are ignored, so
  that a trim at an arc's own end is not a silent no-op commit.
- Sequential cutters on a Circle target: the first 2-point cutter turns it into an Arc, and later
  cutters trim that Arc. A Circle cut by two cutters at one point each stays a no-op (AC 4, per
  cutter), and a test pins it.
- LCV-050 AC#9 behaviour change (one undo per click). The changed unit test cites LCV-160 AC 9.
