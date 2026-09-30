# LCV-167 — Tasks

- [x] T1 [AC1] [AC2] Test first: open File, Edit, View (+ Object Snap), Format, Tools, Help by
  painted clicks; every item run passes the Title Case check (plan §Test approach) and the rows
  `Export Layers`, `Bed Size…`, `Keyboard Shortcuts…`, `AI Settings…`, `Object Snap` exist; the
  Bed Size, Keyboard Shortcuts (F1) and AI Settings windows paint those titles; AI Settings
  paints `Restore Default` (files: tests/it/ui/wording_and_status.rs, tests/it/ui/mod.rs)
- [x] T2 [AC1] Menu rows `Export Layers`, `Bed Size…`, `Keyboard Shortcuts…\tF1`, submenu
  `Object Snap`; tests keyed on the old rows follow (`visual_refresh.rs`, `object_snap_menu.rs`)
  (files: src/ui/menubar.rs, src/ui/menubar/object_snap.rs, tests/it/ui/object_snap_menu.rs)
- [x] T3 [AC1] Window titles `Bed Size`, `Keyboard Shortcuts`; button `Restore Default`; tests
  keyed on the old titles/ids follow (`ui/bed_dialog.rs`, `ui/shortcuts_dialog_fits.rs`)
  (files: src/app/bed_dialog.rs, src/ui/shortcuts_dialog.rs, src/agent/settings_ui.rs)
- [x] T4 [AC2] Help item `AI Settings…`, window `AI Settings`; `Id::new`/`area_rect`/title
  lookups in `agent/panel_width_and_settings.rs`, `agent/prompt_editor.rs`,
  `agent/panel_and_settings.rs` follow; T1 green (files: src/ui/menubar.rs, src/app/panels.rs)
- [x] T5 [AC3] Test only (holds since LCV-183): rail toggle paints `AI`, its tooltip is
  `AI Assistant`, the panel heading is `AI Assistant`, the dock destination for `:draw` is `AI`
  (files: tests/it/ui/wording_and_status.rs)
- [x] T6 [AC4] Test first: timeout and 401 texts contain `Help > AI Settings…`; `:draw` with no
  key sets the dock line `! AI unavailable: set the API key in Help > AI Settings…`; a capture
  with capture off is refused with `CAPTURE_DISABLED` naming `Help > AI Settings…` (files:
  src/agent/transport/tests.rs, src/app/cmdline/tests.rs, tests/it/agent/canvas_capture.rs)
- [x] T7 [AC4] Transport texts and the dock's AI lines (`! AI unavailable: …`, `AI prompt is
  empty.`, `AI is busy — …`, echo `→ AI: "…"`); `cmdline/agent_routing.rs` follows (files:
  src/agent/transport.rs, src/app/cmdline.rs, tests/it/cmdline/agent_routing.rs)
- [x] T8 [AC4] `CAPTURE_DISABLED` = `canvas capture is disabled in Help > AI Settings…`;
  `agent_capture.rs` uses the const, not its literal; T6 green (files: src/app/agent_apply.rs,
  src/app/agent_capture.rs)
- [x] T9 [AC5] Test first: `turn_label` gives `AI: <prompt>` (plain, trimmed, truncated cases);
  a committed turn's undo label starts `AI: ` (files: src/app/agent_turn/tests.rs)
- [x] T10 [AC5] `turn_label` prefix `AI: ` and its doc; composite doc example (files:
  src/app/agent_turn.rs, src/document/commands/composite.rs)
- [x] T11 [AC6] Test first: `(1, -1)` and `(-1234.5, 9999.99)` give equal char counts; padding is
  U+2007, never U+0020, inside a number; `None` unchanged; painted expectations follow (files:
  src/ui/statusbar/tests.rs, tests/it/ui/compact_chrome_and_action_hints.rs)
- [x] T12 [AC6] `format_coords` width 8 with figure-space padding, doctest updated; T11 green
  (files: src/ui/statusbar.rs)
- [ ] T13 [AC7] Test first: `STATUS_ERROR` = #ff6b6b, WCAG ≥4.5:1 on `BG_PANEL`,
  `visuals.error_fg_color == STATUS_ERROR` after `apply_theme`; the panel test's error colour
  reads the visuals (files: src/ui/theme.rs, src/agent/panel/tests.rs)
- [ ] T14 [AC7] `STATUS_ERROR` + `TOKENS` row + `v.error_fg_color`; DESIGN.md §3 `status.error`
  Home → `ui/theme.rs::STATUS_ERROR` in the same commit (TOKENS ↔ §3 test); T13 green (files:
  src/ui/theme.rs, DESIGN.md)
- [ ] T15 [AC7] Test first: painted AI panel `error` row and the `! AI unavailable` dock line are
  #ff6b6b; an ordinary dock feedback stays `status.warning`; source scan: no `Color32::RED` in
  `src/` outside tests (files: tests/it/ui/wording_and_status.rs)
- [ ] T16 [AC7] `draw_chat_row` error rows and `! ` dock feedback read `error_fg_color`; T15
  green (files: src/agent/panel.rs, src/ui/command_line.rs)
- [ ] T17 [AC8] Test only (holds since LCV-184): after `apply_theme`, `selection.bg_fill` is
  #005c80 and no widget fill is `ACCENT` (files: tests/it/ui/wording_and_status.rs)
- [ ] T18 [AC9] Test first: `format_autosave` four states and precedence (failed > pending >
  autosaved > none); `record_autosave_outcome(false)` sets `autosave_failed` only with a path,
  `true` clears it; painted flow: path = a directory → `× autosave failed` in #ff6b6b, then a
  writable path → `○ autosaved` (files: src/ui/statusbar/tests.rs, src/app/autosave.rs,
  tests/it/ui/wording_and_status.rs)
- [ ] T19 [AC9] `App::autosave_failed`, set/clear in `record_autosave_outcome`, `format_autosave`
  `failed` argument, failed badge painted in `error_fg_color`; T18 green (files: src/app/mod.rs,
  src/app/autosave.rs, src/ui/statusbar.rs)
- [ ] T20 [AC10] DESIGN.md §3 (accent foreground-only, amends LCV-071 AC 4), §4 (figure-space
  coordinates), §7 (fourth autosave state, amends LCV-116 decision 3), §9 (AI names, gap
  paragraph closed) (files: DESIGN.md)
- [ ] T21 CHANGELOG `[Unreleased]`: Title Case labels, one AI name (`AI Settings…`, `AI:` undo
  steps), steady coordinates, readable error red, `× autosave failed` (files: CHANGELOG.md)
