# LCV-181 — MIRROR command

- **Status**: Done
- **Depends on**: LCV-158
- **Implementation**: 35d6efb..a53391b

## Problem

Symmetric parts are drawn twice by hand. MIRROR flips the selection across a line,
completing a symmetric part from one half. It reuses the transform layer from LCV-158.

## Stories

- As an operator, I want to mirror the selection across a picked line, keeping or deleting the
  source.

## Acceptance criteria

1. WHEN the user types `mirror` or `mi` THE SYSTEM SHALL activate MIRROR with the prompt
   `MIRROR Specify first point of mirror line:`.
2. WHEN MIRROR is active with an empty selection and the user picks a point THE SYSTEM SHALL
   leave the drawing unchanged.
3. WHEN the first point is fixed THE SYSTEM SHALL prompt
   `MIRROR Specify second point of mirror line:` and preview the mirrored selection.
4. IF the second point equals the first (within `EPSILON`) THEN THE SYSTEM SHALL ignore it
   and keep prompting.
5. WHEN the second point is fixed THE SYSTEM SHALL prompt `MIRROR Erase source objects? [Yes/No] <N>:`.
6. WHEN the user answers `n`, `no` or a blank Enter THE SYSTEM SHALL add the mirrored entities
   on their source layers and keep the sources.
7. WHEN the user answers `y` or `yes` THE SYSTEM SHALL replace the sources with the mirrored
   entities.
8. WHEN an arc is mirrored THE SYSTEM SHALL produce an arc whose points are the mirror images
   of the source arc's points (the sweep direction reverses), so the export stays a single
   `A` path command.
9. WHEN a mirror is committed THE SYSTEM SHALL record it as one undo step and return to SELECT.
10. WHEN the agent calls `mirror_entity` with an index, two line points and `erase_source`
    THE SYSTEM SHALL apply the same command, and the built-in system prompt SHALL describe it.

## Out of scope

- The MIRRTEXT variable (text is lines after TEXT, so it mirrors like any geometry).

## Open questions
