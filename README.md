# LaserCAD v2

[![CI](https://github.com/grstein/LaserCAD-R14-V2/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/grstein/LaserCAD-R14-V2/actions/workflows/ci.yml)
*(This repository is private; the badge above renders "not found" if you're viewing it while logged out of an account with access.)*

A KISS 2D CAD for laser cutting — pure Rust, single binary, LaserGRBL-compatible SVG output.

LaserCAD v2 is a green-field rewrite of [LaserCAD R14 v1](../LaserCAD-R14/) that drops the TypeScript + Tauri stack in favor of pure Rust with [`egui`](https://docs.rs/egui/) via [`eframe`](https://docs.rs/eframe/). The product goal is the same: an AutoCAD R14-shaped CAD surface for making simple, precise 2D geometry that exports clean SVG for LaserGRBL — with no general-purpose ambitions.

## Status

Beta (v0.6.0). The core draw-to-SVG workflow works today: drawing and modify tools, snaps, ortho lock, undo/redo, SVG export/import (LaserGRBL-compatible), autosave, recent files, and native file dialogs, on Linux. The command line now drives the drawing tools directly: typed absolute/relative coordinates, direct distances and tool aliases reach LINE, PLINE, RECT, CIRCLE, ARC, MOVE and TEXT, each with an AutoCAD-R14-style prompt and arrow-key command recall (see LCV-110, LCV-111, LCV-112, LCV-126 — this superseded an earlier "stub" state). The optional agent chat panel now edits the live drawing rather than only narrating it: a model-issued action commits through the same `Command` + history stack as any other edit, and an uninterrupted turn undoes in one Ctrl+Z (see LCV-121 through LCV-125). Feature work is tracked in [`PLAN.md`](PLAN.md); see [`docs/build-local.md`](docs/build-local.md) for building and packaging from a clean checkout. The first release, v0.2.0, ships an AppImage and a `.deb` on the GitHub releases page (LCV-089). The agent now also remembers earlier turns, draws batches from JSON, and can look at the canvas when you allow it (LCV-142..153). v0.3.0 adds layers with one LaserGRBL export file per layer, COPY, ROTATE, MIRROR, SCALE, polar input (`@50<30`) and DIST (LCV-156..159, 181, 182). v0.4.0 adds trim and extend with arcs, quadrant/perpendicular/tangent/nearest snaps, an R14 crosshair with screen-space picking, and hover, window/crossing and trim/erase previews (LCV-160..163). v0.5.0 brings an icon tool rail, a calmer theme, menu icons and shortcuts, R14 prompts with Enter-to-repeat, save/export confirmations and keyboard-driven dialogs (LCV-164..169, 183, 184). v0.6.0 makes the AI assistant more reliable: whole-set edits, layer moves, stable entity ids, framed captures, refusals that say how to fix the call, a visible step budget, per-turn metrics, and a `CHECK` command that finds open ends and duplicates before export (LCV-154, 185..193).

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

## Agent workflow

Development follows lean Spec-Driven Development with Claude Code (details in [`AGENTS.md`](AGENTS.md)):

- `/specify LCV-NNN <idea>` — write `docs/specs/LCV-NNN-*/spec.md` (EARS acceptance criteria); you approve.
- `/design LCV-NNN` — write `plan.md` + `tasks.md`; you approve.
- `/implement LCV-NNN` — `implementer-rust` (Opus) executes the tasks, `scripts/gate.sh` must be green, `reviewer-rust` (Fable) reviews once, the demand closes as Done.
- `/next` — what is ready now and what can run in parallel.

Run `git config core.hooksPath .githooks` once per clone.

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))
- MIT License ([`LICENSE-MIT`](LICENSE-MIT))

at your option.
