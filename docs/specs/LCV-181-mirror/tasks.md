# LCV-181 — Tasks

ADR 0003 amendment (5) (commit `868bb01`) admits `ToolKind::Mirror`; no ADR task here.

- [x] T1 [AC8] Test: mirror point/line/circle/arc, mirror∘mirror = id, arc endpoints reflect, export `sweep` inverted and `large` kept (files: tests/it/geometry/transform_props.rs, tests/it/io_svg/mirror_arc.rs, tests/it/io_svg/mod.rs)
- [x] T2 [AC8] Implement `Transform::Mirror` (files: src/geometry/transform.rs)
- [x] T3 [AC6] [AC7] [AC9] Test then implement `TransformEntities` `keep_source` (append on source layers, undo/redo) (files: src/document/commands/transform.rs)
- [ ] T4 [P] [AC1] Test then add `ToolKind::Mirror`, aliases `mirror`/`mi`, `make` arm (files: src/cmdline/mod.rs, src/cmdline/parse.rs, src/tools/mod.rs)
- [x] T5 [AC1] [AC2] [AC3] [AC4] [AC5] [AC6] [AC7] [AC9] Test then implement `MirrorTool` (phases, raw Yes/No, Escape, successor) (files: src/tools/mirror.rs, src/tools/mod.rs)
- [ ] T6 [AC5] [AC6] [AC7] Integration: `mi` ⏎ two points ⏎ blank / `y` through `App` (files: tests/it/cmdline/transform_commands.rs)
- [ ] T7 [AC10] Test then implement `mirror_entity` parse → `AgentAction::Mirror` (files: src/agent/bridge.rs, src/agent/tools.rs, src/agent/tools/schema.rs)
- [ ] T8 [AC10] Test then implement apply arm + prompt line (files: src/app/agent_apply/edit.rs, src/agent/prompt.rs, tests/it/agent/transform_tools.rs)
- [ ] T9 CHANGELOG line (files: CHANGELOG.md)
