# LCD-004 - Offset Line And Circle By Distance

Status: Ready
Priority: P2
Owner: Product Owner
Implementation: Unassigned

## Problem

Laser-cut parts often need repeated spacing, clearance, or concentric geometry: a tab line parallel to an edge, a clearance circle around a hole, or a simple construction mark at a known distance. Today the user must recreate that geometry manually, which is slower and risks small measurement errors.

The useful product need is a constrained CAD offset for the simplest geometry LaserCAD already owns precisely. This is not a request for general curve, path, polyline, or kerf compensation offset.

## Smallest Useful Outcome

Add a command-line Offset tool for exactly one preselected `line` or `circle`. The user runs `offset` or `o`, provides or confirms a positive distance in millimeters, and indicates the side or direction with the mouse. The tool creates one new entity and leaves the original unchanged.

No toolbar button is added in this version.

## Scope

In:

- Command aliases `offset` and `o`.
- Tool activation only when exactly one entity is already selected.
- Supported selected entities: `line` and `circle`.
- Line offset creates a parallel line on the cursor-indicated side.
- Circle offset creates a concentric circle outward or inward based on cursor position.
- Positive offset distance in millimeters.
- Dashed preview while the offset is being indicated.
- Commit through the document command/history path so undo and redo work.
- Command-line errors for missing, invalid, multiple, or unsupported selection without mutating the document.

Out:

- Polyline, rectangle, arc, path, spline, joined-chain, or multi-entity offset.
- Robust curve offset or corner join behavior.
- Kerf compensation automation.
- Repeated offset mode.
- New toolbar button or chrome expansion.
- New event bus events or state fields unless an ADR is added.

## Acceptance Criteria

- Running `offset` or `o` with exactly one selected `line` arms the Offset tool.
- Running `offset` or `o` with exactly one selected `circle` arms the Offset tool.
- Running the command with no selection, multiple selection, or an unsupported selected entity emits a command-line error and does not add, remove, or mutate entities.
- For a `line`, the committed result is one new line translated by the perpendicular normal toward the cursor-indicated side.
- The offset line keeps the original line length and endpoint ordering.
- The perpendicular distance between the original line and the new line is exactly the requested distance.
- For a `circle`, a cursor point outside the current radius creates one concentric circle with radius `r + d`.
- For a `circle`, a cursor point inside the current radius creates one concentric circle with radius `r - d`.
- Inward circle offsets where `r - d <= 0` are rejected with a command-line error and no document mutation.
- Numeric command-line distance is canonical, is interpreted in millimeters, and must be finite and greater than zero.
- Mouse-only commit may use the current geometric distance from the selected entity when that distance is finite and greater than zero.
- Zero, negative, non-finite, and missing distances are rejected without document mutation.
- Preview geometry is dashed and clears on commit or cancel.
- Undo removes the created offset entity, and redo restores it.

## Expected Tests

- Geometry/unit test: horizontal line offset upward by `10` mm creates a parallel line exactly `10` mm away.
- Geometry/unit test: horizontal line offset downward by `10` mm creates a parallel line exactly `10` mm away.
- Geometry/unit test: diagonal line offset preserves length and has perpendicular distance `d`.
- Geometry/unit test: circle outward offset with `r = 20`, `d = 5` creates radius `25`.
- Geometry/unit test: circle inward offset with `r = 20`, `d = 5` creates radius `15`.
- Geometry/unit test: inward circle offset rejects `d >= r`.
- Geometry/unit test: zero, negative, non-finite, and missing distances are rejected.
- Interaction/tool test: `offset` and `o` arm the tool for valid selected entities.
- Interaction/tool test: no selection, multiple selection, and unsupported selection show a command-line error and do not create an entity.
- Interaction/tool test: commit uses document command/history so undo and redo work.
- Interaction/tool test: dashed preview clears on commit and cancel.
- Regression: existing draw, edit, export, and command-line tests continue passing.

## Notes For Implementation Agents

- Relevant docs/files: `README.md`, `docs/design.md`, `docs/plan.md`, `src/app/state.ts`, command-line parsing, selection state, tool manager, document commands, geometry helpers, and SVG preview rendering.
- Risk: general offset algorithms can expand quickly. Keep this to one selected `line` or `circle` and one created entity.
- Risk: state and history bugs are likely if the tool mutates entities directly. Use the existing state API or command path.
- Product decision: this is a narrow laser-cutting spacing and clearance tool, not robust polyline/path offset and not kerf compensation.
