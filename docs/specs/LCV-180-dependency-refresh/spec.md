# LCV-180 — Dependency refresh

- **Status**: Specified
- **Depends on**: none
- **Implementation**: -

## Problem

The runtime dependencies are pinned to 2024-era majors: `egui`/`eframe` 0.29, `rfd` 0.14,
`directories` 5, `roxmltree` 0.20, `reqwest` 0.12, `base64` 0.22 (crates.io on 2026-09-30:
0.36, 0.17, 6, 0.21, 0.13, 0.23). Upstream fixes to text input, IME, HiDPI, Wayland and native
dialogs do not reach operators, advisories land on crates we no longer track, and agents writing
UI code reach for newer `egui` APIs that do not exist in 0.29. An `egui` major can change
painting, fonts, input and layout, so the upgrade must be proven, not assumed.

## Stories

- As an operator, I want the current GUI and dialog stack, so that platform fixes reach me.
- As a maintainer, I want dependencies close to upstream, so that advisories and fixes are cheap to
  take.

## Acceptance criteria

1. WHEN the refresh lands, THE SYSTEM SHALL pin every direct dependency in `Cargo.toml` to its
   latest crates.io release at `/implement` time, each exception listed with its reason in
   `plan.md`, one commit per crate family, `egui`/`eframe` first.
2. IF a latest release needs a newer Rust than `rust-toolchain.toml` or drops a feature the code
   uses (e.g. `reqwest`'s `blocking`), THEN THE SYSTEM SHALL keep the newest release that fits
   and record the reason.
3. WHEN `scripts/gate.sh` runs after each commit, THE SYSTEM SHALL pass with no new
   `#[allow]`/`#[expect]` added only to silence deprecations.
4. WHEN the documents of `docs/examples/` and `tests/it/io_svg/` are exported, THE SYSTEM SHALL
   write bytes identical to golden files captured before the first bump.
5. WHEN the paint-harness tests (`tests/harness/paint.rs`) run, THE SYSTEM SHALL pass with no
   tolerance loosened and no assertion removed.
6. WHILE outside `crate::run()`, THE SYSTEM SHALL keep native dialogs disarmed (ADR 0005 tests
   unchanged).
7. WHEN `App::new()` resolves user paths on Linux, THE SYSTEM SHALL resolve the same settings,
   autosave and recent-file locations as before the `directories` bump.
8. WHEN the agent transport tests run against the mock server, THE SYSTEM SHALL send the same
   request body and headers and honor the same timeout with a blocking client.
9. WHEN `cargo tree --duplicates` runs, THE SYSTEM SHALL list no more duplicated crates than the
   baseline recorded in `plan.md`.
10. WHEN the Linux release binary is smoke-tested (draw, snap, select, save, open, export layers,
    agent panel), THE SYSTEM SHALL match `DESIGN.md`, or each visual difference is either restored
    or listed in `CHANGELOG.md`.

## Out of scope

- New features enabled by the newer crates; replacing any crate with a different one.
- The Rust toolchain and edition bump (fast-lane chore); Windows/macOS smoke (v0.7 packaging).
- Any change to the SVG export contract or ADR 0005.

## Open questions

- None. Decided (self-approved per user goal): manual smoke on Linux only (Windows/macOS come with
  v0.7 packaging); `reqwest` moves to its latest major if it keeps `blocking`, else stays on 0.12
  with the reason recorded; golden export files are captured in the first commit.
