# LCV-158 — Tasks

ADR 0003 amendment (5) (commit `868bb01`) admits `ToolKind::Rotate`. The `agent/tools/schema.rs` and
`app/agent_apply/edit.rs` seams land in LCV-157 (T5/T6).

- [x] T1 [P] [AC6] Test: rotate point/line/circle/arc; property test for rigidity, radii, sweep, arc endpoints (files: tests/it/geometry/transform_props.rs, tests/it/geometry/mod.rs)
- [x] T2 [AC6] Implement `Transform::Rotate` + methods, re-export (files: src/geometry/transform.rs, src/geometry/mod.rs)
- [x] T3 [AC6] Test then implement `Entity::transformed` (files: src/document/entity.rs)
- [x] T4 [AC7] Test then implement `TransformEntities`: do/undo/redo bit-exact, layer and selection kept (files: src/document/commands/transform.rs, src/document/commands/mod.rs, src/document/mod.rs)
- [x] T5 [P] [AC1] Test then add `ToolKind::Rotate`, aliases `rotate`/`ro`, `make` arm (files: src/cmdline/mod.rs, src/cmdline/parse.rs, src/tools/mod.rs)
- [x] T6 [AC2] [AC3] [AC4] [AC5] [AC7] [AC8] Test then implement `RotateTool`: prompts, empty-selection no-op, preview, typed degrees, picked angle, zero angle, successor SELECT (files: src/tools/rotate.rs, src/tools/mod.rs)
- [x] T7 [AC4] [AC5] Integration: `ro` ⏎ `0,0` ⏎ `90` ⏎ and a picked angle through `App` (files: tests/it/cmdline/transform_commands.rs, tests/it/cmdline/mod.rs)
- [x] T8 [AC9] Test then implement `rotate_entity` parse → `AgentAction::Rotate` (files: src/agent/bridge.rs, src/agent/tools.rs, src/agent/tools/schema.rs)
- [x] T9 [AC9] Test then implement the apply arm + built-in prompt line (files: src/app/agent_apply/edit.rs, src/agent/prompt.rs, tests/it/agent/transform_tools.rs)
- [x] T10 CHANGELOG line (files: CHANGELOG.md)
