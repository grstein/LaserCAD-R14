# LCV-158 — ROTATE command

- **Status**: In Progress
- **Depends on**: LCV-157
- **Implementation**: -

## Problem

Parts often need to be turned to fit the bed or the material, and the toolkit cannot do it.
ROTATE is the first of three R14 core edits split out of the old "ROTATE, MIRROR and SCALE"
Draft (user decision 2026-09-29); MIRROR is LCV-181, SCALE is LCV-182. All three share one
kernel transform layer that this spec introduces.

## Stories

- As an operator, I want to rotate the selection about a base point by a typed or picked angle.

## Acceptance criteria

1. WHEN the user types `rotate` or `ro` THE SYSTEM SHALL activate ROTATE with the prompt
   `ROTATE Specify base point:`.
2. WHEN ROTATE is active with an empty selection and the user picks a point THE SYSTEM SHALL
   leave the drawing unchanged.
3. WHEN a base point is fixed THE SYSTEM SHALL prompt `ROTATE Specify rotation angle:` and
   preview the selection rotated by the angle from the base point to the cursor.
4. WHEN the user types a number at the angle prompt THE SYSTEM SHALL rotate every selected
   entity about the base point by that many degrees, counter-clockwise positive.
5. WHEN the user picks a point at the angle prompt THE SYSTEM SHALL rotate by the angle of
   the vector from the base point to that point, measured CCW from +X.
6. WHEN a line, circle or arc is rotated THE SYSTEM SHALL map line endpoints and centers
   exactly as a rigid rotation, keep radii, and add the angle to arc start and end angles.
7. WHEN a rotation is committed THE SYSTEM SHALL record it as one undo step, keep each
   entity's layer and selection, and return to SELECT.
8. IF the angle is zero (mod 360°, within `EPSILON`) THEN THE SYSTEM SHALL commit nothing.
9. WHEN the agent calls `rotate_entity` with an index, a base point and degrees THE SYSTEM
   SHALL apply the same command, and the built-in system prompt SHALL describe the tool.

## Out of scope

- The Reference sub-option.
- Rotating a copy (R14 2000+ `Copy` sub-option).

## Open questions
