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

- 2026-09-13 — LCV-118 opened, slotted before LCV-114 — the LCV-113 review found a hazard class ADR 0003 covers only half of. The existing ban on sending Ctrl+O/Ctrl+S headless is a property of every test SITE; what actually reaches a native dialog is decided by product LOGIC. Inverting has_unsaved_changes made a correctly-written test fall through to the real action_open(), reach rfd and HANG the test binary — a CI timeout with no output on three platforms instead of a named failure. Architect decided it in ADR 0005 (f44eeb3): an AtomicBool at the rfd boundary, armed only by crate::run(), panicking when disarmed. They corrected my brief on the decisive point: a cfg(test) guard cannot work, because integration tests link the library with cfg(test) OFF, so the arm would be missing from exactly the binary that hung. Does not block Marco 1.
- 2026-09-13 — ADR 0004 accepted (fa56438) — AGENTS.md's "300 LOC per .rs" contradicted ADR 0002's implementation-line rule and produced a false blocking review finding against src/text/hershey.rs and layout.rs, which are 53 and 87 implementation lines against 476 and 380 total. AGENTS.md lines 62 and 155 now carry the counting rule, an awk recipe, and the two real exemptions. Architect also pre-decided the seam for src/app/file_ops.rs (281/300) along the egui boundary, to be executed by whichever demand crosses 300 rather than speculatively now.
- 2026-09-13 — LCV-113 In Progress — confirm-discard shipped at 475d666 + a7df030; saved_revision is a second mark independent of the autosave debounce, and the window X is intercepted with ViewportCommand::CancelClose. 729 → 746 lib, 95 → 100 integration, CI green (run 34738711513). Review returned REWORK REQUIRED on one finding: the demand's central invariant, that autosave must never mark the document safe to discard, has no automated guard. The reviewer made flush_if_due write saved_revision and all 746 tests stayed green. Six other mutations bit correctly, including removing the CancelClose send.
- 2026-09-13 — LCV-117 Done — 23 Hershey glyph entries re-authored so every alphanumeric sits on the baseline; descenders normalised to exactly +4; whole-table vertical-metrics invariant plus three anti-restyle guards. Shipped d696f7c + c8cebad, reworked 5d48027 after the reviewer showed the Q tail assertion scanned every stroke and so could not fail, since Q's bowl is byte-identical to O's. 718 → 729 lib tests. CI green (run 34737781075). APPROVED.

- 2026-09-13 — LCV-117 In Progress — Hershey repair shipped at d696f7c + c8cebad (23 glyph entries re-authored, whole-table vertical-metrics invariant, ASCII-art visual instrument kept as an #[ignore]d test), 718 → 729 lib tests, CI green (run 34736564259). Review returned REWORK REQUIRED on one real finding: the AC 14 test scanned every stroke of Q for a lower-right point, but Q's bowl is byte-identical to O's and always supplies one, so a truncated tail passed the whole suite. The reviewer's second finding — that hershey.rs (476 lines) and layout.rs (380) breach the 300-line cap — is VOID; ADR 0002:371-375 counts implementation lines only, making them 53 and 87. No file split ordered.
- 2026-09-13 — LCV-117 opened, slotted ahead of LCV-113 — the LCV-112 review surfaced a defect that predates it: 22 alphanumeric glyphs plus '?' were authored with the origin at the vertical centre of the glyph body, so they render below the baseline. A 10 mm 'O' came out 20 mm tall with half of it under the line, in every exported SVG and therefore in LaserGRBL. Shipped with LCV-055; survived because all four vertical-metrics tests use 'H', one of the 40 glyphs that was never broken. My own defect report was wrong in two places, both caught by product-owner: an affine transform DOES repair 'O' (I missed a data point), and my proposed x-height invariant would have failed on seven correct glyphs. Refined bb37da6.
- 2026-09-12 — LCV-112 Done — TEXT prompts for the string then the height in raw-input mode, one undo step; on_text_input deleted. Shipped 2ef12af, reworked 307cfad. The blocking finding was a coverage gap rather than a wrong behaviour: the focus release on commit was correct but untested, and reverting it left all 39 tests green. Re-review APPROVED after a surgical mutation proved the rewritten lcv103 test catches the real Enter double-dispatch and not a buffer-corruption side effect. 704 → 718 lib tests. CI green (run 34733893629).
- 2026-09-12 — LCV-111 Done — the command line now drives LINE, PLINE, RECT, CIRCLE, ARC and MOVE through on_command_input; per-phase R14 prompts; an unbound alphanumeric focuses the field; arrow keys recall. Shipped 533e24a + 2f39e9d, reworked d6a23b7. The blocking finding was spec drift: AC 9.4 predated the LCV-110 rework that made push() reset the recall cursor on a blank entry, so the call-site guard made already-approved behaviour unreachable. 661 → 704 lib tests, 68 → 86 integration. APPROVED after rework.

- 2026-09-12 — LCV-110 In Progress — parser shipped at 66af950 and CI-green, but review returned REWORK REQUIRED: CommandHistory::push must reset the recall cursor even on a blank entry, matching v1's unconditional historyIndex reset. Fix routed back to the implementer.
- 2026-09-12 — LCV-110 Ready → shipped 66af950 — pure src/cmdline/ parser kernel (X,Y / @X,Y / distance / aliases / toggles / zoom), 50-entry recall ring, ToolKind + tools::make as the single tool-identity map. Retired BOTH string-keyed maps: ui/shortcuts.rs::TOOL_KEYS and ui/toolbar.rs::make_tool. 632 → 660 lib tests.
- 2026-09-12 — Marco 1 opened — LCV-110..116 created and refined to Ready by two product-owner agents (27f2a85, 36b0056); ADR 0003 accepted (ac9e796) fixing the command-line input contract that LCV-110..112 hang on.
- 2026-09-12 — LCV-107 continued and closed — first-ever CI matrix run exposed two real breaks: the Windows fmt gate rejected every file because rustfmt.toml sets newline_style=Unix with no .gitattributes to stop the CRLF checkout, and macos-13 is a dead runner label (retired 2025-12-04) so that job queued forever. Moved to macos-15, which is arm64, which in turn exposed a latent packaging break: the workflow hardcoded lasercad-x86_64.dmg while build-dmg.sh emits lasercad-aarch64.dmg on Apple Silicon. That would have failed on the v0.1.0 tag push. Commits 958fb12, 4244d40, 0a1d680. CI is now GREEN on ubuntu-24.04, windows-2022 and macos-15 for both test and build (run 34727477158).
- 2026-09-12 — Remote created by the user — origin = https://github.com/grstein/LaserCAD-R14-V2 (private). Agents may now push to main; tags remain the user's alone.
- 2026-09-12 — Marco 0 complete — LCV-100..108 Done, LCV-054 Rejected; gate green (632 lib + 67 integration + 5 doc tests); awaiting the user's manual smoke test before tag v0.1.0.
- 2026-09-12 — LCV-108 Done — CHANGELOG, README, AGENTS.md and every demand Status line synced to reality; the fictional [0.1.0] - 2025-06-15 section folded back into [Unreleased].
- 2026-09-12 — LCV-105 Done — src/app.rs split into src/app/ submodules, all under the 300-line cap; orphans and MODULE placeholders retired; Select All is undoable; window title is now LaserCAD v2.
- 2026-09-12 — LCV-107 Done — CI Windows package job invokes build-msi.ps1 through PowerShell and installs Rust and cargo-wix. No remote created and nothing pushed; that decision is the user's.
- 2026-09-12 — LCV-106 Done, LCV-054 Rejected — OffsetTool deleted; it was an explicit product non-goal shipped by accident under the LCV-054 id.
- 2026-09-12 — LCV-104 Done — TextTool reachable at last: toolbar button, D shortcut, Tools menu; toolbar completed; Help > Agent settings opens the agent window.
- 2026-09-12 — LCV-102 Done — autosave actually fires, driven by a History revision counter with an 800 ms debounce; proven to reach disk in an isolated probe.
- 2026-09-12 — LCV-103 Done — keyboard input routed through one focus-gated dispatcher; F8 and Ctrl+Z act once per press; Enter reaches the active tool.
- 2026-09-12 — LCV-101 Done — settings load at startup; agent endpoint defaults to OpenRouter; corrupt settings are backed up rather than lost.
- 2026-09-12 — LCV-100 Done — SVG export and import flip Y, so exported files open the right way up in LaserGRBL and Inkscape. The worst defect in the project.
- 2026-09-12 — ADR 0002 accepted — headless input regression tests via App::update_ui, and autosave dirty tracking on a History revision counter.
- 2026-09-12 — Marco 0 opened — backlog reconciled against git history (LCV-010..092 closed as shipped, LCV-089 Blocked); LCV-100..108 created and refined to Ready.
- 2026-05-18 — LCV-040 Done — Tool trait + ToolManager shipped, Phase 4 open.
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
