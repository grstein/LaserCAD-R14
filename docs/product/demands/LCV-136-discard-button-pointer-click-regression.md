# LCV-136 - Discard responds to real pointer clicks

- **Status**: Draft
- **Phase**: 11
- **Depends on**: LCV-113, LCV-118, LCV-119
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: -

## Problem

The user reports that clicking Discard on the confirmation dialog (File > New,
File > Open, and the window close button, all guarded since LCV-113) does not
reliably dismiss the dialog and continue the destructive action.

The gap is in test coverage, not just in the report: `tests/lcv113.rs` and
`src/app/file_ops.rs`'s inline tests exhaustively cover the *state machine* —
`request_new`/`request_open`/`request_exit` parking the right `PendingAction`,
and `apply_dialog_result` running it exactly once on `Confirmed` and restoring
nothing on `Cancelled` — but every one of those tests calls
`apply_dialog_result` directly. None of them drives a real pointer through
`crate::ui::confirm_dialog`'s `ui.button(confirm_label).clicked()` the way an
operator's mouse does. So the reported failure mode — the click landing
somewhere other than the button — is exactly the one path this demand's
dependencies never exercised. The root cause is unconfirmed; this demand
exists to reproduce it for real before anyone picks a fix.

## Scope

- Reproduce through the real `App` and the real `confirm_dialog` widget —
  locate the painted button and click it — before selecting a fix.
- Diagnose pointer hit testing, window layering/ordering, input ownership and
  repeated Close events independently; do not assume which one is at fault.
- Preserve existing destructive-action semantics (LCV-113): no click-through
  to the drawing underneath the modal, no double-dispatch of the parked
  action.
- Add the outstanding `pixels_per_point == 1.0` precondition to
  `tests/harness/paint.rs::painted_runs_at` (parked during the LCV-132/134
  drive): this is the first demand, in implementation order, to write a test
  that locates a widget through painted text, so it is the one that closes
  that precondition out.

## Out of scope

Save-and-continue, an Exit-only workaround presented as a general fix, native
dialog automation, or waiting for a future harness consolidation. Redesigning
`confirm_dialog` itself (e.g. giving it a non-immediate-mode API) — if the
root cause turns out to need that, file a follow-up demand rather than
expanding this one.

## Acceptance criteria

1. A settled-frame test locates the real "Discard" (and "Cancel") button
   through `tests/harness/paint.rs`'s painted text runs — never a hardcoded
   screen coordinate and never `apply_dialog_result` called directly — and
   drives a warm-up `PointerMoved` to that position followed by a
   `PointerButton` press/release pair across frames, per the harness's
   pointer-click convention (ADR 0002 §A4 rule 3).
2. `tests/harness/paint.rs::painted_runs_at` asserts `ctx.pixels_per_point()
   == 1.0` before collecting runs, so a test that forgets to call
   `ctx.set_pixels_per_point(1.0)` first fails loudly instead of silently
   collecting positions at the wrong scale. Every existing caller in the tree
   already sets it, so no existing test's behavior changes.
3. Through that real pointer path, separate New, OpenPath (against a
   test-owned SVG fixture) and Exit cases each run their parked action
   exactly once, dismiss the confirmation dialog, and do not replay the
   action on a later, otherwise-idle frame.
4. Through the same real pointer path, clicking Cancel for each of the three
   parked actions preserves entities, selection, history revision,
   `current_file` and dirty state exactly as they were before the dialog
   opened.
5. A parked OpenPath whose file is missing or fails to parse, confirmed
   through the real pointer path, preserves the original drawing, history and
   filename and surfaces the existing error modal (`app.error_message`); it
   neither clears the drawing nor retries automatically.
6. While the dialog is open, pointer input at a canvas location behind it
   does not draw, select, pan or otherwise activate the canvas or any
   underlying control; a second click on Discard/Cancel after the first has
   already been handled does not dispatch a second destructive action.
7. A repeated native Close request (`egui::ViewportEvent::Close` arriving on
   more than one frame) while the dialog is open neither replaces the parked
   action nor consumes a subsequent Discard/Cancel click; a confirmed Exit
   does not reopen the confirmation dialog on a later frame.
8. The handover states, for each of New/Open/Exit, whether the real-pointer
   path reproduces a failure to discard, with the frame/input evidence, and —
   only if a defect is found — the corrected root cause and the fix applied.
   Non-reproduction is reported as such, never papered over with an invented
   "fix" for a bug that could not be shown to exist.

## Expected tests

- AC 1, 3: real-`App` pointer regressions for New, OpenPath and Exit, each
  locating its button via painted text, including an idle frame after release
  to prove the action is not replayed.
- AC 2: a unit test in `tests/harness/paint.rs` (or a caller) proving the new
  assertion fires when `pixels_per_point` is left at its non-1.0 default, and
  that every existing suite that already calls `set_pixels_per_point(1.0)`
  stays green.
- AC 4: pointer Cancel for every pending action, with exact before/after state
  comparisons (entity list, selection, `history.revision()`, `current_file`,
  `has_unsaved_changes()`).
- AC 5: a test-owned missing-path and a test-owned malformed-SVG fixture,
  both driven through the real pointer path against OpenPath.
- AC 6: an armed drawing tool and an underlying clickable control (e.g. a
  toolbar button) behind the dialog, plus a double-click sequence on Discard.
- AC 7: a repeated `ViewportEvent::Close` fixture across two or more frames,
  and a confirmed-Exit frame followed by an idle frame checked for a
  reopened dialog.
- AC 8: manual native smoke — New/Open/Exit with unsaved changes, Open
  cancellation, and a successful Open selection — using disposable
  configuration/data/files, reported in the handover regardless of outcome.

## Open questions

None. Reproduction and root cause are investigation outcomes this demand
exists to produce, not a product decision to make in advance.

## Notes

Primary files: `src/ui/dialogs.rs` (`confirm_dialog`), `src/app/file_ops.rs`
(`draw_discard_dialog`, `apply_dialog_result`, the four `request_*` guards),
`src/app/panels.rs` (`draw_dialogs`, called after `viewport::draw` — egui's
own rule is that a `Window` must be added after every top-level panel, so the
dialog's `Area` should already out-rank the canvas for hit-testing; this is a
line of investigation, not a conclusion), `tests/lcv113.rs`,
`tests/harness/paint.rs`. Follow LCV-118/119 and ADR 0002/0005/0006: tests
never arm native dialogs and never send Ctrl+O/Ctrl+S/Ctrl+Shift+S.

`egui::containers::panel::PanelState` and `ctx.memory(|m| m.area_rect(id))`
are the two ways to read a *container's* geometry without touching painted
text; a button has neither, which is why AC 1 goes through painted text
instead.
