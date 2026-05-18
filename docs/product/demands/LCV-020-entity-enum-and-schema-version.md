# LCV-020 — Entity enum + schema_version=1

- **Status**: Done
- **Phase**: 2
- **Depends on**: LCV-013
- **Suggested agent**: architect
- **Suggested model**: opus
- **Implementation**: implementer-rust (task #69); b6cdd46

## Problem

Every Phase-2+ demand (`Command` trait, `Document` struct, history stack, selection, render, tools, SVG export) needs to talk about "an entity in the document" with one canonical type. Today the kernel exposes individual geometry value types (`Line`, `Circle`, `Arc`) and the snap engine carries a transitional `SnapEntity` enum that stands in for the missing document enum (see LCV-016). Without a single `document::Entity`, each downstream demand would reinvent the variant set, and serialization (LCV-058) would have nothing to version.

This demand introduces the canonical `document::Entity` enum and pins the document file schema version at `1`. It also retires the `pub const MODULE` placeholder that LCV-001 added to `src/document/mod.rs`.

User outcome: a single enum that all later demands consume, so an operator's drawing has one unambiguous in-memory and on-disk shape, and a schema version stamp that future format-incompatible changes can bump.

## Scope

- New file `src/geometry/`-style purity file `src/document/entity.rs` defining:
  - `pub enum Entity { Line(Line), Circle(Circle), Arc(Arc) }` with `#[derive(Copy, Clone, Debug, PartialEq)]` (no `Eq`/`Hash` because variants carry `f64`).
  - `impl Entity` with:
    - `pub fn bbox(&self) -> (Vec2, Vec2)` — delegates to the variant's `bbox` method (`Line::bbox`, `Circle::bbox`, `Arc::bbox`).
    - `pub fn kind_name(&self) -> &'static str` — returns `"line"`, `"circle"`, or `"arc"` exactly.
  - `pub const SCHEMA_VERSION: u32 = 1;` — canonical document-format schema version. Bumped only on incompatible serialization changes.
- New file `src/document/schema.rs` (≤50 LOC) — narrative rationale only. Contents:
  - Module doc comment explaining what `SCHEMA_VERSION` represents: the integer that names the on-disk envelope shape. The envelope sketch (informational, not binding code): a future `.lcd` file is `{ "schema_version": 1, "entities": [...] }`. Bumps happen when a serialization change would make a previous file unreadable without migration code. Adding a new optional field does not bump; renaming or removing a field does.
  - Re-exports `pub use super::entity::SCHEMA_VERSION;` so `document::schema::SCHEMA_VERSION` and `document::SCHEMA_VERSION` both work.
  - No code logic. No serde derives. This demand does not ship serialization.
- Update `src/document/mod.rs`:
  - Remove the `pub const MODULE: &str = "document";` placeholder line.
  - Add `pub mod entity;` and `pub mod schema;`.
  - Add `pub use entity::{Entity, SCHEMA_VERSION};`.
  - Keep the existing module header comment.
- Update `tests/skeleton.rs`:
  - Replace `&document::MODULE` in the tuple with `&document::SCHEMA_VERSION`. The test continues to compile-check that every top-level module is wired; the document module now contributes a real public item.
- Per-module unit tests under `#[cfg(test)] mod tests` in `entity.rs`. No new file under `tests/`.
- `entity.rs` and `schema.rs` MUST NOT import `egui`, `eframe`, or `rfd`. Each file stays ≤300 LOC. `entity.rs` may import `crate::geometry::{Vec2, Line, Circle, Arc}`. `schema.rs` only re-exports `SCHEMA_VERSION` and carries its rationale comment.

## Out of scope

- The `Document` struct (the container that holds `Vec<Entity>` plus selection) — owned by **LCV-021**.
- The `Command` trait and mutation contract — owned by **LCV-022**.
- Serialization / deserialization round-trip (`serde`, JSON, file I/O) — owned by **LCV-058** and the SVG / file demands; this demand only stamps the version constant.
- Entity IDs (a stable identifier surviving across history operations) — deferred; no current consumer requires it. Entities are addressed by their index in `Document::entities` until proven inadequate.
- Z-order, layers, colors per entity. The `cut`/`mark`/`engrave` preset coloring is applied at export time (LCV-056), not stored on the entity.
- Hatch, text, polyline, ellipse, spline variants. Polylines and text arrive in Phase 4 via tools that emit existing primitives; new variants need their own demands.
- Replacing the transitional `SnapEntity` from LCV-016 with `document::Entity` — that migration is a separate Phase-4 demand (LCV-054 area). LCV-020 explicitly does **not** touch `src/geometry/snap/*`.

## Acceptance criteria

1. `src/document/entity.rs` exists and defines `pub enum Entity { Line(Line), Circle(Circle), Arc(Arc) }` with `#[derive(Copy, Clone, Debug, PartialEq)]` (no `Eq`, no `Hash`).
2. `src/document/entity.rs` defines `pub const SCHEMA_VERSION: u32 = 1;` and a test asserts `assert_eq!(SCHEMA_VERSION, 1)`.
3. `Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(3.0, 4.0))).bbox()` returns the same `(Vec2, Vec2)` as `Line::new(Vec2::new(0.0, 0.0), Vec2::new(3.0, 4.0)).bbox()` (compared component-wise within `EPSILON`).
4. `Entity::Circle(Circle::new(Vec2::new(5.0, 5.0), 3.0)).bbox()` returns the same `(Vec2, Vec2)` as `Circle::new(Vec2::new(5.0, 5.0), 3.0).bbox()` (compared component-wise within `EPSILON`).
5. `Entity::Arc(arc).bbox()` returns the same `(Vec2, Vec2)` as `arc.bbox()` for an arbitrary `arc: Arc` (e.g., quarter arc from LCV-013), compared component-wise within `EPSILON`.
6. `Entity::Line(_).kind_name()` returns `"line"`; `Entity::Circle(_).kind_name()` returns `"circle"`; `Entity::Arc(_).kind_name()` returns `"arc"` — exact string match.
7. `src/document/schema.rs` exists and re-exports `SCHEMA_VERSION` so both `document::SCHEMA_VERSION` and `document::schema::SCHEMA_VERSION` resolve to the same constant.
8. `src/document/mod.rs` no longer contains `pub const MODULE: &str = "document";`. It declares `pub mod entity;`, `pub mod schema;`, and re-exports `entity::{Entity, SCHEMA_VERSION}`.
9. `tests/skeleton.rs` uses `&document::SCHEMA_VERSION` instead of `&document::MODULE` and continues to compile and pass (`cargo test --test skeleton` exits 0).
10. Kernel-purity: `grep -nE '^use (egui|eframe|rfd)' src/document/entity.rs src/document/schema.rs` returns no matches.
11. Size: `wc -l src/document/entity.rs` reports `<= 300`; `wc -l src/document/schema.rs` reports `<= 50`.
12. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: test `entity_variants_construct_via_literal` — instantiates `Entity::Line(...)`, `Entity::Circle(...)`, `Entity::Arc(...)` to prove the enum surface and field visibility.
- **Unit (AC 2)**: test `schema_version_is_one` — `assert_eq!(SCHEMA_VERSION, 1);`.
- **Unit (AC 3)**: test `entity_line_bbox_delegates_to_line_bbox`.
- **Unit (AC 4)**: test `entity_circle_bbox_delegates_to_circle_bbox`.
- **Unit (AC 5)**: test `entity_arc_bbox_delegates_to_arc_bbox` covering a quarter arc.
- **Unit (AC 6)**: test `kind_name_returns_expected_strings` covering all three variants.
- **Unit (AC 7)**: test `schema_version_reachable_via_schema_module` — `assert_eq!(crate::document::schema::SCHEMA_VERSION, crate::document::SCHEMA_VERSION);`.
- **Integration (AC 9)**: existing `tests/skeleton.rs` continues to pass after the swap; `cargo test --test skeleton` exits 0.
- **Static check (AC 10)**: `grep -nE '^use (egui|eframe|rfd)' src/document/entity.rs src/document/schema.rs` returns no matches.
- **Size check (AC 11)**: `wc -l src/document/entity.rs` reports `<= 300`; `wc -l src/document/schema.rs` reports `<= 50`.
- **Build gate (AC 12)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- The transitional `SnapEntity` enum in `src/geometry/snap/mod.rs` is **deliberately not touched** by this demand. LCV-016's notes already track that a follow-up demand (planned around LCV-054, snap integration) will replace `SnapEntity` with `document::Entity` and delete the transitional enum. Touching `snap/*` here would balloon the diff and entangle two unrelated migrations.
- `kind_name` returns lowercase ASCII so it is safe for telemetry, logs, and any future `serde` tag without further mapping.
- `SCHEMA_VERSION` is `u32` for headroom; a `u8` would also work but `u32` matches typical JSON integer ranges and avoids cast noise.
- Adding new variants (e.g., `Entity::Polyline`) is **not** a schema bump per se, but it makes older builds unable to read newer files. The schema bump rule is documented in `schema.rs`: anything that would prevent an older build from cleanly loading a newer file requires a bump.
- Reference: v1's `document/entity.ts` used a discriminated union `{ kind: 'line', p1, p2 } | ...`. v2 uses a Rust enum directly, which is more idiomatic and avoids stringly-typed match arms.
- AGENTS.md §"Units and types" already lists `Entity` as a canonical type declared in `src/document/entity.rs` — this demand satisfies that contract.
