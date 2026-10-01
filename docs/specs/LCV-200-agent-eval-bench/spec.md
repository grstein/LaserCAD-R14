# LCV-200 — Agent evaluation bench

- **Status**: Draft
- **Depends on**: LCV-193
- **Implementation**: -

## Problem

Harness changes (LCV-185..199) are judged by anecdote. Published CAD benchmarks score executed
geometry against a reference (voxel IoU and Chamfer distance in BenchCAD, raster and geometric
checks elsewhere); without a comparable, repeatable score LaserCAD cannot tell which change helps
or which model to recommend.

## Stories

- As a maintainer, I want to run a fixed suite of laser-cutting requests against a model and get
  a score per task, so that harness and model choices rest on data.

## Acceptance criteria

1. WHEN `scripts/agent-bench.sh <provider-config>` runs, THE SYSTEM SHALL run each task of a
   fixed suite (at least: plate with holes, box face with finger tabs, gear outline, text label)
   in a fresh document and write one result line per task.
2. WHEN a task finishes, THE SYSTEM SHALL score it by raster IoU against the reference drawing
   (reusing `src/render/raster.rs`) plus the task's geometric assertions, and record steps,
   tokens and wall time from the turn metrics (LCV-193).
3. WHEN `scripts/gate.sh` runs, THE SYSTEM SHALL run the suite against recorded model replies
   only, with no network access, proving the scoring is deterministic.
4. IF no provider config is given, THEN THE SYSTEM SHALL refuse to call any live model.

## Out of scope

- Publishing a leaderboard; running live models in CI.
- LLM-as-judge scoring.

## Open questions

- Where the suite lives (`tests/bench/` fixtures vs `docs/`), and how references are authored
  (hand-drawn SVGs imported through `src/io/svg/`).
