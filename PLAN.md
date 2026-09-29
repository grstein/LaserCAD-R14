# LaserCAD v2 — Roadmap

Status of every demand lives in `docs/specs/*/spec.md` and is rendered in
[`docs/product/backlog.md`](docs/product/backlog.md) by `scripts/backlog.sh`. This file holds
only direction. The workflow is in [`AGENTS.md`](AGENTS.md) §Workflow.

## Vision

A pure-Rust, single-binary, egui reimplementation of LaserCAD R14 v1: feature parity with
v1.0.0 plus the TEXT command and an agent that edits the live drawing. KISS AutoCAD R14 clone
for laser cutting. Linux first; Windows and macOS later.

## Stack

Rust (toolchain pinned) · `egui`/`eframe` · `rfd` dialogs · blocking `reqwest` on a
`std::thread` for the agent (no `tokio`) · handwritten SVG export, `roxmltree` import ·
`serde_json` + `directories` persistence · single binary, AppImage / `.deb`.

## Where we are

- Phases 0–11 (foundation → kernel → document → render → tools → IO → chrome → agent harness →
  Linux distribution → live-document agent) are done; legacy demand files LCV-001..148 were
  retired in the SDD migration and remain in git history (`git log -- docs/product/demands`).
- **1.0 scope** (user decisions 2026-09-27/28): LCV-142..145, 149..153 — all Done.
  LCV-146 is deferred past 1.0.
- **Release**: v0.2.0 (LCV-089) tagged and published on 2026-09-28 with the user's authorization; the user smoke-tests it.
- **Next — v0.3 "workshop-ready"** (proposal approved by the user 2026-09-29), in order:
  0. close v0.2 — the user's smoke-test findings (fast lane) and CI green once billing resumes;
  1. LCV-156 layers, LightBurn-style, one export file per layer (changes the SVG export
     contract; ADR at /design);
  2. LCV-157 COPY; 3. LCV-158 ROTATE / MIRROR / SCALE; 4. LCV-159 polar input `@d<a` + DIST;
  5. LCV-160 TRIM/EXTEND with arcs; 6. LCV-161 more object snaps; 7. LCV-154.
  v0.3.0 when 0–4 are Done; 5–7 may slip to 0.3.x. Each new command also reaches the agent
  (tool entry + built-in prompt line).
- **Not in 0.3**: DXF, blocks/xref, fillet/chamfer/offset, G-code, LCV-146 skills.
- **Tooling**: LCV-152 and LCV-155 made build and test faster.

## Later

- Windows MSI/NSIS, macOS dmg + notarization, CI matrix.
