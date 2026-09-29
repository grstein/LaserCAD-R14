# LCV-180 — Dependency refresh

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

The runtime dependencies are pinned to 2024-era majors: `egui`/`eframe` 0.29, `rfd` 0.14,
`directories` 5, `roxmltree` 0.20, `reqwest` 0.12. Upstream fixes to text input, IME, HiDPI,
Wayland and native dialogs do not reach operators, security advisories land on crates we no longer
track, and agents writing UI code are drawn to newer `egui` APIs that do not exist in 0.29.
An `egui` major can change painting, fonts, input and layout, so the upgrade can change what the
operator sees and must be proven, not assumed.

## Stories

- As an operator, I want the current GUI and dialog stack, so that platform fixes reach me.
- As a maintainer, I want dependencies close to upstream, so that advisories and fixes are cheap to
  take.

## Direction

- Move every direct dependency to its latest release at `/specify` time, `egui`/`eframe` first;
  one commit per crate family.
- The SVG export contract (AGENTS.md §SVG export) and ADR 0005 (dialogs disarmed in tests) stay
  unchanged.
- Visual changes forced by `egui` are either restored to `DESIGN.md` or listed in `CHANGELOG.md`.

## Acceptance criteria

To be written by /specify. Candidates:

- Exported SVG for every fixture in `tests/it/io_svg/` is byte-identical before and after.
- The paint harness (`tests/harness/paint.rs`) passes without loosened tolerances.
- Native dialogs remain disarmed outside `crate::run()`.
- `cargo tree --duplicates` shows no more duplicate majors than before.

## Out of scope

- New features enabled by the newer crates.
- Replacing any dependency with a different crate.
- The Rust toolchain and edition bump (fast-lane chore).

## Open questions

- Is a manual smoke of the release binary on Linux, Windows and macOS required before Done?
- Should `reqwest` move to its next major within this spec, or stay on 0.12 if that major drops
  the `blocking` client used by `agent/transport.rs`?
