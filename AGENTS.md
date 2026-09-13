# LaserCAD v2 — Agent Guide

This file is the single source of agent instructions for the repository. Every coding agent — Claude Code, the project-manager, the workers — reads this file. `CLAUDE.md` is a thin pointer at this document.

LaserCAD v2 is a 2D micro-CAD for laser cutting, compatible with LaserGRBL. It is a **pure-Rust, single-binary, egui-based** green-field rewrite of LaserCAD R14 v1 (which was TypeScript + Tauri). The product bias is strict: KISS clone of AutoCAD R14, preserve precision, avoid UI or architecture growth unless it directly improves the core CAD-to-LaserGRBL workflow.

Primary platform: **Linux**. Windows and macOS are long-term.

## Language Convention

All versioned artifacts (source, comments, documentation, commit messages, file names) are written in English. Conversation with the user can happen in any language; the repository itself remains English-only.

## Commit Conventions

- **Never** add `Co-Authored-By: Claude …` (or any AI-coauthor) trailer to commits in this repository. The human author is the sole author of record. This applies to every agent — project-manager, implementer, demand-manager, or any future automation.
- Conventional commits: `feat(LCV-NNN): …`, `fix(LCV-NNN): …`, `refactor(LCV-NNN): …`, `test(LCV-NNN): …`, `docs(LCV-NNN): …`, `chore: …`.
- Prefer creating a new commit over amending unless the user explicitly asks.
- Never bypass hooks (`--no-verify`) or signing without explicit user authorization.

## Commands

```bash
cargo run                     # debug build, opens the app
cargo run --release           # release build, opens the app
cargo build                   # debug compile only
cargo build --release         # release artifact in target/release/lasercad
cargo test --all              # all unit + integration tests
cargo fmt --all               # apply formatting
cargo fmt --all -- --check    # CI-style check
cargo clippy --all-targets -- -D warnings
cargo doc --open              # render and open API docs
```

Run a single test: `cargo test -p lasercad --lib geometry::vec2` (or pass a substring after `cargo test`).

Native prerequisites are documented in `docs/build-local.md` once Phase 0 lands. For now: stable Rust ≥ 1.88 (pinned via `rust-toolchain.toml`; required by transitive deps `image` and `idna_adapter`).

## Architecture

### Module tree

```
src/
├── main.rs                 # thin: parse argv, init tracing, run app
├── app/                    # eframe::App impl, top-level state, wiring (mod.rs, input.rs, viewport.rs, panels.rs, autosave.rs, ortho.rs, snap.rs, agent_poll.rs)
├── lib.rs                  # re-exports for tests
├── geometry/               # pure kernel: vec2, line, circle, arc, intersect, snap, rect, epsilon
├── cmdline/                # pure kernel: command-line grammar (parse, parse_number), ToolKind/ToggleKind/ZoomKind, recall ring
├── document/               # entity model, schema, commands, history, selection
├── render/                 # camera, viewport, grid, bed, entities, preview, snaps
├── tools/                  # Tool trait, ToolManager, one file per tool
├── io/                     # settings, autosave, recent, file dialogs (rfd wrapper)
├── io/svg/                 # export, import (roxmltree)
├── ui/                     # menubar, toolbar, statusbar, command_line, dialogs, shortcuts, theme
├── agent/                  # transport (reqwest::blocking), tool registry, multi-turn loop, settings_ui, panel (chat UI)
├── text/                   # Hershey font, layout
└── util/                   # units (mm bed constants, world↔SVG Y flip)
```

Hard rules on the tree:

- **One responsibility per file.** Hard cap: **300 _implementation_ LOC per `.rs`**. Implementation LOC = total lines **minus** the top-level `#[cfg(test)] mod tests` block and everything after it. Inline tests are free; do not ration them. Measure it, never `wc -l` it:

  ```bash
  awk '/^#\[cfg\(test\)\]/{print NR-1; f=1; exit} END{if(!f) print NR}' <file>
  ```

  The anchor is a **bare `#[cfg(test)]` at column 0**. A pattern that matches `#[cfg(test)]` anywhere on a line also matches doc-comment prose and gives a wrong count (it scores `src/app/mod.rs` at 153; the real number is 287). Witnesses: `src/text/hershey.rs` → 53, `src/text/layout.rs` → 87, `src/app/file_ops.rs` → 281.

  Exempt: a file that is **only** a `const`/`static` data table with no `fn` (`src/text/hershey_data.rs`), and a file that is **only** test code, declared from its parent as `#[cfg(test)] mod tests;` (`src/geometry/snap/tests.rs`). No other exemption exists.

  If you'd cross the cap, split (or route to `architect`). Between 270 and 300, do **not** split on sight — flag it to `architect`, who records the seam for the demand that eventually crosses. Full rule: [ADR 0004](docs/adr/0004-measuring-the-300-loc-cap.md), extending [ADR 0002](docs/adr/0002-headless-input-tests-and-dirty-tracking.md) §"The 300-LOC cap and `src/app.rs`".
- **Each `mod.rs` re-exports its module's public surface.** No deep-path imports from outside the module.

### Purity rule

The following modules MUST NOT import `egui`, `eframe`, or `rfd`:

- `src/geometry/*`
- `src/document/*`
- `src/io/svg/*`
- `src/text/*`
- `src/cmdline/*`

This is the "kernel". It must remain testable as a pure Rust library and runnable in a future headless / CLI / WASM context. Reviewers (`reviewer-rust`) check this on every demand.

`src/agent/` is not kernel, but it carries three containment rules of its own ([ADR 0007](docs/adr/0007-agent-turn-mutates-the-live-document.md) §D8):

- **Only `src/agent/panel.rs` and `src/agent/settings_ui.rs` may import `egui`.** No file under `src/agent/` may import `eframe` or `rfd`. Everything else there — `classifier.rs`, `wire.rs`, `transport.rs`, `tools.rs`, `bridge.rs`, `loop_.rs` — is kernel-pure and unit-testable with no UI context.
- **Only `src/agent/transport.rs` may import `reqwest`.** It is the single HTTP boundary of the crate.
- **`panel.rs` renders and reports. It never spawns a thread and never constructs a `Document` or a `History`.** Spawning a turn is app-side wiring and lives in `src/app/agent_turn.rs`.

### Units and types

- **Millimeters are canonical** across document, kernel, command line, and SVG export. Pixels only inside `render/camera`.
- **Angles**: radians in the kernel and state. Degrees only at UI presentation boundaries.
- Canonical types (declared in `src/geometry/mod.rs` and `src/document/entity.rs`):
  - `Vec2 { x: f64, y: f64 }`
  - `Entity` enum: `Line { p1, p2 }`, `Circle { center, r }`, `Arc { center, r, start_angle, end_angle, ccw }`.
  - `Command` trait with `do_` / `undo` semantics in `src/document/commands.rs`.
  - `Tool` trait in `src/tools/tool.rs`.
  - `SnapResult` in `src/geometry/snap.rs`.

### State and mutation (hard contract)

The `App` struct in `src/app/mod.rs` owns mutable state. The contract:

- **All entity mutation goes through `Command` trait + the history stack in `src/document/history.rs`.**
- **Tools never mutate `Document` directly**; they construct a `Box<dyn Command>` and call `App::commit(cmd)`.
- **History depth**: 200 commands (matches v1).
- **Selection** is part of `Document`, mutated via `SelectionCommand`s.
- **Active tool, toggles, command-line input, camera, viewport size, cursor** live on `App`. Setters on `App` are the only mutation path.

### Event flow

egui's `update(&mut self, ctx, frame)` is the single tick. Inside it:

1. Process pending input (handled by tools through pointer events from egui).
2. Apply any committed commands (already done synchronously when the tool calls `commit`).
3. Repaint the viewport (grid, bed, entities, preview, snap markers).
4. Render UI chrome (menubar, toolbar, command line, statusbar, dialogs).

Async work (HTTP for the agent) runs on a plain `std::thread` (`src/agent/panel.rs:141`) that calls a blocking `reqwest::blocking::Client` (`src/agent/transport.rs:94`); this crate does not depend on `tokio`. The result comes back through a `std::sync::mpsc::Receiver` polled once per frame (`src/app/agent_poll.rs`), and `ctx.request_repaint()` is called each frame to keep the UI live while a turn is in flight. The autosave timer is not async at all: it is a debounce check against `History::revision()` run inline in `App::update` (`src/app/autosave.rs`).

**Repaint policy.** Every `ctx.request_repaint*` call is conditional on something actually being live: an in-flight agent turn (`src/app/mod.rs`), a pending autosave write (`autosave::schedule_flush_repaint`), or a hovered / dragged / previewing canvas. An unconditional per-frame repaint is a review blocker — it stops the app ever idling, and it masks the conditional wake-ups underneath it so they read as dead code. One violation is live today, `src/app/viewport.rs:46`, and LCV-120 closes it: do not add a second, and do not delete `schedule_flush_repaint` when it lands, because that is the call that keeps autosave alive once the canvas idles.

### SVG export (LaserGRBL compatibility)

`src/io/svg/export.rs` follows the LaserGRBL export checklist:

- `xmlns` on the root `<svg>`; `width`/`height` are **the document's bed size in mm** (`Document::bed_mm`, seeded from `DEFAULT_BED_WIDTH_MM`/`DEFAULT_BED_HEIGHT_MM` in `src/util/units.rs` and configurable through `File > Bed size…`); `viewBox` is `0 0 <bed_width> <bed_height>` in SVG coordinates (no unit suffix). Import reads the same header back and the opened document adopts it.
- **Y is mirrored on export**: `y_svg = flip_y(y_world, bed_height)` via `crate::util::flip_y`, where `bed_height` is `Document::bed_mm[1]` of the document being written — never a constant — because the world is Y-up and SVG is Y-down. Import applies the same involution in reverse, around the bed height the file itself declares. X, radii and stroke widths are untouched.
- `fill="none"` forced; no live text; no `filter`/`mask`/`clipPath`.
- One `<g>` per preset color: **cut** red `#ff0000`, **mark** blue `#0000ff`, **engrave** green `#00aa00`; `stroke-width="0.1"` mm.
- Arcs as `<path d="M sx sy A r r 0 large sweep ex ey"/>` (not bézier). The mirror reverses handedness, so `sweep` is **inverted** relative to `Arc::ccw` (`sweep = 0` for a CCW world arc) while `large` is unchanged; start and end points keep their roles.

Changing these rules breaks LaserGRBL import — confirm via `product-owner` before doing so.

## Subagent Suite

Six Claude agents live under [`.claude/agents/`](.claude/agents/). The main Claude Code agent (and any human-driven session) **must delegate to them** whenever the work matches their domain — do not do their job inline.

| Subagent | File | Model | When to delegate |
|---|---|---|---|
| `project-manager` | [`.claude/agents/project-manager.md`](.claude/agents/project-manager.md) | opus | Drive `PLAN.md`. Pick the next demand, spawn the right worker, track via tasks, report to user. Sole writer of `PLAN.md` status table. |
| `architect` | [`.claude/agents/architect.md`](.claude/agents/architect.md) | opus | Architecture decisions, ADRs, module-boundary calls, cross-cutting type contracts. Does not edit code. |
| `product-owner` | [`.claude/agents/product-owner.md`](.claude/agents/product-owner.md) | opus | Refine `Draft` demands into `Ready`. Body, scope, acceptance, non-goals. Does not edit code. |
| `demand-manager` | [`.claude/agents/demand-manager.md`](.claude/agents/demand-manager.md) | sonnet | State machine for LCV-XXX. Sole writer of `Status:` lines, `backlog.md` tables, `CHANGELOG.md`. |
| `implementer-rust` | [`.claude/agents/implementer-rust.md`](.claude/agents/implementer-rust.md) | sonnet | Turn one `Ready` demand into shipped, tested Rust code. Sole writer of `src/`, `tests/`, `Cargo.toml`. |
| `reviewer-rust` | [`.claude/agents/reviewer-rust.md`](.claude/agents/reviewer-rust.md) | sonnet | Review freshly-committed demands. KISS + architecture + test-coverage checks. Cannot edit code. |

### Coordination contract

All six subagents coordinate through Claude Code's task system (`TaskCreate` / `TaskList` / `TaskGet` / `TaskUpdate`).

- **Tasks are the bulletin board.** Every cross-agent handoff is a task with `owner: <agent-name>` and enough context that the receiver can pick it up cold.
- **Claim before working.** Set `owner` and flip to `in_progress` before acting; flip to `completed` only when fully done.
- **No silent role crossing.** If you discover work that belongs to another role, create a task for that role and either block on it (`addBlockedBy`) or hand the original task off — never just do it yourself.
- **The demand state machine is single-writer.** Only `demand-manager` edits `Status:` / `Implementation:` lines in demand files and the tables in `docs/product/backlog.md`.
- **The PLAN.md status table is single-writer.** Only `project-manager` edits the demand-status table inside `PLAN.md`.
- **Product scope is single-writer.** Only `product-owner` writes demand bodies.
- **Code is single-writer.** Only `implementer-rust` edits `src/`, `tests/`, `Cargo.toml`.

If you are the main Claude Code agent and the user asks for project work, default to spawning `project-manager` rather than doing the work inline.

## Implementation Rules

- Keep **millimeters canonical** in document, geometry, command line, and SVG export. Radians in the kernel.
- Keep the **kernel pure**: no `egui`/`eframe`/`rfd` imports in `geometry/`, `document/`, `io/svg/`, `text/`, `cmdline/`.
- **All entity mutation through `Command` trait + history stack.** No direct `Document.entities` mutation outside `document::commands` and `document::history`.
- **The agent mutates like everything else.** The background agent thread holds no `Document`, no `History`, no entity snapshot and no `Arc<Mutex<_>>`; it asks the UI thread to apply one `AgentAction` at a time and waits for the real outcome. `Document` stays `!Clone` — adding `Clone` for the agent's benefit is a review blocker. See [ADR 0007](docs/adr/0007-agent-turn-mutates-the-live-document.md).
- **No `unsafe`** without an inline justification and an ADR.
- **`rfd` lives only in `src/io/dialogs.rs`.** Never add an `rfd` call anywhere else — it would sit outside the dialog guard. Per [ADR 0005](docs/adr/0005-native-dialogs-disarmed-by-default.md) the three wrappers are **disarmed until `crate::run()` arms them**, so a dialog reached from a test panics instead of hanging CI; that guard is implemented (LCV-118). ADR 0002 §A4 rule 1 remains the first line of protection: no test may send `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S`. No test ever arms the dialogs.
- **Real user paths are resolved at boot and injected.** `directories::ProjectDirs` belongs in `src/io/settings.rs` and `src/io/autosave.rs` only, called only from `App::new()`, which stores the resolved paths on `App`; `App::default()` — the test constructor — leaves them `None` and persistence is a no-op. See [ADR 0006](docs/adr/0006-real-user-paths-are-injected.md), which also records when to reach for ADR 0005's arm-by-main flag instead (native OS surfaces) and when to inject a path (the filesystem). Since LCV-119 a test drives `action_new` / `action_open_path` / `action_save` with `settings_path` / `autosave_path` pointing at a temporary directory it owns, or leaves them `None` and nothing is written.
- **No `unwrap()` / `expect()`** in library code except where an invariant is documented; tests can unwrap.
- **One responsibility per file**, ≤300 **implementation** LOC — total lines minus the inline `#[cfg(test)] mod tests` block. Measure it with the `awk` recipe in §Module tree; `wc -l` is not the rule and has already produced a false blocking review finding. See [ADR 0004](docs/adr/0004-measuring-the-300-loc-cap.md).
- **Doc comments** on `pub` items; module headers on `mod.rs`.
- **Tests**: `#[cfg(test)] mod tests` next to implementation for unit, `tests/` for integration. Add a test per acceptance criterion.
- **Before declaring a demand done**: run `cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test --all`. All three must be green.

## Product Philosophy

LaserCAD v2 is not a general design tool. It is a focused CAD surface for making simple, precise 2D geometry that exports clean SVG for LaserGRBL.

Default answers:

- Prefer command line and keyboard-first flows.
- Prefer SVG-native, plain, inspectable output.
- Prefer small tools that compose over smart tools with hidden behavior.
- Prefer deterministic geometry over visual convenience.
- Prefer rejecting a feature over carrying accidental product complexity.

## Documentation Hygiene

- **Single source of truth.** `AGENTS.md` owns the agent rules. `CLAUDE.md` is a thin pointer. `PLAN.md` is the live roadmap. `src/` is the source of truth for behavior.
- **Status markers, not stale content.**
  - **Demands** (`docs/product/demands/`): use the state machine in `docs/product/product-owner-agent.md`. When a demand ships, flip its header to `Done` and record the shipping commit in `Implementation:`; move it in `docs/product/backlog.md`.
  - **ADRs** (`docs/adr/`): when an ADR is reversed, add a `**Superseded**` status header pointing at the new ADR or commit. Keep the original text.
- **Living docs sit next to code.** Module headers (`//!`) and doc comments (`///`) carry component-level notes. `docs/` describes intent and contracts, not implementation.
- **No generation cruft.** Strip artifacts (`citeturn…` tokens, `sandbox:/mnt/data/…` links, malformed tables) when they appear.
- **English-only filenames.**
- **Sync the CHANGELOG.** When a `Done` demand changes user-visible behavior, `demand-manager` updates `CHANGELOG.md` under `[Unreleased]`.

## Reference documentation

- [`PLAN.md`](PLAN.md) — live roadmap (demand table, phases, PM execution log).
- [`docs/adr/0001-pure-rust-egui.md`](docs/adr/0001-pure-rust-egui.md) — framework decision.
- [`docs/adr/0002-headless-input-tests-and-dirty-tracking.md`](docs/adr/0002-headless-input-tests-and-dirty-tracking.md) — headless `App::update_ui` regression-test pattern, the single keyboard gate, and `History::revision()` as the autosave dirty signal.
- [`docs/adr/0003-command-line-input-contract.md`](docs/adr/0003-command-line-input-contract.md) — the command-line input contract: the `cmdline` kernel module, `ToolKind`/`ToggleKind`/`ZoomKind`, the recall ring, and the `Tool::on_command_input` wiring.
- [`docs/adr/0004-measuring-the-300-loc-cap.md`](docs/adr/0004-measuring-the-300-loc-cap.md) — how the 300-LOC cap is counted and measured, its two exemptions, and the "name the seam at 270, split at 300" rule.
- [`docs/adr/0005-native-dialogs-disarmed-by-default.md`](docs/adr/0005-native-dialogs-disarmed-by-default.md) — native `rfd` dialogs are disarmed outside the app binary, so a test that reaches one panics instead of hanging CI.
- [`docs/adr/0006-real-user-paths-are-injected.md`](docs/adr/0006-real-user-paths-are-injected.md) — the settings and autosave paths are resolved once at boot and carried on `App`; a process that was not given a path writes nothing, and tests point theirs at a tempdir.
- [`docs/adr/0007-agent-turn-mutates-the-live-document.md`](docs/adr/0007-agent-turn-mutates-the-live-document.md) — the agent thread owns no document state; it rendezvouses one action at a time against the live `Document` through `Command` + `History`, fenced on `History::revision()`, and one turn coalesces into one undo entry.
- [`docs/product/README.md`](docs/product/README.md) — product principles.
- [`docs/product/product-owner-agent.md`](docs/product/product-owner-agent.md) — demand format and lifecycle.
- [`docs/product/backlog.md`](docs/product/backlog.md) — prioritized backlog by state.
- [`CHANGELOG.md`](CHANGELOG.md) — user-visible changes per release.
