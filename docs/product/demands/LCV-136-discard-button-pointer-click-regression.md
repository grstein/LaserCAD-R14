# LCV-136 - Discard responds to real pointer clicks

- **Status**: Draft
- **Phase**: 11
- **Depends on**: LCV-113, LCV-118, LCV-119
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: -

## Problem

The user reports that Discard fails for New, Open and Exit. The five existing
`tests/lcv113.rs` cases pass without clicking Discard, and direct
`apply_dialog_result` tests bypass the widget. The root cause is unconfirmed.

## Scope

- Reproduce through the real App and confirmation widget before selecting a fix.
- Diagnose pointer hit testing, window layering, input ownership and repeated Close events independently.
- Preserve existing destructive-action semantics, with no click-through to the drawing.

## Out of scope

Save-and-continue, an Exit-only workaround presented as a general fix, native
dialog automation, or waiting for LCV-132's harness consolidation.

## Acceptance criteria

1. A settled-frame test locates the actual Discard button and sends PointerMoved, primary press and release across frames; it does not invoke `apply_dialog_result` to simulate the click.
2. Separate New, OpenPath with a test-owned SVG, and Exit cases perform the parked action exactly once, dismiss confirmation and do not replay it on later frames.
3. Cancel, clicked by the same real pointer path, preserves entities, selection, history/revision, filename and dirty state for each action.
4. A missing or malformed OpenPath preserves the original drawing/history/filename and displays the existing error surface; it neither clears the drawing nor retries automatically.
5. Confirmation input cannot draw, select, pan, activate an underlying control or dispatch a second destructive action.
6. Repeated native Close requests neither replace the parked action nor consume Discard/Cancel input; confirmed Exit does not reopen confirmation.
7. The handover records which paths reproduce, frame/input evidence and the justified root correction. Non-reproduction is reported, not assigned an invented cause.

## Expected tests

- AC 1-2: real-App pointer regressions for New/OpenPath/Exit, including idle frames after release and Close output inspection.
- AC 3: pointer Cancel for every pending action and exact state comparisons.
- AC 4: test-owned missing/malformed SVG fixtures.
- AC 5-6: armed underlying tool, underlying clickable control and repeated Close-event fixtures.
- AC 7: manual native New/Open/Exit, native Open cancellation and successful selection, using disposable configuration/data/files.

## Open questions

No unresolved product choice. Reproduction and root cause are investigation
outcomes required before a fix is implemented.

## Notes

Primary files: `src/ui/dialogs.rs`, `src/app/file_ops.rs`,
`src/app/panels.rs`, `tests/lcv113.rs`. Follow LCV-118/119 and ADR 0002/0005/0006:
tests never arm native dialogs or send Ctrl+O/Ctrl+S/Ctrl+Shift+S.
