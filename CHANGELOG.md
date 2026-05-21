# Changelog

All notable changes to LaserCAD v2 are documented in this file. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

This is the v2 (green-field, pure Rust + egui) line of LaserCAD R14. The v1 line (TypeScript + Tauri) is maintained in a separate directory and its history is recorded in its own CHANGELOG.

## [Unreleased]

### Added

- Initial scaffold: Cargo project skeleton, agent harness under `.claude/agents/` (six agents), `AGENTS.md`, `CLAUDE.md`, `PLAN.md`, `README.md`, dual MIT/Apache license, empty module tree, ADR 0001 recording the pure-Rust + egui decision, CI workflow scaffold (Linux), bootstrap egui window placeholder.
- App shell upgraded to a real CentralPanel viewport (dark canvas placeholder). See LCV-030.
- Native bootstrap window opens at 1280×800 with title "LaserCAD v2 — bootstrap". See LCV-007.
- Geometry kernel introduces Vec2 and EPSILON. See LCV-010.
- Geometry kernel adds Line primitive (bbox, closest point, distance helpers). See LCV-011.
- Geometry kernel adds Circle primitive (bbox, point-at-angle, signed distance, containment). See LCV-012.
- Geometry kernel adds Arc primitive (wrap-aware bbox, contains_angle, sweep). See LCV-013.
- Geometry kernel adds line-line / line-circle / circle-circle intersection routines. See LCV-014.
- Geometry kernel adds Rect with contains/crosses predicates for line, circle, arc. See LCV-015.
- Geometry kernel adds snap engine (endpoint/midpoint/center/intersection kinds). See LCV-016.
- Geometry kernel: cross-module integration tests added (5 scenarios in tests/geometry.rs). See LCV-017.
- Document model gains canonical Entity enum and SCHEMA_VERSION=1. See LCV-020.
- Document model adds Document struct with bounds, Default, and Selection placeholder. See LCV-021.
- Document model adds Command trait with do_/undo round-trip contract. See LCV-022.
- Document model adds CreateLine/Circle/Arc commands with captured-index undo. See LCV-023.
- Document model adds DeleteEntities + MoveEntities commands and Entity::translate. See LCV-024.
- Document model adds TrimEntity + ExtendEntity commands (Line/Circle pairs; Arc targets deferred). See LCV-025.
- Document model adds 200-deep undo/redo history stack. See LCV-026.
- Document model concretizes Selection (HashSet-backed) and adds SelectionCommand. See LCV-027.
- Grid renderer: responsive 1-2-5 decade ladder grid (minor spacing adapts across zoom range; major lines every 10 minors). See LCV-033.
- Bed renderer: 400×400 mm work-area rectangle with dark outside overlay (LCV-034).
- Entity painter: draws lines, circles, and arcs from the Document on the viewport (LCV-035).
