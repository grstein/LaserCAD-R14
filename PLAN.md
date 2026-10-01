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
- **Release**: v0.2.0 (LCV-089) published 2026-09-28; v0.3.0 published 2026-09-30 after the user smoke test, with the user's authorization;
  v0.4.0 (LCV-160..163) and v0.5.0 (LCV-164..169, 183, 184) bumped 2026-09-30, v0.6.0 (LCV-154, 185..193),
  v0.7.0 (LCV-194..200), v0.8.0 (LCV-180, 201), v0.9.0 (LCV-170..179) and v1.0.0 (LCV-202) bumped 2026-10-01, untagged until the user smoke-tests them.
- **v0.3 "workshop-ready"** (approved 2026-09-29): LCV-156 layers, 157 COPY, 159 polar `@d<a` +
  DIST, 158 ROTATE, 181 MIRROR, 182 SCALE — all Done 2026-09-30 (158 was split into 158/181/182).
  Released as v0.3.0.
- **Version roadmap** (proposal 2026-09-29; pre-1.0 semver: features → minor, fixes → patch):
  - v0.3.x — smoke-test fixes only.
  - v0.4.0 "precise editing" — 160 TRIM/EXTEND with arcs, 161 snaps, 162 crosshair/picking,
    163 selection feedback.
  - v0.5.0 "UI polish" — 183 icon tool rail and 184 visual refresh first (user decision
    2026-09-30: AutoCAD-style icons, modern but KISS), then 164–169 (154 moved to v0.6: it was built with the agent work).
  - v0.6.0 "agent harness" (from the agent's own session feedback, 2026-09-30) — 185 batch schema
    fidelity, 192 refusal guidance, 189 budget visibility, 186 set transforms, 191 layer assignment,
    187 framed capture, 190 drawing check (also a user `CHECK` command), 193 turn metrics; then
    188 stable ids (needs an ADR amending ADR 0007). Agent-only code, so it may run in a worktree
    alongside v0.5.
  - v0.7.0 "agent CAD loop" — 194 measure, 195 feedback after mutation, 196 batch primitives,
    197 verify before reply, 198 turn checkpoints, 199 reference image, 200 eval bench.
  - v0.8.0 "platform" — 180 dependency refresh (alone, first), 201 Windows/macOS packaging.
  - v0.9.0 "SVG conformance" — 170–179 (planned as 0.9 + 0.10; built on one branch, released together).
  - v1.0.0 — LCV-202.
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
