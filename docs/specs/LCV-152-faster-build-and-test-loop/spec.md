# LCV-152 — Faster build and test loop

- **Status**: Done
- **Depends on**: none
- **Implementation**: 159ce84..5f0fb27, 952ebaf, 9be1506

## Problem

Every gate relinks 46 separate integration-test binaries against egui, so `scripts/gate.sh`
dominates each demand's wall time and makes every agent iteration slow.

## Stories

- As a developer (human or agent), I want the gate to finish much faster so that each task's
  feedback loop is short.

## Acceptance criteria

1. WHEN `cargo test --all --no-fail-fast` runs THE SYSTEM SHALL build at most two integration-test
   binaries (`tests/it/` plus any binary that must stay separate, each justified in `plan.md`).
2. WHEN the suite runs THE SYSTEM SHALL execute the same set of test functions as before the change
   (same count by name, none skipped or ignored that were not already).
3. WHEN a single `src/` file changes THE SYSTEM SHALL complete an incremental `scripts/gate.sh`
   measurably faster than the recorded baseline; before/after timings are recorded in `plan.md`.
4. IF `mold` is not installed THEN THE SYSTEM SHALL still build with the default linker.
5. WHEN CI runs THE SYSTEM SHALL pass with the same gates as before.

## Out of scope

- Changing test logic or assertions; cargo-nextest adoption; release profile changes.

## Open questions

- none
