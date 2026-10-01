# LCV-162 — Plan

## Approach

`app/viewport.rs::draw` is reordered to sync the camera, then pan, then `handle_hover`, then
`paint` (fast-lane F1, AC 4). `handle_hover` returns the resolved world point (after snap and
Ortho). `paint` takes it as `cursor: Option<Vec2>` and paints the crosshair from it (AC 3).
`None` means the canvas is not hovered, so nothing is painted (AC 10).

A new `render/cursor.rs` paints the crosshair: two 1 pt lines across the canvas rect in
`cursor_color()` (AC 2). It also paints the pickbox, a hollow square that `paint` requests only
when `ToolManager::wants_entity_pick()` is true (AC 5/6). While the canvas is hovered, `draw`
sets `egui::CursorIcon::None`. egui resets the icon every frame, so leaving the canvas restores
the arrow on its own (AC 1/10).

Tolerances move to screen points: `PICK_APERTURE_PT = 5.0` and `DRAG_THRESHOLD_PT = 2.0` in
`tools/tool.rs`. `draw` calls `ToolManager::set_pick_scale(camera.mm_per_px)` every frame before
`handle_hover`. The manager stores the value and forwards it to the active tool, and to a new
tool in `set_tool`. Select, TRIM and EXTEND turn points into mm with it (AC 7–9). The default
scale is 1.0 mm/pt, the same as `Camera::default`, so tools built directly in unit tests keep
today's 5 mm / 2 mm.

**Design decision (flag).** While `wants_entity_pick()` is true, `handle_hover` does not resolve
the running snap and clears `active_snap`. Without this, Select would measure the pick from a
snapped endpoint up to 12 pt away, and AC 7 "SHALL NOT pick one farther away" would fail with
snap on. It is also R14 behaviour: there is no running osnap at "Select objects". As a side
effect, the snap glyph no longer shows while Select is idle.

## Touches

- `src/tools/tool.rs`: gains `PICK_APERTURE_PT`, `DRAG_THRESHOLD_PT`, and two default methods,
  `Tool::wants_entity_pick() -> bool` (default `false`) and
  `Tool::set_pick_scale(&mut self, mm_per_pt: f64)` (default no-op). Both keep the trait
  object-safe.
- `src/tools/manager.rs`: new field `mm_per_pt`, `set_pick_scale` (store + forward),
  `set_tool` forwards the stored value, `wants_entity_pick()`, and `pick_aperture_mm()`
  (LCV-163 reuses the last two for the hover highlight).
- `src/tools/select/{mod.rs,hit.rs}`: `pick_closest(pos, entities, radius_mm)` and a
  scale-aware drag threshold. `wants_entity_pick` returns `!Dragging`.
- `src/tools/trim.rs`, `src/tools/extend.rs`: pick radius = aperture × scale, and
  `wants_entity_pick` returns `true`. `TrimTool` becomes a struct with `Default`, so
  `tools/mod.rs::make` and the tests use `TrimTool::default()`.
- `src/render/cursor.rs` (new) + `render/mod.rs` re-export: `draw_crosshair`, `draw_pickbox`,
  `cursor_color`.
- `src/app/viewport.rs`: reorder, pass the cursor point to `paint`, set the cursor icon,
  suppress snap while picking. `src/app/viewport/tests.rs`: the LCV-120 needle becomes the new
  `paint(` call, and the LCV-137 AC 1 test passes `None`.
- `DESIGN.md` §3 (`cursor` token), §5 (tolerances in points), §6 (paint order row, crosshair
  shipped, F1 note) and §11 (drop F1).
- ADRs: none. The two trait methods are additive defaults; see Risks.

## Risks

- **Boundary**: the `Tool` trait gains two methods that `app/` consumes. They are additive and
  follow the `take_message`/`anchor` precedent, so no ADR is planned. If `/design` wants the
  architect, that is the only question.
- **LOC**: `app/viewport.rs` is at 247 and grows by about 20–25. If T11 would pass 270, the seam
  is to move `paint` into `src/app/viewport/paint.rs` (a `mod paint;` beside `tests.rs`) with
  no behaviour change. `manager.rs` goes from 224 to about 250, which is fine.
- **Sequencing**: LCV-160 rewrites the pick code of `trim.rs`/`extend.rs` and LCV-161 extends
  `app/snap.rs`. T6 applies the aperture to whatever pick functions exist after 160, and this
  plan does not edit `app/snap.rs`.
- **Source-scan tests** in `app/viewport/tests.rs` pin `paint(ui, rect, app);` and the
  `handle_hover` signature. They are updated in the same task as the code, keeping their intent.
- **Harness trap**: pointer tests need a warm-up frame, and leaving the canvas is proven with
  `Event::PointerGone` (harness rules 3 and 5).
- Mutation testing: no. No `agent/`, `io/svg/export.rs` or `History` is touched.
