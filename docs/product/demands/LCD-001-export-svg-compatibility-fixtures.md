# LCD-001 - Export SVG Compatibility Fixtures

Status: Ready
Priority: P1
Owner: Product Owner
Implementation: Unassigned

## Problem

SVG export is the handoff from LaserCAD R14 to LaserGRBL. The README describes compatibility rules, but implementation agents need regression fixtures that prove exported files stay plain, measured in millimeters, and free of unsupported SVG features.

Without fixtures, future changes to entities, document bounds, or export presets can silently break the main product workflow.

## Smallest Useful Outcome

Add focused export regression coverage for the existing entity types and presets. The goal is not to test LaserGRBL itself; the goal is to prove that LaserCAD emits the conservative SVG subset already documented.

## Scope

In:

- Test export of a simple document with line, circle, and arc geometry.
- Verify root `svg` has `xmlns`, `width`/`height` in `mm`, and a coherent `viewBox`.
- Verify exported geometry is grouped by preset and uses the documented preset colors.
- Verify export forces `fill="none"` and does not include text, filters, masks, or clip paths.

Out:

- Importing the SVG back into LaserCAD.
- Automating LaserGRBL.
- Adding new export presets.
- Changing visual rendering in the editor.

## Acceptance Criteria

- `npm test` includes export SVG regression tests.
- The tests fail if unsupported SVG constructs appear in exported output.
- The tests cover at least one arc exported as SVG path arc data.
- Existing docs do not need behavior changes unless implementation discovers current docs are inaccurate.

## Expected Tests

- Unit test around `src/io/export-svg.ts`.
- XML/string assertions are acceptable if scoped to stable export attributes.
- Existing geometry tests must continue passing.

## Notes For Implementation Agents

- Relevant docs/specs/files: `README.md`, `docs/plan.md`, `src/io/export-svg.ts`, `src/core/types.ts`.
- Risk: brittle snapshot testing can make harmless ordering changes painful. Prefer targeted assertions.
- Product decision: keep export conservative and LaserGRBL-first.

