# LCD-007 - English-Native Documentation Cleanup

Status: Done
Priority: P1
Owner: Product Owner
Implementation: Shipped in `dbac3bc chore(i18n): make the repository English-native (LCD-006/007/008)`

## Problem

`docs/` mixes English product workspace files with pt-BR architecture and design docs, and the entire `specs/` tree was authored for the pre-migration architecture (classical scripts under `window.LaserCAD`, 28 JS files, opening via `file://` without a build). After the migration to TypeScript + Vite + Tauri 2.x, `src/` is the source of truth for the behavior those specs described, so reading specs misleads agents and contributors with instructions that no longer match the code.

A clean documentation surface in English, with stale specs removed, is required for the project's stated community and IA-friendliness goal. Translating dead specs is wasted effort; the right move is to keep the living docs and delete the obsolete ones.

## Smallest Useful Outcome

`docs/` is English-only and contains only living documentation. `specs/` is removed. Cross-references across the repository point at surviving paths.

## Scope

In:

**Translate in place:**

- `docs/plan.md` - keep as the canonical export/architecture rules document.
- `docs/design.md` - keep as the UI behavior document.
- `docs/build-local.md` - native build prerequisites by OS.
- `docs/adr/0001-base-architecture.md` - kernel purity and unit rules are still applicable.

**Translate and rename:**

- `docs/atalhos.md` -> `docs/shortcuts.md`.
- `docs/adr/0002-riscos-de-integracao.md` -> `docs/adr/0002-integration-risks.md`.

**Delete:**

- All of `specs/` (`specs/index-html.md`, `specs/main.md`, `specs/event-bus.md`, `specs/_conventions/**`, `specs/app/**`, `specs/render/**`, `specs/tools/**`, `specs/ui/**`, `specs/core/**`, `specs/assets/`).
- `docs/release/1.0.md` - already summarized in `CHANGELOG.md`.

**Update cross-references** so no link points at a deleted or renamed file:

- `README.md` (links at lines around 35, 54, 71 in the current file: `docs/atalhos.md` link, `specs/_conventions/state-contract.md` reference, and the `specs/` line in the directory layout).
- `AGENTS.md` (remove the line that lists `specs/_conventions/state-contract.md` as a required read; replace with `src/app/state.ts` and `src/app/event-bus.ts` as sources of truth, or with a pointer to CLAUDE.md).
- `CHANGELOG.md` (remove the link to `docs/release/1.0.md`).
- `CLAUDE.md` (remove the `specs/_conventions/state-contract.md` reference; this overlaps with LCD-008, see Notes).
- `docs/product/demands/LCD-002-document-command-line-behavior-gaps.md` (update `docs/atalhos.md` -> `docs/shortcuts.md`).
- `docs/product/demands/LCD-004-offset-line-circle-by-distance.md` (replace `specs/_conventions/state-contract.md` with `src/app/state.ts`).
- `docs/product/demands/LCD-005-normal-file-menu-svg-workflow.md` (replace `specs/_conventions/state-contract.md`, `specs/ui/menubar.md`, `specs/ui/dialogs.md` with `src/app/state.ts`, `src/ui/menubar.ts`, `src/ui/dialogs.ts`).
- ADR 0001 and ADR 0002 cross-references between each other after the rename.

Out:

- Translating `CLAUDE.md` or changing the language convention (LCD-008).
- Translating source files or bundle metadata (LCD-006).
- Rewriting technical content beyond what is needed for a faithful translation. This is not the moment to refactor the documented architecture.
- Adding new docs, an ADR index, or a contributor handbook.
- Migrating canonical state-contract rules into a new spec file. The rules that LCD-007 needs to keep alive already live in `CLAUDE.md` (Estado e mutação section); LCD-008 handles any remaining promotion.

## Acceptance Criteria

- `find docs -name '*.md' | xargs grep -lE 'ção|ões|você|não|específic'` returns empty.
- `find specs -type f` returns empty (directory may be left in place if git tracks it as removed; outcome is no files under `specs/`).
- No surviving file in the repository links to `specs/...`, `docs/atalhos.md`, `docs/adr/0002-riscos-de-integracao.md`, or `docs/release/1.0.md`. Verify with `grep -rnE 'specs/|atalhos\.md|0002-riscos|docs/release/1\.0' --include='*.md' .` returning no matches.
- `docs/shortcuts.md` exists in English with the same content shape (keyboard shortcut table) as the previous `docs/atalhos.md`.
- `docs/adr/0002-integration-risks.md` exists in English; ADR 0001 cross-reference, if present, points at the new filename.
- `README.md`, `AGENTS.md`, `CHANGELOG.md`, `CLAUDE.md` (if untouched by LCD-008 at that time) link only to surviving paths.
- `npm run typecheck`, `npm run lint`, and `npm test` pass unchanged (zero `.ts` files were edited).

## Expected Tests

- No code tests added. Verification is a documentation grep and a manual read-through of the translated files.
- Manual checks:
  - Open the translated `docs/plan.md` and confirm export rules section (LaserGRBL compatibility) still reads correctly.
  - Open `docs/shortcuts.md` and confirm the shortcut table is intact and accurate.
  - Run `grep -rnE 'specs/|atalhos\.md|0002-riscos|docs/release/1\.0' .` and confirm only `.git/` history shows up.

## Notes For Implementation Agents

- Relevant docs/files:
  - `docs/plan.md`
  - `docs/design.md`
  - `docs/build-local.md`
  - `docs/atalhos.md`
  - `docs/adr/0001-base-architecture.md`
  - `docs/adr/0002-riscos-de-integracao.md`
  - `docs/release/1.0.md`
  - `specs/` (entire tree, to be deleted)
  - `README.md`, `AGENTS.md`, `CHANGELOG.md`, `CLAUDE.md`
  - `docs/product/demands/LCD-002-document-command-line-behavior-gaps.md`
  - `docs/product/demands/LCD-004-offset-line-circle-by-distance.md`
  - `docs/product/demands/LCD-005-normal-file-menu-svg-workflow.md`
- Risks:
  - Translation drift on technical terms. Keep terms precise: kernel, viewBox, command, undo, snap, millimeters, plain SVG, LaserGRBL preset (cut/mark/engrave).
  - Coordinating the `README.md` patch with LCD-006: if LCD-006 lands first, only update the renamed link here; if LCD-007 lands first, do the full sweep.
  - Deleting `specs/_conventions/state-contract.md` while it is still listed as canonical in CLAUDE.md/AGENTS.md/README.md. Update those references in the same PR.
- Product decisions:
  - The code under `src/` is the new source of truth for behavior previously documented in `specs/`. No replacement spec file is created.
  - Release notes for v1.0 are dropped because `CHANGELOG.md` already covers the release; future releases will live only in `CHANGELOG.md` unless explicitly demanded.
  - Renames are intentional even though they cost link-fix work; they remove the last pt-BR filenames in the repository.

