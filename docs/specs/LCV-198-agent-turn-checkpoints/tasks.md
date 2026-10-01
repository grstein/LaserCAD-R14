# LCV-198 — Tasks

- [x] T1 Seam, no behavior change: move the group `impl` block of `History` to `history/group.rs`
  (files: src/document/history.rs, src/document/history/group.rs)
- [x] T2 [AC2] [AC6] `History::group_len` and `rewind_group`, with unit tests: reverse order, one
  revision bump, no-op past `group_len` or with no group, `end_group` after a rewind seals only
  the survivors and nothing when none survive (files: src/document/history/group.rs,
  src/document/history/tests.rs)
- [x] T3 [AC1] [AC2] `AgentAction::{Checkpoint, Rollback}`, `tool_name`, parse arms, schemas,
  `expected_form` rows, with parser unit tests (files: src/agent/bridge/action.rs,
  src/agent/tools.rs, src/agent/tools/schema.rs)
- [x] T4 [AC4] [AC5] [AC7] [AC9] `Checkpoints` + `valid_name` + `checkpoint`/`rollback` in
  `agent_checkpoint.rs` with unit tests. The `TurnState` field is reset by `arm_turn`
  (files: src/app/agent_checkpoint.rs, src/app/agent_turn.rs, src/app/mod.rs)
- [x] T5 [AC1] [AC2] Wire both actions into `agent_apply::apply` (files: src/app/agent_apply.rs)
- [x] T6 [AC1]–[AC9] Integration tests, one per AC, driven by `arm_turn` + `poll_agent_rx` as in
  `turn_metrics.rs`. AC3 deletes and moves `e2` after a checkpoint, rolls back, and expects
  `e2` at its old geometry (files: tests/it/agent/checkpoints.rs, tests/it/agent/mod.rs)
- [x] T7 `DEFAULT_PROMPT` checkpoint sentence; update the prompt and tool-list scans if they
  enumerate tools (files: src/agent/prompt.rs, tests/it/repo/prompt_scans.rs)
- [x] T8 Append the plan's "ADR amendment" to ADR 0007 as the next free "Amended (n)": a header
  line plus the §D12 block (files: docs/adr/0007-agent-turn-mutates-the-live-document.md)
- [ ] T9 `scripts/mutants.sh` on the diff; kill or justify survivors (files: src/document/history/tests.rs)
- [x] T10 CHANGELOG line (files: CHANGELOG.md)
