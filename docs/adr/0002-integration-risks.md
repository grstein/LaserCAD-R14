# ADR 0002 — Module integration risks

This ADR consolidates three decisions that address risks flagged by the WS-A, WS-B, and WS-C subagents at the end of Sprint 1 (specs). They are a prerequisite for Sprint 2 (implementation): the three code workstreams will consume these rules simultaneously.

- **Global status:** Accepted
- **Date:** 2026-05-12
- **Decision-makers:** LaserCAD team
- **Risks addressed:** WS-A risk #1 (core→render coupling), WS-B risk #1 (mount↔getScreenCTM order), WS-C risk #1 (DOM hosts).

---

## 1. Lazy namespace lookup for the core → render bridge

### 1.1 Status

Accepted — 2026-05-12 — Decision-makers: LaserCAD team.

### 1.2 Context

`plan.md` L222 fixes that `core/geometry` and `core/document` are pure: no DOM, no global state, no bus. But `plan.md` L269 also requires that `world↔screen` conversions be centralized (via `getScreenCTM()`) to avoid screen↔document drift. Sprint 1 resolved this by specifying that `render/camera` is the **sole** owner of the implementation and `core/geometry/project` only re-exposes the interface for callers outside `render/`.

`namespace.md` defines the load order from leaves to roots: `project.js` loads at slot 6, `camera.js` at slot 15. A **literal** reference to `window.LaserCAD.render.camera` inside `project.js` at registration time (top-level of the IIFE) would fail — `render.camera` does not yet exist when `project.js` is evaluated.

### 1.3 Decision

Adopt **lazy lookup** inside the methods of `core.geometry.project`. The functions look up `window.LaserCAD.render.camera` **at call time**, not at load time:

```js
// src/core/geometry/project.js (skeleton)
(function (LaserCAD) {
  'use strict';
  const ns = LaserCAD.core.geometry;
  ns.project = {
    worldFromScreen(screenPt, camera) {
      const impl = window.LaserCAD.render && window.LaserCAD.render.camera;
      if (!impl)
        throw new Error(
          'LaserCAD: render.camera not loaded — project.worldFromScreen requires script order per namespace.md',
        );
      return impl.worldFromScreen(screenPt, camera);
    },
    screenFromWorld(worldPt, camera) {
      const impl = window.LaserCAD.render && window.LaserCAD.render.camera;
      if (!impl) throw new Error('LaserCAD: render.camera not loaded');
      return impl.screenFromWorld(worldPt, camera);
    },
  };
})(
  (window.LaserCAD = window.LaserCAD || {
    core: { geometry: {}, document: {} },
    render: {},
    tools: {},
    ui: {},
    io: {},
    app: {},
    bus: {},
  }),
);
```

### 1.4 Consequences

- `core.geometry.project` is the **only** documented exception to kernel purity — only in this file does the core look at `render`. Everything else under `core/` stays pure.
- The load order in `namespace.md` remains valid; the lazy lookup does not require changing it.
- Ordering errors surface clearly (a message naming the missing module), not as silent `NaN`.
- Runtime cost is negligible: one property hop into the namespace per call. In hot paths (cursor:moved), modules call `render.camera.*` directly — `project.*` is only for callers outside `render/`.
- The pattern can be reused if other exceptional bridges show up in the future (each new exception recorded in its own ADR).

---

## 2. Mandatory bootstrap order: mount before input wire-up

### 2.1 Status

Accepted — 2026-05-12 — Decision-makers: LaserCAD team.

### 2.2 Context

WS-B flagged that `camera.worldFromScreen` depends on `getScreenCTM()` of the root `<svg>`; before the `<svg>` is in the DOM (mounted), `getScreenCTM()` returns `null` or the identity matrix — any coordinate conversion produces garbage. If `bootstrap` registered `pointermove` listeners before mounting the `<svg>`, the `cursor:moved` event at the first render would emit invalid coordinates and contaminate the state.

WS-B additionally notes that `viewport:resized` may fire before the camera has valid `viewportW/viewportH`, racing `refreshViewBox` in `svg-root`.

### 2.3 Decision

The `LaserCAD.app.bootstrap.start()` function runs **strictly** in this order, without parallelization:

```
1. validateNamespacePresence()      // confirm that window.LaserCAD.core.geometry.vec2 (and others) exist
2. state = LaserCAD.app.state.init()
3. config = LaserCAD.app.config.load()
4. validateDomHosts()               // see §3 of this ADR
5. svgRoot = LaserCAD.render.svgRoot.mount('#viewport-host')
6. state.setViewportSize(svgRoot.getSize())   // first measurement
7. LaserCAD.render.camera.attach(svgRoot)     // camera starts using getScreenCTM()
8. LaserCAD.render.grid.mount(svgRoot)
9. LaserCAD.render.overlays.mount(svgRoot)    // crosshair + coords label
10. LaserCAD.ui.menubar.mount('#menubar-host')
11. LaserCAD.ui.toolbar.mount('#toolbar-host')
12. LaserCAD.ui.commandLine.mount('#commandline-host')
13. LaserCAD.ui.statusbar.mount('#statusbar-host')
14. LaserCAD.ui.dialogs.init()
15. LaserCAD.tools.toolManager.init()
16. LaserCAD.tools.toolManager.register('select', LaserCAD.tools.selectTool)
17. LaserCAD.app.shortcuts.attach(window)     // global keyboard listeners
18. wireViewportInput(svgRoot)                // viewport pointer events — ONLY HERE
19. wireResizeObserver(svgRoot)               // viewport:resized — ONLY HERE
20. LaserCAD.bus.emit('app:ready', {})
```

Steps 1–17 are synchronous and never touch input. Input wire-up (steps 18–19) happens only after `svg-root` is mounted **and** the camera is attached.

### 2.4 Consequences

- `getScreenCTM()` is called only when guaranteed valid. World↔screen coordinates are reliable from the very first frame.
- If any of steps 1–17 throws, `bootstrap` reports the error in the DOM (a banner over the body) and **does not** register listeners — the app stays "dead" but inspectable, instead of "live but with invalid coordinates".
- `viewport:resized` never fires before `app:ready` (step 19 comes after). Race condition eliminated.
- This order is fixed and versioned in `src/app/bootstrap.ts`. Changes require a new ADR.
- For manual testing: `console.log(LaserCAD.render.camera.worldFromScreen({x:100,y:100}, LaserCAD.app.state.camera))` returns a finite point after `app:ready`, never before.

---

## 3. DOM hosts as a public contract between HTML and bootstrap

### 3.1 Status

Accepted — 2026-05-12 — Decision-makers: LaserCAD team.

### 3.2 Context

WS-C flagged that `bootstrap.md` mounts components via IDs (`#viewport-host`, `#menubar-host`, `#toolbar-host`, `#commandline-host`, `#statusbar-host`) that must be present in the `index.html` produced by WS-B. Silently renaming a host breaks bootstrap with no obvious error (only the corresponding component "disappears").

`design.md` L80–L112 lays out the 4-region + menubar layout; `index.html` materializes those IDs; `src/app/bootstrap.ts` consumes them.

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

ADR 0001 established the baseline (SVG-first, mm, no ES modules). ADR 0002 covers the three critical boundaries where the general rule meets operational exceptions:

1. **Kernel purity × centralized precision** → resolved with lazy lookup (§1)
2. **No build × implicit order** → resolved with a mandatory bootstrap sequence (§2)
3. **No framework × shared DOM** → resolved with hosts as a contract (§3)

Each decision preserves the simplicity of ADR 0001 without masking integration errors.
