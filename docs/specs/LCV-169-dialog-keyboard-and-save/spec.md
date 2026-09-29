# LCV-169 — Dialog keyboard, destructive styling and Save/Discard/Cancel

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

Dialogs are the only overlays, yet they are mouse-only and inconsistent:

- **Keyboard.** Enter and Escape do nothing in a dialog. ADR 0002 §A6 has no dialog row, and
  LCV-069 made dialogs a visual convention, not truly modal.
- **Closing.** The six windows close in four ways: × only (`About LaserCAD`,
  `Keyboard shortcuts`), OK (`Error`), OK/Cancel (`Bed size`), and Done plus ×
  (`Agent Settings`).
- **Discard.** `Discard` in `app/discard.rs::draw_discard_dialog` looks like any other button.
  The dialog has no Save option, so the operator must Cancel, press Ctrl+S, and then redo the
  action (LCV-113 kept it to two buttons on purpose).

## Stories

- As an operator, I want Enter to confirm and Escape to cancel any dialog, so that I never leave
  the keyboard.
- As an operator, I want a destructive button to look destructive.
- As an operator, I want to save straight from the "Discard unsaved changes?" prompt.

## Direction

- Enter triggers the primary button and Escape the cancel/close action of the topmost dialog.
  While a dialog is open, the tool and command line do not see those keys. This needs an ADR 0002
  §A6 row (a "dialog" class).
- Button order is primary → Cancel. A destructive primary (`Discard`) uses the `danger` token as
  text colour, not as a fill.
- One close pattern: every dialog has a Cancel or Close button. The title-bar × does the same
  thing.
- The discard dialog offers `Save`, `Discard` and `Cancel`. Save runs the normal save path and
  then the parked action.
- DESIGN.md §7 and §8 are updated in this spec's last task.

## Acceptance criteria

To be written by /specify.

## Out of scope

- True modal blocking of the canvas (egui 0.29 has no `Modal`; LCV-069 still holds).
- Tab focus order inside dialogs beyond egui's default.
- New dialogs.

## Open questions

- ✱ ADR 0002 §A6: a new "dialog" class for Enter/Escape. Is that an amendment or a new ADR?
- ✱ LCV-113 (Out of scope) excludes a Save button from the discard dialog. This spec reverses
  that.
- ✱ ADR 0003: one Escape clears the command line and releases focus. With a dialog open, Escape
  should close the dialog and leave the command line untouched.
- ✱ LCV-114 pins the Bed size dialog's OK/Cancel behaviour. Enter = OK must keep its validation.
- Lowest priority of the UI Drafts. Worth doing only if the other dialog complaints come back.
