# LCV-189 — Tasks

- [x] T1 [AC1] Test: after a run batch, the last tool result ends with `Steps left this turn: <n> of <budget>.`; exact-budget batch still runs (files: src/agent/loop_/tests.rs)
- [x] T2 [AC1] Append the steps-left line to the batch's last tool result (files: src/agent/loop_.rs)
- [ ] T3 [AC2] [AC3] Test: overrun answers each call with the not-run text, nothing dispatched, one more send; a second consecutive overrun returns `IterationLimitExceeded`; a run batch in between resets the grace (files: src/agent/loop_/tests.rs)
- [ ] T4 [AC2] [AC3] Grace-reply path with the `overran` flag (files: src/agent/loop_.rs)
- [ ] T5 [AC5] Test: `AuthorizeUpload` is not counted in steps left; budget clamp 1..=4096 unchanged (files: src/agent/loop_/tests.rs, tests/it/agent/turn.rs)
- [ ] T6 [AC4] Test + prompt: STEP BUDGET paragraph states per-call counting, create_drawing/set = one step, remaining count in tool results (files: tests/it/agent/default_prompt.rs, src/agent/prompt.rs)
- [ ] T7 Amend ADR 0007 §D13 (grace reply) (files: docs/adr/0007-agent-turn-mutates-the-live-document.md)
- [ ] T8 CHANGELOG line (files: CHANGELOG.md)
