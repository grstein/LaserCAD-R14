# LCV-164 — Tasks

- [x] T1 [AC1] Test: grid token contrast (minor ≥1.35:1, major ≥2.2:1 on gray 40) and painted
  `draw_grid` at 1 and 1.5 ppp — 1 pt, token colour, pixel-centre coordinate (files: src/render/palette.rs, src/render/grid/tests.rs)
- [ ] T2 [AC1] `GRID_MINOR`/`GRID_MAJOR` tokens; `draw_grid` 1 pt on pixel centres (files: src/render/palette.rs, src/render/grid.rs)
- [ ] T3 [AC5] Test: painted circle and arc paths at 0.01, 1 and 100 mm/pt and a sub-pt circle —
  chords in [8, 1024], each sagitta ≤0.25 pt (files: tests/it/ui/canvas_legibility.rs, tests/it/ui/mod.rs)
- [ ] T4 [AC5] `render/tessellate.rs` (`chords_per_turn`, `screen_points`, `stroke_entity`) with
  unit tests; `draw_entities` uses it; drop `PaintOptions::arc_segments` (files: src/render/tessellate.rs, src/render/mod.rs, src/render/entities.rs)
- [ ] T5 [AC6] Test: selection halo, hover and preview of an arc and a circle paint one shape per
  entity (files: tests/it/ui/canvas_legibility.rs)
- [ ] T6 [AC6] Halo/hover/preview through `stroke_entity`, `draw_dashed` through `screen_points`;
  `layer_colors.rs` reads circles as paths (files: src/render/selection.rs, src/render/preview.rs, tests/it/app/layer_colors.rs)
- [ ] T7 [AC2, AC3] Test: for all 8 kinds, an edge shape in `SNAP_EDGE` ≥1 pt wider precedes the
  glyph, and a text shape with the lower-case name sits clear of the point (files: src/render/snaps.rs)
- [ ] T8 [AC2] `SNAP_EDGE` token; `glyph_shapes` painted as an edge pass, then the glyph (files: src/render/palette.rs, src/render/snaps.rs)
- [ ] T9 [AC3] `snaps/label.rs`: `snap_label`, `LABEL_OFFSET_PT`, label painted after the glyph (files: src/render/snaps.rs, src/render/snaps/label.rs)
- [ ] T10 [AC4] Test: one origin path, (12 pt +X) → (0,0) → (12 pt +Y), after the bed border and
  before entities; the LCV-137 order test learns `Path` shapes (files: src/app/viewport/tests.rs, tests/it/ui/canvas_legibility.rs)
- [ ] T11 [AC4] `ORIGIN` tokens and `bed.rs::draw_origin`, exported (files: src/render/palette.rs, src/render/bed.rs, src/render/mod.rs)
- [ ] T12 [AC4] `paint` calls `draw_origin` after `draw_bed` (files: src/app/viewport/paint.rs)
- [ ] T13 [AC7] Test: pending flag frames on the first frame with area; Open (`action_open_path`
  on a temp SVG) and Bed dialog OK frame; `App::new` sets the flag (source scan), `App::default`
  does not (files: tests/it/ui/canvas_legibility.rs, src/app/init.rs)
- [ ] T14 [AC7] `Camera::frame_bed`; `do_fit_to_bed` delegates to it (files: src/render/camera.rs, src/ui/menubar.rs)
- [ ] T15 [AC7] `App::frame_bed_pending`: default false, `App::new` true, consumed in
  `viewport::draw` after the size sync (files: src/app/mod.rs, src/app/init.rs, src/app/viewport.rs)
- [ ] T16 [AC7] Open paths and Bed dialog OK set the flag (files: src/io/file_actions.rs, src/app/bed_dialog.rs)
- [ ] T17 [AC8] DESIGN.md §3 grid/origin/snap-edge tokens, §5 chord rule, §6 origin, snap label,
  framing and paint order; gap notes removed (files: DESIGN.md)
- [ ] T18 CHANGELOG line (files: CHANGELOG.md)
