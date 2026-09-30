# LCV-163 — Plan

## Approach

ADR 0013. `Tool` gains one default method, `feedback(&self, doc, cursor) -> Vec<Mark>`, whose
default wraps `preview()` as `Mark::Preview`, so every tool that does not override it paints as
today. `Mark` (`Preview`, `Dashed`, `Hover(usize)`, `Danger`) is plain egui-free data in
`tools/feedback.rs`. `cursor` is the world point `viewport.rs::handle_hover` resolved and sent
as this frame's `Move` (so hover and click agree, AC 6), or `None` off the canvas / while muted.
Tools emit `Hover`/`Danger` only for `Some(cursor)`.

`app/viewport/paint.rs::paint` calls `ToolManager::feedback` once in place of `draw_preview`:
`Hover` marks right after the selection halo, then the other marks in tool order, then snap
glyph, pickbox, crosshair (AC 7). `app.preview_entities = preview()` stays for liveness. Render
never sees `Mark`: it gains `render/selection.rs::draw_hover` (entity in its layer colour at the
`hover` width) and `render/preview.rs::draw_dashed` (egui 0.29 `Shape::dashed_line` over the
projected polyline; circles and arcs via `arc_polyline`). Colours and the hover width live in a
new `render/palette.rs` (AC 8): `PREVIEW` (moved from `preview_stroke`), `DANGER` = #ff4d6a
(≈4.6:1 on gray 40; red-pink, away from amber/orange), `HOVER_WIDTH_PT` = 2.5.

Per tool:
- **Select**: while `Dragging`, the box is `Preview` (left→right) or `Dashed` (right→left), both
  from `hit::box_preview`; otherwise `Hover(i)` for `hit::pick_closest(cursor, …, aperture)` —
  the same function the release uses (AC 1–3, 6).
- **TRIM**: `trim_steps` becomes `trim_fold(doc, target, pos) -> (Vec<usize>, Entity)` (cutters
  that changed it, final entity). The click turns the indices into `TrimEntity` commands; the
  feedback emits `Hover(target)` plus one `Danger` per piece of
  `document/commands/trim/removed.rs::removed_pieces(original, kept)` (AC 3, 4, 6). Line: the
  0–2 end segments outside the kept one; Circle: the complementary arc; Arc: the 0–2 sub-arcs
  outside the kept span, same centre, radius and direction. Degenerate (< EPSILON) pieces drop.
- **EXTEND**: `Hover(ti)` plus today's `Preview(grown)`, from `hover(cursor, …)` (AC 3; the
  added segment stays amber per spec).
- **ERASE**: one `Danger` per selected entity when `cursor` is `Some` (AC 5).

Esc (AC 9): `ToolManager` records the last `Move` point; `handle_key(Escape)` stores it as
`muted_at`; `feedback` passes `None` while the incoming cursor equals `muted_at`; a `Move` to a
different point clears it. Pointer-leave already gives `cursor = None`.

## Touches

- `src/tools/feedback.rs` (new), `src/tools/mod.rs` — `Mark`, re-export.
- `src/tools/tool.rs::Tool::feedback` — default method (211 → ~222).
- `src/tools/manager.rs` — `feedback`, `last_move`, `muted_at` (254 → ~270).
- `src/tools/select/mod.rs`, `src/tools/trim.rs`, `src/tools/extend.rs`, `src/tools/delete.rs`.
- `src/document/commands/trim/removed.rs` (new, kernel-pure), `trim/mod.rs` (`mod` line).
- `src/render/palette.rs` (new), `render/mod.rs`, `render/preview.rs` (module doc: the
  "no dashed_line in 0.29" note is wrong), `render/selection.rs`.
- `src/app/viewport/paint.rs::paint` — mark dispatch and order; `src/app/viewport/tests.rs`
  source scans if they pin `draw_preview`.
- `AGENTS.md` ADR list + `docs/adr/0013-…` (committed with T1); `DESIGN.md` §3/§6; `CHANGELOG.md`.
- ADRs: ADR 0013 (new).

## Risks

- LOC cap: `tools/manager.rs` lands near 270. Seam: if it passes 270, move the mute logic
  (`last_move`, `muted_at`, the compare) into `tools/feedback.rs` as a small `FeedbackGate`
  struct the manager owns. All other touched files stay under 230.
- Mutation testing: no (no `src/agent/`, SVG export or `History` change).
- TRIM agreement (AC 6): feedback and click share `trim_fold`; a test clicks after hovering and
  compares the removed geometry to the danger pieces.
- `Shape::dashed_line` returns many segments; tests tell solid from dashed by segment count along
  one box edge, not by colour.
- DESIGN.md is also edited in the main tree by another session; the §3/§6 task may conflict at
  merge — keep that commit small and last.
- The LCV-137 AC 1 paint-order unit test (`viewport/tests.rs`) gains the hover step; the empty
  fixture still paints nothing extra.
