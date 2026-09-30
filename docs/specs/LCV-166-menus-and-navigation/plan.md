# LCV-166 — Plan

## Approach

Every menu row goes through two helpers in a new `ui/menubar/row.rs`. `menu_row(ui, icon, label,
shortcut)` adds an `egui::Button` whose text is a `LayoutJob` with `leading_space` equal to the icon
slot, and whose `shortcut_text` egui paints right-aligned (menus lay out `top_down_justified`, so the
shortcuts form one column per menu). The icon is then painted into the slot at the left of the
button's rect. `check_row` is the same row: it paints a check mark in the slot while the flag is on
and flips the flag on click. Submenus (`Open Recent`, `Object Snap`) use the same `LayoutJob`
as their `menu_button` title, so they line up, and egui's own `⏵` is the only arrow. The new
glyphs go in `ui/icons/menu.rs`, on the LCV-183 20-unit grid. Zoom All becomes `Camera::frame_all`,
which frames the union of the bed and `Document::bounds` with the same 80 % rule as `frame_bed`
(LCV-164). Ctrl+A is one more `ctrl_only` arm in `dispatch_shortcuts`, gated on `!wants_kbd`.

## Touches

- `src/ui/menubar/recent.rs` (new): `recent_submenu`, `recent_labels`, `basename`, `disambiguated`
  moved out of `menubar.rs` without changes (seam; see Risks).
- `src/ui/menubar/row.rs` (new): `MENU_ICON` (16 pt), `slot_text`, `menu_row`, `check_row`.
- `src/ui/icons/menu.rs` (new): `new_file`, `open`, `save`, `undo`, `redo`, `zoom_in`, `zoom_out`,
  `zoom_extents`, `fit_bed`, `check`; `src/ui/icons.rs` gets `pub(crate) mod menu;`.
- `src/ui/menubar.rs`: every row goes through `row.rs`, with no `\t` left. The View menu takes
  the AC 5 order. The Edit menu gains `Delete` (shortcut `Del`), which erases the selection
  through the Delete-key path. The Tools menu takes `entry.icon`. New `do_zoom_extents` and
  `do_zoom_all`.
- `src/ui/menubar/object_snap.rs`: rows become `check_row` and the title becomes `slot_text`.
- `src/render/camera.rs::Camera::frame_all(bed_mm, bounds)`: new, next to `frame_bed`.
- `src/tools/delete.rs::commit_delete`: raised to `pub(crate)` so Edit > Delete reuses it.
- `src/ui/shortcuts.rs::dispatch_shortcuts`: `Ctrl+A` → `menubar::do_select_all` when
  `!wants_kbd`. `src/ui/shortcuts_dialog.rs::SHORTCUT_GROUPS` lists `Ctrl+A  Select All`.
- `src/app/input.rs`, `src/app/cmdline.rs`: `F`, `Ctrl+0` and `zoom e` call `do_zoom_extents` (AC 6).
- ADRs: ADR 0002 §A6 gets an amendment note adding the row `select all | Ctrl+A | no` (T15).
  DESIGN.md §7 and §8, and the CHANGELOG.

## Decisions (self-approved per user goal)

- **Ctrl+A focus gate.** Ctrl+A follows `wants_kbd`, meaning any text widget has focus, not
  only the command line. Its gate-table row therefore says "no". AC 10's "(global commands)" is
  read as "listed next to the global commands", because AC 9 rules out firing while a field has
  focus. T15 adds a one-line note to the spec saying so.
- **Edit > Delete.** AC 2 names an Edit > Delete icon, but the row doesn't exist yet. It erases
  the current selection in one undo step, like `Delete` under Select, and is disabled when
  nothing is selected. Its shortcut column reads `Del`.
- **Zoom All.** It has no icon, because AC 2 lists none, so its slot stays empty. It isn't a
  typed `zoom` word either: that is out of scope and `ZoomKind` is unchanged.
- **Menu glyphs.** The icon slot is 16 pt with the same `ICON_STROKE`. The check mark is a
  two-segment tick in the same text colour. Disabled rows paint their icon in the disabled text
  colour.
- **Check rows.** Clicking one keeps the menu open, as `ui.checkbox` does today. If
  `leading_space` misbehaves in egui 0.29.1, `menu_row` becomes a custom `allocate_exact_size` row.

## Test approach

Menus are opened by real pointer clicks, the way `tests/it/ui/object_snap_menu.rs` does it
(nested menus open on hover). AC 1: every row's label run and shortcut run are separate. No run
contains `\t`. All shortcut runs in one menu share a right edge (±1 pt) that lies right of every
label. AC 2/3: line shapes are painted inside the slot rect, which spans from the menu's left edge
to just left of the label x on the same row: there for iconed rows and Grid/Snap/Ortho while on,
empty for Save As, Export Layers and Zoom All. AC 4: no run contains `▶`. AC 5: the order of
the View menu's runs. AC 6/7: a click on the menu row leaves the same camera as `F` on a twin
`App`, and as `frame_all` or `frame_bed` when the document is empty. AC 8/9: a harness `tap` of
Ctrl+A, then the selection count and one Ctrl+Z; with the command line focused, the selection is
unchanged.

## Risks

- LOC cap: `menubar.rs` has 287 implementation lines, over the 270 mark. T1 moves the Open Recent
  code (about 70 lines) into `menubar/recent.rs` before anything grows, leaving about 225;
  after this spec it is about 250. The next seam is `menubar/view.rs`, for the View menu and
  the `do_zoom_*` helpers. `icons.rs` is at 152, so the new glyphs go in `icons/menu.rs`.
- Some existing tests find rows by their tab labels (`"New\tCtrl+N"` in
  `tests/it/ui/discard_dialog_pointer_click.rs`, `Open Recent ▶` in
  `tests/it/app/document_title_and_file_feedback.rs`). Others scan the source for
  `"Grid\tF7"` (`src/ui/menubar/tests.rs`). T5 rewrites them as label-run tests in the same
  commit.
- Order dependency: this spec lands after LCV-164 (`Camera::frame_bed`), LCV-167 (Title Case row
  names: `Export Layers`, `Bed Size…`, `AI Settings…`, `Object Snap`) and LCV-165. Use the row
  names as they are on the branch at T5.
- Mutation testing: no. No change to `src/agent/`, `export.rs` or `History`.
