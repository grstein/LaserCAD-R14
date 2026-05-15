# LaserCAD R14

2D CAD for laser cutting, compatible with LaserGRBL. Available as a web app and as a native executable (Linux, Windows, macOS) via Tauri 2.

## Usage

### As a native executable (recommended for end users)

Download the installer for your platform from the [releases page](../../releases). Available for Linux (`.AppImage`, `.deb`), macOS (`.dmg`), and Windows (`.msi`, `.exe`).

### As a web app (for development)

```bash
npm install
npm run dev      # http://localhost:1420
```

See [`docs/build-local.md`](docs/build-local.md) for building binaries locally, including the per-OS system dependencies.

## What is ready

- Infinite SVG viewport in millimeters, with pan/zoom (middle button or `Space`+drag, scroll to zoom).
- 1mm/10mm grid + X/Y axes.
- Drawing tools: **Line**, **Polyline**, **Rectangle**, **Circle**, **Arc**.
- Editing: **Select**, **Move**, **Trim**, **Extend**, **Delete**.
- Snaps: **endpoint**, **midpoint**, **center**, **intersection** (toggle with F3).
- Ortho lock with **Shift** or the F8 toggle.
- Undo/Redo (`Ctrl+Z` / `Ctrl+Y`), up to 200 levels.
- Command-line input: commands (`line`, `circle`…), absolute coordinates `124.5, 87.3` and relative `@50, 0`, plain distance for preview tools.
- **Plain SVG** export compatible with LaserGRBL: **cut** (red), **mark** (blue), **engrave** (green) presets. `Ctrl+S` saves the `cut` preset.
- Autosave in `localStorage` (web build) or `tauri-plugin-store` (native build).

## Shortcuts

See [`docs/shortcuts.md`](docs/shortcuts.md). Summary:

| Key               | Action                                |
| ----------------- | ------------------------------------- |
| `L P R C A`       | Line / Polyline / Rect / Circle / Arc |
| `S M T E`         | Select / Move / Trim / Extend         |
| `Del`             | Delete selection                      |
| `Esc`             | Cancel tool                           |
| `F3 F7 F8`        | Toggle Snap / Grid / Ortho            |
| `Ctrl+Z / Ctrl+Y` | Undo / Redo                           |
| `Ctrl+S`          | Save SVG (cut preset)                 |

## Architecture

- **Frontend**: TypeScript (ES2022 modules) + native SVG + CSS Grid.
- **Build**: Vite 5.
- **Native shell**: Tauri 2 (Rust + system WebView).
- **Tests**: Vitest (jsdom).
- **Core types**: `src/core/types.ts` defines `Vec2`, `Entity`, `AppState`, `Command`, `Tool`.
- **State**: mutation only via `state.set*` or `toolManager.commit(cmd)`. The sources of truth are `src/app/state.ts` and `src/app/event-bus.ts`.

## Structure

```
index.html                — HTML shell, single entry point
assets/css/               — reset, app, theme (450nm laser palette)
src/core/                 — pure kernel (geometry + document)
src/core/types.ts         — core types (Vec2, Entity, AppState…)
src/render/               — SVG pipeline (camera, grid, overlays, entity-renderers)
src/tools/                — tool manager + 9 tools
src/ui/                   — menubar, toolbar, command-line, statusbar, dialogs
src/app/                  — state, config, shortcuts, event-bus, bootstrap
src/io/                   — export-svg, file-download, autosave
src/tauri-bridge.ts       — runtime bridge: detects Tauri vs. plain browser
src-tauri/                — Rust shell + tauri.conf.json
docs/adr/                 — architectural decisions
docs/build-local.md       — how to build locally
```

## Scripts

```bash
npm run dev               # vite dev server (web)
npm run build             # vite build → dist/
npm run typecheck         # tsc --noEmit
npm run lint              # eslint
npm run format            # prettier --write
npm test                  # vitest
npm run tauri:dev         # native window with HMR (requires Rust + deps)
npm run tauri:build       # native binaries (requires Rust + deps)
```

## LaserGRBL compatibility

The exporter follows the checklist in `plan.md` §"SVG export for LaserGRBL":

- `xmlns` on the root `<svg>`
- `width`/`height` in **mm**, `viewBox` in world coordinates
- forced `fill="none"`; no live text; no `filter`/`mask`/`clipPath`
- one group per preset (`<g id="CUT" stroke="#ff0000" stroke-width="0.1">`)
- arcs as `<path d="A">`

## Contributing

The stack is accessible to any web developer (TypeScript + Vite). Packaging as an executable requires Rust stable plus system dependencies (see `docs/build-local.md`).
