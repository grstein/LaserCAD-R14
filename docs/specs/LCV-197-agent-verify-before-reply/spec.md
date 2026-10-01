# LCV-197 — Agent verify before reply

- **Status**: Draft
- **Depends on**: LCV-190, LCV-194
- **Implementation**: -

## Problem

The agent ends its turn as soon as its calls succeed; nothing makes it compare the result with
what was asked. Verification is where the published gains are: CADCodeVerify has the model write
yes/no checks from the request, answer them against renders and fix the "no" answers (≈5% higher
success, <https://arxiv.org/abs/2410.05340>); BlenderGym finds that extra compute pays off most
when spent on verification (<https://blendergym.github.io>). LaserCAD now has tools to verify
(check, measure, capture); the harness does not ask for them.

## Stories

- As an operator, I want the agent to check its drawing against my request before it answers,
  and tell me what it verified, so that I trust "done".

## Acceptance criteria

1. WHEN the default system prompt is built, THE SYSTEM SHALL instruct the model to derive a short
   checklist of measurable requirements from the request, and after drawing to verify each item
   with `measure`, `check_drawing` or (when offered) a capture, fixing any failure before replying.
2. WHEN the agent's final reply follows a turn that mutated the drawing, THE SYSTEM SHALL expect
   the reply to list each checklist item with pass or fail; the prompt test pins this wording.
3. WHEN the model ends a mutating turn without any read-only verification call after its last
   mutation, THE SYSTEM SHALL send it one reminder to verify before accepting the reply
   (at most once per turn, within the step budget).
4. IF the step budget is exhausted, THEN THE SYSTEM SHALL end the turn as today, without the
   reminder.

## Out of scope

- A second model acting as verifier, or tree search over candidates.
- Blocking the reply on failed checks (the agent reports; the operator decides).

## Open questions

- Is the one-time reminder (AC 3) worth its cost? Decide with LCV-200 data.
