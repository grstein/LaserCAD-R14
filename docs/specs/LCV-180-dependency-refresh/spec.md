# LCV-180 — Dependency refresh

- **Status**: Planned
- **Depends on**: none
- **Implementation**: -

## Problem

Runtime dependencies are pinned to 2024-era majors: `egui`/`eframe` 0.29 (0.36 current), `rfd`
0.14 (0.17), `directories` 5 (6), `roxmltree` 0.20 (0.21), `reqwest` 0.12 (0.13), `base64` 0.22
(0.23). Upstream fixes to text input, IME, HiDPI, Wayland and native dialogs do not reach
operators, advisories land on crates we no longer track, and agents writing UI code reach for
`egui` APIs that 0.29 lacks. An `egui` major can change painting, fonts, input and layout, so the
upgrade must be proven, not assumed. Windows/macOS packaging (LCV-201) builds on the result.

## Stories

- As an operator, I want the current GUI and dialog stack, so that platform fixes reach me.
- As a maintainer, I want dependencies close to upstream, so that advisories and fixes are cheap.

## Acceptance criteria

1. WHEN the refresh lands THE SYSTEM SHALL depend on the latest stable release, as of
   implementation start, of every direct dependency and dev-dependency in `Cargo.toml`.
2. WHEN any document is exported (every `tests/it/io_svg/` golden and corpus fixture) THE SYSTEM
   SHALL write bytes identical to those written before the refresh.
3. WHEN the paint-harness tests run (`tests/harness/paint.rs` users) THE SYSTEM SHALL pass with
   unchanged expected colours, positions and tolerances, except changes listed in `CHANGELOG.md`
   and reflected in `DESIGN.md`.
4. WHEN a test runs outside `crate::run()` THE SYSTEM SHALL keep native dialogs disarmed
   (ADR 0005 tests unchanged).
5. WHEN the app is built THE SYSTEM SHALL keep the `glow` renderer: `cargo tree -e normal` lists
   no `wgpu` crate.
6. WHEN the agent transport tests run THE SYSTEM SHALL keep the blocking client, timeouts, and
   error truncation unchanged, with no `tokio` in `Cargo.toml`.
7. WHEN `cargo tree --duplicates -e normal` runs THE SYSTEM SHALL list no more duplicated crates
   than before the refresh (the count is recorded in `plan.md`).
8. WHEN the stripped release binary is built THE SYSTEM SHALL be at most 15 % larger than the
   v0.5.0 binary.
9. WHEN `scripts/gate.sh` and `cargo deny check` run THE SYSTEM SHALL pass both.
10. WHEN the settings, autosave and recent-files paths resolve on Linux THE SYSTEM SHALL use the
    same directories as v0.5.0, so existing user files are still found (`directories` 6).

## Out of scope

- New features enabled by the newer crates; replacing a dependency with another crate.
- Rust toolchain or edition bump (stays 1.98, which meets the 1.95 MSRV of `egui` 0.36).
- Windows/macOS builds and smoke (LCV-201).

## Open questions

- None. Decided (self-approved per user goal, 2026-09-30):
  - `reqwest` moves to 0.13: its `blocking` feature still exists. Its default TLS becomes
    `rustls`, which is accepted and drops the OpenSSL system dependency.
    *Amended at /design (2026-10-01, self-approved per user goal):* rustls costs 1.2–3.4 MB and
    breaks AC 8, so `reqwest` 0.13 keeps `native-tls` (OpenSSL), as 0.12 does (`plan.md`).
  - The renderer stays `glow`: `eframe` 0.36 defaults to `wgpu`, so default features are turned
    off and the 0.29 feature set is listed explicitly. This avoids a GPU-driver risk and keeps
    the binary small.
  - Manual smoke before Done: Linux only, by the user, as for every release. Windows and macOS
    smoke belongs to LCV-201.
  - The refresh lands alone on `main` after v0.7 merges and before the SVG track rebases, one
    commit per crate family (`egui`/`eframe` first).
