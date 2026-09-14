# LCV-138 - Native document title and honest file feedback

- **Status**: Draft
- **Phase**: 11
- **Depends on**: LCV-113, LCV-119, LCV-136
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: -

## Problem

The native title does not identify the current drawing or unsaved changes.
Recovery feedback can be mistaken for a saved SVG, and recent-file basenames
alone are ambiguous.

## Scope

Native filename/modified title, honest recovery wording, distinguishable
recent-file entries and explicit recent-file error reporting.

## Out of scope

Changing saved-revision semantics, content-based dirty detection, a save
dashboard, a new file lifecycle or persistent title state.

## Acceptance criteria

1. The native title derives from `current_file`, `has_unsaved_changes()` and `APP_TITLE`; use `Untitled.svg` without a path and prefix `*` only when the existing dirty predicate is true.
2. Update the title after frame mutations, including dialog/agent completion, so an idle app does not retain stale identity. Cache only the last emitted title in App.
3. Successful Save/Open/Save As update title state; failed/cancelled operations do not falsely clear the marker or change identity. Autosave never means the SVG is saved.
4. Recovery status explicitly describes recovery and explains the distinction on hover without changing scheduling.
5. Recent entries with identical basenames show distinguishing path context and full-path tooltips. A failed lookup/open displays an error and preserves the drawing/history.
6. Title changes use viewport commands, not an extra repaint site; all persistence tests use test-owned paths.

## Expected tests

- AC 1-3: root viewport Title output for blank, edited, saved, opened, recovered and failed-operation states.
- AC 3-4: successful autosave keeps the unsaved-file marker; existing dirty/recovery regressions remain meaningful.
- AC 5: same basename under different directories, full-path hover content, missing/malformed files and unchanged-state assertions.
- AC 6: no title repaint site, transient-state initialization and injected persistence paths.

## Open questions

None at product level. Architect records the narrow seam before extending
`app/mod.rs` at 292 implementation LOC or `file_ops.rs` at 281.

## Notes

Prefer `src/app/document_title.rs`; related files are `src/app/init.rs`,
`src/ui/statusbar.rs` and `src/ui/menubar.rs`. Retain the application-name
constant as the suffix rather than duplicating it.
