# LCV-169 — Plan

## Approach

Enter and Escape are taken for dialogs **before** anything else reads them, so no other reader
has to know about dialogs:
- **Order.** `app/dialog_order.rs` (new) names the seven dialogs (`Dialog`) and keeps
  `App::dialog_order: Vec<Dialog>`. Each frame it keeps the dialogs that are still open, in
  order, and appends newly opened ones, so the last entry is the topmost dialog.
- **Key take.** `app/input.rs::take_dialog_key` is the first call in `App::update_ui`. While a
  dialog is open it takes Enter and Escape out of the frame with `InputState::consume_key`, so
  `process_shortcuts`, `process_input`, the command-line `TextEdit` and the tool never see them.
  egui drops focus on Escape during `begin_pass`, so an Escape taken this way sets
  `focus_command_line` again when `command_line_focused` was true (AC 2). This lives in
  `input.rs`, so ADR 0002 §A6 still has exactly two key readers.
- **Delivery.** `draw_dialogs(ctx, app, key)` passes `Some(DialogKey)` to the topmost dialog
  only, and each dialog treats it as a click: `ui.button(primary).clicked() || enter`. Enter
  therefore runs the same code as OK, including the LCV-141 Done close guard and the LCV-114
  clamp (AC 3), with no second path.
- **Discard.** `app/discard.rs` draws its own window with `Save`, `Discard` (text in
  `palette::DANGER`) and `Cancel`. `confirm_dialog` has no other caller, so it is removed. Save
  calls `App::action_save`. The parked action runs only if `!has_unsaved_changes()` afterwards;
  otherwise it is dropped (AC 8).

## Touches

- `src/app/dialog_order.rs` (new): `Dialog`, `sync_dialog_order`, `topmost`.
  `src/app/mod.rs`: one field and the `update_ui` call. `src/app/init.rs`: the default value.
- `src/app/input.rs::take_dialog_key`, plus a new gate-table row in its module doc.
- `src/ui/dialogs.rs`: `DialogKey { Enter, Escape }`. `about_dialog` gets `Close`.
  `error_dialog`'s `OK` becomes `Close`, and it gets a ×. `confirm_dialog` is removed and
  `ui/mod.rs` updated. Each dialog takes a `key: Option<DialogKey>` argument.
- `src/ui/shortcuts_dialog.rs`: a `Close` row below the `ScrollArea`; `key` argument.
- `src/app/bed_dialog.rs`: × = Cancel; `key` argument. `src/ui/layers_dialog.rs`: `key`
  argument (Enter = Apply, Escape = Close).
- `src/app/panels.rs::draw_dialogs`, `::error_modal`, `::agent_settings_dialog`: take the key.
  `src/agent/settings_ui.rs`: `Done` becomes `Close`, and the system-prompt editor gets a fixed
  `SYSTEM_PROMPT_ID`.
- `src/app/discard.rs`: the three-button window, `DiscardChoice`, and `apply_discard_choice`
  (replaces `apply_dialog_result`). `app/file_ops.rs`: doc mentions only.
- ADR 0002 §A6: amendment note adding a `dialog | Enter, Escape | yes — topmost dialog` row.
  DESIGN.md §7, §8; CHANGELOG.

## Decisions (self-approved per user goal)

- **One-button dialogs.** About, Keyboard Shortcuts and Error each have a single `Close`
  button, which is also primary. Error's `OK` is renamed. AI Settings' `Done` becomes `Close`:
  it already ran the × path (LCV-141 AC 6), so the label changes and the behaviour doesn't.
  LCV-141 is pre-SDD, so DESIGN.md §7 records the change.
- **Primaries.** Bed Size: `OK`. Layers: `Apply`, with `Close` as its cancel. Discard: `Save`.
  Every dialog gets a ×, which runs its Cancel/Close.
- **Multiline exception.** While the AI Settings system-prompt editor has focus, Enter is not
  taken, so it still types a newline. Escape is always taken.
- **Ties.** Two dialogs opened in the same frame are ordered by `Dialog`'s declaration order:
  About, Shortcuts, AI Settings, Layers, Bed, Discard, Error. Error is last, so it is topmost.
- **AC 3.** LCV-114 shipped a clamp, not a refusal, and `DragValue` updates while you type. So
  "exactly as OK" means Enter produces OK's clamped result, and a twin-app test proves it.
- **Save success.** "The write succeeded" means `has_unsaved_changes()` is false after
  `action_save`. That covers a failed write, a cancelled Save As, and the disarmed dialog in
  tests (ADR 0005) without a new return type.

## Test approach

`tests/it/ui/dialog_keyboard.rs` (new) uses real key taps and painted runs. For each of the
seven dialogs: the button text runs and their order (AC 4/5), a × click at the title bar's right
end of `area_rect` giving the Cancel/Close state, and Enter/Escape results. Topmost: About then
Shortcuts, where Escape closes only Shortcuts. AC 2: LINE after its first point, the command line
holding `12,3` with focus; Escape leaves all three. AC 6 uses painted colour and no `DANGER` fill.
AC 8: units in `discard.rs` using a tempdir path, a path inside a missing folder, and untitled.

## Risks

- LOC cap: `settings_ui.rs` 259 → ~262 (seam: move the prompt editor into `settings_ui/prompt.rs`
  if it passes 270); `app/mod.rs` ~295 after LCV-165, so T1 moves the autosave fields first.
  `input.rs` 147 → ~185, `dialogs.rs` 132 → ~165, `discard.rs` 105 → ~150.
- `shortcuts_dialog_fits.rs` (ADR 0009 slack floor) may tighten with the `Close` row; the
  `ScrollArea` absorbs it, and the floor test must stay green without being changed.
- `discard_dialog_pointer_click.rs` and the Done/OK-keyed tests are updated in the tasks that
  change them. No test sends Ctrl+O/S (ADR 0005).
- Mutation testing: no (no `src/agent/` logic, export or `History` change).
