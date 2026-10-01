# LCV-194 — Tasks

Prerequisites: LCV-188, LCV-190, LCV-192 Done on `agent-harness`.

- [x] T1 [AC1] Test: closest points for every pair of point, line, circle and arc. Cases:
  - crossing segments → 0;
  - parallel and skew segments;
  - a line outside, inside and through a circle;
  - a line whose perpendicular foot misses an arc's span → the endpoint;
  - circles that are separate, nested or concentric;
  - an arc and an arc;
  - a point and each kind.

  (files: src/geometry/distance.rs)
- [x] T2 [AC1] `Prim`, `closest`: intersections first, then the candidate minimum with the span
  filter (files: src/geometry/distance.rs, src/geometry/mod.rs)
- [x] T3 [AC5] Test: `overlaps`:
  - collinear overlapping segments → true;
  - collinear segments touching end to end → false;
  - the same circle → true;
  - concentric equal-radius arcs with shared or disjoint spans;
  - an arc lying on a circle.

  (files: src/geometry/overlap.rs)
- [x] T4 [AC5] `overlaps` (files: src/geometry/overlap.rs, src/geometry/mod.rs)
- [x] T5 [AC8] Test (parser):
  - each query builds `Measure`;
  - refusals in the LCV-192 shape: unknown `query`; both `indices` and `ids`; a wrong operand count
    per query; `points` on `length`/`bbox`/`intersections`/`angle`; a non-finite
    `points[1].y`; an empty `indices`.

  (files: src/agent/tools/tests.rs)
- [x] T6 [AC8] `MeasureQuery`, `MeasureRequest`, the `Measure` variant and `tool_name`
  (files: src/agent/bridge/action/measure.rs, src/agent/bridge/action.rs, src/agent/bridge.rs)
- [x] T7 [AC8] `tools/measure.rs::parse` and the `parse_tool_call` arm
  (files: src/agent/tools/measure.rs, src/agent/tools.rs)
- [x] T8 [AC8][AC9] The schema entry after `check_drawing`, the `expected_form` rows, and the
  registry-order test (files: src/agent/tools/schema.rs, src/agent/tools/args.rs,
  src/agent/tools/tests.rs)
- [x] T9 [AC1–AC6][AC8] Test (apply), exact strings for every query on a fixture document:
  - distance point–point, point–entity and entity–entity, with touching entities giving 0;
  - the length of a line, an arc and a circle;
  - bbox of listed entities and of the whole drawing, including an Output-off layer, with an arc
    bulge;
  - bbox on an empty drawing → `bbox: the drawing is empty`;
  - intersections: points, `none` and `overlap`;
  - angle directed and undirected (90/270, parallel, antiparallel);
  - `angle` on a circle → refused, naming `indices[1]`;
  - an unknown index and an unknown id → refused.

  (files: tests/it/agent/measure.rs, tests/it/agent/mod.rs)
- [x] T10 [AC1–AC6] `agent_apply/measure.rs::answer` and the `plan` arm
  (files: src/app/agent_apply/measure.rs, src/app/agent_apply.rs)
- [x] T11 [AC7] Test (headless turn): `measure` answers and the step tally goes up by one. The
  revision, selection, undo depth and dirty flag are unchanged, and a later `create_line` in the
  same turn still applies (no fence trip). (files: tests/it/agent/measure.rs)
- [ ] T12 [AC9] Test, then the `DEFAULT_PROMPT` lines for `measure` and its five queries
  (files: src/agent/prompt.rs, tests/it/agent/default_prompt.rs)
- [ ] T13 Docs: the AGENTS.md purity list gains `bridge/action/measure.rs` and `tools/measure.rs`;
  CHANGELOG line (files: AGENTS.md, CHANGELOG.md)
- [ ] T14 `scripts/mutants.sh` on the diff (`src/agent/`, `src/geometry/distance.rs`,
  `overlap.rs`). Kill the survivors or justify them in the commit body.
