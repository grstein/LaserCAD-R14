# ADR 0001 — LaserCAD R14 base architecture

This ADR consolidates three structural decisions that locked the project's technical baseline before any implementation work. §1 (SVG-first) and §2 (millimeters) are still in force. §3 (no ES modules, `window.LaserCAD` namespace) was **superseded** by the migration to TypeScript + Vite + Tauri 2.x — see its status header.

- **Global status:** §1, §2 Accepted (in force); §3 Superseded
- **Date:** 2026-05-12
- **Decision-makers:** LaserCAD team
- **Related:** [`0002-integration-risks.md`](0002-integration-risks.md), `CHANGELOG.md` v1.0.0.

---

## 1. SVG-first as both rendering and export

### 1.1 Status

Accepted — 2026-05-12 — Decision-makers: LaserCAD team.

### 1.2 Context

LaserCAD R14 is a 2D micro-CAD that runs in the browser, whose final operational goal is to produce SVG files consumable by LaserGRBL (plan.md L7, L13). `plan.md` L9 explicitly states the "KISS and SVG-first" architecture: an in-house geometric model in JavaScript as the source of truth, rendering in native SVG in the DOM, and plain-SVG export for local download. `plan.md` L25 reinforces SVG as both rendering **and** output, citing SVG 2 (basic shapes equivalent to paths). The discarded-technology table (plan.md L67–75) explicitly rules out Canvas-first, Fabric.js/Konva, Paper.js/Two.js as a mandatory base, and WebGL — all for the same reason: they complicate the clean SVG export that LaserGRBL requires or add unnecessary abstraction layers between the model and the final SVG.

### 1.3 Decision

Adopt **SVG as the sole rendering and export technology**. The viewport (design.md L110, L159) is a native `<svg>` in the DOM, with `viewBox` in millimeters (plan.md L26, L274). The exporter serializes essentially the same SVG already rendered, with transform flattening and normalization to "plain SVG" per the checklist (plan.md L291–305). There is no Canvas, no WebGL, no intermediate rendering library.

### 1.4 Consequences

- The viewport is a single `<svg>` element controlled via `document.createElementNS()` (plan.md L19), with children created, updated, and removed directly in the SVG DOM.
- The renderer (`src/render/`) maps model entities to SVG nodes: `line` → `<line>`, `circle` → `<circle>`, `arc` → `<path d="M ... A ...">` (plan.md L246, L297).
- The exporter (Sprint Edit and Export, plan.md L121) reuses the existing SVG tree, flattens `transform`, and forces `fill="none"` (plan.md L298).
- Raster-based techniques (Canvas 2D, WebGL, OffscreenCanvas) cannot be used to accelerate rendering or effects — any visual effect must fit what SVG 2 offers natively.
- Performance is bound by what the browser's SVG engine delivers. Mitigation: limit the active `<g>` to what is visible per the camera (plan.md L265) only if necessary; a spatial index is out of scope for the MVP.
- Consistency between what is seen and what is exported is structural — not accidental — because it is the same tree.

---

## 2. Millimeters as the canonical document and export unit

### 2.1 Status

Accepted — 2026-05-12 — Decision-makers: LaserCAD team.

### 2.2 Context

LaserCAD is technical drawing for laser cutting. `plan.md` L26 states "Precision in mm: document, commands, and export in mm; `viewBox` defines user space". `plan.md` L103 sets as MVP acceptance criterion that "the user opens a new document, sees grid/cursor, and measures everything in millimeters". `plan.md` L217 mandates: "mm throughout the document and export; pixel only in camera/render". `plan.md` L218 splits angles: degrees in the UI, radians in the kernel. The recurring reference area is 128×128 mm (plan.md L346, design.md L102 indirectly via examples).

### 2.3 Decision

**Millimeters are the canonical unit** of the document model, user input, tools, geometric kernel, and SVG export. Pixels only appear at the camera/viewport layer and in `screen ↔ world` conversions via `getScreenCTM()` (plan.md L19). Angles follow the split rule: **degrees in the UI** (input/display) and **radians in the kernel** (trigonometric calculations).

### 2.4 Consequences

- The root `<svg>` declares `viewBox` in world coordinates (mm) — e.g. `viewBox="0 0 128 128"` for a 128×128 mm document (plan.md L280, L346).
- The SVG exporter writes `width="128mm" height="128mm"` on the root element (plan.md L278–280, L294).
- The document schema (`core/document/schema.js`) describes all coordinates, distances, radii, and lengths as numbers in mm — no suffix, no embedded unit.
- The kernel (`core/geometry/*`) operates exclusively in mm. The `EPS` tolerance (plan.md L225) is fixed in mm (e.g. `1e-6` mm).
- `mm ↔ pixel` conversion is the exclusive responsibility of `render/camera.js`. No `core/` or `tools/` module may know about pixels.
- The command line accepts coordinates in mm with no suffix: `124.5,87.3` means `(124.5 mm, 87.3 mm)` (design.md L194, L215).
- Angles: the user types "30" and the UI converts to `π/6 rad` before handing it to the kernel. The status bar and dimensions overlay display degrees.
- The status bar shows coordinates in mm with 3 decimal places: `124.500, 87.300 mm` (design.md L102, L225).
- Visual stroke width scales with zoom; the logical export value is fixed at `0.1 mm` (design.md L167, L266; plan.md L281).

---

## 3. Dropping ES modules in favor of classical scripts with a global namespace

### 3.1 Status

**Superseded** by the migration to TypeScript + Vite + Tauri 2.x (`d09346b feat(migration): TypeScript + Vite + Tauri 2.x`). The runtime requirement that drove this decision (open `index.html` by double-click) was retired when the product adopted a native desktop bundle. The repository now uses native ES modules through Vite/TypeScript; there is no `window.LaserCAD` namespace and no IIFEs.

The remainder of this section is kept for historical context and to make the trade-off visible to future contributors.

### 3.1.bis Original status

Accepted — 2026-05-12 — Decision-makers: LaserCAD team.

### 3.2 Context

`plan.md` was written assuming `<script type="module" src="./src/main.js">` and operation via a local static server. Line 228 explicitly warns: "for native modules, the app must run via a local server. MDN warns that loading HTML via `file://` produces CORS errors for modules". `plan.md` L232–234 cemented `python -m http.server 8080` as a development prerequisite.

The LaserCAD R14 operational requirement, however, is that `index.html` opens by **double-click** in any modern browser (`file://` protocol), with no server, no external dependency, no build step. That requirement collides head-on with `<script type="module">`, which fails under `file://` because of CORS restrictions applied to the module loader.

The choice is binary:

1. Keep ES modules and require a local server (the original `plan.md`).
2. Switch to classical `<script src="...">` and a global namespace (this decision).

### 3.3 Decision

**Switch from ES modules to classical scripts with a global `window.LaserCAD` namespace.** Each file is an IIFE that attaches sub-objects to `window.LaserCAD.<ns>.<name>`. `index.html` loads files via an ordered sequence of classical `<script src="...">` tags, without `type="module"`, `import`, or `export`. No bundler. No build step. The exact namespace convention and script order lived in a dedicated namespace convention document.

This decision **supersedes** the guidance in `plan.md` L23, L142, L220, L228–235 that assumed native modules. The `src/` folder remains as in `plan.md` L144–212 (modular organization by domain is kept — only the linkage mechanism changes).

### 3.4 Consequences

- **Manual order is mandatory.** `index.html` declares `<script src="...">` from leaves to roots (whoever has no dependents goes first). The canonical sequence is fixed in the namespace convention document, and each new file must be inserted at the correct position.
- **No `import`/`export`.** Each file is an IIFE shaped like `(function(LaserCAD){ ... })(window.LaserCAD = window.LaserCAD || { ... });`. No stray global variables.
- **No bundler, no build step.** The repository is served directly as-is, and `index.html` opens by double-click. CI validates syntax, tests, and smoke only; there is no packaging step.
- **Explicit and fragile dependencies.** The author of `line.js` must ensure `vec2.js` has loaded first — the system does not detect this on its own. Mitigation: order documented in `namespace.md` and each IIFE checks basic preconditions (`if (!LaserCAD.core.geometry.vec2) throw ...`) during development.
- **No tree-shaking.** Every file listed in `index.html` always loads. Acceptable while the bundle is < 200 KB minified (there is no minification in the MVP, so < 200 KB raw); revisit this decision when that limit is exceeded.
- **Less IDE/linter friendliness without `import/export`** — autocompletion and go-to-definition are partial. Mitigation: **JSDoc with `@typedef`** (plan.md L224) describing entities, event payloads, and state shapes; each file declares a header with the relevant `@typedef`s and uses `@param`/`@returns` JSDoc on every public function. ESLint is configured to recognize `window.LaserCAD` as a read global.
- **Operational testing convention:** every module is inspectable at `window.LaserCAD.*` in DevTools.
- **CI adjusted:** the `static-check` and `unit-geometry` jobs (plan.md L256–257) use Node + JSDOM or a shim that injects `window.LaserCAD` before evaluating each file in canonical order. There is no module resolution to do.
- **`plan.md` L23, L142, L220, L228–235 are superseded by this ADR.** Derived documentation must reference this ADR when touching those points.
