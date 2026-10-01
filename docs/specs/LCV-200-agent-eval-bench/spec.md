# LCV-200 — Agent evaluation bench

- **Status**: Specified
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

1. WHEN the suite is listed, THE SYSTEM SHALL hold exactly four tasks — plate with holes, box face
   with finger tabs, gear outline, text label — each a prompt, a reference SVG and geometric
   assertions, under `tests/fixtures/agent-bench/<task>/`.
2. WHEN a task runs, THE SYSTEM SHALL start from a fresh default document and drive one agent turn
   through the real loop and apply path (`drive_turn`, LCV-193 `answer_act`), with no window.
3. WHEN a task finishes, THE SYSTEM SHALL score it as the raster IoU of the drawing against the
   reference (both through `src/render/raster.rs`, same bed and resolution) and the pass/fail of
   each geometric assertion.
4. WHEN a task finishes, THE SYSTEM SHALL write one JSON line: task, IoU, assertions passed/total,
   and the LCV-193 counts (steps, applied, refused, repeated, captures, replies) plus wall time.
5. WHEN `scripts/gate.sh` runs, THE SYSTEM SHALL replay every task from its recorded model replies
   with no network, and assert each task's IoU and assertion results equal the recorded ones.
6. WHEN the recorded replies of a task contain a known defect (a missing hole), THE SYSTEM SHALL
   score it below the clean recording, proving the scorer discriminates.
7. WHEN `scripts/agent-bench.sh <settings.json>` runs, THE SYSTEM SHALL run the suite against the
   live model that settings file names and write the results to `target/agent-bench/<model>.jsonl`.
8. IF no settings file is given or it names no model, THEN THE SYSTEM SHALL refuse with a usage
   line and contact no network.

## Out of scope

- Publishing a leaderboard; running live models in CI; LLM-as-judge scoring.
- Token or cost accounting (LCV-193 keeps it out); Chamfer distance.

## Decisions (self-approved per user goal, 2026-09-30)

- Fixtures live in data-only `tests/fixtures/` (the LCV-170 allowance); if this lands first, its
  plan adds that allowance. References are hand-authored SVGs read through `src/io/svg/`.
- Recorded replies are JSON tool-call scripts; the live run also writes its replies so a
  maintainer can promote one to a fixture. Wall time is reported, never asserted.
- The live runner is an `#[ignore]` test in the one integration binary, started by the script —
  no second binary (LCV-152).

## Open questions

- None.
