# LCV-021 — Document struct + bounds + Default

- **Status**: Done
- **Phase**: 2
- **Depends on**: LCV-020
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: implementer-rust (task #68); 0bf5943

## Problem

The operator's drawing — the in-memory representation of "every line, circle, and arc they have placed plus whatever they currently have selected" — needs a single owning type. Without it, the render pipeline (Phase 3) has no source to paint, the SVG exporter (LCV-056) has nothing to walk, the autosave loop (LCV-059) has nothing to snapshot, and the zoom-extents action (part of LCV-031) has no bounding rectangle to fit.

This demand introduces `document::Document` and gives it the two methods every immediate consumer needs: a `Default` empty document and a `bounds()` that returns the union bbox over its entities (or `None` when empty). It also declares the `Selection` placeholder type that LCV-027 will fill, so `Document` can carry it from day one.

User outcome: the application has a single value that means "the drawing the user is editing". When the user clicks "Zoom Extents" (Phase 3) the camera fits this document's `bounds()`; when the user has no entities yet, the camera falls back to the default view because `bounds()` returned `None`.

## Scope

- New file `src/document/state.rs` (named `state.rs` rather than `document.rs` to avoid the awkward `document::document::Document` path) defining:
  - `pub struct Document { pub entities: Vec<Entity>, pub selection: Selection }`.
    - Derives: `Debug`, `Default` (manual or via `derive`, either is acceptable as long as `Document::default()` yields an empty document with an empty selection).
    - No `Copy`, no `Clone` (avoid accidental cheap snapshots; if a future demand needs cloning, it can derive `Clone` then).
  - Public methods on `Document`:
    - `pub fn bounds(&self) -> Option<(Vec2, Vec2)>` — `None` when `entities` is empty. Otherwise the axis-aligned union of every entity's `bbox()` (component-wise `min` for the lower-left, component-wise `max` for the upper-right).
    - `pub fn entity_count(&self) -> usize` — convenience wrapper for `self.entities.len()`. Used by the status bar (Phase 6) and tests.
  - `pub struct Selection { /* private fields, to be filled by LCV-027 */ }`.
    - Derives: `Debug`, `Default`. LCV-027 will fill the fields and the implementation; this demand only declares the type so `Document` can own it without circular waits.
    - The body of this struct in LCV-021 is intentionally minimal — either an empty struct `pub struct Selection;` or a struct with one private placeholder field. LCV-027 takes ownership of expanding it.
  - File-level doc comment (`//!`) explaining the `state.rs` filename choice and that `Selection` is a placeholder filled by LCV-027.
- Update `src/document/mod.rs`:
  - Add `pub mod state;`.
  - Add `pub use state::{Document, Selection};`.
- `src/document/state.rs` MUST NOT import `egui`, `eframe`, or `rfd`. The file stays ≤300 LOC. It may import from `crate::document::Entity` and `crate::geometry::Vec2`.
- Per-module unit tests under `#[cfg(test)] mod tests` in `state.rs`. No new file under `tests/`.

## Out of scope

- Command-driven mutation. `Document::entities` is `pub` for this demand because no `Command` trait exists yet; LCV-022 / LCV-026 enforce that all mutation goes through commands and the history stack. Tools and the agent will not push to `entities` directly — they go through `Command`. (The field visibility may tighten to `pub(crate)` later; not in this demand.)
- The `Command` trait — owned by **LCV-022**.
- Selection logic (`is_selected`, `add`, `remove`, iteration, `SelectionCommand`) — owned by **LCV-027**. LCV-021 only declares the placeholder type.
- Serialization (`serde` derives, JSON round-trip) — owned by **LCV-058** and the file-I/O demands.
- Entity IDs / stable identifiers — deferred. Entities are addressed by their `Vec` index until proven inadequate.
- Z-order, layers, colors per entity.
- Per-entity metadata (creation time, layer, color override). Cut/mark/engrave preset assignment happens at export time (LCV-056), not on the entity.
- A "dirty" flag or change tracking — autosave (LCV-059) decides its own debounce policy.

## Acceptance criteria

1. `src/document/state.rs` exists and defines `pub struct Document { pub entities: Vec<Entity>, pub selection: Selection }` with `Debug` and `Default` derived (or manually implemented).
2. `src/document/state.rs` defines `pub struct Selection { /* placeholder */ }` with `Debug` and `Default`. The exact internal fields are LCV-027's concern; this demand only requires the type name, public visibility, and that `Selection::default()` compiles.
3. `Document::default().entities.is_empty()` returns `true`, and `Document::default().entity_count()` returns `0`.
4. `Document::default().bounds()` returns `None`.
5. After pushing one `Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 5.0)))` into `entities`, `bounds()` returns `Some((Vec2::new(0.0, 0.0), Vec2::new(10.0, 5.0)))` (within `EPSILON`).
6. After pushing a second entity `Entity::Circle(Circle::new(Vec2::new(20.0, 20.0), 3.0))` into `entities`, `bounds()` returns `Some((Vec2::new(0.0, 0.0), Vec2::new(23.0, 23.0)))` (within `EPSILON`) — the union of the line bbox and the circle bbox.
7. `entity_count()` equals `entities.len()` for an arbitrary mix (tested with 0, 1, and 3 entities).
8. `src/document/mod.rs` re-exports `Document` and `Selection` so `use lasercad::document::{Document, Selection};` compiles from outside the module.
9. Kernel-purity: `grep -nE '^use (egui|eframe|rfd)' src/document/state.rs` returns no matches.
10. Size: `wc -l src/document/state.rs` reports `<= 300`.
11. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: test `document_struct_literal_construction` — instantiates `Document { entities: Vec::new(), selection: Selection::default() }` via struct literal.
- **Unit (AC 2)**: test `selection_default_constructs` — `let _ = Selection::default();`.
- **Unit (AC 3)**: test `default_document_is_empty` — covers `entities.is_empty()` and `entity_count() == 0`.
- **Unit (AC 4)**: test `default_document_bounds_is_none`.
- **Unit (AC 5)**: test `single_line_bounds` — pushes one line, asserts `bounds()` matches the line's bbox.
- **Unit (AC 6)**: test `union_bounds_over_two_entities` — pushes a line and a circle, asserts the union bbox.
- **Unit (AC 7)**: test `entity_count_matches_vec_len` — covers 0, 1, 3 entities.
- **Integration (AC 8)**: a small integration test under `tests/` (or extend `tests/skeleton.rs`) that does `use lasercad::document::{Document, Selection};` and constructs a default `Document` — proves the re-export. Manual smoke is acceptable if added inline to `skeleton.rs`.
- **Static check (AC 9)**: `grep -nE '^use (egui|eframe|rfd)' src/document/state.rs` returns no matches.
- **Size check (AC 10)**: `wc -l src/document/state.rs` reports `<= 300`.
- **Build gate (AC 11)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **Naming**: the file is `state.rs` (not `document.rs`) so the module path is `document::state::Document`, not the noisy `document::document::Document`. The `mod.rs` re-exports flatten this to `document::Document` for callers.
- `entities: Vec<Entity>` is `pub` in this demand because no enforcement mechanism (the `Command` trait + history stack) exists yet. LCV-022 + LCV-026 land the invariant that "all entity mutation goes through `Command`". This demand does not relitigate that contract — it just makes the field accessible so LCV-022 has something to operate on. A future demand may tighten visibility to `pub(crate)` if it proves necessary; that change is not in scope here.
- `bounds()` returns `Option<(Vec2, Vec2)>` (not `Rect`, even though LCV-015 added a `Rect` type) because the zoom-extents consumer (LCV-031) and SVG export consumer (LCV-056) both work in `(min, max)` tuples; introducing a `Rect` here would force every consumer to import an extra type. If a future consumer prefers `Rect`, it can wrap.
- `Selection` is declared here, populated in LCV-027. This avoids two synchronization issues: (a) `Document` cannot exist without owning its selection state, (b) LCV-027 should not have to also restructure `Document`. The contract: LCV-027 may extend `Selection`'s fields and methods freely; LCV-021's tests must still pass.
- Reference: v1's `document/Document.ts` was a similar plain struct with `entities` and `selection` plus history; v2 keeps the same shape but separates history (LCV-026) out of `Document` itself.
- AGENTS.md §"State and mutation (hard contract)" applies the moment LCV-022 lands. This demand is the last one where `entities.push` is allowed in test code outside `commands::*`.
