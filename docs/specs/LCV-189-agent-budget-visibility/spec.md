# LCV-189 — Agent step budget visibility

- **Status**: Specified
- **Depends on**: none
- **Implementation**: -

## Problem

A turn allows 256 tool calls by default (`src/agent/loop_.rs::AGENT_STEP_BUDGET_DEFAULT`). Every
tool call counts one step, including each call inside a provider's parallel-call wrapper, and
`create_drawing` counts one whatever its size. The model is told the static rule in the prompt but
never how much budget remains, and a reply whose calls would cross the budget ends the turn with
`IterationLimitExceeded`. In an agent session (2026-09-30, 180 actions) the model could not plan
its remaining work against the budget.

## Stories

- As an operator, I want the agent to know how many tool calls it has left so that it finishes the
  job, or reports honestly what is left, instead of being cut off mid-edit.

## Acceptance criteria

1. WHEN a batch of tool calls from one reply has run, THE SYSTEM SHALL end the last tool result of
   that batch with `Steps left this turn: <n> of <budget>.`
2. IF the tool calls of one reply exceed the steps left THEN THE SYSTEM SHALL run none of them,
   answer each with `not run: this reply has <k> tool calls but <n> steps are left`, and give the
   model one more reply.
3. IF that next reply again exceeds the steps left THEN THE SYSTEM SHALL end the turn with
   `IterationLimitExceeded`, as today.
4. WHEN the built-in prompt describes the budget, THE SYSTEM SHALL state that each call in a
   parallel batch counts one step, that `create_drawing` and set operations count one, and that
   the remaining count arrives in tool results.
5. WHEN steps are counted, THE SYSTEM SHALL keep today's rules: `AuthorizeUpload` is not a step and
   the budget range stays 1..=4096.

## Out of scope

- Changing the default budget.
- Token or cost budgets.

## Open questions

- None.
