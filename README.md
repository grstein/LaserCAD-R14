# LaserCAD

[![CI](https://github.com/grstein/LaserCAD-R14/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/grstein/LaserCAD-R14/actions/workflows/ci.yml)

A KISS 2D CAD for laser cutting — pure Rust, single binary, LaserGRBL-compatible SVG output.

LaserCAD is written in pure Rust with [`egui`](https://docs.rs/egui/) via [`eframe`](https://docs.rs/eframe/). The product goal: an AutoCAD R14-shaped CAD surface for making simple, precise 2D geometry that exports clean SVG for LaserGRBL — with no general-purpose ambitions.

## Status

Stable (v1.0.1). LaserCAD 1.0 covers the whole original scope — each capability is mapped to its command and the tests that prove it in [`docs/product/parity-1-0.md`](docs/product/parity-1-0.md) — and the files it writes stay valid: the SVG export contract is frozen for 1.x, and changing it needs a major version ([ADR 0018](docs/adr/0018-svg-export-contract-frozen-at-1-0.md)). Install it with [`docs/install.md`](docs/install.md); learn it with the [user guide](docs/user-guide.md).

How it got here: the core draw-to-SVG workflow works: drawing and modify tools, snaps, ortho lock, undo/redo, SVG export/import (LaserGRBL-compatible), autosave, recent files, and native file dialogs, on Linux. The command line now drives the drawing tools directly: typed absolute/relative coordinates, direct distances and tool aliases reach LINE, PLINE, RECT, CIRCLE, ARC, MOVE and TEXT, each with an AutoCAD-R14-style prompt and arrow-key command recall (see LCV-110, LCV-111, LCV-112, LCV-126 — this superseded an earlier "stub" state). The optional agent chat panel now edits the live drawing rather than only narrating it: a model-issued action commits through the same `Command` + history stack as any other edit, and an uninterrupted turn undoes in one Ctrl+Z (see LCV-121 through LCV-125). Feature work is tracked in [`PLAN.md`](PLAN.md); see [`docs/build-local.md`](docs/build-local.md) for building and packaging from a clean checkout. The first release, v0.2.0, ships an AppImage and a `.deb` on the GitHub releases page (LCV-089). The agent now also remembers earlier turns, draws batches from JSON, and can look at the canvas when you allow it (LCV-142..153). v0.3.0 adds layers with one LaserGRBL export file per layer, COPY, ROTATE, MIRROR, SCALE, polar input (`@50<30`) and DIST (LCV-156..159, 181, 182). v0.4.0 adds trim and extend with arcs, quadrant/perpendicular/tangent/nearest snaps, an R14 crosshair with screen-space picking, and hover, window/crossing and trim/erase previews (LCV-160..163). v0.5.0 brings an icon tool rail, a calmer theme, menu icons and shortcuts, R14 prompts with Enter-to-repeat, save/export confirmations and keyboard-driven dialogs (LCV-164..169, 183, 184). v0.6.0 makes the AI assistant more reliable: whole-set edits, layer moves, stable entity ids, framed captures, refusals that say how to fix the call, a visible step budget, per-turn metrics, and a `CHECK` command that finds open ends and duplicates before export (LCV-154, 185..193). v0.7.0 gives the agent a CAD loop: it measures instead of guessing, sees the drawing after each change, draws polylines, rectangles, polygons, text and arrays in one call, checks its work before replying, can roll back a failed attempt, works from an attached sketch, and maintainers can score models with `scripts/agent-bench.sh` (LCV-194..200). v0.8.0 moves to egui 0.36 and current dependencies and adds a portable Windows `.zip` and an Apple Silicon macOS `.dmg`; see [`docs/install.md`](docs/install.md) (LCV-180, 201). v0.9.0 opens real-world SVG: every path command, true units, CSS styles, transforms, `<rect>`/`<polyline>`/`<polygon>`, `<use>` and `<switch>`, text as outlines in an installed font, and native ellipses and Bézier curves that edit and save as such (LCV-170..179). v1.0.0 is the stable release: a user guide, the parity table, a release smoke checklist, and an SVG export frozen for 1.x (LCV-202). v1.0.1 fixes the AI assistant on OpenAI models and ships the first Windows and macOS downloads alongside Linux.

## Non-goals

LaserCAD stays a small CAD for the CAD → LaserGRBL flow. It deliberately does not do:

- DXF import or export.
- G-code output (LaserGRBL makes the G-code).
- Fillet, chamfer or offset.
- Blocks or external references.

## Install

Releases ship a Linux AppImage and `.deb`, a portable Windows `.zip` and an Apple Silicon macOS `.dmg`. The Windows and macOS builds are unsigned; [`docs/install.md`](docs/install.md) has the first-run steps and where settings and autosave are kept.

## Build

Requires Rust ≥ 1.98 (pinned in `rust-toolchain.toml`; `rustup` will fetch it automatically).

```bash
cargo run                    # debug build, opens the app
cargo run --release          # release build
cargo build --release        # produces target/release/lasercad
cargo test --all
```

System packages needed for `eframe` (`glow` backend) on Linux:

```bash
# Fedora / RHEL
sudo dnf install -y gcc libxkbcommon-devel libX11-devel libXcursor-devel libXrandr-devel libXi-devel mesa-libGL-devel

# Debian / Ubuntu
sudo apt install -y build-essential libxkbcommon-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libgl1-mesa-dev
```

## Architecture

See [`AGENTS.md`](AGENTS.md) for the full agent and architecture rules. Short version:

- `src/` is one Rust crate, divided into pure-kernel modules (`geometry`, `document`, `io/svg`, `text`, `cmdline`) and UI modules (`render`, `tools`, `ui`, `io`, `agent`, `app`).
- Kernel modules MUST NOT import `egui`, `eframe`, or `rfd` — they stay testable as a pure library.
- mm canonical everywhere except `render/camera.rs`. Radians in the kernel.
- All entity mutation through the `Command` trait + history stack.
- One responsibility per file; hard cap 300 implementation LOC per `.rs` (`scripts/loc-cap.sh`).

## Development workflow

Development follows lean Spec-Driven Development (details in [`AGENTS.md`](AGENTS.md)). Each demand
gets a folder `docs/specs/LCV-NNN-slug/` and moves through four states, recorded only in the
`Status` line of its `spec.md`:

- **Draft → Specified** — `spec.md` states the problem and EARS acceptance criteria.
- **Specified → Planned** — `plan.md` (files, ADRs, risks) and `tasks.md` (a checklist, one commit per task).
- **Planned → Done** — the tasks are implemented, one test per acceptance criterion, and `scripts/gate.sh` is green.

`scripts/backlog.sh` regenerates [`docs/product/backlog.md`](docs/product/backlog.md) from the spec headers;
`scripts/backlog.sh --next` lists what is ready now.

Run `git config core.hooksPath .githooks` once per clone.

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))
- MIT License ([`LICENSE-MIT`](LICENSE-MIT))

at your option.
