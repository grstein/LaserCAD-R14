# LCV-202 — Plan

## Approach

Release paperwork plus four guard tests; no product code changes. **T1 starts only after LCV-180
(branch `ui`) and then the `svg` branch (LCV-170..179, ADRs 0015–0017) have merged into main**;
LCV-202 is implemented last, on main.
- **AC 2** is ADR 0018 (this commit). T1 adds the one-line pointers to AGENTS.md.
- **AC 3** pins the frozen contract in bytes: `tests/it/io_svg/contract_1_0.rs` builds one
  document through `Document::from_parts` / `push_entity` with every kind that exists after the
  svg merge — line, Polyline output (lines), TEXT strokes (`text::layout_text`, lines), circle,
  arc, full ellipse rotated and unrotated, elliptical arc, quadratic, cubic — on three layers
  (one named `Engrave & mark` for escaping; one empty, Output off). It compares `export_svg` with
  `tests/fixtures/svg/contract-1.0.svg` and re-imports it within `FORMAT_TOL`. The fixture is
  written once by the test's own output and reviewed line by line against AGENTS.md §SVG export.
- **AC 4** fixtures come from v0.5.0 itself: `git worktree add --detach ~/.cache/lasercad-v05
  8c6701d`, `CARGO_TARGET_DIR=~/.cache/lasercad-v05-target`, a throwaway `#[test] #[ignore]` in
  that worktree's `src/io/mod.rs` tests calling `io::settings::save_to`,
  `io::autosave::save_autosave_to` and `io::export_svg`; then `git worktree remove`. Only the
  three data files land in `tests/fixtures/v0_5/` (data-only rule); the generator body is quoted
  in the `//!` header of the compat test. The loaders are `pub(crate)` and not pure, so the test
  is a test-only module `src/io/v0_5_compat.rs` (`#[cfg(test)] mod v0_5_compat;` in
  `src/io/mod.rs`); it copies each fixture to a tempdir first (`load_from` backs up a corrupt
  file beside it) and asserts every v0.5 field, layer, membership and entity survives.
- **AC 5** coverage test lives in `src/ui/toolbar.rs`'s test module (after `#[cfg(test)]`): it
  reads `docs/user-guide.md` via `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), …))` and
  asserts every `TOOLS` label appears (expected set derived from `TOOLS`). Aliases come from
  `cmdline/parse.rs` word rows, written by hand.
- **AC 1** rows: one per bullet of `docs/product/README.md` §Scope target (12). The "SVG export:
  cut/mark/engrave presets" row maps to layers + Export layers (ADR 0012 replaced presets).
  `tests/it/repo/release_1_0.rs` checks row count = bullet count, no empty cell, and that each
  cited test name exists as `fn <name>` under `src/` or `tests/`.
- **AC 6, AC 8** are text scans in the same `release_1_0.rs`, in the style of `packaging.rs`.
- **AC 7, AC 9** are release checks recorded in `spec.md`, not gate tests: LCV-201 T16 (the
  three-OS `workflow_dispatch`) needs the user. If it has not run, AC 7 lists LCV-201 as the one
  open dependency and AC 9 records Windows `.zip`/macOS `.dmg` as pending the user; LCV-202 is
  not blocked. Linux AppImage and `.deb` are built locally from the 1.0 tree
  (`scripts/build-appimage.sh`, `scripts/build-deb.sh`; `dist/` is ignored).
- **AC 8** is the last task: `1.0.0` in `Cargo.toml` (and `Cargo.lock`), `[1.0.0]` CHANGELOG
  section. No tag, no GitHub release.

## Touches

- `AGENTS.md` — §SVG export pointer + ADR list line for 0018
- `tests/it/io_svg/contract_1_0.rs`, `tests/it/io_svg/mod.rs`, `tests/fixtures/svg/contract-1.0.svg`
- `tests/fixtures/v0_5/{settings.json,autosave.json,mother.svg}`, `src/io/v0_5_compat.rs`,
  `src/io/mod.rs` (one `mod` line)
- `src/ui/toolbar.rs` (test module only), `docs/user-guide.md`
- `docs/product/parity-1-0.md`, `docs/release/smoke-1-0.md`, `README.md`
- `tests/it/repo/release_1_0.rs`, `tests/it/repo/mod.rs`
- `docs/specs/LCV-202-release-1-0/spec.md` (AC 7/9 record), `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`
- ADRs: new ADR 0018 (SVG export contract frozen at 1.0)

## Risks

- LOC cap: `src/ui/toolbar.rs` has 135 implementation lines; the new test is below
  `#[cfg(test)]`. `src/io/v0_5_compat.rs` is test-only and stays well under 300 lines.
- Mutation testing: no — `src/io/svg/export.rs` is not touched.
- Blessing a bug in the contract fixture: T3 reviews every line against the AGENTS.md bullets
  (mirror, sweep inversion, `rotate` only when non-zero, escaping, empty layer) before commit.
- A v0.5 fixture that fails to load is a real compat bug: fix it in a new task, never edit the
  fixture.
- Settings fixture: use a dummy API key (`fixture-not-a-key`) so no secret scanner trips.
- AC 7/9 depend on the user's CI dispatch (LCV-201 T16); recorded as pending, not blocking.
