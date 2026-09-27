# Backlog

Single-writer: `demand-manager`. Reflects the **demand file `Status:` lines** verbatim. PLAN.md mirrors this view.

## Ready

| ID | Title | Phase | Depends on | Notes |
|---|---|---|---|---|
| LCV-138 | Native document title and honest file feedback | 11 | LCV-113, LCV-119, LCV-136 | In 1.0 scope per user decision 2026-09-27 |
| LCV-139 | Readable command input and destination preview | 12 | LCV-111, LCV-112, LCV-124, LCV-132 | In 1.0 scope per user decision 2026-09-27 |
| LCV-140 | Compact R14 chrome and useful action hints | 11 | LCV-115, LCV-116, LCV-132, LCV-139, LCV-141 | In 1.0 scope per user decision 2026-09-27 |
| LCV-142 | Larger tool budgets without losing turn undo | 12 | LCV-123, LCV-125, LCV-129 | In 1.0 scope per user decision 2026-09-27; suggested agent implementer-rust / opus (ADR 0007 am.(7) closes the architecture gate) |
| LCV-143 | Editable harness-aware system prompt | 12 | LCV-125, LCV-141, LCV-142 | In 1.0 scope per user decision 2026-09-27; depends-on gained LCV-142 (TurnConfig) |
| LCV-144 | Create a drawing from declarative JSON | 12 | LCV-122, LCV-123, LCV-142, LCV-143 | In 1.0 scope per user decision 2026-09-27; suggested agent implementer-rust / opus (ADR 0010 closes the architecture gate) |

**Marco 2 — complete, 2026-09-13.** All five demands are `Done` and reviewed: **LCV-121** (`d584f4d`), **LCV-122**
(`61d609b`…`ff267ec`), **LCV-123** (`6102c68`, `e7ba0a6`, `af5ef82`),
**LCV-124** (`96a8fb4`, `a1b37aa`) and **LCV-125** (`8b5fa2c`, `c6e9d9d`).
LCV-121 → LCV-122 → LCV-123 ran as one unbroken sequence; LCV-124 and LCV-125
then went in parallel, as planned.

What the milestone bought: the agent went from narrating a drawing it could not
touch to **editing the operator's live document** — a whole turn folds into a
single Ctrl+Z, a fence stops it dead if the operator draws underneath it, the
command line is a keyboard path in, and the panel makes what it did legible.

The release hazard that was live between LCV-121 and LCV-123 **closed with
LCV-123**: `panel.rs::submit` and its throwaway `Document::default()` were
deleted outright, and the AC 3 exception list in `tests/lcv122_source_scans.rs`
is now empty (`[&str; 0]`, proven non-vacuous by the reviewer from both
directions).

**Verification of Marco 2 was local, not CI.** GitHub Actions has been down for
this repository since roughly 17:28 UTC on 2026-09-13 — every job dies in 2–5
seconds with zero steps recorded, Markdown-only commits included. **LCV-122 is
the last CI-verified demand in this project.** LCV-123, LCV-124 and LCV-125 were
gated locally and re-run independently by the reviewer each time: fmt clean,
clippy `-D warnings` clean, and `cargo test --all --no-fail-fast` at 1153, 1184
and 1208 passed, 0 failed, 2 ignored. Anyone reading a red check on those
commits is reading the outage, not the code.

**Nobody tags anything.** The user tags `v0.1.0` after their own manual smoke
run, and **LCV-089 stays `Blocked`** until they do. Its checklist is now
complete at **15 steps** — the first live agent run against OpenRouter, in the
order the two reviewers ranked them. Step 5, the fence under a mid-turn user
edit, is the one both of them called the step that decides whether the build is
taggable, because it is the only property no test in the suite can reach.

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
| LCV-120 | The viewport never lets the app idle | 11 | 2026-09-13 | 0e11473, f118b7c, c61f2b5 |
| LCV-121 | The transport speaks tool calls | 12 | 2026-09-13 | d584f4d |
| LCV-122 | The bridge: one action, one command, one undo entry | 12 | 2026-09-13 | 61d609b, 2a7a9b4, ccaaf6e, ff267ec |
| LCV-123 | The agent turn draws on the operator's real drawing | 12 | 2026-09-13 | 6102c68, e7ba0a6, af5ef82 |
| LCV-124 | The command line can reach the agent | 12 | 2026-09-13 | 96a8fb4, a1b37aa |
| LCV-125 | The panel shows what the agent did to the drawing | 12 | 2026-09-13 | 8b5fa2c, c6e9d9d |
| LCV-126 | The F1 shortcuts dialog does not mention the command line | 11 | 2026-09-13 | f4c2113, 768ee9e |
| LCV-127 | The drag term of `viewport_is_live` has no behavioural test | 11 | 2026-09-13 | 97d1867 |
| LCV-129 | A hung endpoint wedges the app: the agent turn has no timeout and no cancel | 12 | 2026-09-13 | d7b72c5, 9d9d04e |
| LCV-130 | The test suite's isolation from the network depends on the absence of a proxy | 12 | 2026-09-14 | b008c3c, 8840add, 260afeb, c3cfab0 |
| LCV-131 | The command line does not know the words that name its own commands | 12 | 2026-09-14 | b3f00e3, f3cf2cc |
| LCV-128 | `AGENTS.md`'s normative per-file and per-symbol enumerations have no scan | 12 | 2026-09-13 | 8e2b7fe, 3b0e4a8 |
| LCV-133 | The F1 dialog hides five of its eight groups behind its own fold | 11 | 2026-09-13 | aa36bd8, 14708d3 |
| LCV-134 | The F1 dialog has 8pt of headroom and the next tool needs 21 | 11 | 2026-09-13 | f40f20c, fb8c4ae |
| LCV-135 | CI bills Windows and macOS on every push; gate to tags and dispatch | 11 | 2026-09-13 | a3b9769 |
| LCV-132 | A rendering criterion is paid for in painted text, not in a source scan | 12 | 2026-09-14 | 1b89f0e, 4069efc, 8495a2a, dbca0a3 |
| LCV-147 | The tree-wide loopback scan ships disabled, blocked by a control that breaks the rule it controls for | 12 | 2026-09-14 | f2aca88 |
| LCV-148 | An unrecognised line answers locally and never reaches the model | 12 | 2026-09-14 | 421d730, 4caac01, e5d33d7 |
| LCV-136 | Discard responds to real pointer clicks | 11 | 2026-09-27 | af86b34, 149797e, 54ada25, 26d5c0d, f061a56 |
| LCV-141 | Agent panel stays within the right third | 12 | 2026-09-27 | eff5c93, b703fdd, 2a2e447, 92f2367, 77b316b, 4bb53eb |
| LCV-137 | Visible grid and consistent viewport coordinates | 11 | 2026-09-27 | 451b66d, b6cb733, b330695 |

## Blocked

| ID | Title | Reason |
|---|---|---|
| LCV-089 | First 0.1.0 release tag + GitHub release | Awaiting the user's own 15-step manual smoke run and the v0.1.0 tag. CI is down repo-wide since 2026-09-13; the local gate is what Marco 2 was verified against |

## Draft (awaiting refinement)

| ID | Title | Phase | Notes |
|---|---|---|---|
| LCV-145 | Opt-in canvas observations for vision models | 12 | In 1.0 scope per user decision 2026-09-27; Ready-quality body (product-owner refined, ADR 0011 recorded) but held at Draft — its own Open Questions section still carries a pending user confirmation of ADR 0011's drawing-only raster (bed outline + entities, no grid/selection/preview). Suggested agent updated to implementer-rust / opus. If the user confirms, flip to Ready without further refinement; if the user declines, return to `architect` per the demand's own note. |
| LCV-146 | Future Markdown and frontmatter agent skills (deferred) | 12 | Stays Deferred past 1.0 per user decision 2026-09-27; not part of the current implementation queue |

LCV-136 through LCV-144 were refined to `Ready` and registered as in-1.0-scope
by user decision on 2026-09-27 (see the Ready table above); their bodies were
refined by `product-owner` (commits b04cd54, ffdfb2a, c5a03d3, ddfef11,
2e54fd8) and their architecture gates closed by `architect` (ADR 0007 am.(7),
ADR 0010, ADR 0011 — b10ebf4). Discard reproduction and the one-third panel
cap are the first usability priorities. Preserve LCV-134 before LCV-132 for
shared paint coverage. LCV-142 must preserve whole-turn undo beyond the
200-entry history depth before raising the tool budget. LCV-145 is
Ready-quality but held at Draft on a pending user confirmation (see its row
above); LCV-146 is deferred past 1.0 and must not be selected for
implementation. LCV-131 is outside this initiative; per the user's 2026-09-14
decision, LCV-148 (prefix-only routing) and LCV-131 (full command words) both
shipped with routing specified in ADR 0007.

**Drive order after Marco 2**: LCV-129 → LCV-127 → LCV-126 → LCV-128, with
LCV-130 and LCV-131 newly opened and not yet placed — the `project-manager`
owns where they land. LCV-129 is now `Ready` (refined at `4c0b46f`) and leads,
because it is the only known defect that can wedge the application in a real
user's hands — an unresponsive endpoint hangs the turn forever, latches
`agent_busy`, and repaints every frame for the rest of the session (LCV-120
reopening through a different door), with no recovery but quitting. The user is
about to run their first live prompt against OpenRouter. It is still **not**
part of Marco 2, and its `Depends on` now names LCV-125 as well at file level:
both edit `src/agent/panel.rs` and LCV-125 lands first.

The two new ones, opened 2026-09-13 from the LCV-124 review:

- **LCV-130** (`348fd0c`, `303a3d1`, `dbad29f`) — the suite's isolation from
  the network holds only while no proxy is configured. LCV-124 fixed its own
  fixture at `a1b37aa`; what that did not reach is the rest of the suite. This
  is the class of defect that leaks a credential from a green test run, so it
  deserves a hard look at where it sits in the order.
- **LCV-131** (`0c24170`) — with a key configured, typing a full command name
  such as `line` or `circle` is unrecognised by the CAD grammar and is sent to
  the model, at cost. Suggested agent is **`architect`**, not an implementer:
  part of it amends ADR 0003 §A2's frozen alias set.
- **LCV-132** (`1f3cb27`) — a rendering criterion is paid for in painted text,
  not in a source scan. It promotes the paint-list seam LCV-125's rework built
  into `tests/harness/`: `Context::run` returns `FullOutput { shapes, .. }`, and
  `Shape::Text(TextShape { pos, galley, .. })` yields the exact painted string
  and where it landed. No new dependency and **no `egui` upgrade** — it works at
  the pinned 0.29.1. This repo has shipped six un-failable source scans; this is
  strictly stronger and no more expensive. It has a customer before refinement:
  `39a6d6b` amended LCV-129 to name the harness as the conditional test for its
  Cancel button. Depends on LCV-125, which is `Done`.

## Rejected

| ID | Title | Reason |
|---|---|---|
| LCV-054 | OffsetTool — Offset a Line or Arc by Distance | Removed by LCV-106 (commit adbc878): OFFSET is an explicit non-goal (`docs/product/README.md`); the id was originally planned for "snap integration into all drawing tools," a capability already delivered by LCV-041. |

## Superseded

| ID | Title | Superseded by |
|---|---|---|

_None._
