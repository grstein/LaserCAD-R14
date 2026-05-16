# LCD-006 - English-Native Bundle, UI Strings, and Source Comments

Status: Done
Priority: P1
Owner: Product Owner
Implementation: Shipped in `dbac3bc chore(i18n): make the repository English-native (LCD-006/007/008)`

## Problem

LaserCAD R14 is positioned for a global laser-cutting community and AI-friendly collaboration, but the distributed build still leaks pt-BR through Tauri bundle metadata, the HTML document language attribute, the root `README.md`, and JSDoc/comments in `src/core/types.ts`. Outside contributors and coding agents hit Portuguese strings on first contact with the artifact and the kernel type contract, which contradicts the product positioning and increases onboarding friction.

The user-facing application chrome (menubar, toolbar, command line, statusbar, dialogs), the command aliases, the autosave keys, error messages, and the test suite are already English-native. This demand only closes the residual pt-BR surface in the bundle and source comments.

## Smallest Useful Outcome

Every shipped artifact - Tauri bundle metadata, web bundle, root README, and source comments under `src/` - reads as English-native, with no Portuguese remnants.

## Scope

In:

- Translate `shortDescription` and `longDescription` in `src-tauri/tauri.conf.json` to English.
- Change `index.html` root element to `lang="en"`.
- Translate the file header and JSDoc in `src/core/types.ts` (including the comment block at the top of the file and any inline comments on `Entity`, `AppState`, `Command`, `Tool`, and `SnapResult`).
- Rewrite Portuguese sentences in `README.md` into English while preserving the existing structure and code blocks. The link to `docs/atalhos.md` is updated to `docs/shortcuts.md` (this rename ships in LCD-007; if LCD-007 lands first, only the link in `README.md` needs adjustment).
- Sweep `src/` for residual Portuguese in comments and string literals (regex such as `\b(ção|ões|não|você|para|funcionalidade)\b`) and translate any hits.

Out:

- Translation of `docs/`, `specs/`, `CLAUDE.md`, `AGENTS.md`, or product docs (handled by LCD-007 and LCD-008).
- Introducing an i18n framework, a language toggle, or bilingual UI. The product is English-only at the artifact level.
- Renaming source files, modules, or symbols.
- Touching commit history or release notes.

## Acceptance Criteria

- `grep -rnE '\b(ção|ões|não|você|para|funcionalidade|específic)\b' src/ src-tauri/ index.html README.md` returns no matches in code, comments, or shipped strings (Portuguese inside fixtures or test data, if any, is not in scope and there should be none today).
- `src-tauri/tauri.conf.json` exposes English `shortDescription` and `longDescription`. After `npm run tauri:build`, the generated `.desktop` file on Linux (or platform equivalent) shows the English description.
- `<html lang="en">` is present in `index.html` and survives Vite build into `dist/index.html`.
- `src/core/types.ts` reads as English at the file header and around every exported type.
- `README.md` has no Portuguese sentences. Any link to the renamed shortcuts doc is correct.
- `npm run typecheck`, `npm run lint`, and `npm test` all pass.
- `npm run dev` launches and the app behaves identically to before (UI was already English; smoke is purely a regression check).

## Expected Tests

- Existing unit and interaction tests pass unchanged. No new behavior, no new tests required.
- Manual smoke:
  - Open `http://localhost:1420`, confirm `<html lang="en">` in DevTools.
  - Run `npm run tauri:build`, inspect the platform installer's description metadata.
- Manual review of `src/core/types.ts` to confirm the JSDoc still documents the same contract in English.

## Notes For Implementation Agents

- Relevant docs/files:
  - `src-tauri/tauri.conf.json`
  - `index.html`
  - `src/core/types.ts`
  - `README.md`
  - `docs/product/demands/LCD-007-english-native-documentation-cleanup.md` (link target rename)
- Risks:
  - Translation drift: keep technical terms accurate (millimeters, viewBox, kernel, snap, command line). Do not paraphrase the kernel-purity rule or the unit contract.
  - `README.md` mixes English and pt-BR sentences in the same lists; translate sentences in place rather than reorganizing sections to avoid noisy diffs.
- Product decisions:
  - No language toggle. Product is English-only at the artifact level.
  - Translation is fidelity-first; do not introduce new examples, new commands, or expanded prose.

