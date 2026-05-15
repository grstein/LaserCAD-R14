# LCD-005 - Normal File Menu With SVG Workflow

Status: Ready
Priority: P1
Owner: Product Owner
Implementation: Unassigned

## Problem

Laser-cutting users expect the File menu to expose the basic document workflow directly: start a new drawing, open an existing drawing, save the current drawing, configure the machine bed/table size, and exit. The current File menu exposes multiple SVG save presets instead of a normal file workflow, which makes a simple CAD task feel like an export utility.

The product should not add a separate native project format yet. SVG is already the product handoff to LaserGRBL and should remain the only user-visible file format for this demand.

## Smallest Useful Outcome

Replace the current File menu shape with one narrow SVG-first workflow:

- `New` starts a blank drawing.
- `Open SVG...` opens a file picker and loads only LaserCAD-compatible plain SVG into editable geometry.
- `Save SVG...` opens a save picker/download flow and writes the current drawing as plain SVG.
- `Bed size...` opens the existing table/document size configuration.
- `Exit` exits the native app when available and is disabled or harmless in the browser.

## Scope

In:

- File menu items in this order: `New`, `Open SVG...`, `Save SVG...`, separator, `Bed size...`, separator, `Exit`.
- Shortcuts: `Ctrl+N`, `Ctrl+O`, `Ctrl+S` for the first three actions.
- `Open SVG...` uses an OS file picker in Tauri and a browser file input in web builds.
- `Save SVG...` uses an OS save picker in Tauri and browser download in web builds.
- Open support is limited to the plain SVG subset emitted by LaserCAD R14 for current editable entity types.
- Unsupported SVG must be rejected with a clear user-facing message and must leave the current document unchanged.
- `Bed size...` edits `state.documentBounds` in millimeters through the existing state path and refreshes viewport/export dimensions.
- New and Open replace the whole current document: entities, selection, undo/redo history, and autosave snapshot.
- If the current document contains entities, New and Open require a simple discard confirmation before replacing it.

Out:

- A native `.lasercad`, JSON, ZIP, or database-backed project format.
- Importing arbitrary SVG from illustration tools.
- Text, fills, masks, filters, CSS-driven geometry, symbols, nested transforms, images, or rich SVG effects.
- Multiple save presets in the File menu.
- Recent files.
- Dirty-state tracking, automatic overwrite of a remembered path, or full "Save vs Save As" behavior.
- Material, machine, or laser power configuration.

## Acceptance Criteria

- File menu shows exactly the scoped items, with no separate `Save SVG (cut)`, `Save SVG (mark)`, or `Save SVG (engrave)` entries.
- `Ctrl+N` and File -> `New` clear the drawing after confirmation when entities exist, then show an empty drawing in millimeters.
- `Ctrl+O` and File -> `Open SVG...` open a file picker, accept a LaserCAD-exported SVG, replace the current document, and render the imported geometry at the same millimeter coordinates and document bounds.
- Opening an unsupported SVG shows a clear rejection message and does not mutate entities, selection, document bounds, or history.
- `Ctrl+S` and File -> `Save SVG...` open a save/download flow and produce a LaserGRBL-compatible plain SVG using the existing conservative export rules.
- File -> `Bed size...` opens the table size configuration, stores positive millimeter dimensions through the state API, and affects the exported SVG `width`, `height`, and `viewBox`.
- File -> `Exit` closes the Tauri window when running natively; in the browser it is disabled or displays a short "not available in browser" message without breaking state.
- All state mutation for New, Open, Save side effects, and Bed size follows the state API/commands contract; no direct external mutation of `state.entities`, `state.selection`, or `state.documentBounds`.

## Expected Tests

- Unit tests for importing the exact SVG subset emitted by current LaserCAD export for line, polyline/rect, circle, and arc path geometry.
- Unit tests that unsupported SVG constructs are rejected without partial document mutation.
- Unit/export tests that `Save SVG...` still satisfies existing LaserGRBL plain SVG compatibility assertions.
- Interaction tests or focused DOM tests for File menu labels, enabled/disabled state, and shortcuts `Ctrl+N`, `Ctrl+O`, `Ctrl+S`.
- State/history tests proving New and Open reset selection and undo/redo history.
- Manual checks in web build and Tauri build for picker behavior, browser download fallback, and native exit behavior.

## Notes For Implementation Agents

- Relevant docs/files:
  - `README.md`
  - `docs/design.md`
  - `docs/plan.md`
  - `src/app/state.ts`
  - `src/ui/menubar.ts`
  - `src/ui/dialogs.ts`
  - `src/ui/document-size-dialog.ts`
  - `src/io/export-svg.ts`
  - `src/io/file-download.ts`
  - `src/tauri-bridge.ts`
- Risks:
  - SVG import can grow into a compatibility project. Keep it to LaserCAD-authored plain SVG only.
  - Direct state mutation currently appears in nearby File menu code; implementation must route replacement/reset through the state API or add the smallest needed state method with tests.
  - Tauri plugins must remain dynamically imported.
- Product decisions:
  - SVG is the only user-visible file format for this demand.
  - `Save SVG...` is intentionally a save picker/download every time; remembering paths and recent files are deferred.
  - File menu should feel normal, but not become a full desktop document-management subsystem.
