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
- **v0.3 "workshop-ready"** (approved 2026-09-29): LCV-156 layers, 157 COPY, 159 polar `@d<a` +
  DIST, 158 ROTATE, 181 MIRROR, 182 SCALE — all Done 2026-09-30 (158 was split into 158/181/182).
  Ready to tag `v0.3.0` once the user smoke-tests it; tagging is the user's call.
- **Version roadmap** (proposal 2026-09-29; pre-1.0 semver: features → minor, fixes → patch):
  - v0.3.x — smoke-test fixes only.
  - v0.4.0 "precise editing" — 160 TRIM/EXTEND with arcs, 161 snaps, 162 crosshair/picking,
    163 selection feedback.
  - v0.5.0 "UI polish" — 164–169, plus 154.
  - v0.6.0 "platform" — 180 dependency refresh (alone, first), Windows/macOS packaging.
  - v0.7–0.8 "SVG conformance" — 170–175, then 176–179.
  - v1.0.0 — R14 v1 parity, stable SVG contract, multi-OS; after 1.0 an SVG contract change is a major.
- **Not in 0.3**: DXF, blocks/xref, fillet/chamfer/offset, G-code, LCV-146 skills.
- **Tooling**: LCV-152 and LCV-155 made build and test faster.

## Later

- Windows MSI/NSIS, macOS dmg + notarization, CI matrix.
- UI roadmap Drafts LCV-162..169 (the gaps in `DESIGN.md`); scheduling is the user's call.
- **SVG 2 conformance track** (user decision 2026-09-29): export a Conforming SVG Generator, import
  a secure-static Conforming SVG Interpreter; native ellipse/elliptical-arc and Bézier entities;
  `<text>` imported as outlines. Drafts LCV-170..179 in that order; coverage map in
  `docs/research/svg-spec-coverage.md`. Scheduling against v0.3 is the user's call.
