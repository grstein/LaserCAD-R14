# LaserCAD v2

A KISS 2D CAD for laser cutting — pure Rust, single binary, LaserGRBL-compatible SVG output.

LaserCAD v2 is a green-field rewrite of [LaserCAD R14 v1](../LaserCAD-R14/) that drops the TypeScript + Tauri stack in favor of pure Rust with [`egui`](https://docs.rs/egui/) via [`eframe`](https://docs.rs/eframe/). The product goal is the same: an AutoCAD R14-shaped CAD surface for making simple, precise 2D geometry that exports clean SVG for LaserGRBL — with no general-purpose ambitions.

## Status

Pre-alpha. The scaffold is in; feature work is tracked in [`PLAN.md`](PLAN.md). First target: feature parity with v1.0.0 (drawing + modify tools, snaps, undo/redo, command line, SVG export/import, autosave, native dialogs, agent harness) on Linux.

## Build

Requires Rust ≥ 1.88 (pinned in `rust-toolchain.toml`; `rustup` will fetch it automatically).

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

- `src/` is one Rust crate, divided into pure-kernel modules (`geometry`, `document`, `io/svg`, `agent/classifier`, `text`) and UI modules (`render`, `tools`, `ui`, `io`, `agent`).
- Kernel modules MUST NOT import `egui`, `eframe`, or `rfd` — they stay testable as a pure library.
- mm canonical everywhere except `render/camera.rs`. Radians in the kernel.
- All entity mutation through the `Command` trait + history stack.
- One responsibility per file; hard cap 300 LOC per `.rs`.

## Agent workflow

This repository is designed to be driven by Claude Code agents. The agent suite lives under [`.claude/agents/`](.claude/agents/):

- `project-manager` (Opus) — drives [`PLAN.md`](PLAN.md), picks the next demand, spawns workers.
- `architect` (Opus) — ADRs and module-boundary decisions.
- `product-owner` (Opus) — demand refinement.
- `demand-manager` (Sonnet) — demand state machine and backlog tables.
- `implementer-rust` (Sonnet) — actually writes the Rust code.
- `reviewer-rust` (Sonnet) — verifies KISS / architecture / tests on shipped demands.

To start a session, open a Claude Code shell in this directory and prompt:

> *"You are the project-manager. Read AGENTS.md and PLAN.md, then drive the next demand."*

The PM will pick the lowest-ID `Ready` demand whose dependencies are `Done` (or route a `Draft` demand to `product-owner` for refinement first) and orchestrate it through the worker agents.

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))
- MIT License ([`LICENSE-MIT`](LICENSE-MIT))

at your option.
