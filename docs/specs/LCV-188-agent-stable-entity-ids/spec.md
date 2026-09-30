# LCV-188 — Stable entity ids for agent edits

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

Agent tools address entities by position; a delete shifts every later index, and the tools only
narrate the shift (`src/app/agent_apply/edit.rs::shift_note`). ADR 0007 keeps positional indices
"until a stable-id demand argues otherwise". In a long agent session (2026-09-30, 180 actions over
three views and a frame) the model had to re-query and re-count after each delete, which is
fragile. It also had no way to label which entities form the side, front and top view. The turn
fence against outside changes (ADR 0007 §D14) is a good safeguard and must stay.

## Stories

- As an operator, I want the agent to refer to an entity by an id that survives other edits so
  that long edits do not hit the wrong entity.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Ids in the exported SVG (ADR 0012 forbids `id`; the export contract is unchanged).
- Relaxing the turn fence.

## Open questions

- Needs an ADR amending ADR 0007: ids for the whole document or per turn only?
- Ids only, or also named groups/tags ("side view", "frame")? Tags add a document concept; weigh it
  against layers and the KISS principle.
