# ADR 0001 — Pure Rust + egui (no Tauri, no JS)

- **Status**: Accepted
- **Date**: 2026-05-17
- **Deciders**: project owner (Stein), architect

## Context

LaserCAD R14 v1 shipped on TypeScript + Vite + Tauri 2.x. The dual-runtime architecture (web bundle + Tauri native) imposed ongoing complexity: a `tauri-bridge.ts` shim, dual codepaths for file IO and persistence (Tauri plugins vs `localStorage` + `Blob`), a CSP that had to allow `ipc:` and external agent endpoints, and a CI matrix that built both web and native artifacts.

The project goals favor:

1. **KISS AutoCAD R14 clone** with no general-purpose ambitions.
2. **Agent-friendly codebase**: small files, clear contracts, easy to load only what's needed per task.
3. **Easy long-term maintenance** — fewer moving parts, fewer languages.
4. **Linux as the primary target**; Windows and macOS long-term.

A migration plan from v1 to a Rust kernel (with TypeScript glue) was considered but rejected by the user in favor of a green-field v2.

## Decision

LaserCAD v2 is **pure Rust** using **`egui`** as the immediate-mode GUI library, embedded via **`eframe`** for windowing. There is no JavaScript, no TypeScript, no WebView, no Tauri. The crate produces a single native binary.

Companion choices made under this ADR:

- **File dialogs**: `rfd`. Cross-platform native (Linux GTK/Zenity, Windows IFileDialog, macOS NSOpenPanel/NSSavePanel).
- **Persistence paths**: `directories` crate (XDG on Linux, Known Folders on Windows, Application Support on macOS).
- **Serialization**: `serde` + `serde_json`. SVG export is handwritten; SVG import uses `roxmltree`.
- **HTTP for the agent**: `reqwest` (rustls TLS) on a `tokio` runtime owned by the app.
- **Rendering backend**: `eframe`'s `glow` (OpenGL) backend. `wgpu` requires a separate ADR.
- **Distribution**: single binary; Linux via AppImage and `.deb` first.

## Consequences

**Positive:**

- One language. The IPC boundary disappears. The kernel is testable as a pure Rust library.
- One binary per platform. No system WebView dependency, no Cargo + npm dual toolchain, no CSP, no capabilities files.
- Module structure maps cleanly to demand structure (each `mod.rs` is a contract).
- Strong typing across the whole stack; refactoring is `cargo check`.
- Easier headless / WASM future if ever needed (kernel is pure).

**Negative / costs:**

- We lose HTML/CSS for the UI chrome. egui is themable but visually less polished than CSS by default.
- We lose Vite HMR. Iteration loop is `cargo run`; `cargo-watch` or `bacon` is the closest substitute. Rust rebuilds are slower than TS, mitigated by incremental compilation.
- Custom widgets that browser provides for free (rich text edit, native context menus) must be built or borrowed from `egui_extras`.
- All v1 TypeScript code is discarded as source. It remains reference, not source.

**Commitments locked in:**

- Kernel modules (`geometry`, `document`, `io/svg`, `agent/classifier`, `text`) MUST NOT import `egui` / `eframe` / `rfd`. Reviewer enforces.
- One responsibility per file, ≤300 LOC. Reviewer enforces.
- All entity mutation through the `Command` trait + history stack. Reviewer enforces.
- mm canonical in kernel and SVG; radians in kernel angles.

## Alternatives considered

- **`iced`** (Elm-style retained mode) — cleaner reactivity than egui, but custom-canvas work (the CAD viewport) is heavier and the ecosystem around CAD-style hit-testing is thinner.
- **`slint`** — declarative UI with a markup language and Rust backend. Rejected: adds a non-Rust file type per UI screen (anti-KISS); license has a commercial tier; the canvas story for CAD is less first-class.
- **`makepad`** — designed for visual tools, GPU-accelerated. Rejected: too young; ecosystem and documentation are not yet at egui's level; risk to parity timeline.
- **Custom `winit` + `wgpu` stack** — maximum control, maximum responsibility. Rejected: anti-KISS.
- **Stay on Tauri** (migrate kernel into Rust gradually) — rejected by the user in favor of green-field.
- **Bevy** (game engine + Bevy UI) — overkill; ECS as a CAD document model is unusual and noisy.

## Revisit criteria

This ADR is revisited if any of the following occur:

- egui regression that breaks CAD-scale viewport painting (~10k entities).
- A demand requires a UI capability (e.g., embedded HTML preview) that egui cannot serve and a small wrapper around something else (e.g., webview-rs) is genuinely cleaner than building it in egui.
- A second platform (Windows or macOS) requires a behavior that `rfd` / `directories` / `eframe` cannot deliver.

Until then, the decision stands.
