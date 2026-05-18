# Changelog

All notable changes to LaserCAD v2 are documented in this file. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

This is the v2 (green-field, pure Rust + egui) line of LaserCAD R14. The v1 line (TypeScript + Tauri) is maintained in a separate directory and its history is recorded in its own CHANGELOG.

## [Unreleased]

### Added

- Initial scaffold: Cargo project skeleton, agent harness under `.claude/agents/` (six agents), `AGENTS.md`, `CLAUDE.md`, `PLAN.md`, `README.md`, dual MIT/Apache license, empty module tree, ADR 0001 recording the pure-Rust + egui decision, CI workflow scaffold (Linux), bootstrap egui window placeholder.
- Native bootstrap window opens at 1280×800 with title "LaserCAD v2 — bootstrap". See LCV-007.
- Geometry kernel introduces Vec2 and EPSILON. See LCV-010.
- Geometry kernel adds Line primitive (bbox, closest point, distance helpers). See LCV-011.
- Geometry kernel adds Circle primitive (bbox, point-at-angle, signed distance, containment). See LCV-012.
- Geometry kernel adds Arc primitive (wrap-aware bbox, contains_angle, sweep). See LCV-013.
- Geometry kernel adds line-line / line-circle / circle-circle intersection routines. See LCV-014.
- Geometry kernel adds Rect with contains/crosses predicates for line, circle, arc. See LCV-015.
