# LCV-001 — Cargo project skeleton + dependency lock

- **Status**: Done
- **Phase**: 0
- **Depends on**: none
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: implementer-rust (task #3); fd6a31d

## Problem

LaserCAD v2 is a green-field pure-Rust rewrite. Every later demand (kernel, document model, render, tools, I/O, agent) assumes a working Cargo workspace whose dependency surface, edition, MSRV, and module tree are already fixed. Without a locked-in skeleton, contributors and subsequent demands cannot reason about what compiles, what re-exports exist, or which third-party crates may be added. This demand freezes the bottom of that stack so every later demand can start from a green `cargo build` without reshaping the project layout or arguing about deps.

## Scope

- A single binary+library Cargo package named `lasercad` at the repository root.
- Rust edition `2021` and `rust-version = "1.88"` declared in `Cargo.toml`.
- Exactly two direct runtime dependencies: `eframe = "0.29"` and `egui = "0.29"`. No dev-dependencies, no build-dependencies, no optional features enabled.
- A release profile tuned for binary size and predictability: `opt-level = "s"`, `lto = true`, `codegen-units = 1`, `strip = true`, `panic = "abort"`.
- A module tree under `src/` matching `AGENTS.md` / `PLAN.md` exactly: `main.rs`, `app.rs`, `lib.rs`, plus the module directories `geometry/`, `document/`, `render/`, `tools/`, `io/` (with `io/svg/` nested), `ui/`, `agent/`, `text/`, `util/`. Each module directory has a `mod.rs` with a doc-comment header describing the module's responsibility and (where applicable) the kernel-purity rule.
- `lib.rs` declares `pub mod` for every top-level module so external integration tests under `tests/` can import them.
- The crate compiles cleanly in debug and release; `cargo test --all` runs and reports zero failures (even with no tests yet); `cargo run` opens the bootstrap window declared in `src/lib.rs::run`.
- `Cargo.lock` is committed (this is a binary, not a library, so the lockfile is part of the contract).

## Out of scope

- `rustfmt.toml`, `clippy.toml`, and `rust-toolchain.toml` *content* — owned by **LCV-002**. (These files already exist on disk from scaffolding; LCV-001 does not modify them, and a future LCV-002 may rewrite them.)
- CI workflow (`.github/workflows/*`) — owned by **LCV-004**.
- README, LICENSE, CHANGELOG content — owned by **LCV-005** (already `Done`).
- Adding `rfd`, `reqwest`, `tokio`, `roxmltree`, `serde`, `serde_json`, `directories`, `tracing`, or any other dependency. Each future dep is introduced by the demand that needs it (e.g., `rfd` arrives with LCV-061, `reqwest`+`tokio` with LCV-077, `serde_json`+`directories` with LCV-058, `roxmltree` with LCV-057).
- Filling in any module body beyond the doc-comment header (`Vec2`, `Entity`, `Tool`, etc. belong to their phase-specific demands).
- Packaging (AppImage, .deb, icons, .desktop) — Phase 8.
- Multi-platform build matrix — Phase 9.

## Acceptance criteria

1. `cargo build` exits 0 from a clean checkout on Linux with stable Rust 1.88, producing `target/debug/lasercad`.
2. `cargo build --release` exits 0 and produces `target/release/lasercad`. The release profile in `Cargo.toml` matches exactly: `opt-level = "s"`, `lto = true`, `codegen-units = 1`, `strip = true`, `panic = "abort"`.
3. `cargo test --all` exits 0 with `test result: ok. 0 passed; 0 failed` (no test files yet is acceptable; the harness must still run cleanly).
4. `Cargo.toml` `[dependencies]` table lists exactly two entries — `eframe = "0.29"` and `egui = "0.29"` — and no `[dev-dependencies]`, no `[build-dependencies]`, no `[features]`. No other crate names appear under any dependency table.
5. `Cargo.toml` `[package]` declares `name = "lasercad"`, `edition = "2021"`, `rust-version = "1.88"`, and `license = "MIT OR Apache-2.0"`.
6. The following files exist under `src/` and are valid Rust modules: `main.rs`, `lib.rs`, `app.rs`, `geometry/mod.rs`, `document/mod.rs`, `render/mod.rs`, `tools/mod.rs`, `io/mod.rs`, `io/svg/mod.rs`, `ui/mod.rs`, `agent/mod.rs`, `text/mod.rs`, `util/mod.rs`.
7. `src/lib.rs` declares `pub mod` for each of: `agent`, `app`, `document`, `geometry`, `io`, `render`, `text`, `tools`, `ui`, `util`. (`app` is intentionally public so integration tests can construct the bootstrap state.)
8. The kernel `mod.rs` files (`geometry/mod.rs`, `document/mod.rs`, `io/svg/mod.rs`, `text/mod.rs`) each contain a doc-comment line stating that the module MUST NOT import `egui`, `eframe`, or `rfd`. `grep -nE '^use (egui|eframe|rfd)' src/geometry/*.rs src/document/*.rs src/io/svg/*.rs src/text/*.rs` finds zero matches.
9. `cargo run` opens a native window whose title contains the string `LaserCAD v2 — bootstrap`. The window closes on the OS close button without panicking. (Manual smoke step — see Expected tests.)
10. `Cargo.lock` is present at the repository root and tracked by git.

## Expected tests

- **Unit / build (criterion 1, 2, 3, 4, 5, 10)**: a manual or scripted check that runs `cargo build`, `cargo build --release`, and `cargo test --all` from a clean checkout and verifies all three exit with code 0. Verified via the project's standard commands documented in `AGENTS.md` § Commands.
- **Integration test (criterion 6, 7)**: add `tests/skeleton.rs` containing a single test that asserts the `lasercad` crate's module surface is importable, e.g.:
  ```rust
  #[test]
  fn module_tree_is_wired() {
      // Pure compile-time check: every top-level module re-exported by lib.rs
      // must be addressable. The test body can be empty; the `use` lines do the work.
      use lasercad::{agent, app, document, geometry, io, render, text, tools, ui, util};
      let _ = (&agent::TYPE_ID, &app::TYPE_ID, &document::TYPE_ID, &geometry::TYPE_ID,
               &io::TYPE_ID, &render::TYPE_ID, &text::TYPE_ID, &tools::TYPE_ID,
               &ui::TYPE_ID, &util::TYPE_ID);
  }
  ```
  Implementer may replace `TYPE_ID` with any always-present module path (e.g., a `pub const MODULE: &str = "geometry"` placeholder added to each `mod.rs`) — the test exists to guarantee the import graph is intact. The placeholder may be removed by the first demand that fills in the module's real surface.
- **Static check (criterion 4)**: `cargo metadata --format-version=1 --no-deps | jq '.packages[0].dependencies | map(.name) | sort'` returns exactly `["eframe", "egui"]`.
- **Static check (criterion 8)**: `grep -nE '^use (egui|eframe|rfd)' src/geometry/*.rs src/document/*.rs src/io/svg/*.rs src/text/*.rs` returns no matches and exits with status 1 (grep "not found").
- **Manual smoke (criterion 9)**: run `cargo run`; observe a window titled `LaserCAD v2 — bootstrap` opens, render is non-empty, and the window closes cleanly via the OS close button. Record the result in the demand's `Implementation:` line when the implementer completes the demand.

## Open questions

(none)

## Notes

- The repository was scaffolded in commit `cb563b2` ("chore: initial scaffold for LaserCAD v2 (pure Rust + egui)"), which already satisfies most of this demand. LCV-001 is therefore largely a **retroactive lock-in**: its real value is fixing the dependency surface and the module tree so later demands cannot drift.
- The MSRV `1.88` is dictated by transitive dependencies of `image` and `idna_adapter` (pulled via `eframe`). Documented in `AGENTS.md` § Commands.
- `eframe` and `egui` versions are deliberately pinned to the same `0.29` line. They must be upgraded together; the demand that upgrades them must update both versions in one commit.
- ADR [`docs/adr/0001-pure-rust-egui.md`](../adr/0001-pure-rust-egui.md) explains why `eframe` + `egui` are the only Phase-0 deps.
- Module-tree contract: see `AGENTS.md` § Architecture / Module tree. The 300-LOC-per-file rule is enforced at review time, not in this demand.
- Future demand LCV-002 will rewrite `rustfmt.toml`, `clippy.toml`, and `rust-toolchain.toml` content. LCV-001 only requires that the *crate* builds on stable 1.88; the *pin* itself is LCV-002's contract.
