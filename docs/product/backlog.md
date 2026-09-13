# Backlog

Single-writer: `demand-manager`. Reflects the **demand file `Status:` lines** verbatim. PLAN.md mirrors this view.

## Ready

| ID | Title | Phase | Depends on |
|---|---|---|---|
| LCV-120 | The viewport never lets the app idle | 11 | — |
| LCV-127 | The drag term of `viewport_is_live` has no behavioural test | 11 | LCV-120 |
| LCV-121 | The transport speaks tool calls | 12 | — |
| LCV-122 | The bridge: one action, one command, one undo entry | 12 | LCV-121 |
| LCV-123 | The agent turn draws on the operator's real drawing | 12 | LCV-121, LCV-122 |
| LCV-124 | The command line can reach the agent | 12 | LCV-123 |
| LCV-125 | The panel shows what the agent did to the drawing | 12 | LCV-121, LCV-123 |

**Marco 2 drive order**: LCV-121 → LCV-122 → LCV-123 run as one unbroken
sequence — do not start the next until the previous one is `Done`. Once
LCV-123 ships, LCV-124 and LCV-125 may run in parallel.

> **Release hazard, LCV-121 → LCV-123 (do not tag or cut a release in this
> window).** Once LCV-121 ships, the transport can return tool calls, but
> `panel.rs::submit` still dispatches them against a throwaway
> `Document::default()` until LCV-123 ships — the agent will confidently
> report geometry that never appears on the operator's canvas. The
> application is strictly worse in this window than it is today. This
> hazard closes the moment LCV-123 reaches `Done`.

## In Progress

| ID | Title | Phase | Owner | Started |
|---|---|---|---|---|

_None._

## Done

| ID | Title | Phase | Shipped | Commit |
|---|---|---|---|---|
| LCV-001 | Cargo project skeleton + dependency lock | 0 | 2026-05-17 | fd6a31d |
| LCV-002 | rustfmt + clippy config + rust-toolchain pin | 0 | 2026-05-17 | b6464983 |
| LCV-003 | ADR 0001 — pure Rust + egui decision | 0 | 2026-05-17 | cb563b2 |
| LCV-004 | CI workflow (fmt + clippy + test, Linux) | 0 | 2026-05-17 | fe4f5c94 |
| LCV-005 | README + LICENSE-MIT + LICENSE-APACHE + CHANGELOG scaffold | 0 | 2026-05-17 | cb563b2 |
| LCV-006 | Agent harness alive — confirm .claude/agents/*.md round-trip | 0 | 2026-05-17 | cb563b2 |
| LCV-007 | Bootstrap egui window — title 'LaserCAD v2' | 0 | 2026-05-17 | d94e038 |
| LCV-010 | Vec2 + epsilon | 1 | 2026-05-17 | 2f2a8bf |
| LCV-011 | Line type + bbox + helpers | 1 | 2026-05-17 | c6f09f9 |
| LCV-012 | Circle type + bbox + helpers | 1 | 2026-05-17 | 4ff9ee4 |
| LCV-013 | Arc type + bbox + containsAngle + endpoints | 1 | 2026-05-17 | caad4e4 |
| LCV-014 | Line-line / line-circle / circle-circle intersections | 1 | 2026-05-17 | fdade9b |
| LCV-015 | Rect predicates (contains/crosses line/circle/arc) | 1 | 2026-05-17 | b7a9882 |
| LCV-016 | Snap engine (endpoint/midpoint/center/intersection) | 1 | 2026-05-17 | ccadb5e |
| LCV-017 | Geometry test suite consolidation | 1 | 2026-05-17 | bdb4830 |
| LCV-020 | Entity enum + schema_version=1 | 2 | 2026-05-17 | b6cdd46 |
| LCV-021 | Document struct + bounds + Default | 2 | 2026-05-17 | 0bf5943 |
| LCV-022 | Command trait + do/undo semantics | 2 | 2026-05-17 | af28f9b |
| LCV-023 | CreateLine / CreateCircle / CreateArc commands | 2 | 2026-05-17 | e9f3f82 |
| LCV-024 | DeleteEntities + MoveEntities commands | 2 | 2026-05-17 | c8f4e1e |
| LCV-025 | TrimEntities + ExtendEntities commands | 2 | 2026-05-17 | 59be8e7 |
| LCV-026 | History stack (200-deep, undo/redo) | 2 | 2026-05-17 | 2fd3960 |
| LCV-027 | Selection model + SelectionCommand | 2 | 2026-05-17 | d7429cf |
| LCV-030 | eframe::App impl + central panel | 3 | 2026-05-17 | 24b118a |
| LCV-031 | Camera (world↔screen, zoom, pan, zoom-extents) | 3 | 2026-05-17 | 33bae5a |
| LCV-032 | Viewport wiring (pointer input → tools) | 3 | 2026-05-17 | 33bae5a |
| LCV-033 | Grid renderer (responsive minor/major) | 3 | 2026-05-17 | a51f2b9 |
| LCV-034 | Bed renderer (rectangle + dark outer overlay) | 3 | 2026-05-17 | ee8cea4 |
| LCV-035 | Entity painter (line/circle/arc) | 3 | 2026-05-17 | 422f457 |
| LCV-036 | Selection highlight rendering | 3 | 2026-05-17 | fbd0e99 |
| LCV-037 | Preview overlay (live tool preview) | 3 | 2026-05-17 | 6530bad |
| LCV-038 | Snap marker rendering | 3 | 2026-05-17 | 7db1ddb |
| LCV-040 | Tool trait + ToolManager | 4 | 2026-05-17 | ca7c234 |
| LCV-041 | Pointer input plumbing through ToolManager | 4 | 2026-05-17 | a628859 |
| LCV-042 | SelectTool (point pick + window/crossing box) | 4 | 2026-05-17 | ee2a16d |
| LCV-043 | LineTool | 4 | 2026-05-17 | e36b840 |
| LCV-044 | PolylineTool | 4 | 2026-05-17 | 1af93b9 |
| LCV-045 | RectTool | 4 | 2026-05-17 | db24e9c |
| LCV-046 | CircleTool | 4 | 2026-05-17 | fe57b7c |
| LCV-047 | ArcTool | 4 | 2026-05-17 | d0656a3 |
| LCV-048 | TextTool (Hershey-based) | 4 | 2026-05-17 | b59aa63 |
| LCV-049 | MoveTool | 4 | 2026-05-17 | 498eb0d |
| LCV-050 | TrimTool | 4 | 2026-05-17 | a88fe1f |
| LCV-051 | ExtendTool | 4 | 2026-05-17 | f0c6fb1 |
| LCV-052 | DeleteTool / Delete key | 4 | 2026-05-17 | c976c36 |
| LCV-053 | Ortho lock toggle | 4 | 2026-05-17 | 90ea0b4 |
| LCV-055 | Hershey font data + text layout | 5 | 2026-05-17 | 60138bf |
| LCV-056 | SVG export (cut/mark/engrave presets, LaserGRBL) | 5 | 2026-05-17 | f6a402e |
| LCV-057 | SVG import (roxmltree, strict subset) | 5 | 2026-05-17 | d5b2edb |
| LCV-058 | Settings store (JSON file via directories) | 5 | 2026-05-17 | 30f9ca0 |
| LCV-059 | Autosave (debounced, restore on boot) | 5 | 2026-05-17 | 011d7ee |
| LCV-060 | Recent files (store-backed) | 5 | 2026-05-17 | 76f0af4 |
| LCV-061 | File dialogs (rfd wrapper) | 5 | 2026-05-17 | 4e664ea |
| LCV-062 | New / Open / Save / Save As / Exit actions | 5 | 2026-05-17 | e7566c4 |
| LCV-065 | Menubar (File / Edit / View / Tools / Help) | 6 | 2026-05-17 | e47d974 |
| LCV-066 | Toolbar with tool buttons | 6 | 2026-05-17 | e5e94e5 |
| LCV-067 | Status bar (coords / units / active tool) | 6 | 2026-05-17 | 9584a71 |
| LCV-068 | Command-line widget (bottom dock) | 6 | 2026-05-17 | 4c36e58 |
| LCV-069 | Modal dialogs (confirm / error / about) | 6 | 2026-05-17 | 7960ec9 |
| LCV-070 | Keyboard shortcuts (L/P/R/C/A, F3/F7/F8, Ctrl+Z/Y/N/O/S) | 6 | 2026-05-17 | 35a804a |
| LCV-071 | Theme + visual polish | 6 | 2026-05-17 | 31c492d |
| LCV-075 | Undo/Redo keyboard shortcuts (Ctrl+Z / Ctrl+Y) + status bar flash | 7 | 2026-05-17 | 1ac0219 |
| LCV-076 | Agent settings dialog (API key / model / endpoint) | 7 | 2026-05-17 | 8b545c6 |
| LCV-077 | Agent transport — blocking reqwest client for OpenAI-compatible API | 7 | 2026-05-17 | 16a7683 |
| LCV-078 | Agent tool registry (CAD actions exposed to LLM) | 7 | 2026-05-17 | 86c473e |
| LCV-079 | Multi-turn loop with iteration cap | 7 | 2026-05-17 | cbd63ec |
| LCV-080 | Agent panel — chat UI for AI assistant | 7 | 2026-05-17 | ee10f2a |
| LCV-085 | Linux AppImage build | 8 | 2026-05-17 | 2a8883c |
| LCV-086 | Linux .deb package | 8 | 2026-05-17 | 5de9175 |
| LCV-087 | Release profile tuning + binary strip | 8 | 2026-05-17 | 5cc4909 |
| LCV-088 | Icons + .desktop entry | 8 | 2026-05-17 | c5b9028 |
| LCV-090 | Windows MSI/NSIS build | 9 | 2026-05-17 | 318909a |
| LCV-091 | macOS dmg + notarization | 9 | 2026-05-17 | 3ca6879 |
| LCV-092 | CI multi-platform pipeline | 9 | 2026-05-17 | 263e7cc |
| LCV-100 | SVG export/import Y-axis inversion (world Y-up ↔ SVG Y-down) | 10 | 2026-09-12 | 3d8aec4 |
| LCV-101 | Load persisted settings at startup (+ OpenRouter default endpoint) | 10 | 2026-09-12 | e3258d2 |
| LCV-102 | Autosave actually fires (History revision as the dirty signal) | 10 | 2026-09-12 | a958d29 |
| LCV-103 | Keyboard routing: single gate, no double dispatch, Enter reaches the tool | 10 | 2026-09-12 | 88bc4c9 |
| LCV-104 | Reachability: TEXT tool, full toolbar, Tools menu, Agent settings item | 10 | 2026-09-12 | 4b8c243 |
| LCV-105 | Cleanup: orphan files, Select-All through a Command, `app/` split, MODULE retirement, window title | 10 | 2026-09-12 | 0d1d52b |
| LCV-106 | OffsetTool: product decision and removal | 10 | 2026-09-12 | adbc878 |
| LCV-107 | CI: Windows package job invokes `build-msi.ps1` via PowerShell | 10 | 2026-09-12 | c5b5be4 |
| LCV-108 | Docs sync: CHANGELOG, README status, AGENTS.md reality, backlog consistency | 10 | 2026-09-12 | b24da80 |
| LCV-110 | Command-line parser: a pure kernel module for `X,Y`, `@X,Y`, distance, aliases, toggles and zoom | 11 | 2026-09-12 | 66af950 |
| LCV-111 | Wire the parser into the tools: `on_command_input`, R14 prompts, focus-on-typing | 11 | 2026-09-12 | 533e24a |
| LCV-112 | TEXT complete: raw-input mode for the string, then the height, in exactly one undo step | 11 | 2026-09-12 | 2ef12af |
| LCV-113 | Confirm discard on New / Open / Exit when the document has unsaved changes | 11 | 2026-09-13 | a7df030 |
| LCV-114 | Configurable bed size (1..2000 mm), stored in the document and read back on import | 11 | 2026-09-13 | bd49c11 |
| LCV-117 | Hershey glyph table: 22 alphanumeric glyphs (plus `?`) render below the baseline | 11 | 2026-09-13 | d696f7c |
| LCV-118 | Disarm native file dialogs outside the app binary | 11 | 2026-09-13 | fa158c0, 5e74557 |
| LCV-115 | Export preset selector (cut / mark / engrave) | 11 | 2026-09-13 | 2f20fdd, 24e36aa |
| LCV-116 | Chrome completion: clickable mode toggles, autosave indicator, F1 shortcuts dialog, Ortho in the View menu | 11 | 2026-09-13 | fd9b4e4, 900f0c7 |
| LCV-119 | Real user paths are injected: `cargo test` stops writing the developer's config and data directories | 11 | 2026-09-13 | 2d81a14, e90c3e0, 0e9c9c4 |

## Blocked

| ID | Title | Reason |
|---|---|---|
| LCV-089 | First 0.1.0 release tag + GitHub release | Remote + CI in place; awaiting user manual smoke test and v0.1.0 tag |

## Draft (awaiting refinement)

| ID | Title | Phase |
|---|---|---|
| LCV-126 | The F1 shortcuts dialog does not mention the command line | 11 |

## Rejected

| ID | Title | Reason |
|---|---|---|
| LCV-054 | OffsetTool — Offset a Line or Arc by Distance | Removed by LCV-106 (commit adbc878): OFFSET is an explicit non-goal (`docs/product/README.md`); the id was originally planned for "snap integration into all drawing tools," a capability already delivered by LCV-041. |

## Superseded

| ID | Title | Superseded by |
|---|---|---|

_None._
