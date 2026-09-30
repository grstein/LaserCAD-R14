# LCV-160 — Tasks

- [x] T1 [AC1] [AC2] [AC6] Geometry: `line_arc`, `circle_arc`, `arc_arc`, span-filtered. Unit tests
  first, covering a CW arc, an arc across ±π, a tangent case and endpoint inclusion (files:
  src/geometry/intersect/arc.rs, src/geometry/intersect/mod.rs, src/geometry/mod.rs)
- [x] T2 [AC1–9] Test: one failing integration test per AC that drives `TrimTool`/`ExtendTool` on
  `Document` + `History`. AC 8 also checks redo and the target's layer; AC 4 also covers a
  Circle cut at one point by each of two cutters (files: tests/it/document/trim_extend_arcs.rs,
  tests/it/document/mod.rs)
- [x] T3 [AC2] [AC6] Line-target helpers: `trim_line_at_points` and `extend_line_to_points`
  (Circle and Arc boundaries), with unit tests (files: src/document/commands/trim/line.rs)
- [x] T4 [AC1] [AC3] [AC5] [AC7] `trim_circle_at_points`, plus the new `trim_arc_at_points` and
  `extend_arc` with the full-turn guard, with unit tests (files: src/document/commands/trim/circle.rs,
  src/document/commands/trim/arc.rs)
- [x] T5 [AC1–3] [AC5–8] Dispatch: `cut_points`, `trim_step`, `extend_reach`. `TrimEntity` and
  `ExtendEntity` delegate to them, and `ExtendEntity` takes Arc targets. Update the module docs
  (files: src/document/commands/trim/mod.rs)
- [x] T6 [AC1–4] [AC9] `TrimTool`: pick and cutters via `cut_points`. Fold over a copy and commit
  nothing, a bare `TrimEntity`, or one `CompositeCommand` "Trim". Update the unit test
  `trim_two_cutters_undo_step_by_step` so it cites LCV-160 AC 9 (files: src/tools/trim.rs)
- [ ] T7 [AC5] [AC6] [AC7] `ExtendTool`: Line and Arc endpoints, and the boundary with the least
  `extend_reach` travel. The preview is an `Entity`, the local geometry copies go, and the status
  text says "line or arc endpoint" (files: src/tools/extend.rs)
- [ ] T8 CHANGELOG `[Unreleased]` line: TRIM and EXTEND work with arcs, and a trim click is one
  undo step (files: CHANGELOG.md)
