# LCV-189 — Agent step budget visibility

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

A turn allows 256 tool calls by default (`src/agent/loop_.rs::AGENT_STEP_BUDGET_DEFAULT`); every
tool call counts one step, including each call inside a provider's parallel-call wrapper, and
`create_drawing` counts one whatever its size. The model is told the static rule in the prompt but
never how much budget remains, and a reply whose calls would cross the budget ends the turn with
`IterationLimitExceeded`. In an agent session (2026-09-30, 180 actions) the model could not plan
its remaining work against the budget.

## Stories

- As an operator, I want the agent to know how many tool calls it has left so that it finishes the
  job, or reports honestly what is left, instead of being cut off mid-edit.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Changing the default budget or its settings range.
- Token or cost budgets.

## Open questions

- Remaining count on every tool result, or only once it runs low?
- Should an over-budget reply be refused with the remaining count, leaving the model one reply to
  fit or report, instead of ending the turn?
- State in the prompt how parallel calls are counted.
