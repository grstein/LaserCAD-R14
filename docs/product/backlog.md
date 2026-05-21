# Backlog

Single-writer: `demand-manager`. Reflects the **demand file `Status:` lines** verbatim. PLAN.md mirrors this view.

## Ready

| ID | Title | Phase | Depends on |
|---|---|---|---|
| LCV-033 | Grid renderer (responsive minor/major) | 3 | LCV-031 |
| LCV-034 | Bed renderer | 3 | LCV-031 |
| LCV-035 | Entity painter | 3 | LCV-021, LCV-031 |
| LCV-036 | Selection highlight rendering | 3 | LCV-035, LCV-027 |
| LCV-037 | Preview overlay | 3 | LCV-035 |
| LCV-038 | Snap marker rendering | 3 | LCV-031, LCV-016 |

## Draft (awaiting refinement)

| ID | Title | Phase |
|---|---|---|
| LCV-040 | Tool trait + ToolManager | 4 |
| LCV-041 | Pointer input plumbing | 4 |
| LCV-042 | SelectTool | 4 |
| LCV-043 | LineTool | 4 |
| LCV-044 | PolylineTool | 4 |
| LCV-045 | RectTool | 4 |
| LCV-046 | CircleTool | 4 |
| LCV-047 | ArcTool | 4 |
| LCV-048 | TextTool (Hershey) | 4 |
| LCV-049 | MoveTool | 4 |
| LCV-050 | TrimTool | 4 |
| LCV-051 | ExtendTool | 4 |
| LCV-052 | DeleteTool | 4 |
| LCV-053 | Ortho lock toggle | 4 |
| LCV-054 | Snap integration into drawing tools | 4 |
| LCV-055 | Hershey font data + text layout | 5 |
| LCV-056 | SVG export (LaserGRBL-compatible) | 5 |
| LCV-057 | SVG import (roxmltree) | 5 |
| LCV-058 | Settings store | 5 |
| LCV-059 | Autosave | 5 |
| LCV-060 | Recent files | 5 |
| LCV-061 | File dialogs (rfd) | 5 |
| LCV-062 | New / Open / Save / Save As / Exit | 5 |
| LCV-065 | Menubar | 6 |
| LCV-066 | Toolbar | 6 |
| LCV-067 | Status bar | 6 |
| LCV-068 | Command-line widget | 6 |
| LCV-069 | Modal dialogs | 6 |
| LCV-070 | Keyboard shortcuts | 6 |
| LCV-071 | Theme | 6 |
| LCV-075 | Agent classifier | 7 |
| LCV-076 | Agent settings dialog | 7 |
| LCV-077 | HTTP transport | 7 |
| LCV-078 | Agent tool registry | 7 |
| LCV-079 | Multi-turn loop | 7 |
| LCV-080 | Command-line `:` / `/ai` wiring | 7 |
| LCV-085 | Linux AppImage build | 8 |
| LCV-086 | Linux .deb package | 8 |
| LCV-087 | Release profile tuning | 8 |
| LCV-088 | Icons + .desktop entry | 8 |
| LCV-089 | First 0.1.0 release | 8 |
| LCV-090 | Windows MSI/NSIS build | 9 |
| LCV-091 | macOS dmg + notarization | 9 |
| LCV-092 | CI multi-platform pipeline | 9 |

## In Progress

| ID | Title | Phase | Owner | Started |
|---|---|---|---|---|

_None._

## Done

| ID | Title | Phase | Shipped | Commit |
|---|---|---|---|---|
| LCV-030 | eframe::App impl + central panel | 3 | 2026-05-17 | 24b118a |
| LCV-031 | Camera (world↔screen, zoom, pan, zoom-extents) | 3 | 2026-05-17 | 24b118a |
| LCV-032 | Viewport wiring (pointer input → tools) | 3 | 2026-05-18 | 33bae5a |
| LCV-001 | Cargo project skeleton + dependency lock | 0 | 2026-05-17 | fd6a31d |
| LCV-002 | rustfmt + clippy config + rust-toolchain pin | 0 | 2026-05-17 | b6464983 |
| LCV-003 | ADR 0001 — pure Rust + egui decision | 0 | 2026-05-17 | _(scaffold commit)_ |
| LCV-004 | CI workflow (fmt + clippy + test, Linux) | 0 | 2026-05-17 | fe4f5c94 |
| LCV-005 | README + LICENSE-MIT + LICENSE-APACHE + CHANGELOG scaffold | 0 | 2026-05-17 | _(scaffold commit)_ |
| LCV-006 | Agent harness alive — `.claude/agents/*.md` present | 0 | 2026-05-17 | _(scaffold commit)_ |
| LCV-007 | Bootstrap egui window — title "LaserCAD v2 — bootstrap" | 0 | 2026-05-17 | d94e038 |
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

## Rejected

| ID | Title | Reason |
|---|---|---|

_None._

## Superseded

| ID | Title | Superseded by |
|---|---|---|

_None._
