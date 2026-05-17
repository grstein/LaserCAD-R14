---
name: egui-best-practices
description: Guides Rust + egui work in LaserCAD v2 — eframe app shell, Painter for CAD viewport, native dialogs via rfd, settings persistence via directories + serde_json, async HTTP via reqwest + tokio. Use when touching `src/app.rs`, `src/render/`, `src/ui/`, `src/io/`, or `src/agent/transport.rs`, or when adding a Cargo dependency.
---

# egui Best Practices — LaserCAD v2

This skill is a thin index, not an essay. The implementer agent reads the real docs on demand; this file lists what's load-bearing and links to the right page.

## Authoritative references

- **egui** — https://docs.rs/egui/latest/egui/
- **eframe** — https://docs.rs/eframe/latest/eframe/
- **egui_extras** — https://docs.rs/egui_extras/latest/egui_extras/
- **rfd** (file dialogs) — https://docs.rs/rfd/latest/rfd/
- **directories** (platform paths) — https://docs.rs/directories/latest/directories/
- **reqwest** — https://docs.rs/reqwest/latest/reqwest/
- **roxmltree** — https://docs.rs/roxmltree/latest/roxmltree/
- **egui demo source (live patterns)** — https://github.com/emilk/egui/tree/master/crates/egui_demo_lib/src/demo

## Project invariants (do not violate)

1. **Single-binary**. No external runtime, no WebView, no JS. Everything ships in the `lasercad` binary.
2. **Purity rule.** No `egui`, `eframe`, or `rfd` imports inside `src/geometry/`, `src/document/`, `src/io/svg/`, `src/agent/classifier.rs`, or `src/text/`. The kernel is a pure-Rust library.
3. **mm canonical** in geometry, document, command line, SVG export. Pixels only inside `render/camera.rs`.
4. **One responsibility per file**, ≤300 LOC. Split if it grows.
5. **No `unsafe`** without an ADR.

## egui patterns we use

- **eframe app shell** (`src/app.rs`): `impl eframe::App for App { fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) { ... } }`.
- **Central panel for the CAD viewport** (`src/render/viewport.rs`): `egui::CentralPanel::default().show(ctx, |ui| { ... })`, then `ui.painter()` for primitives.
- **Top/bottom/side panels** for menubar / toolbar / command line / statusbar.
- **Custom painting**: `Painter::line_segment`, `Painter::circle_stroke`, `Painter::add(egui::Shape::path(...))` for arcs.
- **Hit-testing** lives in `geometry/` (pure); the viewport calls into it with world-space coords.
- **Pan / zoom**: handle `ui.input(|i| i.raw_scroll_delta)` and middle-drag in the viewport; transforms live in `render/camera.rs`.
- **Repaint discipline**: egui is immediate-mode but doesn't repaint unless something changed. Call `ctx.request_repaint()` only when an external event (autosave timer, async agent response) needs UI refresh.

## Async patterns

- egui's `update` runs on the main thread; never block it.
- Long IO (HTTP for the agent): spawn on a `tokio::runtime` owned by the app, channel results back, `ctx.request_repaint()` on receipt.
- File dialogs via `rfd::AsyncFileDialog` for non-blocking open/save; or `rfd::FileDialog::new()` for blocking modal (acceptable on a manual user action like File→Save As).

## Persistence patterns

- Config dir via `directories::ProjectDirs::from("dev", "lasercad", "lasercad")`. Don't roll your own.
- Settings = `serde_json` to `<config_dir>/settings.json`. Atomic write: write tmp + rename.
- Autosave = same pattern, separate file (`<data_dir>/autosave.json`). Debounce in the app loop.

## Testing patterns

- Pure modules (`geometry/`, `document/`, `io/svg/`, `agent/classifier`, `text/`) → standard `cargo test`. Plain unit tests.
- UI tests: stay manual unless `egui_kittest` is justified by a specific demand. Don't add UI testing infra preemptively.

## Gotchas (worth a paragraph each)

- **glow vs wgpu**: we default to `glow` (OpenGL) for broader Linux compatibility. Switching to `wgpu` requires an ADR.
- **HiDPI**: egui handles DPI automatically; don't hardcode pixel sizes in the viewport. Use `ctx.pixels_per_point()` if you really need raw pixels.
- **Egui state in `App`**: keep mutable state in the `App` struct, not in egui's `ctx.data` (which is best for transient widget state, not document state).
- **Painter coordinates are in screen-space pixels**. Apply the camera transform yourself.

When you discover a new gotcha, add a paragraph here in the same PR.
