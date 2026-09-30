# LCV-189 — Plan

## Approach

All in `loop_.rs::agent_loop`, the only place that counts steps. After a batch runs, append
`\nSteps left this turn: <n> of <budget>.` to the text of the batch's last tool result (AC1;
image parts untouched). The preflight overrun no longer ends the turn at once: push the
assistant message, answer every call with `not run: this reply has <k> tool calls but <n> steps
are left`, set an `overran` flag and send again (AC2). A second consecutive overrun returns
`IterationLimitExceeded(step_budget)` as today (AC3). `AuthorizeUpload` stays outside the count
and the budget range is untouched (AC5). The prompt's STEP BUDGET paragraph is rewritten (AC4).

## Touches

- `src/agent/loop_.rs::agent_loop` — steps-left suffix; one grace reply on overrun.
- `src/agent/loop_/tests.rs` — unit tests with a scripted `send_fn`.
- `src/agent/prompt.rs::DEFAULT_PROMPT` — STEP BUDGET paragraph (parallel calls count one each;
  create_drawing and set operations count one; the remaining count arrives in tool results;
  drop "the turn ends at once, with no message to you"); create_drawing paragraph unchanged.
- ADRs: ADR 0007 §D13 amended — an overrunning reply gets one "not run" answer and one more
  reply before the turn ends; the whole-batch preflight rule itself stays.

## Decisions (self-approved per user goal)

- The grace is "one more reply" per overrun: the flag clears once a batch actually runs, so a
  later overrun gets its own single grace (the spec says "that next reply").
- Not-run answers do not consume steps; the steps-left line is not added to them (they already
  state `<n>`).
- The suffix goes on the last tool result *text* even when that call returned an image.

## Risks

- LOC cap: `loop_.rs` is at 231; the change adds ~25. Seam if it passes 270:
  `src/agent/loop_/budget.rs` (kernel-pure; add to the AGENTS.md purity list).
- Mutation testing: yes — `src/agent/`; off-by-one in `dispatched + calls.len() > budget` and in
  the steps-left arithmetic must be pinned (exact-budget batch runs; budget+1 does not).
- The fence stop path (`FENCE_STOP_PLACEHOLDER`) must not get the suffix twice or be counted as
  a grace; a test with a fence refusal in the batch pins it.
- Cross-spec overlap: `loop_.rs` is edited again by LCV-187 (post-send notes); `prompt.rs` by 185/186/187.
