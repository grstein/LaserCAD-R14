# LCV-150 - New conversation button in the agent panel

- **Status**: Planned
- **Depends on**: LCV-125, LCV-141, LCV-142, LCV-153
- **Implementation**: -

## Problem

With LCV-153 the agent remembers earlier turns of the conversation shown in the chat panel
(`src/agent/panel.rs`). When the operator wants a fresh start — to reset the agent's
understanding or drop a confusing chain of exchanges — the only way today is to restart
the application. There is no in-app way to clear the transcript and the agent's memory.

## Scope

A **New Conversation** button in the agent panel that:
- Clears the visible transcript (`agent.chat`) and the conversation memory (LCV-153)
- Makes the next turn start with only the system prompt and the new prompt
- Disables itself while a turn is in progress (`agent.busy == true`)
- Never touches the drawing, undo history, or any document state
- Is always reachable in the panel (not hidden by scroll or overflow)

## Out of scope

- Confirmation dialog or undo on clear
- Archiving or exporting the cleared history
- Named conversation sessions or history persistence across app restarts
- Any change to the drawing or tool state

## Acceptance criteria

1. THE SYSTEM SHALL show a button labelled "New Conversation" in the agent panel
   (`src/agent/panel.rs`) next to the existing header controls.
2. WHEN the button is clicked THE SYSTEM SHALL empty `agent.chat` and the LCV-153
   conversation memory through one new `AgentState` method.
3. WHILE `agent.busy == true` THE SYSTEM SHALL show the button disabled and ignore clicks;
   WHILE `agent.busy == false` it SHALL be enabled.
4. WHEN the button has been clicked THE SYSTEM SHALL show an empty transcript, and the next
   turn's first request SHALL carry exactly `[system, user]`.
5. WHEN the button is clicked THE SYSTEM SHALL NOT start a turn, make any API call, or
   modify the document or its undo history.
6. WHEN a turn starts or ends THE SYSTEM SHALL update the button's enabled state in the
   same frame.
7. WHILE the application is 800×600 THE SYSTEM SHALL keep the button visible and
   clickable without scrolling the panel.

## Expected tests

- Unit: the `AgentState` clear method empties the transcript and the memory; no-op while busy.
- Paint: the button label is inside the agent panel.
- Pointer: a click on the painted button clears both; greyed out and inert while busy.
- Worker: after a clear, the next turn's first request carries exactly `[system, user]`.

## Notes

Primary files: `src/agent/panel.rs` and `src/app/agent_state.rs`. Depends on LCV-141 (panel
layout) and LCV-153 (the memory this button resets).
