# LCV-161 — Quadrant, perpendicular, tangent and nearest snaps

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

Object snap covers endpoint, midpoint, center and intersection only
(`geometry/snap/mod.rs::SnapKind`). Drawing a tangent line, a perpendicular or a point on a
circle's quadrant needs construction geometry that is later deleted.

## Stories

- As an operator, I want quadrant, perpendicular, tangent and nearest snaps so that I can draw
  precise geometry without helpers.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Snap tracking and polar tracking.

## Open questions

- Snap priority when several kinds hit; per-kind on/off settings or all on?
