# LCV-166 — Tasks

- [x] T1 Refactor: move `recent_submenu`, `recent_labels`, `basename` and `disambiguated` into
  `menubar/recent.rs` with no behaviour change. `menubar/tests.rs` imports them from there; the
  gate stays green (files: src/ui/menubar.rs, src/ui/menubar/recent.rs, src/ui/menubar/tests.rs)
- [x] T2 [AC1] [AC2] [AC3] [AC4] Test first: open File, Edit, View, Tools and Help, plus the
  Open Recent and Object Snap submenus, with real clicks and hovers. Assert: no run contains
  `\t` or `▶`; label and shortcut are separate runs; each menu's shortcut runs share one right
  edge, right of every label. Line shapes are painted in the icon slot for File New/Open/Save,
  Edit Undo/Redo/Delete, View Zoom In/Out/Extents and Fit to Bed, every Tools row, and
  Grid/Snap/Ortho while on. The slot is empty for Save As, Export Layers, Zoom All, and
  Grid/Snap/Ortho while off. The label x is the same across a menu, submenu titles included
  (files: tests/it/ui/menu_rows.rs, tests/it/ui/mod.rs)
- [x] T3 [AC2] [AC3] Glyphs `new_file`, `open`, `save`, `undo`, `redo`, `zoom_in`, `zoom_out`,
  `zoom_extents`, `fit_bed` and `check` on the 20-unit grid. Unit tests: each paints at least one
  shape inside its square, and no two paint the same shapes (files: src/ui/icons/menu.rs,
  src/ui/icons.rs)
- [x] T4 [AC1] [AC2] [AC3] `row.rs`: `MENU_ICON`, `slot_text` (a `LayoutJob` with
  `leading_space`, colour `PLACEHOLDER`), `menu_row` (a `Button` with `shortcut_text`, and the
  icon painted in the slot in the row's text colour) and `check_row` (a check glyph while on,
  flips on click, keeps the menu open). Unit test: the icon's bounding rect lies left of the
  label galley (files: src/ui/menubar/row.rs, src/ui/menubar.rs)
- [x] T5 [AC1] [AC2] [AC3] [AC4] Every row in `menubar.rs` and `object_snap.rs` goes through
  `menu_row`/`check_row`. The Tools menu uses `entry.icon`; the submenu titles use `slot_text`.
  `Open Recent ▶` becomes `Open Recent`. The `"Grid\tF7"` source scans in `menubar/tests.rs`
  become label checks (files: src/ui/menubar.rs, src/ui/menubar/object_snap.rs,
  src/ui/menubar/tests.rs)
- [x] T5b [AC1] Integration tests that locate rows by tab labels (`"New\tCtrl+N"`,
  `Open Recent ▶`) locate the bare label run instead; T2 green (files:
  tests/it/ui/discard_dialog_pointer_click.rs, tests/it/app/document_title_and_file_feedback.rs)
- [x] T6 [AC2] Test first: with a selection, Edit > Delete erases it and one Ctrl+Z restores it;
  with no selection the row is disabled (files: tests/it/ui/menu_rows.rs)
- [x] T7 [AC2] Edit > Delete row (icon `modify::delete`, shortcut `Del`) calls
  `tools::delete::commit_delete`, raised to `pub(crate)` (files: src/ui/menubar.rs,
  src/tools/delete.rs)
- [x] T8 [AC5] [AC6] [AC7] Test first: View runs in the order Zoom In, Zoom Out, Zoom Extents,
  Zoom All, Fit to Bed, Grid, Snap, Object Snap, Ortho. Zoom Extents leaves the camera that `F`
  leaves on a twin `App`. Zoom All frames the bed ∪ extents; an entity outside the bed ends up
  inside the viewport, and with no entities the camera equals `frame_bed` (files:
  tests/it/ui/menu_navigation.rs, tests/it/ui/mod.rs)
- [x] T9 [AC7] `Camera::frame_all(bed_mm, bounds)`, the union framed by the `frame_bed` rule.
  Unit tests: with no bounds it equals `frame_bed`; bounds reaching outside the bed widen the
  frame (files: src/render/camera.rs)
- [x] T10 [AC5] [AC6] [AC7] `do_zoom_extents` (wraps `handle_zoom_extents` with the synced size)
  and `do_zoom_all`. View menu reordered with the Zoom Extents (`F`) and Zoom All rows. `F`,
  `Ctrl+0` and the typed `zoom e` call `do_zoom_extents`. T8 is green (files: src/ui/menubar.rs,
  src/app/input.rs, src/app/cmdline.rs)
- [ ] T11 [AC8] [AC9] Test first: Ctrl+A with no field focused selects every entity, and one
  Ctrl+Z restores the previous selection. With the command line focused, Ctrl+A leaves the
  selection and the tool unchanged. On an empty document Ctrl+A commits nothing (files:
  tests/it/ui/tool_hotkeys.rs)
- [ ] T12 [AC8] [AC9] `dispatch_shortcuts`: a `ctrl_only` arm for `Key::A`, when `!wants_kbd`,
  calls `menubar::do_select_all` and returns `true`. Edit > Select All shows `Ctrl+A` in the
  shortcut column (files: src/ui/shortcuts.rs, src/ui/menubar.rs)
- [ ] T13 [AC8] The F1 dialog lists `Ctrl+A  Select All` in its Edit group, and its
  key-coverage tests follow (files: src/ui/shortcuts_dialog.rs)
- [ ] T14 [AC10] ADR 0002: add an **Amended (4)** header note, "§A6: `Ctrl+A` joins the table as
  `select all | Ctrl+A | no` (LCV-166)", and add that row to the table (files:
  docs/adr/0002-headless-input-tests-and-dirty-tracking.md)
- [ ] T15 [AC10] DESIGN.md §7 (the menu row: 16 pt icon slot, label, right-aligned shortcut
  column; check marks in the slot; no hand-drawn arrows; View order) and §8 (Ctrl+A, Zoom
  Extents and Zoom All in the menu). Spec note: AC 10's gate class reads "no" per AC 9 (files:
  DESIGN.md, docs/specs/LCV-166-menus-and-navigation/spec.md)
- [ ] T16 CHANGELOG: menu icons and an aligned shortcut column, Edit > Delete, View > Zoom
  Extents and Zoom All, Ctrl+A (files: CHANGELOG.md)
