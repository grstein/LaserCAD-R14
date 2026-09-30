# LCV-182 — SCALE command

- **Status**: In Progress
- **Depends on**: LCV-158
- **Implementation**: -

## Problem

Fitting a part to a given size means redrawing it. SCALE resizes the selection uniformly
about a base point. It reuses the transform layer from LCV-158.

## Stories

- As an operator, I want to scale the selection about a base point by a typed factor.

## Acceptance criteria

1. WHEN the user types `scale` or `sc` THE SYSTEM SHALL activate SCALE with the prompt
   `SCALE Specify base point:`.
2. WHEN SCALE is active with an empty selection and the user picks a point THE SYSTEM SHALL
   leave the drawing unchanged.
3. WHEN a base point is fixed THE SYSTEM SHALL prompt `SCALE Specify scale factor:` and
   preview the selection scaled by the cursor's distance from the base point in mm.
4. WHEN the user types a positive number or picks a point THE SYSTEM SHALL scale every
   selected entity about the base point by that factor (the picked point's distance from the
   base point), scaling positions and radii and keeping arc angles.
5. IF the factor is zero, negative or not finite THEN THE SYSTEM SHALL reject it with a
   command-line message and keep prompting.
6. IF the factor is 1 (within `EPSILON`) THEN THE SYSTEM SHALL commit nothing.
7. WHEN a scale is committed THE SYSTEM SHALL record it as one undo step, keep layers and
   selection, and return to SELECT.
8. WHEN the agent calls `scale_entity` with an index, a base point and a factor THE SYSTEM
   SHALL apply the same command, and the built-in system prompt SHALL describe the tool.

## Out of scope

- Non-uniform scale (circles would become ellipses).
- The Reference and Copy sub-options.

## Open questions
