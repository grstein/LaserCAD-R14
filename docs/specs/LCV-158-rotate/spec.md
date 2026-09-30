# LCV-158 — ROTATE, MIRROR and SCALE commands

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

Parts often need to be turned, flipped or resized to fit the bed or the material, and the
toolkit cannot do any of it. ROTATE, MIRROR and SCALE complete the AutoCAD R14 core edit set.

## Stories

- As an operator, I want to rotate the selection about a base point by a typed or picked angle.
- As an operator, I want to mirror the selection across a picked line, keeping or deleting the
  source.
- As an operator, I want to scale the selection about a base point by a typed factor.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Non-uniform scale (it would turn circles into ellipses).
- Reference angle/length sub-options.

## Open questions

- One spec or three? Mirror of an arc inverts its sweep; text entities are lines after TEXT.
