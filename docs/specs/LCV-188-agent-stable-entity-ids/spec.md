# LCV-188 — Stable entity ids for agent edits

- **Status**: Draft
- **Depends on**: LCV-186, LCV-191
- **Implementation**: -

## Problem

Agent tools address entities by position; a delete shifts every later index, and the tools only
narrate the shift (`src/app/agent_apply/edit.rs::shift_note`). ADR 0007 keeps positional indices
"until a stable-id demand argues otherwise". In a long agent session (2026-09-30, 180 actions over
three views and a frame) the model had to re-query and re-count after each delete, which is
fragile. The turn fence against outside changes (ADR 0007 §D14) is a good safeguard and stays.

## Stories

- As an operator, I want the agent to refer to an entity by an id that survives other edits so
  that long edits never hit the wrong entity.

## Acceptance criteria

1. WHEN `query_entities` lists the drawing, THE SYSTEM SHALL show, for every entity, an id `e<N>`
   next to its index.
2. WHEN an entity is created (by any tool, command or import), THE SYSTEM SHALL give it an id never
   used before in this session; deleting an entity SHALL never change another entity's id.
3. WHEN an entity is edited in place (move, rotate, scale, mirror with erase, trim, extend), THE
   SYSTEM SHALL keep its id; undo and redo SHALL restore the ids the entities had.
4. WHEN every tool that takes `index` or `indices` is given `id` or `ids` instead, THE SYSTEM SHALL
   resolve them to the current entities and act as with the matching indices.
5. IF an id is unknown THEN THE SYSTEM SHALL refuse the call and change nothing.
6. WHEN a creation tool succeeds, THE SYSTEM SHALL return the new entities' ids.
7. WHEN the document is saved or exported, THE SYSTEM SHALL write no id (the SVG contract and
   ADR 0012's "no `id`" rule are unchanged).

## Out of scope

- Named groups or tags ("side view", "frame"): layers already separate parts; revisit only if a
  later session shows layers are not enough.
- Relaxing the turn fence.

## Open questions

- Needs an ADR amending ADR 0007 (id lives in `Document`, kernel-pure). Ids restart per session
  (not persisted) — confirm at /specify.
