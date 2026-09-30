# LCV-157 — COPY command

- **Status**: Specified
- **Depends on**: none
- **Implementation**: -

## Problem

Repeating a part (holes, tabs, identical pieces on the bed) means redrawing it: the only
modify commands are MOVE, TRIM, EXTEND and ERASE. COPY is one of the most used AutoCAD R14
commands and is missing.

## Stories

- As an operator, I want to copy the selection from a base point to one or more target points,
  picked or typed, so that I can repeat geometry precisely.

## Acceptance criteria

1. WHEN the user types `copy`, `co` or `cp` THE SYSTEM SHALL activate COPY with the prompt
   `COPY Specify base point:`.
2. WHEN COPY is active with an empty selection and the user picks a point THE SYSTEM SHALL
   leave the drawing unchanged and stay at the base-point prompt.
3. WHEN a base point is fixed THE SYSTEM SHALL prompt `COPY Specify second point:`, take the
   base point as the anchor for `@dx,dy` input, and preview the selection at the cursor.
4. WHEN the user gives a second point THE SYSTEM SHALL add a translated copy of every selected
   entity, each on its source entity's layer, and leave the source entities unchanged.
5. WHEN a copy has been placed THE SYSTEM SHALL stay at the second-point prompt with the same
   base point and the same source set, so each further point places another copy (Multiple).
6. WHEN a copy is placed THE SYSTEM SHALL commit it as its own undo step, so one Ctrl+Z
   removes only the latest placement.
7. IF the second point equals the base point (within `EPSILON`) THEN THE SYSTEM SHALL place
   nothing and keep prompting.
8. WHEN the user presses Enter on a blank line or Escape at the second-point prompt THE SYSTEM
   SHALL end COPY; the placed copies stay.
9. WHEN the agent calls `copy_entity` with an index and `dx`, `dy` THE SYSTEM SHALL add a
   translated copy through the same command, and the built-in system prompt SHALL describe
   the tool.

## Out of scope

- Array (rectangular/polar) patterns.
- Picking objects inside the command (the selection is made first, as in MOVE).

## Open questions
