# LCV-149 - Transport stall test is timing-flaky

- **Status**: Draft
- **Phase**: 12
- **Depends on**: none
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: -

## Problem

`src/agent/transport.rs::tests::ac3_both_halves_of_a_call_give_up_when_the_endpoint_stalls`
failed intermittently during the 2026-09-27 drive — about 1 in 5 full-suite
runs in one reviewer's measurement — but passes when run standalone. The
panic site was near the test's timing-budget assertion. A flaky test in the
authoritative local gate (CI is on billing hold) erodes gate integrity: a red
run cannot be trusted to mean the code is broken, and a green run cannot be
trusted to mean it is not.

## Scope

<product-owner to fill in: pin down the exact timing budget/assertion at
fault, whether it is a fixed sleep/timeout race, contention from running
alongside the rest of the suite, or something else, and the fix.>

## Out of scope

<product-owner to fill in>

## Acceptance criteria

<product-owner to fill in>

## Expected tests

<product-owner to fill in>

## Open questions

- Confirm the failure is reproducible under the same conditions (full-suite,
  `--no-fail-fast`) that surfaced it, and capture the actual panic message /
  timing values from a failing run.
- Determine whether the flake is specific to this test or a symptom of a
  broader timing-budget pattern in `src/agent/transport.rs`'s test suite.

## Notes

Opened from the 2026-09-27 drive. Reviewer-reported flake rate: roughly 1 in
5 full-suite runs; standalone runs (`cargo test transport::tests::ac3_both_halves...`)
were reported green every time, consistent with a timing assumption that only
breaks under full-suite contention.
