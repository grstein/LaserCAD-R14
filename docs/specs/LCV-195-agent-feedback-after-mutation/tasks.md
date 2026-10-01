# LCV-195 — Tasks

Prerequisites: LCV-187, LCV-189, LCV-190 Done on `agent-harness`.

- [x] T1 Refactor (no behaviour change): move the batch body of `agent_loop` to
  `loop_/batch.rs::run_batch`; the loop tests stay green (files: src/agent/loop_.rs,
  src/agent/loop_/batch.rs)
- [ ] T2 Refactor (no behaviour change): move the `settings_ui.rs` copy constants to
  `settings_ui/copy.rs` (files: src/agent/settings_ui.rs, src/agent/settings_ui/copy.rs)
- [ ] T3 [AC6] Test: a settings file without the field loads it `false`, and the field round-trips
  (files: src/io/settings/tests.rs)
- [ ] T4 [AC6] `Settings::agent_feedback_after_changes` (files: src/io/settings.rs)
- [ ] T5 [AC1][AC4][AC5] Test (loop, stub dispatch):
  - A batch that ran asks `Dispatch::Feedback` exactly once, after its last call.
  - `Ok("")` leaves the result as today.
  - `Ok(t)` gives `…\n<t>\nSteps left this turn: …`.
  - A fenced batch and an over-budget batch ask nothing.
  - The steps-left count is unchanged.

  (files: src/agent/loop_/tests.rs)
- [ ] T6 [AC4] `AgentAction::Feedback`, plus an `answer_act` arm that answers `Ok("")` uncounted
  (files: src/agent/bridge/action.rs, src/app/agent_poll.rs)
- [ ] T7 [AC1][AC5] `Dispatch::Feedback`, the append in `run_batch`, and the worker arm
  (files: src/agent/loop_.rs, src/agent/loop_/batch.rs, src/app/agent_worker.rs)
- [ ] T8 [AC1][AC2][AC4][AC6] Test (headless app):
  - Setting on, `create_line` → `feedback` gives the exact `Drawing now: 1 entity, X …, Y … mm.
    CHECK: 2 open ends`.
  - A second feedback with no new apply → `Ok("")`.
  - Setting off → `Ok("")`.
  - Delete everything → `Drawing now: 0 entities. CHECK: no problems found.`
  - Two check kinds are joined with `; `.
  - The tally steps equal the tool calls.

  (files: tests/it/agent/feedback_after_changes.rs, tests/it/agent/mod.rs)
- [ ] T9 [AC1][AC2][AC4] `agent_feedback.rs::{feedback, summary}`, `TurnState::fed_at`, and the
  real `answer_act` arm (files: src/app/agent_feedback.rs, src/app/agent_turn.rs,
  src/app/agent_poll.rs)
- [ ] T10 [AC3][AC5] Test:
  - Setting on with both opt-ins → `Observed` with a PNG. In the loop it is labelled with the last
    call's id, `AuthorizeUpload` is asked before the next send, and a `Canvas image for call <id>
    sent.` note follows.
  - One opt-in off → text only.
  - Empty drawing → no image.
  - The steps count is unchanged.

  (files: tests/it/agent/feedback_after_changes.rs, src/agent/loop_/tests.rs)
- [ ] T11 [AC3] Capture in `feedback`; `agent_capture::allowed` becomes `pub(crate)`; `run_batch`
  takes `Observed` from feedback (files: src/app/agent_feedback.rs, src/app/agent_capture.rs,
  src/agent/loop_/batch.rs)
- [ ] T12 [AC7] Test: Agent Settings shows the checkbox `Feedback after changes` and the hint text,
  and toggling it sets `changed` (files: src/agent/settings_ui/tests.rs)
- [ ] T13 [AC7] The checkbox and hint (files: src/agent/settings_ui.rs,
  src/agent/settings_ui/copy.rs)
- [ ] T14 Docs: AGENTS.md purity list (`loop_/batch.rs`, `settings_ui/copy.rs`); CHANGELOG line
  (files: AGENTS.md, CHANGELOG.md)
- [ ] T15 Docs: append plan.md's "ADR amendment" to ADR 0007 as the next free `Amended (n)`
  (files: docs/adr/0007-agent-turn-mutates-the-live-document.md)
- [ ] T16 `scripts/mutants.sh` on the diff (`src/agent/loop_*`, `agent_feedback.rs`). Kill the
  survivors or justify them in the commit body.
