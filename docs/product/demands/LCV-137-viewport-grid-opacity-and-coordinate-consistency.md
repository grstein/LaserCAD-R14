# LCV-137 - Visible grid and consistent viewport coordinates

- **Status**: Draft
- **Phase**: 11
- **Depends on**: LCV-032, LCV-033, LCV-034, LCV-120
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: -

## Problem

Three confirmed defects in `src/app/viewport.rs` and its renderers, all
stemming from the same root cause — mixing the *application-global* screen
position (where the `CentralPanel`'s `rect` sits inside the whole window,
after the menubar/toolbar/statusbar claim their own space) with the
*viewport-local* one `Camera::world_to_screen` / `Camera::screen_to_world`
actually expect (origin at the canvas's own top-left):

1. **The grid disappears under the bed.** `src/app/viewport.rs::paint` calls
   `draw_grid` and then `draw_bed`; `draw_bed` fills an opaque rectangle over
   the whole bed area. Whatever grid the first call drew inside the bed, the
   second call paints over. `src/render/bed.rs::draw_bed`'s own doc comment
   ("must be called AFTER `draw_grid`") describes the current, broken call
   order as if it were the contract.
2. **The grid's visible range drifts with panel layout.**
   `src/render/grid.rs::draw_grid` computes its world-space bounds as
   `camera.screen_to_world(rect.min)` / `camera.screen_to_world(rect.max)` —
   passing the *global* panel rect straight into a function that treats its
   argument as *local* to the viewport (origin at the canvas's own top-left).
   Every other caller in the same file corrects for this — `handle_hover`
   subtracts `rect.min` before unprojecting, and every projected grid *point*
   still gets `+ offset` added back before painting — but the *bounds*
   computation skips the correction entirely. At the default 1 mm-per-px zoom
   the resulting shift equals the toolbar width and menubar height in world
   millimeters, which is not a rounding error: it is enough to leave a visible
   strip of canvas with no grid lines near the panel's near edges. (Not
   audited here, but read the same way: the wheel-zoom regression below is
   the same bug in a different function.)
3. **Wheel-zoom anchoring uses the wrong origin.**
   `src/app/viewport.rs::handle_hover` calls
   `handle_wheel_zoom(&mut app.camera, hover_pos, factor)` with `hover_pos` —
   the pointer's *global* position — while `Camera::zoom_around` treats its
   `screen_anchor` argument as *local*. The world point under the cursor is
   pinned to the wrong point whenever the viewport does not start at the
   window's top-left corner (i.e. always, since the toolbar and menubar
   always claim space first) — worse with an open agent panel, which moves
   the canvas's own origin further from `(0, 0)`.
4. **`Camera::pan`'s Y sign fights its own Y-up convention.** Given the
   documented `world_to_screen` transform (`screen_y = half_y - (world.y -
   center.y) / mm_per_px`), a middle-drag delta `d` (screen px, positive
   downward) that updates `center_world.y -= d * mm_per_px` moves every fixed
   world point *up* on screen when the operator drags *down* — algebraically
   derivable from the same formula the renderer uses, not a matter of taste.
   `center_world.x -= d.x * mm_per_px` has the opposite (correct) relationship
   on X, which is why horizontal pan already tracks the cursor and vertical
   pan does not.

## Scope

Correct the existing rendering order and the four coordinate-boundary bugs
above, preserving grid spacing, exterior dimming, geometry rendering,
preview, snap and every existing navigation gesture's *shape* (wheel zooms
around the cursor, middle-drag pans, `F` fits extents).

## Out of scope

New grid/theme controls, rulers, automatic camera fitting, SVG changes, a
rendering-pipeline rewrite, or any new `ctx.request_repaint*` call site (the
tree has exactly three, pinned by
`src/app/viewport.rs::every_repaint_request_in_src_is_conditional`; this
demand must still report exactly three after the fix).

## Acceptance criteria

1. `src/app/viewport.rs::paint` paints, in this order: canvas background,
   bed background fill, the grid (when `grid_enabled`), the bed border and
   exterior overlay, entities, selection, preview, and the snap marker.
2. With `grid_enabled == true`, minor and major grid lines are visible
   *inside* the bed rectangle (not painted over by the bed's own fill);
   toggling `grid_enabled` off removes the grid lines but leaves the bed
   fill, border and exterior overlay unchanged. Exterior dimming (the four
   overlay strips) still fully covers the area outside the bed.
3. `draw_grid`'s world-bounds computation and `handle_wheel_zoom`'s anchor
   each subtract the viewport's origin (`rect.min`) from a global screen
   position exactly once before calling `Camera::screen_to_world` /
   `Camera::zoom_around` — matching the existing, already-correct pattern in
   `handle_hover`'s pointer-unprojection line and in `resolve_snap`. Every
   projected *point* handed back to the painter still adds that same origin
   back exactly once.
4. Wheel-zoom keeps the world point under the pointer within 0.5 logical
   point of its pre-zoom position after reprojection, checked with the
   `CentralPanel`'s rect starting at a nonzero screen X and Y (toolbar +
   menubar space) and again with the agent panel open (narrowing the canvas
   further).
5. A line entity placed at a known millimeter coordinate that falls on a
   grid intersection has a grid line painted through its screen position
   (within 0.5 logical point), checked at three different `mm_per_px` zoom
   levels and with the `CentralPanel` origin at both `(0, 0)` and a nonzero
   offset.
6. `Camera::pan` implements `center_world.x -= dx * mm_per_px` and
   `center_world.y += dy * mm_per_px` (sign flipped from today on Y only). A
   middle-drag with a positive, a negative and a diagonal screen delta each
   move a known world point's projected screen position in the same
   direction as the drag, on both axes.
7. After any navigation gesture (wheel zoom, middle-drag pan, `F` fit
   extents), the status-bar cursor readout, the active tool's preview, the
   active snap marker and the next click's committed endpoint all agree on
   the same world point for the same screen input. Navigation alone never
   mutates `Document`/`History`, never changes the bytes an SVG export would
   produce, and adds no new `ctx.request_repaint*` call site.

## Expected tests

- AC 1, 2: a real-frame test driving `App::update_ui` and inspecting the
  `FullOutput.shapes` egui actually painted — identifying the bed's fill
  `Shape::Rect` and the grid's `Shape::LineSegment`s by their computed
  geometry (the bed's known screen rect; a grid line's known endpoints), not
  by color — and asserting their relative emission order (paint order, per
  `tests/harness/paint.rs`'s module doc, trap 1). A control that fails when
  the call order in `paint` is reverted. `grid_enabled` on/off is a second
  case in the same test. This is a rendering claim; a source scan of call
  order alone does not satisfy it (AGENTS.md).
- AC 3: a source scan (mirroring the style of
  `src/app/viewport.rs`'s existing `the_live_predicate_has_exactly_three_terms`
  -style tests) proving `draw_grid` and `handle_wheel_zoom`'s call sites each
  subtract `rect.min`/the viewport origin before the camera call.
- AC 4: real-`App` wheel-zoom regressions with the `CentralPanel` rect
  starting at `(0, 0)`, at a representative nonzero `(x, y)`, and with the
  agent panel open, each asserting the pre/post world position under the
  cursor.
- AC 5: a committed line at known mm coordinates chosen to land on a grid
  intersection at the tested zoom, checked against `draw_grid`'s own
  line-position formula (`Camera::world_to_screen` plus the local/global
  offset), across the three zoom levels and both rect origins.
- AC 6: `Camera::pan` unit tests for positive, negative and diagonal deltas,
  plus a real-`App` middle-drag-pan regression asserting a known entity's
  projected position moves the same direction as the drag on both axes.
- AC 7: hover/snap/click assertions after each navigation gesture; a
  `history.revision()` and SVG-export-bytes comparison before/after
  navigation; and the existing LCV-120 idle-repaint regressions
  (`tests/lcv120_idle_repaint.rs`) plus
  `every_repaint_request_in_src_is_conditional` still passing unmodified in
  count.
- Manual smoke: grid visible inside the bed and dimmed outside it at
  default zoom; a diagonal middle-drag pan follows the cursor; wheel zoom
  stays pinned to the cursor with the agent panel open.

## Open questions

None. Each of the four bugs above is confirmed directly from source (not
inferred from behavior), so there is no product ambiguity — only the
implementation work of fixing each boundary independently rather than
letting two wrong conversions cancel out.

## Notes

Primary files: `src/app/viewport.rs`, `src/render/camera.rs`,
`src/render/grid.rs`, `src/render/bed.rs`, `src/render/mod.rs`.

`src/render/bed.rs::draw_bed`'s doc comment ("Call order: this function must
be called AFTER `draw_grid`") is not evidence the order is correct — it
describes the very call order this demand changes, and needs correcting
alongside the code. Likewise, `Camera::pan`'s own doc comment argues for the
current (wrong) Y sign at length; it is not evidence either, for the same
reason. Fixing AC 1/2 likely means splitting `draw_bed` into a background-fill
half (called before the grid) and a border-plus-overlay half (called after);
that split is an implementation choice, not a product requirement — the
product requirement is the paint order in AC 1.
