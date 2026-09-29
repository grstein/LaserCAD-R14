# LCV-155 — Plan

## Approach

Docs and file moves only; no production behavior changes. Fast lane goes in `AGENTS.md`.
`scripts/check.sh` wraps clippy + `cargo test --lib --tests -- <filter>`. `tests/it/` files are
`git mv`-ed into area folders with descriptive names (LCV id kept in the `//!` header).
Large inline test modules move to `<file>/tests.rs` (`mod.rs` → sibling `tests.rs`), precedent
`src/geometry/snap/tests.rs`. Every relative `include_str!` that a move touches becomes
`concat!(env!("CARGO_MANIFEST_DIR"), "/…")`.

## Touches

- `AGENTS.md` (§Workflow fast lane, §Commands), `.claude/agents/implementer-rust.md`
- `scripts/check.sh` (new), `.claude/rules/tests.md`, `.claude/rules/agent.md`,
  `docs/specs/_templates/tasks.md`
- `tests/it/**`: moves; `main.rs` declares area modules; meta-tests from lcv152
  (`ac1_every_file_in_tests_it_is_a_declared_module`) and lcv132 (`test_binaries`) walk recursively.
- `tests/harness/scan.rs`: one `is_test_file` helper; section-cutting scans (lcv121, 122, 125,
  129, 143, 145, 116 `f1_has_exactly_one_reader`) and in-src scans
  (`app/viewport.rs::every_repaint_request_in_src_is_conditional`, `ui/statusbar.rs::walk_src`)
  skip `tests.rs`.
- `src/**` files whose test module exceeds 300 lines: tests moved out. Never `src/agent/tests.rs`
  (lcv128 AC2 names every file directly in `src/agent/`).
- ADRs: ADR 0004 gets a short amendment note allowing sibling `tests.rs` for large test modules.

## Risks

- A scan silently losing coverage after moves: each scan keeps its `> N files` controls; recount.
- Scans that `expect` a column-0 `#[cfg(test)]` (lcv125, statusbar): keep `#[cfg(test)]\nmod tests;`.
- LOC cap: unaffected (`tests.rs` exempt). Mutation testing: no.
