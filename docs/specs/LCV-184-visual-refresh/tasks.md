# LCV-184 — Tasks

- [x] T1 [AC1] [AC2] [AC3] Test first: `theme.rs` unit tests — `TOKENS` names/values match the
  DESIGN.md §3 rows (`include_str!`); after `apply_theme`, `ctx.style().visuals` has no window
  or popup shadow, `window_stroke` 1 pt `BORDER`, widget rounding 3, window/menu rounding 4,
  inactive/hovered/active fills and `selection.bg_fill` from tokens, `selection.stroke` 1 pt
  `ACCENT`, and no fill field equals `ACCENT`; stub constants so it compiles and fails
  (files: src/ui/theme.rs)
- [x] T2 [AC1] [AC2] [AC3] Token constants + `TOKENS` + `apply_theme` rewrite (plan §Approach);
  DESIGN.md §3 gains `border`, `fill.widget`, `fill.hover`, `fill.active` rows and every chrome
  Home points at `ui/theme.rs`; T1 green (files: src/ui/theme.rs, DESIGN.md)
- [x] T3 [AC1] [AC2] [AC3] Test: source scan for `Color32` literals in `src/ui/` outside
  `theme.rs` (with positive control); painted File menu frame (1 pt `BORDER`, rounding 4, no
  blurred shape); a full frame paints no rect filled `ACCENT` (files:
  tests/it/ui/visual_refresh.rs, tests/it/ui/mod.rs)
- [x] T4 [AC4] [AC3] Test first: SNAP on → pill rect filled `FILL_SELECTED`, text `ACCENT`; GRID
  off → no fill, 1 pt `BORDER` outline, text `TEXT_MUTED`; a click on each pill flips only its
  flag (files: tests/it/ui/visual_refresh.rs)
- [ ] T5 [AC4] `mode_pill` widget with `PILL_PADDING` and its unit tests (files:
  src/ui/statusbar/pill.rs, src/ui/statusbar.rs)
- [ ] T6 [AC4] `draw_statusbar` uses `mode_pill`; rewrite the `selectable_label` source-scan
  needles in the status-bar unit tests; T4 green (files: src/ui/statusbar.rs,
  src/ui/statusbar/tests.rs)
- [ ] T7 [AC5] Test first: a 1 pt vertical line segment lies between every pair of adjacent
  status segments; the coordinate run's galley uses `FontFamily::Monospace` (files:
  tests/it/ui/visual_refresh.rs)
- [ ] T8 [AC5] Coordinates painted as `RichText::monospace()`; T7 green (files:
  src/ui/statusbar.rs)
- [ ] T9 [AC6] Test first: `prompt_spans` unit tests (`LINE Specify first point:`, `MIRROR Erase
  source objects? [Yes/No] <N>:`, `TEXT Specify height <5>:`, `TRIM: Click…`, `Command:`, empty,
  unbalanced `[`), stubbed; painted test: LINE active → verb `ACCENT`, request `TEXT_PRIMARY`;
  MIRROR confirm → `[Yes/No]` and `<N>` `TEXT_MUTED`; Select → `Command:` `TEXT_PRIMARY` (files:
  src/ui/command_line/prompt.rs, src/ui/command_line.rs, tests/it/ui/visual_refresh.rs)
- [ ] T10 [AC6] `prompt_spans` + prompt painted from a `LayoutJob` through the truncating
  label; T9 green (files: src/ui/command_line/prompt.rs, src/ui/command_line.rs)
- [ ] T11 [AC7] Test first: the frame rect enclosing the editor response rect has a 1 pt
  `BORDER` stroke while unfocused and `ACCENT` after a typed character focuses the editor
  (files: tests/it/ui/visual_refresh.rs)
- [ ] T12 [AC7] Editor row in an `egui::Frame` (1 pt stroke, focus read from memory),
  `TextEdit::frame(false)`; T11 and `tests/it/cmdline/context_row.rs` green (files:
  src/ui/command_line.rs)
- [ ] T13 [AC8] Test: active rail button rect filled `FILL_SELECTED`, rounding 3, icon stroke
  `ACCENT`; a hovered button filled `FILL_HOVER`, rounding 3; fix `square_button` only if it
  fails (files: tests/it/ui/visual_refresh.rs, src/ui/icons.rs)
- [ ] T14 [AC9] Test: at 800×600, 1024×600, 1280×800 with the modes all on and all off, menubar
  titles inside the window, status segments contained and bar ≤56 pt, dock ≤64 pt, 17 rail
  centres inside the unscrolled rail; tune `PILL_PADDING` if it fails (files:
  tests/it/ui/visual_refresh.rs, src/ui/statusbar/pill.rs)
- [ ] T15 [AC10] DESIGN.md §1.8 and §7 status-bar gaps closed; §5 border 1 pt, rounding 3/4 pt,
  pill padding; §7 pills, separators, monospace coordinates, prompt colours, editor frame;
  `accent` Home `ui/theme.rs` (files: DESIGN.md)
- [ ] T16 CHANGELOG `[Unreleased]` line: flat dark theme without shadows, on/off mode pills,
  coloured command prompt, focused editor frame (files: CHANGELOG.md)
