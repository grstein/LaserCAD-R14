# LCV-186 — Tasks

- [x] T1 [AC5] Test: empty, >1000, duplicate, negative/non-integer, `index`+`indices` each refused naming the entry (files: tests/it/agent/transform_tools.rs)
- [x] T2 [AC5] [AC6] `SetOp`/`Set` variant and the `indices` parser reusing today's argument validators; route from `parse_tool_call` (files: src/agent/bridge/action.rs, src/agent/tools/transform.rs, src/agent/tools.rs)
- [ ] T3 [AC1] Schemas: optional `indices` on the six tools, `index` no longer required; test the published schema (files: src/agent/tools/schema.rs, tests/it/agent/transform_tools.rs)
- [ ] T4 [AC1] [AC2] [AC3] [AC4] [AC6] Test: set move/rotate/scale/mirror use one base; copies/keep-source mirrors appended ascending on source layers; delete result states count and shift; bad factor changes nothing; out-of-range index refused, nothing changed (files: tests/it/agent/transform_tools.rs)
- [ ] T5 [AC1] [AC2] [AC3] [AC4] [AC5] App side: range check, sort, one command, narration (files: src/app/agent_apply/set.rs, src/app/agent_apply.rs)
- [ ] T6 [AC7] Test: a turn mixing a set action and single calls undoes in one step (files: tests/it/agent/turn_group.rs)
- [ ] T7 [AC8] Test: every tool with `index` gives today's result text and document (files: tests/it/agent/transform_tools.rs)
- [ ] T8 Prompt: the six paragraphs name `indices` (default_prompt.rs whole-word check) (files: src/agent/prompt.rs, tests/it/agent/default_prompt.rs)
- [ ] T9 AGENTS.md purity list gains `tools/transform.rs` (files: AGENTS.md)
- [ ] T10 CHANGELOG line (files: CHANGELOG.md)
