# LCV-157 — COPY command

- **Status**: Draft
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

To be written by /specify.

## Out of scope

- Array (rectangular/polar) patterns.

## Open questions

- Multiple placements per command run (R14 `Multiple`) or one copy per run?
- Undo granularity: one step per placement or per command run?
