# LaserCAD v2 — PLAN.md

> **Live roadmap.** Single-writer: `project-manager` agent. The status table below is the source of truth for what's done, what's next, and what's blocked.

## Vision

Build a pure-Rust, single-binary, egui-based reimplementation of LaserCAD R14 v1 that reaches feature parity with v1.0.0 plus the unreleased TEXT command and Agent Harness MVP. KISS AutoCAD R14 clone for laser cutting. Linux first; Windows + macOS long-term.

## Architecture summary

- **Language**: Rust 2021 edition, stable toolchain pinned to 1.88 (via `rust-toolchain.toml`).
- **UI**: `egui` via `eframe`. No WebView, no JS, no Tauri.
- **Dialogs**: `rfd`.
- **HTTP** (for the agent): `reqwest` + `tokio`.
- **SVG**: handwritten emitter for export, `roxmltree` for import.
- **Persistence**: `serde_json` + `directories`.
- **Bundle**: single binary; Linux via AppImage / `.deb`.

Full rules: [`AGENTS.md`](AGENTS.md). Decision rationale: [`docs/adr/0001-pure-rust-egui.md`](docs/adr/0001-pure-rust-egui.md).

## Module tree

```
src/
├── main.rs        app.rs        lib.rs
├── geometry/      epsilon.rs, vec2.rs, line.rs, circle.rs, arc.rs, intersect.rs, snap.rs, rect.rs
├── document/      entity.rs, schema.rs, commands.rs, history.rs, selection.rs
├── render/        camera.rs, viewport.rs, grid.rs, bed.rs, entities.rs, preview.rs, snaps.rs
├── tools/         tool.rs, manager.rs, select.rs, line.rs, polyline.rs, rect.rs, circle.rs, arc.rs, text.rs, move_.rs, trim.rs, extend.rs, delete.rs
├── io/            settings.rs, autosave.rs, recent.rs, dialogs.rs, svg/{export.rs, import.rs}
├── ui/            menubar.rs, toolbar.rs, statusbar.rs, command_line.rs, dialogs.rs, shortcuts.rs, theme.rs
├── agent/         classifier.rs, transport.rs, tools.rs, loop_.rs, settings_ui.rs
├── text/          hershey.rs, layout.rs
└── util/          units.rs, paths.rs
```

## Phase plan

| Phase | Theme | Range | Outcome |
|---|---|---|---|
| 0 | Foundation | LCV-001..007 | Empty `cargo run` window, CI green, agent harness alive. |
| 1 | Geometry kernel | LCV-010..017 | Pure compute: vec2, line, circle, arc, intersect, snap, rect. |
| 2 | Document model | LCV-020..027 | Entities, schema, command trait, history, selection. |
| 3 | Render | LCV-030..038 | Viewport, camera, grid, bed, entities, preview, snap markers. |
| 4 | Tools | LCV-040..054 | All drawing/modify tools + ortho + snap integration. |
| 5 | I/O | LCV-055..062 | Hershey font, SVG export/import, settings, autosave, recent, dialogs. |
| 6 | UI chrome | LCV-065..071 | Menubar, toolbar, statusbar, command line, dialogs, shortcuts, theme. |
| 7 | Agent harness | LCV-075..080 | Classifier, transport, tools, multi-turn loop, settings dialog. |
| 8 | Distribution (Linux) | LCV-085..089 | AppImage, .deb, icons, desktop entry, 0.1.0 release. |
| 9 | Multi-platform | LCV-090..092 | Windows MSI/NSIS, macOS dmg + notarization, CI matrix. |

## Demand table

> **Status legend**: `Draft` (created, no body) · `Refining` (product-owner working) · `Ready` (implementable) · `In Progress` (implementer-rust on it) · `Done` · `Rejected` · `Blocked`.
> **Suggested model**: hint only; the `project-manager` picks the actual model based on availability.
> **Depends on**: lowercase `done` means dependency is complete; demand IDs without status assume the table above is authoritative.

| ID | Title | Phase | Status | Suggested agent | Suggested model | Depends on |
|---|---|---|---|---|---|---|
| LCV-001 | Cargo project skeleton + dependency lock | 0 | Draft | implementer-rust | sonnet | — |
| LCV-002 | rustfmt + clippy config + rust-toolchain pin | 0 | Draft | implementer-rust | sonnet | LCV-001 |
| LCV-003 | ADR 0001 — pure Rust + egui decision | 0 | Done | architect | opus | — |
| LCV-004 | CI workflow (fmt + clippy + test, Linux) | 0 | Draft | implementer-rust | sonnet | LCV-001 |
| LCV-005 | README + LICENSE-MIT + LICENSE-APACHE + CHANGELOG scaffold | 0 | Done | implementer-rust | sonnet | — |
| LCV-006 | Agent harness alive — confirm `.claude/agents/*.md` round-trip | 0 | Done | project-manager | opus | — |
| LCV-007 | Bootstrap egui window — title "LaserCAD v2 — bootstrap" | 0 | Draft | implementer-rust | sonnet | LCV-001, LCV-002 |
| LCV-010 | Vec2 + epsilon | 1 | Draft | implementer-rust | sonnet | LCV-007 |
| LCV-011 | Line type + bbox + helpers | 1 | Draft | implementer-rust | sonnet | LCV-010 |
| LCV-012 | Circle type + bbox + helpers | 1 | Draft | implementer-rust | sonnet | LCV-010 |
| LCV-013 | Arc type + bbox + containsAngle + endpoints | 1 | Draft | implementer-rust | sonnet | LCV-010 |
| LCV-014 | Line-line / line-circle / circle-circle intersections | 1 | Draft | implementer-rust | sonnet | LCV-011, LCV-012 |
| LCV-015 | Rect predicates (contains/crosses line/circle/arc) | 1 | Draft | implementer-rust | sonnet | LCV-011, LCV-012, LCV-013 |
| LCV-016 | Snap engine (endpoint/midpoint/center/intersection) | 1 | Draft | implementer-rust | sonnet | LCV-014 |
| LCV-017 | Geometry test suite consolidation | 1 | Draft | implementer-rust | sonnet | LCV-016 |
| LCV-020 | Entity enum + schema_version=1 | 2 | Draft | architect | opus | LCV-013 |
| LCV-021 | Document struct + bounds + Default | 2 | Draft | implementer-rust | sonnet | LCV-020 |
| LCV-022 | Command trait + do/undo semantics | 2 | Draft | architect | opus | LCV-021 |
| LCV-023 | CreateLine / CreateCircle / CreateArc commands | 2 | Draft | implementer-rust | sonnet | LCV-022 |
| LCV-024 | DeleteEntities + MoveEntities commands | 2 | Draft | implementer-rust | sonnet | LCV-022 |
| LCV-025 | TrimEntities + ExtendEntities commands | 2 | Draft | implementer-rust | sonnet | LCV-023, LCV-024 |
| LCV-026 | History stack (200-deep, undo/redo) | 2 | Draft | implementer-rust | sonnet | LCV-022 |
| LCV-027 | Selection model + SelectionCommand | 2 | Draft | implementer-rust | sonnet | LCV-022 |
| LCV-030 | eframe::App impl + central panel | 3 | Done | implementer-rust | sonnet | LCV-007 |
| LCV-031 | Camera (world↔screen, zoom, pan, zoom-extents) | 3 | Done | implementer-rust | sonnet | LCV-021, LCV-030 |
| LCV-032 | Viewport wiring (pointer input → tools) | 3 | Done | implementer-rust | sonnet | LCV-031 |
| LCV-033 | Grid renderer (responsive minor/major) | 3 | Done | implementer-rust | sonnet | LCV-031 |
| LCV-034 | Bed renderer (rectangle + dark outer overlay) | 3 | Done | implementer-rust | sonnet | LCV-031 |
| LCV-035 | Entity painter (line/circle/arc) | 3 | Done | implementer-rust | sonnet | LCV-021, LCV-031 |
| LCV-036 | Selection highlight rendering | 3 | Done | implementer-rust | sonnet | LCV-035, LCV-027 |
| LCV-037 | Preview overlay (live tool preview) | 3 | Done | implementer-rust | sonnet | LCV-035 |
| LCV-038 | Snap marker rendering | 3 | Done | implementer-rust | sonnet | LCV-031, LCV-016 |
| LCV-040 | Tool trait + ToolManager | 4 | Done | implementer-rust | sonnet | LCV-022, LCV-032 |
| LCV-041 | Pointer input plumbing through ToolManager | 4 | Draft | implementer-rust | sonnet | LCV-040 |
| LCV-042 | SelectTool (point pick + window/crossing box) | 4 | Draft | implementer-rust | sonnet | LCV-041, LCV-015, LCV-027 |
| LCV-043 | LineTool | 4 | Draft | implementer-rust | sonnet | LCV-041, LCV-023, LCV-037 |
| LCV-044 | PolylineTool | 4 | Draft | implementer-rust | sonnet | LCV-043 |
| LCV-045 | RectTool | 4 | Draft | implementer-rust | sonnet | LCV-043 |
| LCV-046 | CircleTool | 4 | Draft | implementer-rust | sonnet | LCV-041, LCV-023, LCV-037 |
| LCV-047 | ArcTool | 4 | Draft | implementer-rust | sonnet | LCV-041, LCV-023, LCV-037 |
| LCV-048 | TextTool (Hershey-based) | 4 | Draft | implementer-rust | sonnet | LCV-055, LCV-043 |
| LCV-049 | MoveTool | 4 | Draft | implementer-rust | sonnet | LCV-041, LCV-024, LCV-042 |
| LCV-050 | TrimTool | 4 | Draft | implementer-rust | sonnet | LCV-041, LCV-025 |
| LCV-051 | ExtendTool | 4 | Draft | implementer-rust | sonnet | LCV-041, LCV-025 |
| LCV-052 | DeleteTool / Delete key | 4 | Draft | implementer-rust | sonnet | LCV-041, LCV-024 |
| LCV-053 | Ortho lock toggle | 4 | Draft | implementer-rust | sonnet | LCV-041 |
| LCV-054 | Snap integration into all drawing tools | 4 | Draft | implementer-rust | sonnet | LCV-016, LCV-043, LCV-046, LCV-047 |
| LCV-055 | Hershey font data + text layout | 5 | Draft | implementer-rust | sonnet | LCV-011 |
| LCV-056 | SVG export (cut/mark/engrave presets, LaserGRBL) | 5 | Draft | implementer-rust | sonnet | LCV-021, LCV-013 |
| LCV-057 | SVG import (roxmltree, strict subset) | 5 | Draft | implementer-rust | sonnet | LCV-056 |
| LCV-058 | Settings store (JSON file via directories) | 5 | Draft | implementer-rust | sonnet | LCV-007 |
| LCV-059 | Autosave (debounced, restore on boot) | 5 | Draft | implementer-rust | sonnet | LCV-058, LCV-021 |
| LCV-060 | Recent files (store-backed) | 5 | Draft | implementer-rust | sonnet | LCV-058 |
| LCV-061 | File dialogs (rfd wrapper) | 5 | Draft | implementer-rust | sonnet | LCV-007 |
| LCV-062 | New / Open / Save / Save As / Exit actions | 5 | Draft | implementer-rust | sonnet | LCV-056, LCV-057, LCV-061 |
| LCV-065 | Menubar (File / Edit / View / Tools / Help) | 6 | Draft | implementer-rust | sonnet | LCV-062 |
| LCV-066 | Toolbar with tool buttons | 6 | Draft | implementer-rust | sonnet | LCV-040 |
| LCV-067 | Status bar (coords / units / active tool) | 6 | Draft | implementer-rust | sonnet | LCV-030 |
| LCV-068 | Command-line widget (bottom dock) | 6 | Draft | implementer-rust | sonnet | LCV-040 |
| LCV-069 | Modal dialogs (confirm / error / about) | 6 | Draft | implementer-rust | sonnet | LCV-030 |
| LCV-070 | Keyboard shortcuts (L/P/R/C/A, F3/F7/F8, Ctrl+Z/Y/N/O/S) | 6 | Draft | implementer-rust | sonnet | LCV-065 |
| LCV-071 | Theme + visual polish | 6 | Draft | implementer-rust | sonnet | LCV-030 |
| LCV-075 | Agent classifier (regex routing) | 7 | Draft | implementer-rust | sonnet | LCV-068 |
| LCV-076 | Agent settings dialog (API key / model / endpoint) | 7 | Draft | implementer-rust | sonnet | LCV-058, LCV-069 |
| LCV-077 | HTTP transport (reqwest + tokio) | 7 | Draft | architect | opus | LCV-076 |
| LCV-078 | Agent tool registry (CAD actions exposed to LLM) | 7 | Draft | architect | opus | LCV-023, LCV-024 |
| LCV-079 | Multi-turn loop with iteration cap | 7 | Draft | implementer-rust | sonnet | LCV-077, LCV-078 |
| LCV-080 | Command-line wires `:` / `/ai` prefixes to agent | 7 | Draft | implementer-rust | sonnet | LCV-075, LCV-079 |
| LCV-085 | Linux AppImage build | 8 | Draft | implementer-rust | sonnet | All Phase 7 |
| LCV-086 | Linux .deb package | 8 | Draft | implementer-rust | sonnet | LCV-085 |
| LCV-087 | Release profile tuning + binary strip | 8 | Draft | implementer-rust | sonnet | LCV-007 |
| LCV-088 | Icons + .desktop entry | 8 | Draft | implementer-rust | sonnet | LCV-085 |
| LCV-089 | First 0.1.0 release tag + GitHub release | 8 | Draft | project-manager | opus | LCV-085, LCV-086, LCV-088 |
| LCV-090 | Windows MSI/NSIS build | 9 | Draft | implementer-rust | sonnet | LCV-089 |
| LCV-091 | macOS dmg + notarization | 9 | Draft | implementer-rust | sonnet | LCV-089 |
| LCV-092 | CI multi-platform pipeline | 9 | Draft | implementer-rust | sonnet | LCV-090, LCV-091 |

LCV-003, LCV-005, LCV-006 are marked `Done` because the initial scaffold delivered them (the user can verify by reading `docs/adr/0001-pure-rust-egui.md`, `README.md`, `LICENSE-MIT`, `LICENSE-APACHE`, `CHANGELOG.md`, and `.claude/agents/`). All other demands start as `Draft` and need refinement by `product-owner` before `implementer-rust` can claim them.

## Project-manager execution algorithm

On every invocation, the `project-manager`:

1. Reads the demand table above (this file).
2. `TaskList` — checks what's already in flight.
3. Selects the **next demand to drive**:
   - Prefer the first `Ready` row whose deps are all `Done`.
   - If no `Ready` row, take the first `Draft` row whose deps are `Done` and route to `product-owner` (Draft → Refining → Ready).
   - If all `Ready` is `In Progress`, check in-flight tasks; unblock or wait/report.
4. Spawns the worker via `Agent(subagent_type: <name>)`.
5. Tracks via `TaskCreate` / `TaskUpdate`.
6. Updates this file's status table and appends a one-line entry to the execution log.
7. Reports to the user in one paragraph: what shipped, what's next, any blockers.

## Execution log

> Single-writer: `project-manager`. Most recent entries at the top. Format: `YYYY-MM-DD HH:MM — LCV-NNN status — note`.

- 2026-05-18 — Phase 3 complete — all render demands LCV-033..038 Done.
- 2026-05-18 — LCV-038 Done — snap marker rendering reviewed and approved.
- 2026-05-18 — LCV-037 Done — preview overlay reviewed and approved.
- 2026-05-18 — LCV-036 Done — selection highlight reviewed and approved.
- 2026-05-18 — LCV-035 Done — entity painter reviewed and approved.
- 2026-05-18 — LCV-034 Done — bed renderer reviewed and approved.
- 2026-05-18 — LCV-034 In Progress — bed renderer up next.
- 2026-05-18 — LCV-033 Done — grid renderer reviewed and approved.
- 2026-05-18 — LCV-033 In Progress — grid renderer implementation present, awaiting commit+review.
- 2026-05-18 — LCV-032 Done — viewport wiring: hover→world coords, wheel zoom-around, middle-drag pan, F/Ctrl+0 zoom-extents.
- 2026-05-17 — LCV-003 Done — ADR 0001 (pure Rust + egui) accepted at scaffold time.
- 2026-05-17 — LCV-005 Done — README + LICENSEs + CHANGELOG scaffolded.
- 2026-05-17 — LCV-006 Done — six agents present under `.claude/agents/`; PM ready.
- 2026-05-17 — Repo bootstrapped, awaiting first PM invocation.
