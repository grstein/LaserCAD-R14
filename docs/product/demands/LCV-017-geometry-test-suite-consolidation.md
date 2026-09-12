# LCV-017 — Geometry test suite consolidation (cross-module integration)

- **Status**: Done
- **Phase**: 1
- **Depends on**: LCV-016
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: bdb4830 — feat(LCV-017): consolidate geometry kernel integration tests

## Problem

LCV-010 through LCV-016 each ship their own `#[cfg(test)] mod tests` block with unit coverage for the type they introduce. Those unit tests prove each type works in isolation, but they cannot catch regressions where a *later* change to one kernel module silently breaks a *consumer* in another module — for example, a refactor of `Arc::bbox` that no longer accounts for a cardinal extreme would leave `Arc`'s own unit tests green but would break `Rect::contains_arc` and `snap` results.

This demand adds a single small integration-test surface that exercises the kernel as a *whole* — short cross-module scenarios that walk from one type through another. The intent is to catch the "module A's change broke module B's assumption" class of bug at `cargo test --all` time, not at production-bug-report time.

User outcome: when a later demand touches any kernel file, `cargo test --all` either stays green or fails in a way that points at the contract that broke, not at "snap mysteriously returned the wrong thing".

## Scope

- New integration-test file `tests/geometry.rs` (single file). Contents: cross-module scenarios only — **no duplication of the per-module unit tests** that LCV-010..016 already require. Scenarios walk through at least two kernel modules per test.
- Mandatory scenarios:
  1. **Line + Circle + intersect + snap**: construct a `Line` and a `Circle` that intersect at two known points; call `intersect::line_circle` to get the two intersection points; build a `SnapEntity` slice with the line and circle; verify `snap(world, tol, &entities)` at a point near one of the intersections returns `SnapKind::Intersection` with that point and the expected `primary_idx` / `secondary_idx`.
  2. **Rect + Arc + Arc::bbox feeding Rect::contains_arc**: construct a `Rect`; construct an `Arc` whose bbox lies fully inside the rect; assert `rect.contains_arc(&arc)` is `true`. Translate the arc so part of it pokes outside; assert `rect.contains_arc(&arc)` is `false` and `rect.crosses_arc(&arc)` is `true`.
  3. **Line + Line midpoint + snap**: construct two lines whose midpoints differ; cursor near the midpoint of one of them returns `SnapKind::Midpoint` with the correct `primary_idx`.
  4. **Circle + Circle + intersect + snap intersection**: construct two intersecting circles; verify both intersection points appear from `intersect::circle_circle`; verify snap at one of them returns `SnapKind::Intersection` with `secondary_idx == Some(other_idx)`.
  5. **Vec2 + Line + Rect**: a "polyline-style" walk — build three `Line`s sharing endpoints (forming an open path); compute the bbox of each via `Line::bbox`; build a `Rect::new` from the overall min/max; assert each line satisfies `rect.contains_line(&line)`.
- Total LOC for `tests/geometry.rs`: target ≤200 lines including imports, blank lines, and `#[test]` attributes. The file is a smoke-test surface, not a comprehensive replacement of the unit tests.
- Confirm the whole-suite green gate: after the consolidation, `cargo test --all` exits 0. The implementer reports the final test counts (total tests, passing) in the demand's `Implementation:` line.

## Out of scope

- **Property-based testing** (e.g., `proptest`, `quickcheck`). Would require a new dev-dependency; not justified for Phase 1.
- **Fuzzing** (e.g., `cargo-fuzz`). Same reason.
- **Performance benchmarks** (e.g., `criterion`). Same reason; performance is not a Phase-1 concern.
- **Duplicating per-module unit tests in the integration file**. The integration tests must walk multiple modules per test. Single-module assertions belong in the originating module's `#[cfg(test)] mod tests` block.
- **Adding tests for surfaces that do not yet exist** (e.g., arc-line intersection — LCV-014 explicitly excludes it; LCV-017 must not test it).
- **Refactoring or relocating the existing unit tests** in `src/geometry/*.rs`. They stay where they are.
- **Adding tests for `document::Entity`** or other non-`geometry` modules. This demand is geometry-only.
- **Splitting `tests/geometry.rs` into topic-split files** (e.g., `tests/geometry_intersect.rs`, `tests/geometry_snap.rs`). One file is sufficient at this scope; the 300-LOC per-file cap is not threatened by the ≤200-LOC target.

## Acceptance criteria

1. `tests/geometry.rs` exists, is committed, and contains exactly the five `#[test]` functions described in Scope (one per mandatory scenario). The test names are descriptive (e.g., `line_circle_intersect_then_snap`, `rect_contains_then_crosses_arc_after_translation`, `snap_midpoint_distinguishes_two_lines`, `two_circles_intersect_then_snap`, `polyline_walk_bbox_contains_each_segment`).
2. Each test walks **at least two** kernel modules. No test contains only single-module assertions.
3. `tests/geometry.rs` total LOC is `≤ 200` lines (verified via `wc -l tests/geometry.rs`).
4. `tests/geometry.rs` does not duplicate any assertion from the per-module `#[cfg(test)] mod tests` blocks in `src/geometry/*.rs`. Reviewer spot-checks this by grepping for identical literal coordinate tuples; minor coincidence is acceptable, but no test in `geometry.rs` should be a copy-paste of a unit test.
5. `cargo test --all` exits 0. The output mentions the new tests under `tests/geometry.rs` (or however cargo names the binary — `geometry::tests::<name>` is fine; the marker is that the count of `running N tests` for the `geometry` integration test equals exactly 5).
6. `cargo test --test geometry` exits 0 and reports `5 passed; 0 failed`.
7. `tests/geometry.rs` imports nothing from `egui`, `eframe`, or `rfd`. (Integration tests under `tests/` link against the `lasercad` library, so kernel-purity must hold here too.)
8. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **The five integration tests are the tests** for this demand. Each acceptance criterion above is verified by running them and inspecting the suite output:
  - **AC 1, 2**: `cargo test --test geometry -- --list` enumerates exactly 5 tests; reviewer spot-checks each test body to confirm it uses at least two kernel modules.
  - **AC 3**: `wc -l tests/geometry.rs` reports a count `≤ 200`.
  - **AC 4**: a manual reviewer scan — no automation required; reviewer comments on the PR if a duplicate is found.
  - **AC 5, 6**: `cargo test --all` and `cargo test --test geometry` both exit 0 with the expected counts.
  - **AC 7**: `grep -nE '^use (egui|eframe|rfd)' tests/geometry.rs` returns no matches.
  - **AC 8**: the standard build gate.

## Open questions

(none)

## Notes

- The "five scenarios" list is the minimum coverage bar. The implementer may add a sixth or seventh test if a different cross-module path becomes obvious during implementation, as long as the total LOC budget holds and each new test walks ≥2 modules. Each addition should be a *cross-module* scenario, not a unit-test relocation.
- `tests/skeleton.rs` from LCV-001 (updated by LCV-010 to reference `geometry::EPSILON`) continues to exist alongside `tests/geometry.rs`. The two files have different jobs: `skeleton.rs` proves the module surface is wired; `geometry.rs` proves the geometry kernel composes correctly. Do not merge them.
- The 200-LOC target leaves headroom for inline comments explaining each scenario. Tests should read like short narratives ("draw a line, intersect with a circle, snap to one of the intersections").
- Once LCV-020 (`document::Entity`) lands and the transitional `SnapEntity` is retired, the snap-related tests in `tests/geometry.rs` will need a one-line type swap. Flag this in the demand's Notes so the LCV-020 follow-up demand picks it up.
- Reviewer (`reviewer-rust`) is expected to verify (a) no duplication, (b) each test walks ≥2 modules, (c) the LOC cap holds.
