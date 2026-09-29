# LCV-159 — Polar input and DIST query

- **Status**: Draft
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

To be written by /specify.

## Out of scope

- LIST / AREA queries.

## Open questions

- Absolute polar `d<a` too? Angle convention: degrees, CCW from +X, as in R14.
