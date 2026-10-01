# LaserCAD

[![CI](https://github.com/grstein/LaserCAD-R14/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/grstein/LaserCAD-R14/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/grstein/LaserCAD-R14)](https://github.com/grstein/LaserCAD-R14/releases/latest)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

**A small, fast 2D CAD for laser cutting.** Draw precise parts the AutoCAD R14 way — keyboard,
command line, typed coordinates — and save SVG that [LaserGRBL](https://lasergrbl.com/) opens
as is. One binary, no runtime, no account, no cloud.

[**Download**](https://github.com/grstein/LaserCAD-R14/releases/latest) ·
[User guide](docs/user-guide.md) ·
[Install notes](docs/install.md) ·
[Changelog](CHANGELOG.md) ·
[Contributing](#contributing)

## Features

- **R14-style drafting**: Line, Polyline, Rect, Circle, Arc, Text, Move, Copy, Rotate, Mirror,
  Scale, Trim, Extend, Delete and Dist, each with a prompt that says what it wants next.
- **Precise input**: absolute `50,25`, relative `@10,-5` and polar `@50<30` points, typed
  distances, ortho lock, and eight object snaps (endpoint, midpoint, center, intersection,
  quadrant, perpendicular, tangent, nearest).
- **Millimetres on your real bed**: set the bed size of your machine; the drawing and the SVG
  use it.
- **Layers = laser jobs**: one layer per kind of work (`Cut`, `Mark`, `Engrave`), each with its
  color and Output switch. *File > Export Layers* writes one SVG per layer, ready for its own
  speed and power in LaserGRBL.
- **Opens real-world SVG** from Inkscape, Illustrator or LightBurn: paths, curves, ellipses,
  transforms, CSS styles, `<use>`, and text as outlines; whatever it cannot import is listed.
- **Stable output**: the SVG export is frozen for 1.x — the same drawing exports the same bytes
  in every 1.x release.
- **Safe editing**: 200 undo steps, autosave with crash recovery, recent files.
- **Optional AI assistant**: describe a part in plain language, or attach a sketch, and it
  draws on your drawing through the same undo history. Works with any OpenAI-compatible API
  (OpenRouter by default); off until you set it up.

## Install

Download the latest build from the
[releases page](https://github.com/grstein/LaserCAD-R14/releases/latest):

| Platform | File |
|---|---|
| Linux (x86_64) | `lasercad-x86_64.AppImage` or `lasercad_<version>_amd64.deb` |
| Windows (x86_64) | `lasercad-<version>-windows-x86_64.zip` (portable, unzip and run) |
| macOS (Apple Silicon) | `lasercad-<version>-macos-aarch64.dmg` |

The Windows and macOS builds are not code-signed yet; [`docs/install.md`](docs/install.md)
shows how to open them the first time and where settings and autosave live.

## Quick start

1. **File > Bed Size…** — enter your machine's work area in mm.
2. Press `R`, click a corner, type `@80,50` and press Enter: an 80 × 50 mm plate.
3. Press `C`, snap to a point, type `3` and press Enter: a 6 mm hole.
4. **Format > Layers…** — add an `Engrave` layer, make it current, press `D` and type a label.
5. **File > Save**, then **File > Export Layers**, and load each file into LaserGRBL.

Everything else — every tool, command word and shortcut — is in the
[user guide](docs/user-guide.md). Press F1 in the app for the keyboard shortcuts.

## Status

Stable (v1.0.1). LaserCAD covers its whole original scope — each capability is mapped to its
command and the tests that prove it in [`docs/product/parity-1-0.md`](docs/product/parity-1-0.md)
— and the files it writes stay valid: the SVG export contract is frozen for 1.x, and changing it
needs a major version ([ADR 0018](docs/adr/0018-svg-export-contract-frozen-at-1-0.md)).
Linux is the primary platform; Windows and macOS builds ship with every release and need more
testers. Release history: [`CHANGELOG.md`](CHANGELOG.md). Roadmap: [`PLAN.md`](PLAN.md).

## Non-goals

LaserCAD stays a small CAD for the CAD → LaserGRBL flow. It deliberately does not do:

- DXF import or export.
- G-code output (LaserGRBL makes the G-code).
- Fillet, chamfer or offset.
- Blocks or external references.

A feature request outside the CAD → LaserGRBL flow will most likely be declined — small and
predictable is the point.

## Why Rust, and why AI writes the code

LaserCAD is an experiment in building real desktop software where **AI agents write the code
and humans decide what to build**.

- **Rust keeps it light.** One native binary of a few megabytes with no runtime, no Electron
  and no installer requirements; it starts instantly and runs on modest workshop PCs.
- **Rust keeps AI honest.** The compiler, `clippy -D warnings` and 2,600+ tests reject most
  mistakes before a human ever looks. That tight feedback loop is what lets an AI agent work
  on its own and still land correct code.
- **Humans steer, AI implements.** Every change starts as a short spec with testable
  acceptance criteria; a person approves the spec and the plan, an AI agent implements it
  task by task until the gate is green, and another reviews the diff against the criteria.
  A human barely touches the code itself.
- **The rules are written down.** [`AGENTS.md`](AGENTS.md) is the constitution every agent and
  contributor follows: architecture, invariants, the 300-line file cap, and the workflow.

You are welcome to contribute the same way — with Claude Code, Codex, Cursor or by hand. The
gate does not care who typed the code; it only cares that it is right.

## Contributing

LaserCAD needs **testers** and **pull requests**. Both are very welcome.

### Test it and tell us

- Cut something real with it and [open an issue](https://github.com/grstein/LaserCAD-R14/issues)
  for anything that surprised you: a bug, an SVG that does not import, an export LaserGRBL
  misreads, a missing R14 habit.
- Windows and macOS reports are especially valuable — most development happens on Linux.
- A good report has the version (Help > About), the OS, the steps, and the SVG file if one is
  involved.

### Send a pull request

1. Fork the repository and clone it. Run `git config core.hooksPath .githooks` once.
2. Make your change, small and focused. A bug fix comes with a test that fails without it.
3. Run `scripts/gate.sh` — formatting, clippy, the full test suite, the file-size cap and the
   backlog check. It must print `GATE GREEN`.
4. Commit with a [Conventional Commits](https://www.conventionalcommits.org/) message
   (`fix: …`, `feat: …`, `docs: …`) and open the pull request.

Small fixes, refactors, tests and docs go straight to a pull request. A new feature or a change
in behavior starts as a spec in `docs/specs/` (templates in `docs/specs/_templates/`): open an
issue or a draft PR with the spec first, so the "what" is agreed before the "how". Using an AI
assistant? Point it at [`AGENTS.md`](AGENTS.md) first. Commits must not carry
`Co-Authored-By` or other AI trailers; the commit hook rejects them.

## Build from source

Requires Rust ≥ 1.98; `rustup` fetches the pinned toolchain from `rust-toolchain.toml`.

```bash
cargo run                    # debug build, opens the app
cargo build --release        # target/release/lasercad
scripts/check.sh [filter]    # fast inner loop: clippy + matching tests
scripts/gate.sh              # everything CI checks
```

Linux needs a few system packages for the `eframe` `glow` backend:

```bash
# Fedora / RHEL
sudo dnf install -y gcc libxkbcommon-devel libX11-devel libXcursor-devel libXrandr-devel libXi-devel mesa-libGL-devel

# Debian / Ubuntu
sudo apt install -y build-essential libxkbcommon-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libgl1-mesa-dev
```

Packaging (AppImage, `.deb`, Windows `.zip`, macOS `.dmg`): [`docs/build-local.md`](docs/build-local.md).

## Architecture

Pure Rust with [`egui`](https://docs.rs/egui/) via [`eframe`](https://docs.rs/eframe/), in one
crate:

- **Kernel** — `geometry`, `document`, `io/svg`, `text`, `cmdline` — never imports the UI, so
  it is tested as a plain library.
- **UI** — `app`, `render`, `tools`, `ui`, `io`, `agent` — draws the kernel's state and turns
  input into commands.
- Millimetres everywhere and radians in the kernel; every edit is a `Command` in the undo
  history, the AI assistant's included; no `.rs` file over 300 lines of implementation.

Full rules in [`AGENTS.md`](AGENTS.md), decisions in [`docs/adr/`](docs/adr/), UI direction in
[`DESIGN.md`](DESIGN.md).

## License

Dual-licensed under either of

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))
- MIT License ([`LICENSE-MIT`](LICENSE-MIT))

at your option. Unless you state otherwise, any contribution you submit is dual-licensed the
same way, without additional terms or conditions.
