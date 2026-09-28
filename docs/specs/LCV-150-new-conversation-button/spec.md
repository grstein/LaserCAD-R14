# LCV-150 - New conversation button in the agent panel

- **Status**: Draft
- **Depends on**: LCV-125, LCV-141, LCV-142
- **Implementation**: -

## Problem

The agent maintains a multi-turn conversation history in the chat panel
(`src/agent/panel.rs`). When the operator wants to start a fresh conversation
without the accumulated context — to reset the agent's understanding or clear a
confusing chain of exchanges — they must close and reopen the application.
There is no in-app way to clear the transcript and restart with a clean slate
and the current system prompt.

## Scope

A **New Conversation** button in the agent panel that:
- Clears all prior messages from the chat history (the `agent.chat` vector)
- Starts the next turn fresh with only the system prompt and no prior context
- Disables itself while a turn is in progress (`agent.busy == true`)
- Never touches the drawing, undo history, or any document state
- Is always reachable in the panel (not hidden by scroll or overflow)

## Out of scope

- Confirmation dialog or undo on clear
- Archiving or exporting the cleared history
- Named conversation sessions or history persistence across app restarts
- Any change to the drawing or tool state

## Acceptance criteria

1. A button labelled "New Conversation" appears in the agent panel
   (`src/agent/panel.rs`) alongside or adjacent to the existing UI controls
2. Clicking the button clears `agent.chat` by calling a new method on
   `AgentState` (or equivalent) that removes all prior messages, leaving an
   empty vector
3. The button is disabled when `agent.busy == true` (a turn is in progress)
   and enabled when `agent.busy == false`
4. After the button is clicked, the panel shows an empty transcript and the
   next turn starts with only the system prompt as context
5. Clicking the button does not trigger a turn, make any API call, or modify
   the document
6. The button's enabled/disabled state updates immediately when a turn
   starts or ends
7. At 800×600 application size, the button is visible and clickable without
   scrolling the panel

## Expected tests

- Unit: `AgentState` has a method to clear the chat history; calling it
  produces an empty vector
- Integration: a harness paint test or pointer test confirms the button
  label is visible in the agent panel
- Pointer test: simulate a click on the painted button and verify the
  transcript clears
- State test: button is clickable when `agent.busy == false`, greyed out
  when `agent.busy == true`

## Notes

Primary files: `src/agent/panel.rs` and `src/agent/wire.rs` (or equivalent
agent state holder). Depends on LCV-141 (panel layout) and requires the panel
to remain reachable during any new work.
