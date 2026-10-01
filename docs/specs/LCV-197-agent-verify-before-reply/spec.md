# LCV-197 — Agent verify before reply

- **Status**: Specified
- **Depends on**: LCV-190, LCV-194
- **Implementation**: -

## Problem

The agent ends its turn as soon as its calls succeed; nothing makes it compare the result with what
was asked. Verification is where published gains are: CADCodeVerify has the model write yes/no
checks from the request, answer them against renders and fix the "no" answers (≈5% higher success,
<https://arxiv.org/abs/2410.05340>); BlenderGym finds extra compute pays most when spent on
verification (<https://blendergym.github.io>). LaserCAD has tools to verify (check, measure,
capture); the harness does not ask for them.

## Stories

- As an operator, I want the agent to check its drawing against my request before it answers, and
  tell me what it verified, so that I trust "done".

## Acceptance criteria

A *mutating turn* applied at least one action; a *verification call* is `measure`, `check_drawing`,
`capture_canvas` or `query_entities`.

1. WHEN the default system prompt is built, THE SYSTEM SHALL tell the model to derive a short
   checklist of measurable requirements from the request and, after drawing, verify each item with
   a verification call, fixing failures before replying (`tests/it/agent/default_prompt.rs`).
2. WHEN the default system prompt describes the reply style, THE SYSTEM SHALL tell the model to end
   a reply that changed the drawing with each checklist item marked pass or fail.
3. WHEN the model replies without tool calls in a mutating turn whose last applied action is
   followed by no verification call, THE SYSTEM SHALL not end the turn but send once the user
   message `Before you finish: verify the drawing against the request with measure, check_drawing
   or capture_canvas, fix what fails, then report each check as pass or fail.`
4. WHEN that reminder is sent, THE SYSTEM SHALL add the transcript note `Asked the agent to verify
   its work.` and count no step.
5. WHEN the model again replies without tool calls, or the turn already had its reminder, THE
   SYSTEM SHALL end the turn with that reply, as today.
6. IF no step is left, or the turn was cancelled, fenced or failed, THEN THE SYSTEM SHALL end the
   turn as today, without the reminder.
7. WHEN a turn is not mutating, THE SYSTEM SHALL never send the reminder.

## Out of scope

- A second model acting as verifier, or tree search over candidates.
- Blocking the reply on failed checks (the agent reports; the operator decides).
- A setting to turn the reminder off (a prompt override already changes the checklist wording).

## Decisions (self-approved per user goal, 2026-09-30)

- The reminder ships always on: it costs at most one extra model reply per mutating turn and only
  when the model skipped verification; LCV-200 measures it.
- The reminder text is code, not prompt, so a prompt override cannot remove it; it grants nothing.
- `query_entities` counts as verification because exact coordinates verify most requests; LCV-195
  automatic feedback does not, since the model did not ask for it.
- The reminder counts as one model reply in LCV-193 metrics.

## Open questions

- None.
