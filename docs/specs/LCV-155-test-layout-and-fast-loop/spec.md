# LCV-155 — Test layout by area and a faster inner loop

- **Status**: In Progress
- **Depends on**: none
- **Implementation**: -

## Problem

The build is fast (LCV-152), but finding and running the right tests is not: `tests/it/` holds
55 flat files named by demand number, the largest `src` files are mostly inline tests (e.g.
`src/app/agent_worker.rs`, 1580 lines for 199 of implementation), the only command is the full
gate, and even a pure refactor needs a spec folder.

## Stories

- As a developer (human or agent), I want tests grouped by area, small source files, a quick
  filtered check and a lightweight path for non-behavioral changes, so each iteration is short.

## Acceptance criteria

1. WHEN a contributor reads `AGENTS.md` §Workflow THE SYSTEM SHALL describe a fast lane: changes
   with no user-visible behavior change skip the spec folder but still pass `scripts/gate.sh`.
2. WHEN `scripts/check.sh <filter>` runs THE SYSTEM SHALL run clippy with `-D warnings` and only the
   library and integration tests matching the filter, without doctests.
3. WHEN the integration tests are listed THE SYSTEM SHALL group them in area modules under
   `tests/it/<area>/` (agent, app, cmdline, ui, io_svg, document, geometry, repo), with no
   `lcvNNN` file names, and the meta-tests that guard `tests/it/` SHALL walk it recursively.
4. WHEN a file's inline test module exceeds 300 lines THE SYSTEM SHALL keep it in a sibling
   `tests.rs`, and source scans SHALL not treat a `tests.rs` file as implementation.
5. WHEN the suite runs after the change THE SYSTEM SHALL list the same number of tests as before
   (1552 by `cargo test --all -- --list`, the same 2 ignored) and `scripts/gate.sh` SHALL be green.

## Out of scope

- Changing test logic or assertions; cargo-nextest; CI changes.

## Open questions
