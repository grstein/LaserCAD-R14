# LCV-202 — Release 1.0

- **Status**: Done
- **Depends on**: LCV-180, LCV-201, LCV-170, LCV-171, LCV-172, LCV-173, LCV-174, LCV-175, LCV-176, LCV-177, LCV-178, LCV-179, LCV-194, LCV-195, LCV-196, LCV-197, LCV-198, LCV-199, LCV-200
- **Implementation**: d7b46ef..601f456

## Problem

A 1.0 version is a promise: the operator can drop the original TypeScript app, files saved today will still
open and export the same way tomorrow, and LaserCAD runs on the OS next to their LaserGRBL. None
of that is written down or checked today. Parity with the original app was the v0.1 scope target
(`docs/product/README.md`), but no table maps each original capability to the command and test that
prove it. The SVG export contract has grown (layers, ellipses, Béziers) without a freeze point,
and there is no user guide or release smoke checklist.

## Stories

- As an operator, I want to know 1.0 does everything the original app did, so that I can uninstall it.
- As an operator, I want my 1.0 files to stay valid, so that a later update never breaks a job.

## Acceptance criteria

1. WHEN 1.0 is prepared THE SYSTEM SHALL ship `docs/product/parity-1-0.md`: one row per original
   capability from `docs/product/README.md` §Scope target, each naming the command or menu and
   at least one test that proves it, with no row left empty.
2. WHEN 1.0 is prepared THE SYSTEM SHALL record in a new ADR that the SVG export contract
   (AGENTS.md §SVG export, ADRs 0012, 0015, 0016) is frozen, and that changing it after 1.0
   requires a major version.
3. WHEN `tests/it/io_svg/` runs THE SYSTEM SHALL export a contract document holding every
   entity kind (line, circle, arc, ellipse, elliptical arc, Bézier, polyline, text strokes) on at
   least two layers. The output SHALL be byte-identical to `tests/fixtures/svg/contract-1.0.svg`.
4. WHEN a settings file, an autosave and a mother SVG written by v0.5.0 are loaded (fixtures
   committed with this spec) THE SYSTEM SHALL open them without loss or error.
5. WHEN an operator reads `docs/user-guide.md` THE SYSTEM SHALL find every entry of
   `ui/toolbar` TOOLS, every command-line alias, layers, Export Layers and the AI panel. A test
   SHALL fail if a TOOLS label is missing from the guide.
6. WHEN 1.0 is prepared THE SYSTEM SHALL ship `docs/release/smoke-1-0.md`: a numbered checklist
   the user runs on Linux, Windows and macOS (draw, edit, snap, layers, save, reopen, export
   layers, open in LaserGRBL) before tagging.
7. WHEN `scripts/backlog.sh --check` runs at the 1.0 commit THE SYSTEM SHALL show every spec in
   this spec's Depends on list as Done (or Rejected with a CHANGELOG note).
8. WHEN the release commit lands THE SYSTEM SHALL have version `1.0.0` in `Cargo.toml` and a
   `[1.0.0]` CHANGELOG section, and `README.md` SHALL describe 1.0 as stable, link the install
   and user guides, and list the non-goals (DXF, G-code, fillet/chamfer/offset, blocks).
9. WHEN the release artifacts are built from the 1.0 commit THE SYSTEM SHALL produce the Linux
   AppImage and `.deb`, the Windows `.zip` and the macOS `.dmg` (LCV-201).

## Out of scope

- Tagging and publishing the GitHub release: the user does this after the smoke checklist.
- Signing, DXF, G-code, LCV-146 skills.

## Open questions

- None. Decided (self-approved per user goal, 2026-09-30):
  - The original TypeScript source is not on this machine. The parity table therefore uses
    the original scope list recorded in `docs/product/README.md`.
  - The freeze takes the next free ADR number, and the contract fixture covers only entity
    kinds that exist at 1.0. A Depends-on spec that ends Rejected drops its entity kind from AC 3.

## Release record

- **AC 7** (2026-10-01): `scripts/backlog.sh --check` passes, and every spec in Depends on is
  Done: LCV-180, LCV-201, LCV-170..179 and LCV-194..200. None is open or Rejected.
- **AC 9**: the Linux AppImage and `.deb` are built locally from the 1.0 tree
  (`scripts/build-appimage.sh`, `scripts/build-deb.sh`; T13). The Windows `.zip` and macOS
  `.dmg` pipeline is proven green by LCV-201's dispatch, CI run
  https://github.com/grstein/LaserCAD-R14/actions/runs/36833345767. Building those two
  from the 1.0 commit itself needs one more `workflow_dispatch` (or the tag push), which is the
  user's step before publishing.
