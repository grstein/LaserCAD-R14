# LCV-002 — rustfmt + clippy config + rust-toolchain pin

- **Status**: Done
- **Phase**: 0
- **Depends on**: LCV-001
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: b6464983 — feat(LCV-002): lock rustfmt + clippy + rust-toolchain configuration

## Problem

LaserCAD v2 is a multi-agent codebase: `implementer-rust` ships code, `reviewer-rust` reviews it, CI (LCV-004) enforces gates, and any future contributor needs an identical local toolchain. Without a checked-in formatting and linting policy, two implementers can produce equivalent-looking diffs that one another's editors keep reformatting, clippy warnings drift, and CI cannot reproduce a "green on my machine" result. This demand freezes the three policy files (`rustfmt.toml`, `clippy.toml`, `rust-toolchain.toml`) so every later demand starts from a single canonical formatting and linting baseline. It does not introduce new lint rules — only the configuration that lets the standard commands in `AGENTS.md` § Commands behave the same way for every dev and every CI run.

## Scope

- `rust-toolchain.toml` at the repository root pins channel `1.88`, profile `minimal`, and the `rustfmt` + `clippy` components. This file already exists from the LCV-001 scaffold; this demand only verifies the content matches the contract below and rewrites it if it does not.
- `rustfmt.toml` at the repository root declares exactly these stable keys: `edition = "2021"`, `max_width = 100`, `newline_style = "Unix"`. No nightly-only keys, no opinionated reorderers (`reorder_imports`, `group_imports`, etc.) — defaults are deliberately accepted everywhere else so the policy stays minimal and stable across rustfmt versions.
- `clippy.toml` at the repository root exists with a documentation-only header explaining that no project-specific clippy tunables are required; warning denial is delivered by `-D warnings` in the standard command and in CI. The file remains present (so it shows up in editor scans) but carries no key/value pairs.
- The current source tree (post LCV-001) passes `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings` against this configuration without any source-code edits.

## Out of scope

- CI workflow wiring (`.github/workflows/ci.yml`) — owned by **LCV-004**.
- Pre-commit hooks, husky, lefthook, or any local git-hook installer.
- IDE / editor settings (`.editorconfig`, `.vscode/`, `rust-analyzer.toml`).
- Adding new clippy lints beyond the default `clippy::all` group that `-D warnings` denies.
- Toolchain upgrades to a newer stable channel (will be a separate demand when needed).
- `rustfmt` nightly-only keys (e.g. `imports_granularity`, `group_imports`, `wrap_comments`). They tempt drift; reject by default.

## Acceptance criteria

1. `rust-toolchain.toml` at the repository root contains exactly the following keys under `[toolchain]`: `channel = "1.88"`, `components = ["rustfmt", "clippy"]`, `profile = "minimal"`. No other keys.
2. `rustfmt.toml` at the repository root contains exactly the following keys at the top level: `edition = "2021"`, `max_width = 100`, `newline_style = "Unix"`. No other keys.
3. `clippy.toml` at the repository root exists and contains only comment lines (lines starting with `#`) and/or blank lines — no key/value pairs. The header comment explains that project-wide warning denial is delivered by `-D warnings`, not by file-level config.
4. From a clean checkout on Linux with the toolchain pinned by `rust-toolchain.toml`, `cargo fmt --all -- --check` exits 0 against the current `src/` and `tests/` trees, with no source edits required to make it pass.
5. From the same checkout, `cargo clippy --all-targets -- -D warnings` exits 0, with no source edits required to make it pass.
6. The three files (`rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`) are tracked by git at the repository root (not nested inside `src/` or anywhere else).

## Expected tests

- **Static check (criterion 1)**: read `rust-toolchain.toml` and assert the three keys exist with the exact values above and no extras. Done as part of the implementer's pre-commit verification — no automated test is required because the file is a contract artifact, not behaviour.
- **Static check (criterion 2)**: read `rustfmt.toml` and assert it contains exactly the three keys above. Implementer confirms manually before committing.
- **Static check (criterion 3)**: `grep -vE '^\s*(#|$)' clippy.toml` returns no matches (i.e. every non-blank line is a comment). Run as part of the implementer's verification.
- **Build gate (criterion 4)**: run `cargo fmt --all -- --check` from the repo root; exit code must be 0. Documented as a step in `AGENTS.md` § Commands; the implementer runs it before declaring the demand done.
- **Build gate (criterion 5)**: run `cargo clippy --all-targets -- -D warnings`; exit code must be 0. Same enforcement path as criterion 4.
- **Manual smoke (criterion 6)**: `git ls-files | grep -E '^(rust-toolchain\.toml|rustfmt\.toml|clippy\.toml)$'` returns all three filenames.

## Open questions

(none)

## Notes

- The LCV-001 scaffold already shipped a `rust-toolchain.toml` with the correct content and a minimal `rustfmt.toml` containing only `edition = "2021"`. LCV-002 widens the rustfmt config to include `max_width` and `newline_style` (both stable since rustfmt 1.0) and locks the contract.
- `max_width = 100` is the most common Rust ecosystem default; the explicit declaration makes the choice survive a future rustfmt default change.
- `newline_style = "Unix"` keeps diffs and SVG output (also LF — see `AGENTS.md` § SVG export) consistent on Linux-first development.
- The 1.88 MSRV is driven by transitive deps of `eframe` 0.29 (see LCV-001 notes). Bumping the channel requires a separate demand.
- CI (LCV-004) will *consume* this configuration via `rustup show` and the standard `cargo fmt --check` + `cargo clippy` + `cargo test` commands. LCV-002 does not edit CI.
- `clippy.toml` is kept as a stub so future demands have a known location to add a knob if one is ever justified; the empty-of-keys state is the current contract.
