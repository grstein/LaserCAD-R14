# LCV-164 — Plan

## Approach

Canvas-only change in `render/` plus a one-flag framing hook in `app/`. New canvas colours are
tokens in `render/palette.rs`. A new `render/tessellate.rs` is the single curve sampler: it
picks the chord count per full turn from the on-screen radius, `n = ⌈2π / (2·acos(1 − 0.25/r_pt))⌉`
clamped to [8, 1024] (8 when r ≤ 0.25 pt). An arc gets `⌈n · sweep/2π⌉` chords, at least 2. Its
`stroke_entity` paints an entity as **one** shape: `LineSegment` for a line, a closed
`Shape::Path` for a circle, an open `Shape::Path` for an arc. Entities, the selection halo, hover
and preview all go through it, and `draw_dashed` samples from it too. The snap glyph is built as a
shape list and painted twice: first the edge in `SNAP_EDGE` (`CANVAS_BG`) 2 pt wider, then the glyph.
Then its lower-case kind name is painted below and to the right of the point. The origin is one
open path, (12 pt along +X) → (0,0) → (12 pt along +Y), painted right after the bed border.
Framing: `App::frame_bed_pending` is set by `App::new`, by both Open paths and by the Bed dialog's OK. `viewport::draw` reads it
after it syncs the viewport size: when the rect has area it calls `Camera::frame_bed` (the body
of today's `do_fit_to_bed`) and clears the flag. `App::default` leaves the flag off, so test
fixtures keep 1 mm/pt at the origin.

## Touches

- `src/render/palette.rs` — `GRID_MINOR` gray 62 (1.38:1), `GRID_MAJOR` gray 96 (2.34:1),
  `ORIGIN` gray 220 + `ORIGIN_ARM_PT` 12 + `ORIGIN_WIDTH_PT` 2, `SNAP_EDGE` = `crate::ui::CANVAS_BG`.
- `src/render/grid.rs::draw_grid` — tokens, 1 pt, `painter.round_to_pixel_center` on the fixed axis.
- `src/render/tessellate.rs` (new) — `chords_per_turn`, `screen_points`, `stroke_entity`.
- `src/render/entities.rs` — uses `stroke_entity`; `PaintOptions::arc_segments` removed
  (replaces LCV-035 AC 2); `arc_polyline` kept as the sampler.
- `src/render/selection.rs::draw_entity_with_stroke` → `stroke_entity`; `src/render/preview.rs::draw_dashed`
  samples via `screen_points` (drops `CURVE_SEGMENTS`).
- `src/render/snaps.rs::draw_snap_marker` — `glyph_shapes(shape, pos, fill, stroke) -> Vec<Shape>`,
  edge pass then glyph pass; `src/render/snaps/label.rs` (new) — `snap_label(kind)`, label painter,
  `LABEL_OFFSET_PT`.
- `src/render/bed.rs::draw_origin` (new) + `render/mod.rs` export; `src/app/viewport/paint.rs::paint`
  calls it after `draw_bed`.
- `src/render/camera.rs::Camera::frame_bed(bed_mm)`; `src/ui/menubar.rs::do_fit_to_bed` delegates.
- `src/app/mod.rs` (`frame_bed_pending` field), `src/app/init.rs` (default false, `App::new` true),
  `src/app/viewport.rs::draw` (consume), `src/io/file_actions.rs` (both Open paths),
  `src/app/bed_dialog.rs` (OK).
- Tests: `tests/it/ui/canvas_legibility.rs` (new) + `tests/it/ui/mod.rs`; unit tests in
  `palette.rs`, `grid/tests.rs`, `snaps.rs`, `tessellate.rs`, `app/viewport/tests.rs`, `app/init.rs`.
- `DESIGN.md` §3, §5, §6; `CHANGELOG.md`. ADRs: none (no dependency, thread, trait or kernel change).

## Decisions (self-approved per user goal)

- Grid: minor gray 62 (1.38:1), major keeps gray 96 (2.34:1), both 1 pt.
- Origin: one `ORIGIN` token, gray 220, 2 pt. The arms lie on the bed's lower-left border, so
  width and brightness, not hue, set them apart. No red/green (it would clash with `danger`).
- Snap label: body size 12.5 proportional, in `snap`, no backing. Its left-top is at
  `+(MARKER_SIZE_PX/2 + 4, MARKER_SIZE_PX/2 + 2)` pt, in the lower-right quadrant, clear of the point and the
  crosshair lines. `snap_label` lives in `render/` (the kernel `SnapKind` is not touched).
- Edge colour reaches `render/` through `palette::SNAP_EDGE`, the only `render → ui` import.
- A "bed size change" means the Bed dialog's OK. Undo/redo of `SetBedSize` does not reframe
  (`View > Fit to Bed` remains). Autosave recovery happens inside `App::new`, so the flag set there
  covers it. `File > New` is not listed in AC 7 and is left as it is.
- Framing is `App::new`-only on startup: `App::default` (the test constructor, ADR 0002 §A2) never
  frames, so existing screen-coordinate tests keep their camera.

## Test approach

- AC 1: palette contrast unit tests (WCAG, as for `DANGER`); painted `draw_grid` at 1 and 1.5 ppp:
  every `LineSegment` is 1 pt in a grid token, fixed coordinate × ppp at a .5 fraction.
- AC 2/3: `draw_snap_marker` into a `Context` for all 8 kinds: an edge shape in `SNAP_EDGE`, width ≥
  glyph + 1, before the glyph; a `Text` shape whose galley text is the name, rect not containing the point.
- AC 4: painted origin `Path` with 3 points at world (0,0) ± 12 pt; index after the bed border, before
  entities (the LCV-137 test in `app/viewport/tests.rs` gains a `Path` kind).
- AC 5: painted circle/arc paths at several zooms: chord count in [8, 1024], each sagitta ≤ 0.25 pt.
- AC 6: a selected + hovered arc and circle → exactly one halo/hover/preview shape per entity.
- AC 7: flag set → one frame → camera equals `Camera::frame_bed`; `action_open_path` on a temp SVG
  and Bed dialog OK set it; source scan that `App::new` sets it; `App::default` does not.

## Risks

- LOC cap: `app/mod.rs` 287 → ~290 (seam: next view flag groups into a `ViewState` struct);
  `menubar.rs` 288 shrinks by `do_fit_to_bed`; `snaps.rs` 242 → label in `snaps/label.rs`, stays < 270.
- Shape-variant tests change: `tests/it/app/layer_colors.rs` (`Shape::Circle`), `snaps.rs`
  unit tests (one shape per glyph), `grid/tests.rs` ac5 tolerance 0.5 vs pixel-centre rounding
  (widen by ≤0.5 pt with a comment).
- Overlap with LCV-184 (running first on this branch): `DESIGN.md` §3/§5, `tests/it/ui/mod.rs`,
  `ui/theme.rs::CANVAS_BG` alias (kept by 184). Implement after 184 lands; rebase conflicts are docs only.
- Mutation testing: no.
