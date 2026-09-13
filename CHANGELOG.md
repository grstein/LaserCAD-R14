# Changelog

All notable changes to LaserCAD v2 are documented in this file. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

This is the v2 (green-field, pure Rust + egui) line of LaserCAD R14. The v1 line (TypeScript + Tauri) is maintained in a separate directory and its history is recorded in its own CHANGELOG.

No version of LaserCAD v2 has been tagged or released yet. Everything below is unreleased.

## [Unreleased]

### Added

- Initial scaffold: Cargo project skeleton, agent harness under `.claude/agents/` (six agents), `AGENTS.md`, `CLAUDE.md`, `PLAN.md`, `README.md`, dual MIT/Apache license, empty module tree, ADR 0001 recording the pure-Rust + egui decision, CI workflow scaffold (Linux), bootstrap egui window placeholder. See LCV-001, LCV-002, LCV-003, LCV-004, LCV-005, LCV-006.
- App shell upgraded to a real CentralPanel viewport (dark canvas placeholder). See LCV-030.
- Native bootstrap window opens at 1280×800. Its initial title, "LaserCAD v2 — bootstrap", was later renamed to "LaserCAD v2" — see the Changed entry below. See LCV-007.
- Geometry kernel introduces Vec2 and EPSILON. See LCV-010.
- Geometry kernel adds Line primitive (bbox, closest point, distance helpers). See LCV-011.
- Geometry kernel adds Circle primitive (bbox, point-at-angle, signed distance, containment). See LCV-012.
- Geometry kernel adds Arc primitive (wrap-aware bbox, contains_angle, sweep). See LCV-013.
- Geometry kernel adds line-line / line-circle / circle-circle intersection routines. See LCV-014.
- Geometry kernel adds Rect with contains/crosses predicates for line, circle, arc. See LCV-015.
- Geometry kernel adds snap engine (endpoint/midpoint/center/intersection kinds). See LCV-016.
- Geometry kernel: cross-module integration tests added (5 scenarios in tests/geometry.rs). See LCV-017.
- Document model gains canonical Entity enum and SCHEMA_VERSION=1. See LCV-020.
- Document model adds Document struct with bounds, Default, and Selection placeholder. See LCV-021.
- Document model adds Command trait with do_/undo round-trip contract. See LCV-022.
- Document model adds CreateLine/Circle/Arc commands with captured-index undo. See LCV-023.
- Document model adds DeleteEntities + MoveEntities commands and Entity::translate. See LCV-024.
- Document model adds TrimEntity + ExtendEntity commands (Line/Circle pairs; Arc targets deferred). See LCV-025.
- Document model adds 200-deep undo/redo history stack. See LCV-026.
- Document model concretizes Selection (HashSet-backed) and adds SelectionCommand. See LCV-027.
- Camera (world↔screen transform, zoom, pan, zoom-extents) and viewport pointer-input wiring connect the render surface to mouse and wheel input. See LCV-031, LCV-032.
- Grid renderer: responsive 1-2-5 decade ladder grid (minor spacing adapts across zoom range; major lines every 10 minors). See LCV-033.
- Bed renderer: 400×400 mm work-area rectangle with dark outside overlay (LCV-034).
- Entity painter: draws lines, circles, and arcs from the Document on the viewport (LCV-035).
- Selection highlight: selected entities render with a cyan-blue halo (LCV-036).
- Preview overlay: in-progress tool geometry renders as translucent amber (LCV-037).
- Snap markers: orange endpoint/midpoint/center/intersection indicators at snap point (LCV-038).
- Tool framework: Tool trait, ToolManager, SelectTool stub, App::commit (LCV-040).
- Full drawing and modify toolkit, AutoCAD-R14-shaped: Select (point pick, window/crossing box, shift-toggle), Line, Polyline, Rect, Circle, Arc, Text (Hershey stroke font, click-to-place), Move, Trim, Extend, Delete/Erase, and an F8 ortho lock. See LCV-041, LCV-042, LCV-043, LCV-044, LCV-045, LCV-046, LCV-047, LCV-048, LCV-049, LCV-050, LCV-051, LCV-052, LCV-053.
- Hershey stroke font data and text layout engine, backing the Text tool. See LCV-055.
- SVG export with cut/mark/engrave color presets, LaserGRBL-compatible output (`fill="none"`, one `<g>` per preset colour, arcs as `A` path commands). See LCV-056.
- SVG import (via `roxmltree`) reads the same strict subset back in, so exported files round-trip. See LCV-057.
- Settings persisted to a JSON file (via the `directories` crate); a recent-files list; native file open/save dialogs (`rfd`); New / Open / Save / Save As / Exit actions wired to SVG I/O. See LCV-058, LCV-060, LCV-061, LCV-062.
- Autosave with restore-on-boot. See LCV-059.
- Full UI chrome: menubar (File / Edit / View / Tools / Help), a left-side toolbar with tool buttons, a status bar (coordinates, active tool, entity count), a bottom command-line widget, modal dialogs (confirm / error / about), keyboard shortcuts for tools and view toggles, and a dark CAD theme. See LCV-065, LCV-066, LCV-067, LCV-068, LCV-069, LCV-070, LCV-071.
- Undo/Redo keyboard shortcuts (Ctrl+Z / Ctrl+Y) with a status-bar flash on trigger. See LCV-075.
- Optional agent harness (Linux): a settings dialog for endpoint / API key / model, a blocking HTTP transport against OpenAI-compatible chat APIs, a tool registry exposing CAD actions to the model, a multi-turn conversation loop with an iteration cap, and a chat side panel. The agent can read and narrate the drawing but does not yet modify it end-to-end from chat. See LCV-076, LCV-077, LCV-078, LCV-079, LCV-080.
- Packaging: Linux AppImage and `.deb` builds, a tuned release profile (LTO, strip, `panic=abort`), an application icon and `.desktop` entry, a Windows MSI installer, a macOS `.dmg` bundle, and a multi-platform CI pipeline (test / build / package / release jobs for Linux, Windows, macOS). See LCV-085, LCV-086, LCV-087, LCV-088, LCV-090, LCV-091, LCV-092.
- TextTool is now reachable from the toolbar, the `D` shortcut, and the Tools menu; the toolbar gained Rect, Move, Trim, Extend and Text buttons; Help > Agent settings opens the agent configuration window. See LCV-104.
- Documentation sync: `CHANGELOG.md`, `README.md`, `AGENTS.md` and the demand backlog were brought back in line with shipped behavior, closing Marco 0. See LCV-108.
- Repository URL is now set in `Cargo.toml` and a CI status badge added to `README.md`. See LCV-107.
- Command-line parser: a pure kernel module (`src/cmdline/`) for parsing absolute coordinates (`X,Y`), relative offsets (`@X,Y`), bare distances, tool aliases, and toggle/zoom commands; includes a 50-entry recall ring and consolidates tool lookup tables into a single `ToolKind` identity map. See LCV-110.
- Command line now drives the drawing tools: typed absolute and relative coordinates and direct distances reach LINE, PLINE, RECT, CIRCLE, ARC and MOVE; each drawing phase shows an AutoCAD-R14-style prompt; pressing an unbound alphanumeric key focuses the command line; arrow keys recall previous commands. See LCV-111.
- TEXT now completes from the keyboard: after placing the insertion point, the command line switches to a raw-input mode that prompts first for the string, then for the height (defaulting to 5 mm if left blank, and accepting a comma as the decimal separator); heights outside 0.1–2000 mm are rejected with a re-prompt; the whole string commits in exactly one undo step. The old `on_text_input` path is removed. See LCV-112.
- File > New, File > Open (including Open Recent) and File > Exit — plus the window close button — now ask for confirmation before discarding unsaved changes, instead of silently throwing the work away. See LCV-113.

### Changed

- Settings now load at startup: the recent-files list and the agent configuration persist across restarts. The default agent endpoint is now OpenRouter. See LCV-101.
- Window title changed from "LaserCAD v2 — bootstrap" to "LaserCAD v2". See LCV-105 (contract originally frozen by LCV-007).
- Edit > Select All is now undoable: it goes through the same `SelectionCommand` / history path as every other selection change, instead of bypassing it. See LCV-105.

### Fixed

- SVG export and import now flip the Y axis, so files exported from LaserCAD open right-way-up in LaserGRBL and Inkscape; the canvas matches the 400×400 mm bed. See LCV-100.
- Settings that fail to parse are backed up to `settings.json.bak` instead of being silently discarded. See LCV-101.
- Autosave now actually fires: an edit debounces for 800 ms against a document revision counter and then writes to disk, instead of never triggering. See LCV-102.
- Keyboard input now goes through a single focus-gated dispatcher: F8 and Ctrl+Z each act once per key press instead of double-firing, and typing into a text field no longer also zooms the viewport or deletes the selection. Enter now reaches the active tool, so TEXT and Polyline can be finished from the keyboard. See LCV-103.
- Escape while a tool is idle no longer pushes a no-op entry onto the undo history. See LCV-105.
- CI's Windows package job now invokes `build-msi.ps1` through PowerShell, with Rust and `cargo-wix` installed first. Previously the script could not run. See LCV-107.
- Windows CI format gate no longer fails: added `.gitattributes` with `* text=auto eol=lf` so that even on a Windows runner with `newline_style = "Unix"` in `rustfmt.toml`, the checkout preserves Unix line endings. See LCV-107.
- macOS CI no longer hangs on a retired runner: moved from `macos-13` (retired 2025-12-04) to `macos-15` (active, arm64). See LCV-107.
- macOS packaging now works on Apple Silicon: the DMG output name no longer hardcodes `x86_64`; it is now determined at build time by the host triplet, so `scripts/build-dmg.sh` on arm64 produces `dist/lasercad-aarch64.dmg` as expected. See LCV-107.
- CI matrix now passes `test` and `build` stages end-to-end for the first time: ubuntu-24.04, windows-2022, and macos-15. See LCV-107.
- Text drawn with the TEXT tool no longer sags below its baseline: 22 alphanumeric glyphs and `?` in the Hershey font table were authored with the wrong vertical coordinates, so words containing them sat partly below the line on canvas and in the exported SVG. Already-drawn text is not migrated — re-type it to pick up the repair. See LCV-117.

### Removed

- OffsetTool was deleted: it is an explicit product non-goal (see `docs/product/README.md`) that had shipped by accident under the LCV-054 id. See LCV-106 (LCV-054 is rejected).
