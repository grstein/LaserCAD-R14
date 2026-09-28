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
- **1.0 scope** (user decision 2026-09-27): LCV-142..145 in order 142 → 143 → 144 → 145.
  LCV-146 is deferred past 1.0.
- **Release**: LCV-089 (v0.1.0 tag) is blocked on the user's manual smoke run; agents never tag.
- **Tooling**: LCV-152 makes build and test faster.

## Later

- Windows MSI/NSIS, macOS dmg + notarization, CI matrix.
