# ADR 0002 — Module integration risks

This ADR consolidates three decisions that address risks flagged at the end of the spec phase: kernel→render coupling, mount↔`getScreenCTM` ordering, and DOM hosts as a contract. All three remain in force after the migration to TypeScript + Vite + Tauri 2.x; the code samples and module paths have been updated to TS modules. See [`0001-base-architecture.md`](0001-base-architecture.md) for the baseline (SVG-first, millimeters).

- **Global status:** Accepted (in force)
- **Date:** 2026-05-12
- **Decision-makers:** LaserCAD team
- **Risks addressed:** kernel→render coupling (§1), mount↔`getScreenCTM` order (§2), DOM hosts as a contract (§3).

---

## 1. Lazy namespace lookup for the core → render bridge

### 1.1 Status

Accepted — 2026-05-12 — Decision-makers: LaserCAD team.

### 1.2 Context

`docs/plan.md` fixes that `core/geometry` and `core/document` are pure: no DOM, no global state, no bus. But it also requires that `world ↔ screen` conversions be centralized (via `getScreenCTM()`) to avoid screen↔document drift. The resolution is that `render/camera` is the **sole** owner of the implementation, and `core/geometry/project` only re-exposes the interface for callers outside `render/`.

Statically importing `@/render/camera` from `@/core/geometry/project` would push render-layer code into the pure kernel and create a one-way coupling that violates the purity rule.

### 1.3 Decision

`core/geometry/project.ts` is the **sole authorized bridge** from the pure kernel into the render layer. Any other module in `core/` that needs world↔screen conversion calls `project.*` (or, if inside `render/`, calls `render/camera` directly).

Originally this bridge used a **lazy lookup** through a global namespace because the pre-Vite runtime loaded classical scripts and had no module load-order guarantee. After the migration to TypeScript + Vite (ES modules), the load order is solved by the bundler and `project.ts` is a thin import wrapper:

```ts
// src/core/geometry/project.ts — current shape
import { camera } from '@/render/camera.js';

export const project = {
  worldFromScreen(screenPt, cam) { return camera.worldFromScreen(screenPt, cam); },
  screenFromWorld(worldPt, cam) { return camera.screenFromWorld(worldPt, cam); },
};
```

### 1.4 Consequences

- `core/geometry/project.ts` is the **only** documented exception to kernel purity — only this file in `core/` imports from `render/`. Everything else under `core/` stays pure (no DOM, no `window`, no `render.*`).
- The wrapper preserves the seam: future callers outside `render/` go through `project.*`, so a refactor that wants to push camera ownership elsewhere only edits one file.
- Hot paths (`cursor:moved`) call `render.camera.*` directly; `project.*` is reserved for callers outside `render/`.
- A new exceptional bridge (e.g. another core→non-core dependency) requires its own ADR — do not generalize this one into a free pass.

---

## 2. Mandatory bootstrap order: mount before input wire-up

### 2.1 Status

Accepted — 2026-05-12 — Decision-makers: LaserCAD team.

### 2.2 Context

`camera.worldFromScreen` depends on `getScreenCTM()` of the root `<svg>`. Before the `<svg>` is in the DOM, `getScreenCTM()` returns `null` or the identity matrix and any coordinate conversion produces garbage. If `bootstrap` registered `pointermove` listeners before mounting the `<svg>`, the first `cursor:moved` event would carry invalid coordinates into the state.

Symmetrically, `viewport:resized` may fire before the camera has valid `viewportW/viewportH`, racing `refreshViewBox` in `svg-root`.

### 2.3 Decision

`src/app/bootstrap.ts → start()` runs **strictly** in this order, without parallelization:

```
 1. validateDomHosts()               // see §3
 2. state = state.init()
 3. config = config.load()
 4. svgRoot = svgRoot.mount('#viewport-host')
 5. state.setViewportSize(svgRoot.getSize())
 6. camera.attach(svgRoot)           // camera starts using getScreenCTM()
 7. grid.mount(svgRoot)
 8. bed.mount(svgRoot)
 9. overlays.mount(svgRoot)          // crosshair + coords label
10. menubar.mount('#menubar-host')
11. toolbar.mount('#toolbar-host')
12. commandLine.mount('#commandline-host')
13. statusbar.mount('#statusbar-host')
14. dialogs.init()
15. toolManager.init()
16. toolManager.register('select', selectTool)  // plus the other 8 tools
17. shortcuts.attach(window)         // global keyboard listeners
18. wireViewportInput(svgRoot)       // viewport pointer events — ONLY HERE
19. wireResizeObserver(svgRoot)      // viewport:resized — ONLY HERE
20. bus.emit('app:ready', {})
```

Steps 1–17 are synchronous and never touch input. Input wire-up (18–19) happens only after `svg-root` is mounted **and** the camera is attached. The authoritative sequence lives in the file header of `src/app/bootstrap.ts`; this ADR fixes the rule, the file fixes the exact symbols.

### 2.4 Consequences

- `getScreenCTM()` is only called once it is guaranteed valid. World↔screen coordinates are reliable from the very first frame.
- If any of steps 1–17 throws, `bootstrap` writes a banner into `document.body` and **does not** register listeners — the app stays "dead but inspectable" rather than "live with invalid coordinates".
- `viewport:resized` never fires before `app:ready`. Race condition eliminated.
- This order is versioned in `src/app/bootstrap.ts`; changes require a new ADR.

---

## 3. DOM hosts as a public contract between HTML and bootstrap

### 3.1 Status

Accepted — 2026-05-12 — Decision-makers: LaserCAD team.

### 3.2 Context

`bootstrap` mounts components via fixed IDs (`#viewport-host`, `#menubar-host`, `#toolbar-host`, `#commandline-host`, `#statusbar-host`) that must be present in `index.html`. Silently renaming a host breaks bootstrap with no obvious error — only the corresponding component "disappears".

`docs/design.md` ("General layout") lays out the four-region + menubar layout; `index.html` materializes those IDs; `src/app/bootstrap.ts` consumes them.

### 3.3 Decision

The five IDs below are a **public contract** of the project. Renaming requires an ADR. Adding a new host requires an ADR. Bootstrap validates their presence before any mount.

| ID                  | Function                                       | Declared in  | Consumer                   |
| ------------------- | ---------------------------------------------- | ------------ | -------------------------- |
| `#menubar-host`     | container for the 28px menubar                 | `index.html` | `src/ui/menubar.ts`        |
| `#toolbar-host`     | container for the 40px vertical toolbar        | `index.html` | `src/ui/toolbar.ts`        |
| `#viewport-host`    | container for the root `<svg>` (main area)     | `index.html` | `src/render/svg-root.ts`   |
| `#commandline-host` | container for the command line (3 lines, 66px) | `index.html` | `src/ui/command-line.ts`   |
| `#statusbar-host`   | container for the 24px status bar              | `index.html` | `src/ui/statusbar.ts`      |

`validateDomHosts()` in `bootstrap` iterates this list and, if any host is missing, writes a banner into `document.body` reading `[LaserCAD] Missing DOM host: <id>` (`color: var(--status-error)`) and throws to `console.error`. Unmounted hosts stay inert — the failure is never silent.

### 3.4 Consequences

- `index.html` and `src/app/bootstrap.ts` reference this table as the source of truth. Renaming is valid only after an ADR.
- A missing-host error shows up visually within `< 100ms` of page load, inside the app itself, with a precise message.
- Manual tests can change IDs in DevTools and observe the validation behavior (delete `#viewport-host`, reload, see the banner).
- Adding a sixth region in the future (e.g. a properties panel) requires: (a) a new host in `index.html`, (b) an update to this table, (c) a mention in `bootstrap.validateDomHosts()`. Inconsistency is reported in PR review.

---

## Relationship to ADR 0001

ADR 0001 established the baseline (SVG-first and millimeters; §3's "no ES modules" decision is superseded). ADR 0002 covers the three critical boundaries where the general rule meets operational exceptions:

1. **Kernel purity × centralized precision** → resolved by isolating the bridge to `core/geometry/project.ts` (§1)
2. **Async mount × `getScreenCTM` validity** → resolved with a mandatory bootstrap sequence (§2)
3. **No framework × shared DOM** → resolved with hosts as a contract (§3)

Each decision preserves the simplicity of ADR 0001 without masking integration errors.
