# LCV-183 — Tasks

- [x] T1 [AC1] [AC2] Test first: unit tests for the icon set — vector shapes only, inside the
  20 pt square, 1.5 pt stroke in the given colour, 16 pairwise-distinct signatures; stub
  `IconFn`s so the tests compile and fail (files: src/ui/icons.rs, src/ui/mod.rs)
- [x] T2 [AC1] [AC2] Draw-group icons: select, line, polyline, rect, circle, arc, text; shared
  helpers `marker`, `arrow_head`, `dashed` (files: src/ui/icons/draw.rs, src/ui/icons.rs)
- [x] T3 Refactor: move `ToolEntry` and `TOOLS` to `ui/toolbar/table.rs`, re-exported from
  `toolbar.rs`; no behaviour change (files: src/ui/toolbar.rs, src/ui/toolbar/table.rs)
- [x] T4 [AC1] [AC2] Modify-group icons: move, copy, rotate, mirror, scale, trim, extend,
  delete, dist; T1 goes green (files: src/ui/icons/modify.rs, src/ui/icons.rs)
- [x] T5 [AC2] `ToolEntry.icon` field filled for all 16 entries (files: src/ui/toolbar/table.rs)
- [x] T6 [AC3] [AC4] [AC5] [AC6] [AC7] [AC8] [AC9] Test first: `icon_tool_rail.rs` — tooltip
  per computed button centre in draw/modify column order, click activates, selected fill under
  the active button only, `AI` toggle text/tooltip/fill/click, width ≤80 pt, no scroll at the
  three sizes, wheel scroll reaches `AI` at 220 pt (files: tests/it/ui/icon_tool_rail.rs,
  tests/it/ui/mod.rs)
- [ ] T7 [AC6] Tooltip format `<Label> — <key> · <WORD>` / `<Label> — <WORD>` with unit tests
  (Select, Line, Rotate, Delete) (files: src/ui/toolbar.rs)
- [ ] T8 [AC1] [AC3] [AC4] [AC5] [AC7] [AC9] `icon_button` + two-column rail + `AI` toggle in
  `draw_toolbar`; rewrite the LCV-140 rail tests from labels to tooltips (files:
  src/ui/toolbar.rs, src/ui/icons.rs, tests/it/ui/compact_chrome_and_action_hints.rs)
- [ ] T9 [AC8] Fixed `RAIL_WIDTH` and 4 pt frame margin in `panels.rs`; drop the label-width
  measure and its clamp test; T6 goes green (files: src/app/panels.rs)
- [ ] T10 [AC10] DESIGN.md §2 rail budget ≤80 pt, §7 tool rail: painted icons, two columns,
  `AI` toggle, tooltip format (files: DESIGN.md)
- [ ] T11 CHANGELOG `[Unreleased]` line: icon tool rail in two columns, tooltips with key and
  command word (files: CHANGELOG.md)
