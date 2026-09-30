# LCV-163 — Tasks

- [x] T1 ADR 0013 committed and listed in the AGENTS.md ADR list (files:
  docs/adr/0013-styled-canvas-feedback-from-tools.md, AGENTS.md)
- [x] T2 [AC1–9] Seam: `Mark` enum and the `Tool::feedback` default (wraps `preview()` as
  `Mark::Preview`); unit test that a tool with no override returns its preview as `Preview`
  marks (files: src/tools/feedback.rs, src/tools/mod.rs, src/tools/tool.rs)
- [x] T3 [AC8] Test first, then `render/palette.rs`: `PREVIEW` (moved from `preview_stroke`),
  `DANGER` #ff4d6a, `HOVER_WIDTH_PT` 2.5. Unit tests: `DANGER` ≥3:1 WCAG on gray 40 and hue
  ≥30° from `preview`, `snap` (#ffa000) and `status.warning` (#ff8f00); `HOVER_WIDTH_PT` > 1
  (files: src/render/palette.rs, src/render/mod.rs, src/render/preview.rs)
- [x] T4 [P] [AC4] Kernel: `removed_pieces(original, kept) -> Vec<Entity>` with unit tests first:
  line with 0/1/2 removed ends, circle → complementary arc, CCW and CW arc with 0/1/2 pieces,
  across ±π, degenerate pieces dropped (files: src/document/commands/trim/removed.rs,
  src/document/commands/trim/mod.rs)
- [x] T5 [AC1] [AC2] [AC3] [AC4] [AC5] [AC7] [AC9] Test: painted-shape tests through
  `App::update_ui` (pattern of `tests/it/ui/cursor_and_picking.rs`): L→R box = 4 solid segments,
  R→L box = dashed (many short segments on one edge); Select idle hover over a line paints it at
  `HOVER_WIDTH_PT` in the layer colour, nothing when no entity is in the pickbox; TRIM hover paints
  the removed piece(s) dashed in `DANGER` (line cut by two cutters; circle cut by a circle); ERASE
  with a selection paints each selected entity dashed in `DANGER`; order halo < hover < preview/
  danger < snap < pickbox < crosshair; after Esc and after `PointerGone` no hover and no danger
  (files: tests/it/ui/selection_and_edit_feedback.rs, tests/it/ui/mod.rs)
- [x] T6 [AC6] Test: hover-then-click agreement through `App`: Select picks the hovered index;
  TRIM removes exactly the danger pieces (document before minus after equals the painted pieces,
  line and arc targets); ERASE removes exactly the danger set (files:
  tests/it/app/feedback_agreement.rs, tests/it/app/mod.rs)
- [x] T7 [AC2] [AC3] [AC5] Render primitives: `draw_dashed(painter, rect, camera, entity, color)`
  via `Shape::dashed_line` (arcs/circles through `arc_polyline`), fix the stale module doc;
  `draw_hover(painter, rect, camera, doc, index)` in the layer colour at `HOVER_WIDTH_PT`,
  out-of-range index skipped; unit tests (files: src/render/preview.rs, src/render/selection.rs,
  src/render/mod.rs)
- [x] T8 [AC9] `ToolManager::feedback(doc, cursor)`: records `last_move` on `Move`, `muted_at` on
  `handle_key(Escape)`, passes `None` while muted, clears on a different `Move`; unit tests. If
  the file passes 270 LOC, move the gate into `tools/feedback.rs` as `FeedbackGate` (files:
  src/tools/manager.rs, src/tools/feedback.rs)
- [x] T9 [AC7] Paint: replace `draw_preview` with the mark dispatch — `Hover` after the halo,
  the rest in tool order (`Preview` → `draw_preview`-style solid, `Dashed` → `draw_dashed` in
  `PREVIEW`, `Danger` → `draw_dashed` in `DANGER`), then snap, pickbox, crosshair; update the
  doc comment and the LCV-137 AC 1 scan/order tests (files: src/app/viewport/paint.rs,
  src/app/viewport/tests.rs)
- [x] T10 [AC1] [AC2] [AC3] [AC6] `SelectTool::feedback`: `Preview`/`Dashed` box by drag direction
  while `Dragging`; else `Hover(hit::pick_closest(cursor, …, aperture))` (files:
  src/tools/select/mod.rs, src/tools/select/hit.rs)
- [x] T11 [P] [AC3] [AC4] [AC6] TRIM: `trim_steps` → `trim_fold` returning (cutter indices, kept
  entity); click builds commands from the indices; `feedback` = `Hover(target)` + `Danger` per
  `removed_pieces` (files: src/tools/trim.rs)
- [x] T12 [P] [AC3] [AC5] EXTEND `feedback` = `Hover(ti)` + `Preview(grown)` from
  `hover(cursor, …)`; ERASE `feedback` = `Danger` per selected entity when `cursor` is `Some`;
  T5 and T6 go green (files: src/tools/extend.rs, src/tools/delete.rs)
- [x] T13 [AC10] DESIGN.md §3: `hover` and `danger` rows (value, contrast, home
  `render/palette.rs`), `preview` home moved; §6: paint order with hover, the three rows marked
  shipped; also mark the Quadrant/Perpendicular/Tangent/Nearest glyph rows shipped (LCV-161)
  (files: DESIGN.md)
- [ ] T14 CHANGELOG `[Unreleased]` line: window/crossing boxes solid/dashed, hover highlight on
  pick, TRIM/ERASE show what they remove (files: CHANGELOG.md)
