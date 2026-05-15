# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

LaserCAD R14 is a 2D micro-CAD for laser cutting, compatible with LaserGRBL. The frontend is TypeScript + Vite + native SVG; the native shell (Linux/macOS/Windows) is Tauri 2 (Rust + system WebView).

## Language Convention

All versioned artifacts (source, comments, documentation, commit messages, file names) are written in English. Conversation with the product owner can happen in any language; the repository itself remains English-only.

## Commands

```bash
npm install
npm run dev               # vite dev (web) at http://localhost:1420
npm run build             # vite build → dist/
npm run typecheck         # tsc --noEmit
npm run lint              # eslint
npm run format            # prettier --write .
npm test                  # vitest run (jsdom)
npm run test:watch        # vitest watch mode
npm run tauri:dev         # native window with HMR (requires Rust + OS deps)
npm run tauri:build       # native binaries in src-tauri/target/release/bundle/
```

Run a single test: `npx vitest run src/core/geometry/vec2.test.ts` (or `-t "name pattern"`). Tests live next to the implementation (`*.test.ts` under `src/`) or in `tests/`; the environment is jsdom (see `vitest.config.ts` and `tests/setup.ts`).

Native prerequisites (Rust + per-OS libs) are documented in `docs/build-local.md`.

## Architecture

### Layers and purity rule

```
src/core/        pure kernel: geometry (vec2, line, circle, arc, intersect, snap, project) + document (schema, validators, commands, history)
src/render/      SVG pipeline: camera, svg-root, grid, bed, overlays, entity-renderers
src/tools/       tool state machine (tool-manager) + 9 tools (line, polyline, rect, circle, arc, select, move, trim, extend)
src/ui/          HTML chrome: menubar, toolbar, command-line, statusbar, dialogs
src/io/          export-svg, file-download, autosave (localStorage on web, tauri-plugin-store on native)
src/app/         state singleton, config, shortcuts, event-bus, bootstrap
src/tauri-bridge.ts  detects `window.__TAURI_INTERNALS__` and exposes save/open/store through Tauri plugins
src-tauri/       Rust shell + tauri.conf.json
```

- `core/` is **pure**: no DOM, no `window` (with the single authorized exception of `core/geometry/project.ts`, which lazy-looks up `render.camera`; see `docs/adr/0002-integration-risks.md` §1).
- `tools/` never touches the DOM directly; it emits through the bus and applies `Command` (do/undo) via `toolManager.commit(cmd)`.
- mm ↔ pixel conversion lives in `render/camera.ts` and nowhere else.

### Units and types

- **Millimeters are canonical** across document, kernel, command line, and export. Pixels only appear in `render/camera`. Angles: degrees in the UI, **radians in the kernel/state** (see `arc.startAngle`/`endAngle`).
- Core types in `src/core/types.ts`: `Vec2`, `Entity` (`LineEntity | CircleEntity | ArcEntity`), `AppState`, `Command`, `Tool`, `SnapResult`. Import from there instead of redefining.
- `documentBounds` defaults to `128×128 mm`; it becomes the `viewBox` of the root `<svg>` and the `width="...mm" height="...mm"` attributes on export.

### State and mutation (hard contract)

`src/app/state.ts` is the only singleton authorized to mutate `AppState`. The exact shape of `AppState`, the authorized setters, and the canonical event list are defined in `src/app/state.ts` and `src/app/event-bus.ts`; read those before touching state, commands, selection, history, or the event bus. Rules:

- Direct external assignment (`state.activeTool = 'line'`) is **forbidden**. Use the setters: `state.setCamera`, `setViewportSize`, `setCursor`, `setActiveTool`, `setToolState`, `setToggle`, `setCommandInput`, `pushCommandHistory`, `setDocumentBounds`, `setSelection`.
- Mutating nested objects from outside is also **forbidden**: e.g. `state.toggles.snap = false` or `state.entities.push(...)` / `state.selection.splice(...)`. Go through the setter or through `applyCommand`.
- Changes to `entities`/`selection` go through `state.applyCommand(cmd)` (or `toolManager.commit(cmd)`), which pushes onto `history` for Undo/Redo.
- `entity.id` follows `'e_<n>'`; the counter is private to `core/document/commands.ts`.

### Event bus (canonical list)

`src/app/event-bus.ts` keeps a **closed list** of events; emitting or listening for anything outside it triggers a `console.warn`:

```
app:ready  viewport:resized  camera:changed  cursor:moved
tool:request  tool:armed  tool:cancel
command:submit  command:error  toggle:changed  bounds:changed
```

Adding new events requires an ADR (see `docs/adr/`).

### Strict bootstrap order

`src/app/bootstrap.ts` follows steps 1–20 exactly as documented in the file header (originally from ADR 0002 §2). Do not reorder without an ADR: several things depend on `<svg>` being mounted before `getScreenCTM` is called, pointer listeners are only wired after mount, and so on.

DOM hosts expected in `index.html`: `#menubar-host`, `#toolbar-host`, `#viewport-host`, `#commandline-host`, `#statusbar-host`. `bootstrap.start()` aborts with a visible banner if any host is missing.

### Runtime split: web vs Tauri

`src/tauri-bridge.ts` is the only bridge. In a plain browser, all functions return `null`/`undefined` and the app falls back to `localStorage` + Blob downloads. Under Tauri, dynamic imports of the plugins (`@tauri-apps/plugin-dialog`, `-fs`, `-store`) drive native save/open. **Do not import Tauri plugins statically** — only inside functions guarded by `isTauri()` so the web bundle stays lean.

### Imports and aliases

- TS path alias `@/*` → `src/*` (see `tsconfig.json` and `vite.config.ts`). Use `@/app/state.js` (with `.js`) in imports — Vite/TS resolve the corresponding `.ts`.
- ESLint config (`eslint.config.js`) is in flat-config form; it ignores `dist/`, `src-tauri/`, `.playwright-cli/`.

### SVG export (LaserGRBL compatibility)

`src/io/export-svg.ts` follows the SVG export checklist in `docs/plan.md` (the "LaserGRBL SVG export" section):

- `xmlns` on the root `<svg>`; `width`/`height` in mm; `viewBox` in world coordinates
- `fill="none"` forced; no live text; no `filter`/`mask`/`clipPath`
- One `<g>` per preset color: **cut** red, **mark** blue, **engrave** green; `stroke-width="0.1"` mm
- Arcs as `<path d="A">` (not bézier `<path>`)

Changing these rules breaks LaserGRBL import — confirm before doing so.

## Reference documentation

- `docs/plan.md` (~34 KB) — frozen technical plan, source of truth for export rules and conventions.
- `docs/design.md` — UI design (menubar, toolbar, statusbar, command line, state machine).
- `docs/adr/0001-arquitetura-base.md` — SVG-first, mm, classic scripts (replaced by ES modules after the migration, but the purity/unit rules still hold).
- `docs/adr/0002-integration-risks.md` — lazy lookup core→render, bootstrap order, mandatory DOM hosts.
- `docs/shortcuts.md` — keyboard shortcuts (L/P/R/C/A for tools, F3/F7/F8 for toggles, Ctrl+Z/Y/S).
