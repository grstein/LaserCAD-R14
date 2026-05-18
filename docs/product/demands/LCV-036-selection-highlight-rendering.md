# LCV-036 — Selection highlight rendering

- **Status**: Ready
- **Phase**: 3
- **Depends on**: LCV-035, LCV-027
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: <to be filled by demand-manager>

## Problem

Phase 4's `SelectTool` (LCV-042), `MoveTool` (LCV-049), `DeleteTool` (LCV-052), and `TrimTool` (LCV-050) all operate on the document's `Selection` (LCV-027). The operator needs to *see* which entities are selected — otherwise they cannot confirm "the next 'delete' will remove the right things". AutoCAD R14 shows selected entities with a different color and grip handles; v2 takes the lightweight route: a colored "halo" stroke drawn over each selected entity. No grips, no rotation handles — just a clearly visible "this is selected" indication.

User outcome: the operator window-picks two lines and a circle; the three selected entities draw with a thicker, semi-transparent blue/cyan halo on top of their normal stroke. Pressing Escape clears the selection; the halo disappears.

## Scope

- New file `src/render/selection.rs` defining:
  - `pub fn draw_selection_highlight(painter: &egui::Painter, rect: egui::Rect, camera: &Camera, entities: &[Entity], selection: &Selection)`:
    - Walk `selection.iter()`; for each `idx`, if `idx < entities.len()`, draw a halo over `entities[idx]`. Out-of-range indices are silently skipped (defensive — index drift after a `DeleteEntities` followed by stale selection is a known gap per LCV-027 / LCV-024 notes, not a panic).
    - Halo stroke: `Stroke::new(3.0, Color32::from_rgba_unmultiplied(64, 160, 255, 180))` — 3-pixel cyan-blue at ~70% alpha. The semi-transparency lets the underlying entity stroke (1 px, light gray, from LCV-035) remain visible through the halo, giving a "double-stroke" look that reads clearly as "selected".
    - Drawing dispatch is the **same shape** as `draw_entities` (LCV-035): line → `line_segment`, circle → `circle_stroke`, arc → `arc_polyline` + line segments. The implementer is encouraged to factor common code: extract a `fn draw_entity_with_stroke(painter, rect, camera, entity, stroke)` shared between LCV-035 and LCV-036. The location of that helper is open — `src/render/entities.rs` (cleaner imports) or `src/render/draw.rs` (a new tiny shared module). Implementer's choice; both keep LOC under cap.
    - **Order of operations**: caller invokes `draw_selection_highlight` AFTER `draw_entities` so the halo overlays the normal stroke. Documented in the function's `///` doc-comment.
  - The function takes `&Selection` (not `&mut`) and only reads.
- Update `src/render/mod.rs`:
  - Add `pub mod selection;`.
  - Add `pub use selection::draw_selection_highlight;`.
- Update `src/app.rs::update`:
  - Call `draw_selection_highlight(&painter, rect, &self.camera, &self.document.entities, &self.document.selection);` after `draw_entities` and before any future preview / snap-marker calls.
- File size: `src/render/selection.rs` stays ≤300 LOC. (Expected: ~80–120 LOC including helper if not factored out.)
- The file imports `egui::{Painter, Pos2, Rect, Stroke, Color32}` and `crate::geometry::Vec2`, `crate::document::{Entity, Selection}`, `crate::render::Camera`. No `eframe`, no `rfd`.
- Per-module unit tests under `#[cfg(test)] mod tests` in `selection.rs`.

## Out of scope

- **Grip handles** on selected entities (drag-to-resize squares at endpoints / midpoints). AutoCAD has them; v2 does not in Phase 3. Future demand if needed.
- **Rotation handles**. Not in v2.
- **Bounding-box outline** around the entire selection (the dashed cyan rect AutoCAD draws around grouped selection). v2 highlights per-entity; group bbox is a Phase-6+ visual-polish demand if requested.
- **Dimension annotations** ("12.34 mm" labels showing the selected entity's length). Not in v2.
- **Animation** (pulsing halo, fade in/out). Static halo is sufficient and matches "deterministic geometry over visual convenience" (AGENTS.md §"Product Philosophy").
- **Picking semantics** (which entity is under the cursor when clicking) — owned by **LCV-042**. This demand only draws a halo on whatever `Selection` currently holds.
- **Selection-color preference**. Hard-coded cyan-blue. Theme demand (LCV-071) can override later.
- **Multi-color selection** (different halo color for different entity types). Single color; KISS.

## Acceptance criteria

1. `src/render/selection.rs` exists and defines `pub fn draw_selection_highlight(painter: &egui::Painter, rect: egui::Rect, camera: &Camera, entities: &[Entity], selection: &Selection)` with that exact signature.
2. **Empty selection is a no-op**: with `selection = Selection::default()` (empty), the function returns without making any `Painter` draw calls. (Verified via a test-only counter — see Expected tests.)
3. **Out-of-range indices are skipped**: with `entities = [Entity::Line(...)]` (length 1) and `selection.add(5)` (index 5, out of range), the function does not panic and does not draw anything. (Verified via the counter.)
4. **Per-selected-entity dispatch**: with `entities = [line, circle, arc]` and `selection = {0, 2}`, the function visits indices 0 and 2 exactly once each and does not visit index 1. (Verified by injecting a test-only dispatch trace: either expose a `pub(crate) fn selected_entity_indices<'a>(entities: &'a [Entity], selection: &'a Selection) -> impl Iterator<Item = (usize, &'a Entity)> + 'a` helper and unit-test the iterator, or pass a closure-based callback to the draw function. Implementer picks the cleaner factoring.)
5. **Halo stroke is thicker than entity stroke**: the halo's stroke width is strictly greater than `PaintOptions::default().stroke.width` (1.0). Encoded as: a test that constructs the halo stroke (via either a `pub(crate) const HALO_STROKE: Stroke` constant in the module, or a `pub(crate) fn halo_stroke() -> Stroke` helper) and asserts `halo_stroke.width > 1.0`.
6. **Halo color has non-zero alpha and is not gray**: the halo color is encoded somewhere accessible to tests; a test asserts the color's alpha is in `(0, 255)` (semi-transparent) and that R ≠ G or G ≠ B (not a gray). The specific RGB values are not tested; only the "translucent colored" property.
7. `src/render/mod.rs` re-exports `draw_selection_highlight`.
8. `src/app.rs::update` calls `draw_selection_highlight` once per frame after `draw_entities`.
9. **Manual smoke**: with `cargo run` and a debug-only test fixture that places one entity in the document and adds its index to the selection, the entity renders with a visible blue/cyan halo over its normal light-gray stroke. (Alternative: defer to LCV-042 acceptance, when SelectTool exists.)
10. The file imports nothing from `eframe` or `rfd`. (Importing `egui` is allowed.)
11. Size: `wc -l src/render/selection.rs` reports `<= 300`.
12. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: test `draw_selection_highlight_function_exists` — calls `draw_selection_highlight` with empty inputs; verifies the call type-checks.
- **Unit (AC 2)**: test `empty_selection_visits_no_entities` — uses the trace helper from AC 4 to assert the visited count is 0.
- **Unit (AC 3)**: test `out_of_range_indices_are_skipped` — `entities = [line]`, `selection = {5}`, assert visited count is 0 and no panic.
- **Unit (AC 4)**: test `selected_indices_iterator_yields_correct_pairs` — `entities = [line, circle, arc]`, `selection.add(0); selection.add(2);`, asserts the iterator yields exactly `[(0, &line), (2, &arc)]` (order may vary because `Selection` is a HashSet — collect to a `Vec` and sort by index before asserting).
- **Unit (AC 5)**: test `halo_stroke_is_thicker_than_entity_stroke` — accesses the halo stroke (via constant or helper), asserts `width > 1.0`.
- **Unit (AC 6)**: test `halo_color_is_translucent_and_colored` — asserts alpha in (0, 255) and that the color is not a pure gray.
- **Manual smoke (AC 9)**: deferred to LCV-042 acceptance, OR implemented via a debug-only fixture as described.
- **Static check (AC 10)**: `grep -nE '^use (eframe|rfd)' src/render/selection.rs` returns no matches.
- **Size check (AC 11)**: `wc -l src/render/selection.rs` reports `<= 300`.
- **Build gate (AC 12)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **Halo over normal stroke**: drawing order is `entities → selection_highlight`, which means the halo paints over the entity stroke. The translucent halo lets the underlying stroke peek through, producing a "double-stroke" effect that reads as "this is selected". An alternative — drawing a thicker halo first and then the normal entity stroke on top — would hide the halo edges; rejected.
- **Index-vs-id**: `Selection` holds `HashSet<usize>` indices into `Document.entities` (LCV-027 design). After a `DeleteEntities`, indices may shift; LCV-024/LCV-027 documented that LCV-052 (DeleteTool) is responsible for cleaning up. Until then, out-of-range indices can appear in the wild. AC 3 mandates the renderer ignores them silently rather than panicking.
- **Shared dispatch helper**: factoring `draw_entity_with_stroke` reduces duplication between LCV-035 (default stroke) and LCV-036 (halo). The implementer can land this refactor in LCV-036 (since this is when the duplication first appears). If LCV-035 has not yet been merged when LCV-036 is implemented, the refactor lands in LCV-035 instead.
- **Cyan-blue color** (`rgba(64, 160, 255, 180)`): high contrast against dark gray canvas and against light gray entity strokes. Matches v1's selection color. Not a final theme decision (LCV-071 owns that).
- **3 px halo width**: thick enough to read clearly without obscuring small detail. At extreme zoom-in (mm_per_px = 0.001), 3 px is ~3 µm in world units, which still feels like a thin halo. At extreme zoom-out, 3 px is the same regardless — the halo stays a fixed pixel width, not a fixed mm width. This matches AutoCAD R14 behavior.
- **No selection bounding-box**: an additional dashed rectangle around the entire selection (AutoCAD's grouped selection visual) is reserved for a future Phase-6 visual-polish demand if requested. The per-entity halo is sufficient signal.
- Reference: v1's `tools/SelectionRenderer.ts` drew a thick translucent stroke over each selected entity; same approach here.
