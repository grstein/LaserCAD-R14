# LCV-176 — Ellipse and elliptical-arc entities

- **Status**: Draft
- **Depends on**: LCV-172
- **Implementation**: -

## Problem

The document holds `Entity::{Line, Circle, Arc}` only (`document/entity.rs`). SVG `<ellipse>`,
elliptical `A` segments (rx ≠ ry or rotation), rounded rectangles with unequal radii and circles
under non-uniform scale cannot be represented, so import must refuse or report them. The user
chose native entities on 2026-09-29 (`docs/research/svg-spec-coverage.md` §1).

## Stories

- As an operator, I want ellipses and elliptical arcs to open, display, select, snap, move and
  export exactly, so that a round trip through LaserCAD changes nothing.

## Acceptance criteria

To be written by /specify.

## Out of scope

- TRIM/EXTEND/intersections on ellipses (follow-up after LCV-160).
- An ELLIPSE drawing command, unless /specify adds it.

## Open questions

- Export form: `<ellipse>` for full ellipses and `A rx ry φ` for arcs. This changes the SVG
  export contract in `AGENTS.md` and needs the user's explicit approval.
- Agent tools: expose the new kind in `query_entities` / `create_drawing` now or later?
