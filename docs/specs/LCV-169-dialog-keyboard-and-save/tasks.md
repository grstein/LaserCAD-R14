# LCV-169 — Tasks

- [x] T1 Seam: if `src/app/mod.rs` is ≥ 290 implementation lines, move the autosave fields into
  `AutosaveState` in `app/autosave.rs` with no behaviour change; else tick with "not needed"
  (files: src/app/mod.rs, src/app/autosave.rs, src/app/init.rs)
- [x] T2 [AC4] [AC5] Test first: open each of the seven dialogs; its painted button runs are
  `Close` (About, Keyboard Shortcuts, Error, AI Settings), `OK`·`Cancel` (Bed Size),
  `Apply`…`Close` (Layers) and `Save`·`Discard`·`Cancel` (Discard), in reading order, with the
  Cancel/Close run last (files: tests/it/ui/dialog_keyboard.rs, tests/it/ui/mod.rs)
- [x] T3 [AC4] About gets `Close`; Error's `OK` becomes `Close` and it gets a × (files:
  src/ui/dialogs.rs, src/app/panels.rs)
- [x] T4 [AC4] Keyboard Shortcuts gets a `Close` row below its `ScrollArea`;
  `shortcuts_dialog_fits.rs` stays green unchanged (files: src/ui/shortcuts_dialog.rs)
- [x] T5 [AC4] AI Settings `Done` → `Close` (same close path); Bed Size gets a × = Cancel;
  update the Done-keyed tests (files: src/agent/settings_ui.rs, src/app/bed_dialog.rs,
  tests/it/agent/panel_and_settings.rs)
- [x] T6 [AC4] Test: a click on each dialog's title-bar × leaves the same state as its
  Cancel/Close button (files: tests/it/ui/dialog_keyboard.rs)
- [x] T7 [AC6] [AC7] Test first: the Discard dialog paints `Save`, `Discard`, `Cancel` left to
  right; the `Discard` run's colour is `palette::DANGER`; no filled shape uses `DANGER` (files:
  tests/it/ui/dialog_keyboard.rs, tests/it/ui/discard_dialog_pointer_click.rs)
- [x] T8 [AC6] [AC7] `discard.rs` draws its own window with `DiscardChoice`; `confirm_dialog`
  is removed (files: src/app/discard.rs, src/ui/dialogs.rs, src/ui/mod.rs)
- [x] T9 [AC8] Test first: Save with a writable current path runs the parked `New` once; a path
  in a missing folder, and an untitled drawing (disarmed Save As), keep the drawing, drop the
  action and leave `pending_action` `None` (files: src/app/discard.rs)
- [x] T10 [AC8] `apply_discard_choice(Save)`: `action_save`, then run the action iff
  `!has_unsaved_changes()` (files: src/app/discard.rs, src/app/file_ops.rs)
- [x] T11 [AC1] [AC2] Test first: About then Shortcuts open, Escape closes only Shortcuts; Enter
  on Bed Size commits and closes; with LINE past its first point and the focused command line
  holding `12,3`, Escape on a dialog leaves the text, focus and tool prompt unchanged, and Enter
  pushes nothing to the recall ring (files: tests/it/ui/dialog_keyboard.rs)
- [x] T12 [AC1] [AC2] `Dialog`, `sync_dialog_order`, `topmost`; `App::dialog_order` (files:
  src/app/dialog_order.rs, src/app/mod.rs, src/app/init.rs)
- [x] T13 [AC1] [AC2] `input.rs::take_dialog_key` (consume, restore command-line focus), called
  first in `update_ui` (files: src/app/input.rs, src/app/mod.rs)
- [x] T14 [AC1] [AC2] `draw_dialogs` hands the key to the topmost dialog. About, Shortcuts,
  Error and AI Settings treat it as a click (files: src/app/panels.rs, src/ui/dialogs.rs,
  src/ui/shortcuts_dialog.rs)
- [x] T15 [AC1] [AC2] [AC3] Bed Size, Layers and Discard take the key. Test: Enter and OK on
  twin apps with an out-of-range typed draft give the same clamped bed (files:
  src/app/bed_dialog.rs, src/ui/layers_dialog.rs, src/app/discard.rs)
- [x] T16 [AC1] Test and code: with the AI Settings system prompt focused, Enter inserts a
  newline and the window stays open; `SYSTEM_PROMPT_ID` is skipped by `take_dialog_key` (files:
  src/agent/settings_ui.rs, src/app/input.rs, tests/it/ui/dialog_keyboard.rs)
- [x] T17 [AC9] ADR 0002 §A6 amendment note (dialog row) (files: docs/adr/0002-*.md)
- [x] T18 [AC9] DESIGN.md §7 (dialog buttons, destructive text, AI Settings Done → Close for pre-SDD LCV-141) and §8 (Enter/Escape rule)
  (files: DESIGN.md)
- [x] T19 CHANGELOG line (files: CHANGELOG.md)
