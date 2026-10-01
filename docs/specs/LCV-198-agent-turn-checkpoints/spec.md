# LCV-198 — Agent turn checkpoints

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

Inside a turn the agent cannot undo its own work: when an attempt goes wrong it must delete
entities by index (which renumbers them) or ask the operator to press Ctrl+Z, losing the whole
turn. Code-driven CAD agents rely on "save before running code" and retry from a known state
(<https://github.com/ahujasid/blender-mcp>). Iterating toward a correct part needs a cheap way
back.

## Stories

- As an operator, I want the agent to try an approach and roll back its own changes if it fails,
  so that a turn ends with a clean drawing rather than debris.

## Acceptance criteria

1. WHEN the agent calls `checkpoint {name}`, THE SYSTEM SHALL record the turn's current position
   in history under that name, as one step that does not mutate the drawing.
2. WHEN the agent calls `rollback {name}`, THE SYSTEM SHALL revert every change the turn made
   after that checkpoint and report the resulting entity count.
3. WHEN the turn ends, THE SYSTEM SHALL still leave exactly one undo group for the whole turn
   (ADR 0007), containing only the surviving changes.
4. IF the name is unknown, THEN THE SYSTEM SHALL refuse listing the known checkpoints.
5. IF the drawing changed outside the turn, THEN THE SYSTEM SHALL fence the call as today.

## Out of scope

- Rolling back past the start of the turn or across turns.
- Checkpoints visible to the operator.

## Open questions

- Needs an ADR amending ADR 0007 (rewinding inside an open flat group); architect in `/design`.
