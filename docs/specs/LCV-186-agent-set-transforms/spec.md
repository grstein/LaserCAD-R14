# LCV-186 — Agent transforms over entity sets

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

`move_entity`, `copy_entity`, `rotate_entity`, `scale_entity` and `mirror_entity` each take a
single `index`. In an agent session (2026-09-30) shrinking and repositioning a 48-entity view took
96 tool calls: slow, costly in the per-turn step budget, and a turn cut short leaves the view half
transformed. The operator's own commands already work on a selection set; the agent does not.

## Stories

- As an operator, I want the agent to move, copy, rotate, scale or mirror a whole set of entities
  in one call so that edits to a view are fast and never left half done.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Named groups or tags (LCV-188's open question).
- Non-uniform scale, stretch or array.

## Open questions

- Index list only, or also "all entities on layer L" / "the current selection"?
- Extend the existing tools with an index list, or add set-level tools?
- One refused index refuses the whole set (atomic) — confirm, and the undo granularity stays the
  whole turn (ADR 0007).
