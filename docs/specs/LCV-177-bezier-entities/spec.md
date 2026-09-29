# LCV-177 — Cubic and quadratic Bézier entities

- **Status**: Draft
- **Depends on**: LCV-172
- **Implementation**: -

## Problem

SVG `C`/`S`/`Q`/`T` path segments — the bulk of logos, traced art and text converted to paths —
have no representation: `io/svg/import.rs::parse_path` skips them silently, and the `AGENTS.md`
export contract says "never béziers". The user chose native Bézier entities on 2026-09-29
(`docs/research/svg-spec-coverage.md` §1).

## Stories

- As an operator, I want curved artwork from any SVG to open, display, select, snap to endpoints,
  move and export exactly, so that LaserGRBL receives the original curves.

## Acceptance criteria

To be written by /specify.

## Out of scope

- TRIM/EXTEND/intersections on Béziers; drawing Béziers by hand (SPLINE) unless /specify adds it.

## Open questions

- Store quadratics as quadratics or elevate to cubics (exact) internally?
- Export contract change: "Béziers only for Bézier entities; circular arcs stay `A`" — needs the
  user's explicit approval.
- Chained segments: one entity per segment, or a path entity?
