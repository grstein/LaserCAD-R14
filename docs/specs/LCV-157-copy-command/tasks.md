# LCV-157 — Tasks

ADR 0003 amendment (5) (commit `868bb01`) admits `ToolKind::Copy` (and `Rotate`, `Mirror`, `Scale`, `Dist`); no ADR task here.

- [x] T1 [AC4, AC6] Test first, then `CopyEntities`: appends translated clones on their source layers, sources unchanged, undo truncates, redo is identical, undo during a run keeps indices valid (files: src/document/commands/edit.rs, src/document/commands/mod.rs, src/document/mod.rs)
- [x] T2 [AC2, AC3, AC5, AC7, AC8] Test first, then `CopyTool`: prompts, empty-selection no-op, base point as anchor, preview at the cursor, stays armed after a placement, zero-delta ignored, Enter/Escape back to the base-point prompt (files: src/tools/copy.rs)
- [ ] T3 [AC1] `ToolKind::Copy`, word rows `copy`/`co`/`cp` with parse tests, `tools::make` arm (files: src/cmdline/mod.rs, src/cmdline/parse.rs, src/tools/mod.rs)
- [ ] T4 [AC1, AC3–AC6, AC8] Headless command-line test: select a line on a non-current layer, `co` ⏎ `0,0` ⏎ `@10,0` ⏎ `@20,0` ⏎ ⏎. Expect 3 entities, copies on the source layer, one Ctrl+Z removes only the last copy (files: tests/it/cmdline/copy_command.rs, tests/it/cmdline/mod.rs)
- [ ] T5 Seam (no behavior change): move `base_definitions` JSON into `agent/tools/schema.rs`; list it kernel-pure in AGENTS.md (files: src/agent/tools.rs, src/agent/tools/schema.rs, AGENTS.md)
- [ ] T6 [P] Seam (no behavior change): move the Delete/Move arms of `plan` into `app/agent_apply/edit.rs` (files: src/app/agent_apply.rs, src/app/agent_apply/edit.rs)
- [ ] T7 [AC9] `AgentAction::Copy` and the `copy_entity` schema entry (files: src/agent/bridge.rs, src/agent/tools/schema.rs)
- [ ] T8 [AC9] Test first, then the `copy_entity` parse arm and the schema-order doc (files: src/agent/tools.rs, src/agent/tools/tests.rs)
- [ ] T9 [AC9] Test first, then the `Copy` apply arm (out-of-range refused, copy lands on the source layer) and the `copy_entity` line in the built-in prompt (files: src/app/agent_apply/edit.rs, src/app/agent_apply/tests.rs, src/agent/prompt.rs)
- [ ] T10 CHANGELOG line under Unreleased/Added (files: CHANGELOG.md)
