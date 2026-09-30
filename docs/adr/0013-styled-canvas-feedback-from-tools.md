# ADR 0013 — Tools describe canvas feedback as styled marks; the app maps marks to render calls

- **Status**: Accepted
- **Date**: 2026-09-30
- **Deciders**: architect (LCV-163 /design)

## Context

`Tool::preview(&self) -> Vec<Entity>` is the only channel from a tool to the canvas, and
`render/preview.rs::draw_preview` strokes every item in one translucent amber. LCV-163 needs three
more forms: the crossing selection box dashed (window box solid), the entity under the pickbox in
its own layer colour and thicker (Select idle, TRIM, EXTEND), and what TRIM/ERASE will remove
dashed in a `danger` token, all in the paint order of LCV-163 AC 7 (halo, hover, preview, snap,
crosshair). Constraints:

- `preview()` has no `Document`: TRIM cannot compute its piece and a bare `Entity` has no layer,
  so a hover cannot find its colour.
- `preview()` has ~90 call sites in tool and integration tests; changing its return type is churn
  with no behaviour gain.
- `on_pointer_move` runs every hovered frame, after the Esc key gate, so state a tool caches there
  cannot honour "Esc hides the hover" (AC 9); and a cached entity index goes stale when a click
  commits in the same frame.
- `render/` and `tools/` do not import each other today; neither should start.
- AC 6: the hover and the click must agree, so both must come from one pick function on one point.

## Decision

1. **One new default trait method, a pure query at paint time** (`src/tools/tool.rs`):

   ```rust
   fn feedback(&self, doc: &Document, cursor: Option<Vec2>) -> Vec<Mark> {
       self.preview().into_iter().map(Mark::Preview).collect()
   }
   ```

   `cursor` is the world point `viewport.rs::handle_hover` resolved and sent as this frame's
   `PointerEvent::Move` (so picks agree with the click, AC 6), or `None` off the canvas or while
   muted (point 4). A tool returns `Hover`/`Danger` marks only for `Some(cursor)`. `preview()` stays
   as is: every tool that overrides nothing keeps today's picture, and liveness
   (`viewport.rs::viewport_is_live`) keeps reading `App::preview_entities`.

2. **`Mark` is plain data, egui-free** (`src/tools/feedback.rs`, re-exported from `tools/mod.rs`):

   ```rust
   pub enum Mark {
       Preview(Entity),   // solid `preview` token (rubber band, window box)
       Dashed(Entity),    // dashed `preview` token (crossing box)
       Hover(usize),      // doc.entities[i]: its layer colour, thicker stroke
       Danger(Entity),    // dashed `danger` token (TRIM removal, ERASE selection)
   }
   ```

   `Hover` carries an index, not a copy, because the colour is the layer's
   (`Document::layer_color`) and the index is computed from the same `&Document` the frame paints,
   so it cannot be stale. Styles are closed: a fifth form needs an amendment here.

3. **The app maps marks to render primitives; render never sees `Mark`.**
   `app/viewport/paint.rs::paint` calls `ToolManager::feedback` once, paints the `Hover` marks
   right after `draw_selection_highlight`, then every other mark in tool order, then the snap
   glyph and the crosshair (AC 7). Render gains two egui-only primitives taking kernel types:
   `render/selection.rs::draw_hover(painter, rect, camera, doc, index)` and
   `render/preview.rs::draw_dashed(painter, rect, camera, entity, color)` (egui 0.29
   `Shape::dashed_line` over the projected polyline; circles and arcs via `arc_polyline`).
   Colours come from `render/palette.rs` (AC 8), the single home of `preview`, `danger`, `hover`.

4. **Esc mutes feedback until the pointer moves** (AC 9), in `ToolManager` only: it records the
   last `Move` point; `handle_key(Escape)` stores it as `muted_at`; `feedback` passes `None` to the
   tool while `cursor == muted_at`. No tool and no `App` field is involved.

5. **Tools share one pick path with their click.** TRIM folds cutters in one helper that returns
   the cutter indices and the resulting entity; `on_pointer_down` turns the indices into
   `TrimEntity` commands and `feedback` derives the removed piece(s) from the same fold through a
   kernel-pure helper in `src/document/commands/trim/`. Select and EXTEND reuse their existing
   pick functions for `Hover`. ERASE returns `Danger` for each selected entity.

## Consequences

- The purity rule is untouched: `Mark` and the TRIM piece helper are egui-free; `render/` gains
  no dependency on `tools/`; `raster.rs` is not involved.
- `Tool` stays object-safe (no generics, `&self`). Tools that ignore the method are unchanged,
  and so are the existing `preview()` tests.
- `feedback` allocates a small `Vec` per frame and TRIM re-runs its cutter fold per hovered frame:
  O(entities) intersection tests, the same work as one click. Acceptable at LaserCAD sizes.
- The LCV-037 "translucency instead of dashes" rationale in `render/preview.rs` is obsolete
  (`dashed_line` exists in egui 0.29); its module doc is corrected with LCV-163.
- A Select override must restyle its own box (`Preview` vs `Dashed` by drag direction), so the
  box lives in both `preview()` (tests, liveness) and `feedback()`; both read `hit::box_preview`.
