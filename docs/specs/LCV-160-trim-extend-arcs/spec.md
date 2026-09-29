# LCV-160 — TRIM and EXTEND with arcs

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

TRIM and EXTEND silently ignore any pair that involves an arc (`src/tools/trim.rs`,
`src/document/commands/trim/mod.rs`), and a circle cannot be an extend boundary. Drawings with
rounded corners cannot be cleaned up.

## Stories

- As an operator, I want to trim and extend lines and arcs against lines, circles and arcs.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Fillet and chamfer (product non-goals).

## Open questions

- Behaviour when an arc has no boundary hit on the picked side.
