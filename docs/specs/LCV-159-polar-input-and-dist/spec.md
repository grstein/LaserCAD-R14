# LCV-159 — Polar input and DIST query

- **Status**: Planned
- **Depends on**: none
- **Implementation**: -

## Problem

The command line takes `X,Y`, `@X,Y` and bare distances, but not the R14 polar form
`@distance<angle`, so an angled segment of known length needs trigonometry by hand. There is
also no way to measure the drawing (DIST).

## Stories

- As an operator, I want to type `@50<30` to place a point 50 mm away at 30° from the last point.
- As an operator, I want DIST to report the distance, ΔX, ΔY and angle between two picked points.

## Acceptance criteria

1. WHEN the user types `@d<a` while a tool has an anchor THE SYSTEM SHALL give that tool the
   point `anchor + d·(cos a, sin a)`, with `a` in degrees, counter-clockwise from +X.
2. WHEN the user types `d<a` without `@` THE SYSTEM SHALL give the tool the absolute point
   `d·(cos a, sin a)` measured from the origin (0,0).
3. WHEN the distance or the angle is negative or has a decimal part THE SYSTEM SHALL accept
   it (`@10<-45` and `@-10<45` both parse), and spaces around `<` are allowed.
4. IF `@d<a` is typed while the active tool has no anchor THEN THE SYSTEM SHALL show the same
   message it shows today for `@dx,dy` without an anchor.
5. IF the polar text is malformed (`@<30`, `@10<`, `10<<5`) THEN THE SYSTEM SHALL treat it as
   an unknown command, as the grammar does today.
6. WHEN the user types `dist` or `di` THE SYSTEM SHALL activate DIST with the prompt
   `DIST Specify first point:`, then `DIST Specify second point:`, with the first point
   as the anchor.
7. WHEN the second point is given THE SYSTEM SHALL print on the command line
   `Distance = <d>, Angle = <a>°, Delta X = <dx>, Delta Y = <dy>` (mm, 3 decimals, angle
   CCW from +X) and return to SELECT without changing the drawing or the undo history.

## Out of scope

- LIST / AREA queries.
- Polar tracking and direct distance entry by cursor direction.

## Open questions
