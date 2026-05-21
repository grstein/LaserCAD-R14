# LCV-037 — Preview overlay (live tool preview)

- **Status**: Done
- **Phase**: 3
- **Depends on**: LCV-035
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: feat(LCV-037): preview overlay — translucent amber entities for live tool preview (6530bad)

## Problem

Drawing a line in AutoCAD R14 works like this: the operator clicks the first point, then a "rubber-band" preview follows the cursor showing what the line will look like; the second click commits. Same for circles (radius rubber-band from center), arcs (three-point preview), polylines (each segment previews). Without a preview, drawing is "click blindly, see result, undo, try again" — unusable.

This demand ships the preview painter: a function the tools (LCV-043 LineTool, LCV-046 CircleTool, LCV-047 ArcTool, LCV-044 PolylineTool, LCV-045 RectTool) call each frame with a `Vec<Entity>` representing the in-progress preview. The renderer paints those entities with a distinct "preview" style — visually different from committed entities so the operator can tell what is real and what is provisional.

User outcome (delivered when Phase 4 tools land on top of this): the operator clicks once to start a line; as they move the cursor, a thin translucent line tracks from the first click to the current cursor position. The line is committed on the second click and switches to the normal entity stroke; the preview disappears.

## Scope

- New file `src/render/preview.rs` defining:
  - `pub fn draw_preview(painter: &egui::Painter, rect: egui::Rect, camera: &Camera, preview_entities: &[Entity])`:
    - For each entity in `preview_entities`, dispatch on variant (line, circle, arc) and draw with a **preview stroke**. The dispatch logic mirrors LCV-035's `draw_entities`; the implementer is encouraged to reuse the shared helper introduced in LCV-035 / LCV-036 (`draw_entity_with_stroke` or equivalent) rather than duplicate.
    - **Preview stroke**: `Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 220, 100, 160))` — 1-pixel translucent amber/yellow. Choice rationale:
      - **Amber** distinguishes from the light-gray committed stroke (LCV-035) and the cyan-blue selection halo (LCV-036). Three visual channels: gray = committed, cyan = selected, amber = preview.
      - **Translucent (160/255 alpha)** signals "this isn't real yet". The semi-transparency mimics dashed lines without requiring dash-pattern support (egui's `Painter::line_segment` does not natively support dashes; emulating them requires shape composition that's not worth the LOC).
      - **1-pixel width** matches the committed stroke; the operator can directly compare the preview's footprint to what it will look like once committed.
    - **No dashed pattern**: AutoCAD R14 uses dashed previews; v2 uses translucency. Documented in code. If a future demand insists on dashes, it factors `Painter::add(Shape::dashed_line(...))` from `egui` (if available in the pinned version) or polyline-with-gaps. Not in scope here.
    - **Empty input**: `draw_preview(_, _, _, &[])` is a no-op — no draw calls, no panic. Tools that have nothing to preview (e.g., before the first click) pass an empty slice.
  - **No state on the function**: `draw_preview` is stateless; the caller (`App::update`) owns the preview `Vec<Entity>`. The tool layer constructs and discards it per frame.
  - **Order of operations**: caller invokes `draw_preview` AFTER `draw_entities` and `draw_selection_highlight` and BEFORE `draw_snap_marker` (LCV-038). The preview should sit visually above selection halos (the operator's attention is on what they're about to commit) but below snap markers (snap markers are the most important cursor-anchored feedback). Documented in the function's `///` doc-comment.
- Update `src/render/mod.rs`:
  - Add `pub mod preview;`.
  - Add `pub use preview::draw_preview;`.
- Update `src/app.rs`:
  - Add `pub preview_entities: Vec<Entity>` to the `App` struct. Defaults to empty (`vec![]`) via `#[derive(Default)]`.
  - In `App::update`, call `draw_preview(&painter, rect, &self.camera, &self.preview_entities);` in the right order. The field is `pub` so the Phase-4 Tool trait (LCV-040) can mutate it on each pointer-move event without going through a setter.
  - The Phase-4 Tool trait (LCV-040) will define a `fn preview(&self) -> Vec<Entity>` method that each tool implements; the ToolManager (LCV-040) writes `app.preview_entities = active_tool.preview()` each frame. **That wiring is LCV-040's responsibility**, not this demand's. In LCV-037 the field stays empty and the preview painter draws nothing — this demand only proves the painter works and the field is in place.
- File size: `src/render/preview.rs` stays ≤300 LOC.
- The file imports `egui::{Painter, Pos2, Rect, Stroke, Color32}` and `crate::geometry::Vec2`, `crate::document::Entity`, `crate::render::Camera`. No `eframe`, no `rfd`.
- Per-module unit tests under `#[cfg(test)] mod tests` in `preview.rs`.

## Out of scope

- **Tool-specific previews** (the geometry the LineTool / CircleTool / etc. construct each frame): owned by **LCV-043 / LCV-044 / LCV-045 / LCV-046 / LCV-047** in Phase 4.
- **The Tool trait** (defining `fn preview(&self) -> Vec<Entity>`): owned by **LCV-040**.
- **Tool dispatch** (which tool is active, who writes `app.preview_entities`): owned by **LCV-040** (ToolManager) and **LCV-041** (pointer plumbing).
- **Animation** (animated dashes / "marching ants" on the preview). v2 is static; rejected.
- **Multiple preview channels** (one tool preview + one snap projection line + one ortho hint, each in a different style). Phase 4 demands compose into a single `Vec<Entity>` if they need to; the renderer does not multi-channel.
- **Persistent preview** (showing a preview when no tool is active). The field is empty unless a tool writes it.
- **Motion blur / trail effects**. Rejected (decorative).

## Acceptance criteria

1. `src/render/preview.rs` exists and defines `pub fn draw_preview(painter: &egui::Painter, rect: egui::Rect, camera: &Camera, preview_entities: &[Entity])` with that exact signature.
2. **Empty input is a no-op**: `draw_preview(painter, rect, camera, &[])` returns without making any `Painter` draw calls. (Verified via a test-only counter / trace helper.)
3. **Non-empty input dispatches per entity**: with `preview_entities = [line, circle, arc]` the function visits each entity exactly once. (Verified via the trace helper.)
4. **Preview stroke is visually distinct from entity and selection strokes**: the preview color (encoded as `pub(crate) const PREVIEW_STROKE: Stroke` constant in the module, or a `pub(crate) fn preview_stroke() -> Stroke` helper) has alpha strictly less than 255 (translucent) AND has a hue that is neither the entity's gray nor the selection's cyan-blue. Encoded as a unit test: `let s = preview_stroke(); assert!(s.color.a() < 255); assert!(s.color.r() > 200 && s.color.g() > 150 && s.color.b() < 200);` (i.e., yellow/amber-ish — the test pins the band, not the exact value, so the theme can be tuned without breaking the test).
5. **draw_preview does not panic** on: empty slice, one entity of each variant, degenerate camera (`mm_per_px = 1e-9`, `1e9`). No automated rendering assert is required.
6. `src/render/mod.rs` re-exports `draw_preview`.
7. `src/app.rs` `App` struct carries `pub preview_entities: Vec<Entity>` which defaults to empty. `App::default().preview_entities.is_empty()` is `true`.
8. `src/app.rs::update` calls `draw_preview` once per frame, in the documented order (after `draw_entities` and `draw_selection_highlight`).
9. The file imports nothing from `eframe` or `rfd`. (Importing `egui` is allowed.)
10. Size: `wc -l src/render/preview.rs` reports `<= 300`.
11. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

- **Unit (AC 1)**: test `draw_preview_function_exists` — calls `draw_preview` with empty slice; verifies signature compiles.
- **Unit (AC 2)**: test `empty_preview_visits_no_entities` — uses the trace helper to confirm zero visits.
- **Unit (AC 3)**: test `non_empty_preview_visits_each_entity_once` — constructs `[line, circle, arc]`, asserts the trace yields all three exactly once.
- **Unit (AC 4)**: test `preview_stroke_is_translucent_and_distinct` — asserts alpha < 255 and color falls in the amber band as described.
- **Unit (AC 5)**: test `draw_preview_does_not_panic_on_degenerate_camera` — same pattern as LCV-035 AC 9.
- **Integration (AC 7)**: extend `tests/skeleton.rs` (or a new integration test) to construct `lasercad::app::App::default()` and assert `app.preview_entities.is_empty()`. Proves the field is wired through the public surface.
- **Static check (AC 9)**: `grep -nE '^use (eframe|rfd)' src/render/preview.rs` returns no matches.
- **Size check (AC 10)**: `wc -l src/render/preview.rs` reports `<= 300`.
- **Build gate (AC 11)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **Three visual channels** (gray = committed, cyan = selected, amber = preview): give the operator unambiguous color coding. A fourth channel (snap markers — LCV-038) uses orange, which is similar to amber but rendered as a marker shape, not a stroke. Two amber/orange items at once would be confusing — but snap markers only appear at a single point (the cursor) while previews extend across the drawing, so the visual confusion is bounded.
- **Translucency instead of dashes**: emulating dashed lines in egui's immediate-mode `Painter` requires either `Shape::dashed_line` (if available in the pinned egui version — verify in `Cargo.lock`) or hand-built polyline segmentation. Both are more LOC than a single alpha-blended stroke. v2 picks translucency for KISS; if a user complains the preview is too subtle, a future demand can swap to dashes.
- **`Vec<Entity>` not `Box<dyn Entity>`**: `Entity` is already an enum (LCV-020), not a trait. Passing `Vec<Entity>` keeps the API simple and avoids dynamic dispatch.
- **Tool ownership of `preview_entities`**: each tool produces its own preview. The line tool produces one `Line`; the polyline tool produces N `Line`s (all completed segments + the in-progress one); the circle tool produces one `Circle`; the arc tool produces one `Arc`. All fit in `Vec<Entity>` without trait juggling. LCV-040 (Tool trait) formalizes this.
- **Order of operations** (entities → selection → preview → snaps): the preview is more important than the selection halo (the operator is actively focused on the preview during a draw), so the preview sits visually above. Snap markers sit at the very top (small, cursor-anchored, the most important micro-feedback). LCV-038 (snap markers) confirms the order.
- **State on `App`, not on Tool**: the preview lives on `App` because the renderer is decoupled from the tool layer. ToolManager (LCV-040) writes; renderer reads. The tool itself can be stateless or stateful; the wire is on `App`.
- **No "what tool is previewing" annotation**: the preview overlay does not include a label saying "drawing line". The toolbar (Phase 6 LCV-066) and statusbar (LCV-067) carry that information; the viewport stays clean.
- Reference: v1's `tools/PreviewRenderer.ts` drew previews with a dashed amber stroke. v2 substitutes translucency for the dashes; same intent.
