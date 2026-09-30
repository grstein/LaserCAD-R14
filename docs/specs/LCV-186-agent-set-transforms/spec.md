# LCV-186 — Agent transforms over entity sets

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

`move_entity`, `copy_entity`, `rotate_entity`, `scale_entity`, `mirror_entity` and
`delete_entity` each take a single `index`. In an agent session (2026-09-30) shrinking and
repositioning a 48-entity view took 96 tool calls. That was slow, used up much of the per-turn step
budget, and a turn cut short would have left the view half transformed. The operator's own
commands already work on a selection set; the agent's tools do not.

## Stories

- As an operator, I want the agent to move, copy, rotate, scale, mirror or delete a whole set of
  entities in one call so that edits to a view are fast and never left half done.

## Acceptance criteria

1. WHEN one of the six tools is called with `indices` (a list of 1..=1000 document indices) instead
   of `index`, THE SYSTEM SHALL apply the operation to every listed entity as one action and one step.
2. WHEN a set rotation, scale or mirror runs, THE SYSTEM SHALL use the one base point or axis given
   in the call for every entity, as the operator's ROTATE/SCALE/MIRROR do on a selection.
3. WHEN `copy_entity` or a keep-source `mirror_entity` runs on a set, THE SYSTEM SHALL append the
   copies in ascending source-index order, each on its source's layer.
4. WHEN `delete_entity` runs on a set, THE SYSTEM SHALL remove all listed entities, and the result
   SHALL state the new entity count and that later indices shifted.
5. IF `indices` is empty, holds a duplicate or an out-of-range index, or is given together with
   `index`, THEN THE SYSTEM SHALL refuse the call, change nothing and name the offending entry.
6. IF the transform fails for any listed entity (e.g. a scale factor that is not positive) THEN THE
   SYSTEM SHALL change no entity at all.
7. WHEN a set action is undone after the turn, THE SYSTEM SHALL restore it with the rest of the turn
   in one undo (ADR 0007), exactly as a sequence of single-index calls would.
8. WHEN a tool is called with `index`, THE SYSTEM SHALL behave exactly as today.

## Out of scope

- Addressing by layer or by the operator's selection (selection changes trip the fence).
- Named groups or stable ids (LCV-188).
- Non-uniform scale, stretch, array.

## Open questions

- None.
