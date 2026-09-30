# Changelog

All notable changes to LaserCAD v2 are documented in this file. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

This is the v2 (green-field, pure Rust + egui) line of LaserCAD R14. The v1 line (TypeScript + Tauri) is maintained in a separate directory and its history is recorded in its own CHANGELOG.

v0.2.0 is the first tagged release of LaserCAD v2; nothing was tagged before it (0.1.0 was bumped in `Cargo.toml` but never tagged).

## [Unreleased]

### Added

- Layers: every entity belongs to a named layer with its own color and Output flag; new entities go on the current layer and the canvas strokes each entity in its layer's color. `Format > Layers…` (or `LAYER` / `LA`) adds, renames, recolors, deletes and makes layers current and moves the selection onto a layer; the status bar shows the current layer in a dropdown. Every change is one undo step. The mother SVG stores layers as `<g data-layer>` groups and reads them back, accepting any CSS stroke color (named, hex, `rgb()`, `hsl()`, `style`, inherited from a parent group). `File > Export layers` writes one LaserGRBL file per layer with Output on beside the saved drawing. The agent's drawing tools take a `layer` argument and `query_entities` lists the layers. See LCV-156.
- `COPY` (`CO`, `CP`): pick a base point, then place as many translated copies of the selection as you like, typed (`@dx,dy`) or picked; each copy stays on its source's layer and is its own undo step. Enter or Escape ends the run. The agent gets a matching `copy_entity` tool. See LCV-157.
- Polar input: `@50<30` places a point 50 mm from the last point at 30° (counter-clockwise from +X); `50<30` measures from the origin. `DIST` (`DI`) reports the distance, angle, ΔX and ΔY between two points on the command line without changing the drawing. See LCV-159.
- `ROTATE` (`RO`): pick a base point, then type an angle in degrees (counter-clockwise positive) or pick a point to rotate the selection by the angle from the base point to it; the preview follows the cursor. Each entity keeps its layer, the selection stays, and the rotation is one undo step. The agent gets a matching `rotate_entity` tool, in degrees. See LCV-158.
- `MIRROR` (`MI`): pick two points of a mirror line, then answer `Erase source objects? [Yes/No] <N>`: No (or Enter) adds the mirrored copies on their source layers, Yes replaces the sources. The preview follows the cursor, arcs stay single arcs, and the mirror is one undo step. The agent gets a matching `mirror_entity` tool. See LCV-181.
- `SCALE` (`SC`): pick a base point, then type a factor or pick a point whose distance from the base point is the factor; positions and radii scale, arc angles stay. The preview follows the cursor. A zero or negative factor is refused, a factor of 1 changes nothing, and the scale is one undo step that keeps layers and the selection. The agent gets a matching `scale_entity` tool. See LCV-182.

### Fixed

- COPY, ROTATE, MIRROR, SCALE and DIST are now on the tool rail and in the Tools menu, not only on the command line.
- Undoing past the start of a MOVE or COPY run and then picking the next point no longer crashes; the command goes back to its base-point prompt.
- COPY places multi-entity copies in source order, so saved and exported files list entities deterministically.
- A drawing whose half-turn arc was rounded on export (chord slightly longer than the diameter) reopens instead of being rejected.

### Removed

- `File > Export preset` and the preset badge in the status bar; layers replace them. See LCV-156.

## [0.2.0] - 2026-09-28

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
- Optional agent harness (Linux): a settings dialog for endpoint / API key / model, a blocking HTTP transport against OpenAI-compatible chat APIs, a tool registry exposing CAD actions to the model, a multi-turn conversation loop with an iteration cap, and a chat side panel. The AI assistant now edits your live drawing: what it draws appears on the canvas as it works, it can read the geometry that is already there before changing it, and an uninterrupted turn undoes in a single Ctrl+Z. If you edit the drawing yourself while it is working, it stops and says so instead of acting on geometry it can no longer see — whatever it had already drawn stays. See LCV-076, LCV-077, LCV-078, LCV-079, LCV-080, LCV-121, LCV-122, LCV-123.
- Packaging: Linux AppImage and `.deb` builds, a tuned release profile (LTO, strip, `panic=abort`), an application icon and `.desktop` entry, a Windows MSI installer, a macOS `.dmg` bundle, and a multi-platform CI pipeline (test / build / package / release jobs for Linux, Windows, macOS). See LCV-085, LCV-086, LCV-087, LCV-088, LCV-090, LCV-091, LCV-092.
- TextTool is now reachable from the toolbar, the `D` shortcut, and the Tools menu; the toolbar gained Rect, Move, Trim, Extend and Text buttons; Help > Agent settings opens the agent configuration window. See LCV-104.
- Test suite: behavioral test `a_middle_drag_keeps_asking_for_frames` added to witness the `response.dragged()` term of `viewport_is_live`, proving that middle-drag pans keep asking for frames and do not stutter at the edge (reached via `egui::Event::PointerGone`, the documented exception where a drag survives the pointer leaving the viewport). A corresponding rule added to the harness documenting the `PointerGone` trap. See LCV-127.
- Documentation sync: `CHANGELOG.md`, `README.md`, `AGENTS.md` and the demand backlog were brought back in line with shipped behavior, closing Marco 0. See LCV-108.
- Repository URL is now set in `Cargo.toml` and a CI status badge added to `README.md`. See LCV-107.
- Command-line parser: a pure kernel module (`src/cmdline/`) for parsing absolute coordinates (`X,Y`), relative offsets (`@X,Y`), bare distances, tool aliases, and toggle/zoom commands; includes a 50-entry recall ring and consolidates tool lookup tables into a single `ToolKind` identity map. See LCV-110.
- Command line now drives the drawing tools: typed absolute and relative coordinates and direct distances reach LINE, PLINE, RECT, CIRCLE, ARC and MOVE; each drawing phase shows an AutoCAD-R14-style prompt; pressing an unbound alphanumeric key focuses the command line; arrow keys recall previous commands. See LCV-111.
- TEXT now completes from the keyboard: after placing the insertion point, the command line switches to a raw-input mode that prompts first for the string, then for the height (defaulting to 5 mm if left blank, and accepting a comma as the decimal separator); heights outside 0.1–2000 mm are rejected with a re-prompt; the whole string commits in exactly one undo step. The old `on_text_input` path is removed. See LCV-112.
- File > New, File > Open (including Open Recent) and File > Exit — plus the window close button — now ask for confirmation before discarding unsaved changes, instead of silently throwing the work away. See LCV-113.
- Bed size is now a document property, configurable 1–2000 mm via `File > Bed size…`, written into the SVG header and read back on import; `File > New` and a cold boot seed it from settings. See LCV-114.
- Geometry can now be exported into the LaserGRBL cut (red), mark (blue) or engrave (green) group instead of always landing in cut: pick it from `File > Export preset ▸`, see the active choice in the status bar, and opening a file adopts the preset it was exported with. The choice is session-only and resets to Cut on restart — a sticky preset would mean yesterday's mark job cuts today's part. See LCV-115.
- The status bar now shows always-visible, clickable `SNAP` / `GRID` / `ORTHO` indicators that flip the same flags as F3, F7 and F8, plus an autosave indicator with three states (pending, autosaved, no autosave yet this session); `Help > Keyboard shortcuts…` (F1) opens a dialog listing the real keybindings, with the tool rows generated from the toolbar; and the View menu gained an `Ortho` checkbox, so all three modes now have a mouse path. See LCV-116.
- The command line now reaches the AI assistant: prefix a line with `:` or `/ai` and it is always sent as a prompt, while CAD commands, toggles, coordinates and distances always stay local, and text you are typing into a TEXT entity is never sent. With no API key configured nothing leaves the machine — a prefixed line answers `! Agent unavailable: set the API key in Help > Agent settings`, and anything else answers `Unknown command` as before. A line the CAD grammar does not recognise answers `Unknown command: "…"` locally and makes no network call, whatever is configured (LCV-148): `:` and `/ai` are the only way from the command line to the model. See LCV-124, LCV-148.
- The agent panel is now readable, and its settings are reachable. Every transcript row is marked for what it is — your prompt, each tool call the model made, a refusal, an error, the assistant's answer, and an end-of-turn note that says whether one Ctrl+Z takes the whole turn back or several presses are needed. Deleting an entity renumbers the ones after it, and the row that tells you which indices moved is now something you can pick out of the column instead of grey prose. `Help > Agent settings` reaches the model name and the per-turn step budget, so neither needs hand-editing `settings.json` any more, and the dialog now says plainly — always on screen, not on hover — that the API key is stored in plain text in that file. See LCV-125.
- The F1 keyboard shortcuts dialog now includes a "Command line" section documenting how to reach the command line and recall previous commands: any non-tool character seeds the line, and ArrowUp/ArrowDown walk the recall ring while focused. These were shipped features that the dialog never mentioned, leaving operators to discover them only by reading the source. See LCV-126.
- Every agent HTTP call is now bounded by a timeout (120 s per request, 10 s to connect), so a hung or unresponsive endpoint fails with a readable message instead of blocking the worker forever. The operator can now cancel a running turn by clicking the `Cancel` button that appears in the agent panel while a turn is in flight; cancelling still folds the partial turn into a single undo entry, and any actions the agent had already applied stay on the drawing. See LCV-129.
- The F1 keyboard shortcuts dialog now displays all eight shortcut groups without scrolling: the layout has been split into two columns so that every group is visible at once on any screen from 800×600 and up. Previously only three groups fit on screen, with the other five cut off below the fold and no way to reach them by resizing or maximizing the window. See LCV-133.
- The F1 keyboard-shortcuts dialog is now sized to the screen instead of egui's baked 420pt cap, and its headroom is asserted in row pitches: the deepest column must clear the body clip by a full 21.00pt row, every painted run must sit inside the body clip, and the window must stay on screen, at 1280×800, 1024×600 and 800×600. Today's content goes from 8.00pt of slack to 145.32pt and 45.32pt respectively. The dialog moved to its own file (`src/ui/shortcuts_dialog.rs`) in a behaviour-inert first commit. The `ScrollArea` was kept as the safety net; the sizing call sits on top of LCV-133's two columns rather than replacing them. On 600-high screens the body clip bottom lands exactly on the screen edge, so the dialog now asserts the window rectangle against the screen rectangle as well as the painted runs. See LCV-134.
- Command line accepts the full AutoCAD command name for every aliased tool, not just the single-letter shortcut: `line`, `polyline`/`pline`, `rect`/`rectangle`, `circle`, `arc`, `select`, `trim`, `extend`, `move`, `delete`/`del`/`erase` (`text` already worked). See LCV-131.
- The native window title now names the current drawing, live: the file's basename (never the full path), prefixed with `*` while there are unsaved changes, e.g. `*drawing.svg - LaserCAD v2` or `Untitled.svg - LaserCAD v2` before any file is chosen. A document restored from the crash-safety autosave now shows a "Recovered (not saved)" label in the status bar, with hover text explaining the file on disk is untouched until you save; the label clears once you save or replace the drawing via New/Open(/Recent). File > Open Recent entries that share a file name in different folders now show enough of the parent path to tell them apart, and every entry's hover text shows its full path. See LCV-138.
- The command line is now a two-row dock: a bounded context row shows the prompt and feedback (long text is elided on screen, with the full text available on hover, while the stored text stays byte-exact), and a wide editor sits on its own row beneath it. The context row also carries a live "Destination:" label that shows exactly where Enter will send the current line before you press it: `CAD`, `AI`, `tool input`, `AI unavailable`, `AI prompt empty`, or `AI busy`. Checking or editing the line never sends anything or touches the drawing — the label is a preview, not an action. See LCV-139.
- Hover hints were added on every toolbar button, the SNAP/GRID/ORTHO mode indicators, the agent toggle and the preset badge, each naming its keyboard shortcut where one exists (or the preset's export semantics). See LCV-140.
- The agent's system prompt is now shown and editable in `Help > Agent settings`, with a Restore default button that brings back the built-in text. An edited prompt, even a blank one, is used exactly as written from the next agent turn on; tools, the step budget and the drawing-change fence stay enforced whatever the prompt says. See LCV-143.
- The agent can draw many lines, circles and arcs in one `create_drawing` call (up to 1000 entities): the whole batch is validated first, so either every entity is drawn or none is, and it counts as one step and undoes with the rest of the turn. Any tool call whose arguments exceed 1 MiB is refused. See LCV-144.
- The agent can look at the drawing with vision-capable models: two `Help > Agent settings` checkboxes, Allow canvas capture and Model supports images, both off by default. When both are on, the agent may send a grayscale picture of the drawing — bed outline and entities only, never the window, grid, selection or UI — to the configured provider and model, and each upload is noted in the chat. See LCV-145.
- The agent now remembers the conversation for the rest of the session: each turn sees the earlier prompts, its own replies and tool results, and how earlier turns ended (stopped or cancelled). When the drawing changed since its last turn it is told to re-read entity indices. A new `Help > Agent settings` field, Context tokens (default 128000, range 8000–2000000), bounds that memory: past half of it, old tool results are shortened first, then the oldest turns are dropped. Memory is never saved to disk. See LCV-153.
- A New Conversation button in the agent panel header clears the chat transcript and the agent's memory, so the next prompt starts fresh. It is disabled while a turn is running and never touches the drawing or undo history. See LCV-150.

### Changed

- Settings now load at startup: the recent-files list and the agent configuration persist across restarts. The default agent endpoint is now OpenRouter. See LCV-101.
- Window title changed from "LaserCAD v2 — bootstrap" to "LaserCAD v2". See LCV-105 (contract originally frozen by LCV-007).
- Edit > Select All is now undoable: it goes through the same `SelectionCommand` / history path as every other selection change, instead of bypassing it. See LCV-105.
- CI matrix: the `test` and `build` jobs now run Ubuntu alone on ordinary pushes and pull requests, and run all three platforms (Ubuntu, Windows, macOS) on tag pushes and manual `workflow_dispatch`. The `package` and `release` jobs remain gated on tags only. `workflow_dispatch:` was added to the trigger list so the full matrix can be exercised on demand without cutting a release. See LCV-135.
- The agent side panel is now hard-capped at one third of the application window's width, recomputed every frame so the ceiling holds after a drag, after the window shrinks, or on the very first frame — previously it could be resized to consume most of the drawing surface. Agent Settings gained a "Done" button (using the same close-and-persist path as `×`) and a one-sentence note that changes apply immediately and persist on close. See LCV-141.
- The left tool rail is now capped at 120pt and is no longer resizable; it scrolls instead of clipping when the window is too short for all eleven tools, and the Agent toggle stays reachable inside the scrolling area. The coordinate readout in the status bar now shows an explicit `mm` unit for both axes. See LCV-140.
- The agent may now take up to 256 steps per turn by default (range 1–4096, set in `Help > Agent settings`); a budget already saved in `settings.json` is kept as it is and can be raised there. However many steps a turn takes, one Ctrl+Z undoes all of it; if you edit the drawing mid-turn, the agent stops and the work it already applied stays as one undo entry beneath yours. A malformed tool call is now refused and the turn continues instead of failing, and the busy row shows `Thinking… n of limit steps`. See LCV-142.
- The built-in agent prompt now describes every tool with its real argument names, zero-based entity indices, which command-line input reaches the agent, the drawing-change fence and the step budget. See LCV-151.

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
- Exported jobs no longer land vertically offset on any machine whose bed is not 400 mm tall: the SVG export mirror axis was hardcoded to 400 regardless of the real bed, so a 300 × 180 K40 (or any other size) cut in the wrong place even though the drawing looked correct on screen. The mirror now uses the document's actual bed height. See LCV-114.
- `cargo test` no longer destroys developer state: real user paths are now resolved once at boot and injected, so the test suite can never reach the crash-recovery autosave file or the settings file. Previously an ordinary green library test deleted `~/.local/share/lasercad/autosave.json` on every run. See LCV-119.
- The canvas now only asks for a new frame while it is actually live — the pointer is over it, a drag is in progress, or a tool preview is on screen — instead of requesting a repaint on every frame forever. On a compositor that throttles an unchanging window the wasted requests were already being declined (measured idle cost: 0.0% of a core), but on a platform that does not throttle, or on battery, the app no longer pays for frames it does not need. See LCV-120.
- Closing the window with unsaved changes and choosing Discard now actually closes the app; it used to reopen the confirmation dialog forever, because eframe re-delivers the native Close event on a later frame after a confirmed Exit had already been handled. The New and Open discard paths were verified working and are now covered by pointer-driven tests (no behaviour change there). See LCV-136.
- The agent panel's Send button no longer overflows the panel edge, and the busy "Thinking… / Cancel" row no longer overlaps the transcript: the transcript's reserved height now accounts for whether the busy row will render this frame instead of assuming it never does. Agent Settings now scrolls instead of clipping its fields on small windows. See LCV-141.
- The grid is visible again: it used to be painted before the bed, so the bed's own opaque fill painted over every grid line. Grid lines now reach every edge of the canvas even when the canvas does not start at the window's top-left corner (toolbar and side panels claim space first). Mouse-wheel zoom now keeps the point under the cursor fixed instead of drifting. Middle-drag pan now moves the drawing with the cursor vertically as well as horizontally — the Y direction was inverted. See LCV-137.

### Removed

- OffsetTool was deleted: it is an explicit product non-goal (see `docs/product/README.md`) that had shipped by accident under the LCV-054 id. See LCV-106 (LCV-054 is rejected).

### Internal

- The test suite now enforces the architecture rules in `AGENTS.md`: the kernel purity rule, the agent module buckets, and the single-writer claims on agent state are checked by the build. If these rules drift as the code changes, the build fails. This ships no user-visible change, but it is a guard against silent architecture violations that had already gone unnoticed twice. See LCV-128.
