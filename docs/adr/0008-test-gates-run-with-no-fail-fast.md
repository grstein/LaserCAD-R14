# ADR 0008 — Every `cargo test` gate runs with `--no-fail-fast`

- **Status**: Accepted
- **Date**: 2026-09-13
- **Deciders**: architect (raised by `reviewer-rust` during the LCV-121 review)

## Context

`cargo test --all` exits after the **first test target that fails**, not after
the last. The reviewer reproduced it on this repo: with a mutation that breaks
one unit test, the run stops at `unittests src/lib.rs` and the 23 integration
binaries under `tests/` never execute at all. `cargo test --help` states the
behaviour plainly — `--no-fail-fast: Run all tests regardless of failure`.

That default is wrong *for this project specifically*, because of where our
invariants live. Several architectural rules in `AGENTS.md` are enforced only by
integration-level source scans: `tests/lcv121_source_scans.rs` (the `src/agent/`
containment rules), `tests/lcv120_idle_repaint.rs` (the repaint policy), and
others. They are not unit tests and they do not run in the library target.

So a commit that breaks a unit test *and* an invariant at the same time reports
only the unit test. Whoever fixes the unit test gets a green second run — and
that run is the first time the invariant was ever checked. If the fix also
happened to restore the invariant, fine; if it did not, the invariant broke and
no gate ever said so. The failure mode is silent, and it is the same shape as
the Windows path-separator bug that has already bitten CI twice: a gate that
looks green because it did not run.

The counter-argument is cost. `--no-fail-fast` makes a **broken** build slower
and noisier, and the test job is a three-OS matrix. Weighed below.

## Decision

**Every `cargo test` gate in this repository runs `--no-fail-fast`.** All four
sites, kept in sync:

- `.github/workflows/ci.yml`, Gate 3 — `run: cargo test --all --no-fail-fast`.
- `AGENTS.md` §Commands and §Implementation Rules, the "before declaring a
  demand done" bullet.
- `.claude/agents/implementer-rust.md` §Test discipline — the run that lets an
  implementer declare done.
- `.claude/agents/reviewer-rust.md` step 5 — the independent re-run that decides
  the verdict.

The last two matter as much as the first: they are the humans-in-a-loop who
actually type the command, and a rule the runners do not carry is decoration.

It is a gate rule, not a preference: a demand is not done on the strength of a
`cargo test --all` run, and a reviewer who sees one in a hand-off may ask for
the full run. Ad-hoc runs while iterating (`cargo test snap_endpoint`) are
exempt — the rule binds the gate, not the inner loop.

## Consequences

- **Free on a green run.** Every target executes either way; the flag only
  changes what happens after a failure. The matrix cost is zero on the runs that
  matter most (merges to `main`).
- **A red run costs more and says more.** We pay the remaining targets' runtime
  exactly when the build is already broken — which is precisely when we want the
  whole picture rather than the first line of it. One round trip through CI
  beats three.
- **This repo already made this call one level up.** `ci.yml`'s test job sets
  `strategy.fail-fast: false`, so a Windows failure does not cancel Linux and
  macOS. Cancelling the *targets within* a leg while refusing to cancel the
  *legs* was an inconsistency, not a policy. Now it is consistent.
- **Noise is a reporting problem, not a gate problem.** A run with fifty
  failures is harder to read than a run with one, but the answer to that is to
  read the first failure first, not to hide the other forty-nine.
- Exit status is unaffected: `cargo` still exits non-zero if anything failed, so
  no CI logic changes.
- We are committed to keeping the two sites in sync. A future third gate (a
  release workflow, a pre-commit hook) inherits the flag.

## Alternatives considered

- **Leave `cargo test --all` and rely on reviewers.** This is the status quo,
  and the status quo is what produced the finding. A gate that needs a human to
  notice it did not run is not a gate.
- **Split CI into two steps: `--lib` then `--test '*'`.** Gets the integration
  targets to run, costs an extra cargo invocation and a second place to keep in
  sync, and still stops at the first failing integration binary. Strictly worse
  than one flag.
- **`--no-fail-fast` in CI only, plain `cargo test --all` locally.** The local
  gate is the one that decides whether a demand is declared done, so it is the
  gate that most needs to be honest. Two different commands for the same gate is
  also how the two drift.
- **Move the source scans into `src/` as unit tests so the library target covers
  them.** Tempting — but the scans that read the whole `src/` tree belong
  outside it, and `cargo test` would still stop at the first failing target,
  which would now be the one target that matters. Solves nothing, moves code.
