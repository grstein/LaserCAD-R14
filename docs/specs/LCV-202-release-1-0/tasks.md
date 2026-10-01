# LCV-202 — Tasks

Start T1 only after LCV-180 and then the `svg` branch (LCV-170..179) have merged into main.

- [x] T1 [AC2] Point AGENTS.md at ADR 0018: §SVG export says "frozen at 1.0 (ADR 0018); a change
      needs a major version"; the ADR list gains a 0018 line (files: AGENTS.md)
- [x] T2 [AC3] Test: `contract_1_0.rs` exports the every-kind, three-layer contract document and
      compares bytes with the fixture, then re-imports it within `FORMAT_TOL` (fails: no fixture)
      (files: tests/it/io_svg/contract_1_0.rs, tests/it/io_svg/mod.rs)
- [x] T3 [AC3] Write `contract-1.0.svg` from the T2 export; review each line against AGENTS.md
      §SVG export before committing (files: tests/fixtures/svg/contract-1.0.svg)
- [ ] T4 [P] [AC4] Generate the v0.5.0 settings, autosave and mother SVG in a scratch worktree of
      8c6701d under ~/.cache with its own `CARGO_TARGET_DIR` (throwaway ignored test, no GUI);
      remove the worktree (files: tests/fixtures/v0_5/settings.json,
      tests/fixtures/v0_5/autosave.json, tests/fixtures/v0_5/mother.svg)
- [ ] T5 [AC4] Test: load the three v0.5 fixtures from tempdir copies and assert every field,
      layer, membership and entity survives; quote the generator in the `//!` header
      (files: src/io/v0_5_compat.rs, src/io/mod.rs)
- [ ] T6 [P] [AC5] Test: every `TOOLS` label appears in `docs/user-guide.md` (fails: no guide)
      (files: src/ui/toolbar.rs)
- [ ] T7 [AC5] Write the user guide: every TOOLS entry, every command-line alias, layers,
      Export layers, the AI panel; link `docs/install.md` (files: docs/user-guide.md)
- [ ] T8 [AC1] Test: parity table has one row per §Scope target bullet, no empty cell, and every
      cited test exists (files: tests/it/repo/release_1_0.rs, tests/it/repo/mod.rs)
- [ ] T9 [AC1] Write the parity table (files: docs/product/parity-1-0.md)
- [ ] T10 [AC6] Test, then write the numbered smoke checklist for Linux, Windows and macOS (draw,
      edit, snap, layers, save, reopen, export layers, open in LaserGRBL)
      (files: tests/it/repo/release_1_0.rs, docs/release/smoke-1-0.md)
- [ ] T11 [AC8] Test, then README: 1.0 is stable, links install and user guides, lists the
      non-goals (DXF, G-code, fillet/chamfer/offset, blocks) (files: tests/it/repo/release_1_0.rs,
      README.md)
- [ ] T12 [AC7] [AC9] Run `scripts/backlog.sh --check` and confirm every Depends-on spec is Done;
      record in `spec.md` any still open (LCV-201 T16) and the Windows/macOS artifacts as pending
      the user's CI dispatch (files: docs/specs/LCV-202-release-1-0/spec.md)
- [ ] T13 [AC8] [AC9] Test version `1.0.0` and a `[1.0.0]` CHANGELOG section; bump `Cargo.toml`
      (`Cargo.lock` follows), move Unreleased into `[1.0.0]`; build the AppImage and `.deb` from
      this tree; `scripts/gate.sh` green. No tag (files: tests/it/repo/release_1_0.rs, Cargo.toml,
      CHANGELOG.md)
