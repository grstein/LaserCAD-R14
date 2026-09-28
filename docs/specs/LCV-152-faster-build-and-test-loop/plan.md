# LCV-152 — Plan

## Approach

Measure first, then take the three cheap wins in order of expected gain:
1. One integration binary: move `tests/<name>.rs` to `tests/it/<name>.rs` as modules of
   `tests/it/main.rs`; `tests/harness/` becomes `tests/it/harness/` (or `#[path]`), keeping
   its 300-LOC cap. Fix `include_str!`/`CARGO_MANIFEST_DIR` relative paths.
2. Dev profile: `[profile.dev] debug = "line-tables-only"`, `[profile.dev.package."*"] opt-level = 1`.
3. `mold` via `.cargo/config.toml` `[target.x86_64-unknown-linux-gnu]` only when present — use
   a `RUSTFLAGS`-free approach that falls back cleanly (AC 4), e.g. document it in
   `docs/build-local.md` and enable per-developer in `~/.cargo/config.toml` if a repo-level
   conditional is not possible.

## Touches

- `tests/**` (moves only), `Cargo.toml` profile, `.cargo/config.toml`, `docs/build-local.md`,
  `scripts/loc-cap.sh` (harness path), `.claude/rules/tests.md` (paths), CI only if paths change.
- Scans that walk `tests/` by path (`tests/harness/scan.rs` users) may need their root updated.
- ADRs: none (ADR 0008 `--no-fail-fast` unchanged).

## Risks

- Test-name listing before/after is the AC 2 proof: `cargo test -- --list` diffed by name suffix.
- A module that relied on being its own crate (`#![...]` attributes, duplicate helper names) —
  fix by scoping into the module.
- Mutation testing: no.

## Baseline (fill in T1)

| Measure | Before | After |
|---|---|---|
| clean `cargo test --no-run` (after `cargo clean -p lasercad`) | 29.9 s | |
| incremental gate after touching `src/geometry/vec2.rs` | 38.6 s / 35.4 s | |
| test binaries run by `cargo test --all` | 47 (lib, main, 45 under `tests/`) | |
| test functions listed by `cargo test --all -- --list` | 1408 (2 `#[ignore]`) | |

Machine: 16 threads, Fedora 43, default linker (`mold` not installed). The name list is
`cargo test --all -- --list`, each name prefixed with its binary (`tests/<name>.rs` → `<name>`,
unit tests → `unittests`), sorted: sha256 `72fdf827…09298`.
