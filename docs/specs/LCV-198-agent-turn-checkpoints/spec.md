# LCV-198 — Agent turn checkpoints

- **Status**: Planned
- **Depends on**: LCV-188
- **Implementation**: -

## Problem

Inside a turn the agent cannot undo its own work: when an attempt goes wrong it must delete
entities one by one or ask the operator to press Ctrl+Z, losing the whole turn. Code-driven CAD
agents save before running code and retry from a known state
(<https://github.com/ahujasid/blender-mcp>). Iterating toward a correct part needs a cheap way back.

## Stories

- As an operator, I want the agent to try an approach and roll back its own changes if it fails,
  so that a turn ends with a clean drawing rather than debris.

## Acceptance criteria

1. WHEN the agent calls `checkpoint {name}`, THE SYSTEM SHALL record the turn's current position
   under that name, as one step that changes neither the drawing nor the history revision.
2. WHEN the agent calls `rollback {name}`, THE SYSTEM SHALL revert every change the turn made after
   that checkpoint, in reverse order, and reply `Rolled back to <name>: <k> changes undone,
   <n> entities.`
3. WHEN a rollback restores deleted or edited entities, THE SYSTEM SHALL give them back the ids
   they had at the checkpoint (LCV-188).
4. WHEN the agent rolls back to `start`, THE SYSTEM SHALL revert the whole turn; `start` exists in
   every turn and cannot be set by `checkpoint`.
5. WHEN a rollback succeeds, THE SYSTEM SHALL keep the target checkpoint and forget the checkpoints
   set after it; `checkpoint` with an existing name SHALL move that name to the current position.
6. WHEN the turn ends, THE SYSTEM SHALL leave at most one undo group for the whole turn
   (ADR 0007 §D12) holding only the surviving changes, and none when nothing survived.
7. IF the name is unknown or not 1–32 characters of `A–Z a–z 0–9 _ -`, THEN THE SYSTEM SHALL
   refuse the call, change nothing, and list the known checkpoints.
8. IF the drawing or history changed outside the turn (including an operator Undo), THEN THE
   SYSTEM SHALL answer `checkpoint` and `rollback` `Fenced` like every other call (§D14).
9. WHEN a new turn starts, THE SYSTEM SHALL know no checkpoint from an earlier turn except its own
   `start`.

## Out of scope

- Rolling back past the start of the turn or across turns; redo of a rollback.
- Checkpoints visible to the operator; persisting them.

## Decisions (self-approved per user goal, 2026-09-30)

- A rollback is the turn's own change: it re-arms nothing, keeps the fence armed at the new
  revision, and counts as one step against the LCV-189 budget like `checkpoint`.
- Rewinding inside the open flat group needs an ADR 0007 amendment (`History` gains a group
  mark and a rewind to it); architect in `/design`.
- `start` is built in so a forgetful model can still get back to a clean turn.

## Open questions

- None.
