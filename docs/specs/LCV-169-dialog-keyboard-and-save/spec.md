# LCV-169 — Dialog keyboard, destructive styling and Save/Discard/Cancel

- **Status**: Specified
- **Depends on**: LCV-163, LCV-167
- **Implementation**: -

## Problem

Dialogs are mouse-only and inconsistent. Enter and Escape do nothing in a dialog (ADR 0002 §A6
has no dialog row; LCV-069 made dialogs a visual convention, not modal). Windows close in four
ways: × only (`About LaserCAD`, `Keyboard Shortcuts`), OK (`Error`), OK/Cancel (`Bed Size`),
Done plus × (`AI Settings`). `Discard` in `app/discard.rs::draw_discard_dialog` looks like any
other button, and the dialog has no Save, so the operator must Cancel, save, and redo the action.

## Stories

- As an operator, I want Enter to confirm and Escape to cancel any dialog, so that I never leave
  the keyboard.
- As an operator, I want a destructive button to look destructive, and to save straight from the
  "Discard unsaved changes?" prompt.

## Acceptance criteria

The *topmost dialog* is the open dialog opened last.

1. WHILE a dialog is open, WHEN the operator presses Enter THE SYSTEM SHALL trigger the topmost
   dialog's primary button, and the command line and active tool SHALL NOT receive the key.
2. WHILE a dialog is open, WHEN the operator presses Escape THE SYSTEM SHALL cancel or close the
   topmost dialog and leave the command line text, its focus and the active tool unchanged.
3. IF the Bed Size values are invalid THEN Enter SHALL keep the dialog open with its validation
   message, exactly as OK does (LCV-114).
4. THE SYSTEM SHALL give every dialog (About, Keyboard Shortcuts, Error, Bed Size, AI Settings,
   Layers, Discard) a Cancel or Close button, and the title-bar × SHALL do the same.
5. THE SYSTEM SHALL order dialog buttons primary first, Cancel/Close last, left to right.
6. THE SYSTEM SHALL paint a destructive button's text (`Discard`) in the `danger` colour
   (`render/palette.rs`, LCV-163) with no danger fill.
7. WHEN unsaved changes park an action THE SYSTEM SHALL offer `Save`, `Discard`, `Cancel` in that
   order, `Save` being the primary (reverses LCV-113 Out of scope).
8. WHEN the operator chooses Save THE SYSTEM SHALL run the normal save path and, only if the
   write succeeds, run the parked action once; IF the save fails or is cancelled THEN the
   drawing stays open and the parked action is dropped.
9. THE SYSTEM SHALL amend ADR 0002 §A6 with a "dialog" key class and record the pattern in
   DESIGN.md §7 and §8 in this spec's last task.

## Out of scope

- True modal blocking of the canvas (egui 0.29 has no `Modal`; LCV-069 holds).
- Tab focus order beyond egui's default; new dialogs.

## Open questions

- None. Decided (self-approved per user goal): an ADR 0002 amendment, not a new ADR; LCV-113's
  no-Save rule is reversed; Escape with a dialog open never touches the command line (ADR 0003
  unchanged otherwise); Enter keeps LCV-114 validation.
