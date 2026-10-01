# LCV-197 — Tasks

Prerequisites: LCV-190, LCV-194, LCV-195 Done on `agent-harness`. If LCV-195 is not Done, first
do LCV-195 T1 (the `loop_/batch.rs` seam) as T0 here.

- [x] T1 [AC1][AC2] Test, then the `DEFAULT_PROMPT` checklist paragraph (derive measurable checks
  from the request, verify each with `measure`, `check_drawing` or `capture_canvas`, fix failures
  before replying) and the reply-style line (end with each check marked pass or fail)
  (files: src/agent/prompt.rs, tests/it/agent/default_prompt.rs)
- [x] T2 [AC3][AC4][AC7] Test (`VerifyState`):
  - apply → due once; a second ask → not due;
  - apply then an answered `measure`, `check_drawing`, `capture_canvas` or `query_entities` →
    not due;
  - apply then a refused `measure` → due;
  - verification then a later apply → due;
  - a query-only turn → never due.

  (files: src/app/agent_verify.rs)
- [x] T3 [AC3][AC7] `VerifyState`, `is_verification`, `TurnState::verify`
  (files: src/app/agent_verify.rs, src/app/agent_turn.rs, src/app/mod.rs)
- [x] T4 [AC3][AC4] `AgentAction::VerifyDue`, plus the `answer_act` arm (yes → `reminded`, the note
  row `Asked the agent to verify its work.`, `Ok`; no → `Refused`; never counted) and
  `verify.after` around `apply_fenced` (files: src/agent/bridge/action.rs,
  src/app/agent_poll.rs)
- [ ] T5 [AC3][AC5][AC6] Test (loop, stub dispatch):
  - On a text reply with a yes, the loop pushes the assistant text and the user message equal to
    `VERIFY_REMINDER`, sends again, and returns the second text.
  - A no ends the turn with the first text.
  - A second text reply is never asked about.
  - With no step left, it is never asked.
  - After a fence stop (`last_word`), it is never asked.
  - A cancelled ask returns `Cancelled`.
  - Tool calls after the reminder dispatch normally.

  (files: src/agent/loop_/tests.rs)
- [ ] T6 [AC3][AC5][AC6] `Dispatch::VerifyDue`, `loop_/verify.rs::{VERIFY_REMINDER,
  verify_or_end}`, and the worker arm (files: src/agent/loop_.rs, src/agent/loop_/verify.rs,
  src/app/agent_worker.rs)
- [ ] T7 [AC3][AC4] Test (scripted turn on the headless app):
  - `create_line` then a text reply → the reminder is sent once, the `note` row appears, and the
    step tally equals the tool calls.
  - `tally.replies` counts the extra reply.
  - The final reply comes from the second answer.

  (files: tests/it/agent/verify_before_reply.rs, tests/it/agent/mod.rs)
- [ ] T8 [AC6][AC7] Test (scripted turn):
  - A query-only turn → no reminder.
  - A turn whose `create_line` is followed by `check_drawing` → no reminder.
  - A fenced turn → no reminder.
  - Step budget 1 spent on the create → no reminder.

  (files: tests/it/agent/verify_before_reply.rs)
- [ ] T9 [AC5] Test: memory after a reminded turn holds neither the reminder nor the interim reply
  (files: tests/it/agent/verify_before_reply.rs)
- [ ] T10 Docs: AGENTS.md purity list (`loop_/verify.rs`); CHANGELOG line
  (files: AGENTS.md, CHANGELOG.md)
- [ ] T11 `scripts/mutants.sh` on the diff (`src/agent/loop_*`, `agent_verify.rs`). Kill the
  survivors or justify them in the commit body.
- [ ] T12 Docs: append plan.md's "ADR amendment" to ADR 0007 as the next free `Amended (n)`
  (files: docs/adr/0007-agent-turn-mutates-the-live-document.md)
