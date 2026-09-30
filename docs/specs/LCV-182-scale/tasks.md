# LCV-182 — Tasks

ADR 0003 amendment (5) (commit `868bb01`) admits `ToolKind::Scale`; no ADR task here.

- [x] T1 [AC4] Test: scale point/line/circle/arc about a base point; property test for ratio of lengths/radii and unchanged angles (files: tests/it/geometry/transform_props.rs)
- [x] T2 [AC4] [AC6] Implement `Transform::Scale`, including a `Scale { factor ≈ 1 }` arm in `Transform::is_identity` (from LCV-158) (files: src/geometry/transform.rs)
- [x] T3 [P] [AC1] Test then add `ToolKind::Scale`, aliases `scale`/`sc`, `make` arm (files: src/cmdline/mod.rs, src/cmdline/parse.rs, src/tools/mod.rs)
- [x] T4 [AC2] [AC3] [AC4] [AC5] [AC6] [AC7] Test then implement `ScaleTool` (prompts, preview, typed/picked factor, reject ≤0/non-finite, factor 1 no-op, successor) (files: src/tools/scale.rs, src/tools/mod.rs)
- [x] T5 [AC4] [AC5] Integration: `sc` ⏎ `0,0` ⏎ `2` ⏎, and `-1` shows the refusal line, through `App` (files: tests/it/cmdline/transform_commands.rs)
- [ ] T6 [AC8] Test then implement `scale_entity` parse → `AgentAction::Scale` (files: src/agent/bridge.rs, src/agent/tools.rs, src/agent/tools/schema.rs)
- [ ] T7 [AC8] Test then implement apply arm + prompt line (files: src/app/agent_apply/edit.rs, src/agent/prompt.rs, tests/it/agent/transform_tools.rs)
- [ ] T8 CHANGELOG line (files: CHANGELOG.md)
