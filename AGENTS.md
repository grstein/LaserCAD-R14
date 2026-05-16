# LaserCAD R14 — Agent Guide

This file is the single source of agent instructions for the repository. Every coding agent — Claude Code, the Product Owner agent, or any other automation — reads this file. `CLAUDE.md` is a thin pointer at this document.

LaserCAD R14 is a 2D micro-CAD for laser cutting, compatible with LaserGRBL. The frontend is TypeScript + Vite + native SVG; the native shell (Linux/macOS/Windows) is Tauri 2 (Rust + system WebView). The product bias is strict: keep it very simple, preserve precision, and avoid UI or architecture growth unless it directly improves the core CAD-to-LaserGRBL workflow.

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

Native prerequisites (Rust + per-OS libs) are documented in [`docs/build-local.md`](docs/build-local.md).

## Architecture

### Layers and purity rule

```
src/core/        pure kernel: geometry (vec2, line, circle, arc, intersect, snap, project) + document (schema, validators, commands, history)
src/render/      SVG pipeline: camera, svg-root, grid, bed, overlays, entity-renderers
src/tools/       tool state machine (tool-manager) + 9 tools (line, polyline, rect, circle, arc, select, move, trim, extend)
src/ui/          HTML chrome: menubar, toolbar, command-line, statusbar, dialogs
src/io/          export-svg, import-svg, file-download, file-actions, autosave (localStorage on web, tauri-plugin-store on native)
src/app/         state singleton, config, shortcuts, event-bus, bootstrap
src/tauri-bridge.ts  detects `window.__TAURI_INTERNALS__` and exposes save/open/store through Tauri plugins
src-tauri/       Rust shell + tauri.conf.json
```

- `core/` is **pure**: no DOM, no `window` (with the single authorized exception of `core/geometry/project.ts`, which bridges into `render.camera`; see [`docs/adr/0002-integration-risks.md`](docs/adr/0002-integration-risks.md) §1).
- `tools/` never touches the DOM directly; it emits through the bus and applies `Command` (do/undo) via `toolManager.commit(cmd)`.
- mm ↔ pixel conversion lives in `render/camera.ts` and nowhere else.

### Units and types

- **Millimeters are canonical** across document, kernel, command line, and export. Pixels only appear in `render/camera`. Angles: degrees in the UI, **radians in the kernel/state** (see `arc.startAngle` / `arc.endAngle`).
- Core types in `src/core/types.ts`: `Vec2`, `Entity` (`LineEntity | CircleEntity | ArcEntity`), `AppState`, `Command`, `Tool`, `SnapResult`. Import from there instead of redefining.
- `documentBounds` defaults to `128×128 mm`; it becomes the `viewBox` of the root `<svg>` and the `width="…mm" height="…mm"` attributes on export.

### State and mutation (hard contract)

`src/app/state.ts` is the only singleton authorized to mutate `AppState`. The exact shape of `AppState`, the authorized setters, and the canonical event list are defined in `src/app/state.ts` and `src/app/event-bus.ts`; read those before touching state, commands, selection, history, or the event bus.

- Direct external assignment (`state.activeTool = 'line'`) is **forbidden**. Use the setters: `state.setCamera`, `setViewportSize`, `setCursor`, `setActiveTool`, `setToolState`, `setToggle`, `setCommandInput`, `pushCommandHistory`, `setDocumentBounds`, `setSelection`.
- Mutating nested objects from outside is also **forbidden**: e.g. `state.toggles.snap = false` or `state.entities.push(...)` / `state.selection.splice(...)`. Go through the setter or through `applyCommand`.
- Changes to `entities` / `selection` go through `state.applyCommand(cmd)` (or `toolManager.commit(cmd)`), which pushes onto `history` for Undo/Redo.
- `entity.id` follows `'e_<n>'`; the counter is private to `core/document/commands.ts`.

### Event bus (closed list)

`src/app/event-bus.ts` keeps a **closed list** of events; emitting or listening for anything outside it triggers a `console.warn`:

```
app:ready  viewport:resized  camera:changed  cursor:moved
tool:request  tool:armed  tool:cancel
command:submit  command:error  toggle:changed  bounds:changed
```

Adding a new event requires an ADR (see [`docs/adr/`](docs/adr/)).

### Strict bootstrap order

`src/app/bootstrap.ts` follows steps 1–20 exactly as documented in the file header (originally from ADR 0002 §2). Do not reorder without an ADR: several things depend on `<svg>` being mounted before `getScreenCTM` is called, pointer listeners are only wired after mount, and so on.

DOM hosts expected in `index.html`: `#menubar-host`, `#toolbar-host`, `#viewport-host`, `#commandline-host`, `#statusbar-host`. `bootstrap.start()` aborts with a visible banner if any host is missing.

### Runtime split: web vs Tauri

`src/tauri-bridge.ts` is the only bridge. In a plain browser, all functions return `null`/`undefined` and the app falls back to `localStorage` + Blob downloads. Under Tauri, dynamic imports of the plugins (`@tauri-apps/plugin-dialog`, `-fs`, `-store`) drive native save/open. **Do not import Tauri plugins statically** — only inside functions guarded by `isTauri()` so the web bundle stays lean.

### Imports and aliases

- TS path alias `@/*` → `src/*` (see `tsconfig.json` and `vite.config.ts`). Use `@/app/state.js` (with `.js`) in imports — Vite/TS resolve the corresponding `.ts`.
- ESLint config (`eslint.config.js`) is in flat-config form; it ignores `dist/`, `src-tauri/`, `.playwright-cli/`.

### SVG export (LaserGRBL compatibility)

`src/io/export-svg.ts` follows the SVG export checklist in [`docs/plan.md`](docs/plan.md) (the "SVG export for LaserGRBL" section):

- `xmlns` on the root `<svg>`; `width`/`height` in mm; `viewBox` in world coordinates
- `fill="none"` forced; no live text; no `filter`/`mask`/`clipPath`
- One `<g>` per preset color: **cut** red, **mark** blue, **engrave** green; `stroke-width="0.1"` mm
- Arcs as `<path d="A …">` (not bézier `<path>`)

Changing these rules breaks LaserGRBL import — confirm before doing so.

## Product Owner Agent

The Product Owner agent owns demand refinement and backlog quality. It does **not** edit implementation code and does **not** create commits. Its output is product documentation that an implementation agent can execute without deciding scope, acceptance, or user intent.

Canonical product workspace:

- [`docs/product/README.md`](docs/product/README.md) — product principles and scope.
- [`docs/product/product-owner-agent.md`](docs/product/product-owner-agent.md) — operating instructions for the Product Owner agent.
- [`docs/product/backlog.md`](docs/product/backlog.md) — prioritized backlog index.
- [`docs/product/demands/`](docs/product/demands/) — executable demand specs.

When refining a demand, the Product Owner agent must:

- Be critical and demanding. Reject or shrink vague, large, decorative, or speculative requests.
- Preserve KISS. Prefer one narrow workflow improvement over a broad feature family.
- Tie every accepted demand to a laser-cutting user problem.
- Define objective acceptance criteria and expected tests.
- Mark a demand as `Ready` only when an implementation agent can start without asking product questions.
- Record explicit non-goals when they prevent scope creep.

## Implementation Agents

Implementation agents should pull only from `Ready` demands in [`docs/product/backlog.md`](docs/product/backlog.md), unless the user gives a direct override.

Before coding, read:

- The selected demand file in [`docs/product/demands/`](docs/product/demands/).
- `README.md` for current capability and scripts.
- [`docs/design.md`](docs/design.md) for UI behavior.
- [`docs/plan.md`](docs/plan.md) for the frozen technical plan and LaserGRBL export constraints.
- `src/app/state.ts` and `src/app/event-bus.ts` — canonical sources of truth for state and event-bus contracts.

Implementation rules:

- Keep millimeters canonical in document, geometry, command line, and export.
- Keep `core/` pure: no DOM or runtime dependencies beyond the documented exception in `core/geometry/project.ts`.
- Mutate app state only through the state API or commands.
- Do not add new event-bus events without an ADR.
- Do not import Tauri plugins statically.
- Add focused tests for geometry, document commands, interaction behavior, or SVG export when the demand changes those surfaces.
- Update docs only when behavior changes.

## Product Philosophy

LaserCAD R14 is not a general design tool. It is a focused CAD surface for making simple, precise 2D geometry that exports clean SVG for LaserGRBL.

Default answers:

- Prefer command line and keyboard-first flows.
- Prefer SVG-native, plain, inspectable output.
- Prefer small tools that compose over smart tools with hidden behavior.
- Prefer deterministic geometry over visual convenience.
- Prefer rejecting a feature over carrying accidental product complexity.

## Reference documentation

- [`docs/plan.md`](docs/plan.md) — frozen technical plan; source of truth for export rules and conventions.
- [`docs/design.md`](docs/design.md) — UI design (menubar, toolbar, statusbar, command line, state machine, palette).
- [`docs/adr/0001-base-architecture.md`](docs/adr/0001-base-architecture.md) — SVG-first and millimeters (in force); §3's "no ES modules" decision is superseded by the Vite/Tauri migration.
- [`docs/adr/0002-integration-risks.md`](docs/adr/0002-integration-risks.md) — kernel→render bridge, bootstrap order, mandatory DOM hosts.
- [`docs/build-local.md`](docs/build-local.md) — native build prerequisites by OS.
- [`docs/shortcuts.md`](docs/shortcuts.md) — keyboard shortcuts (L/P/R/C/A for tools, F3/F7/F8 for toggles, Ctrl+Z/Y/N/O/S).
- [`docs/product/`](docs/product/) — product workspace (`README.md`, `product-owner-agent.md`, `backlog.md`, `demands/`).
