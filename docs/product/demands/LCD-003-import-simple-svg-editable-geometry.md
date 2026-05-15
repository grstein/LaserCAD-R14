# LCD-003 - Import Simple SVG As Editable Geometry

Status: Needs Refinement
Priority: P2
Owner: Product Owner
Implementation: Unassigned

## Problem

Users may have existing simple SVG files from other tools and may want to edit them in LaserCAD R14 before cutting. This is a real workflow, but SVG import can easily become a large compatibility project that violates the product's simplicity.

## Smallest Useful Outcome

Not ready. The likely useful version is importing only a tiny, documented subset of plain SVG into editable LaserCAD entities, but the exact subset needs refinement before implementation.

## Scope

In:

- Candidate support for `line`, `circle`, and simple arc paths.
- Candidate support for `width`/`height` in `mm` and `viewBox`.
- Clear rejection messages for unsupported SVG.

Out:

- Full SVG compatibility.
- Text, fill, masks, filters, clips, gradients, symbols, CSS-driven geometry, or nested transforms.
- Importing arbitrary files from illustration tools without cleanup.

## Acceptance Criteria

- Not ready for implementation.
- Must be split or tightened before any coding starts.

## Expected Tests

- Future tests should use minimal SVG fixtures and explicit unsupported fixtures.

## Notes For Implementation Agents

- Do not implement from this demand yet.
- Relevant docs/specs/files: `docs/plan.md`, `src/io/export-svg.ts`, `src/core/document/schema.ts`.
- Product decision still needed: exact SVG subset, transform policy, unit fallback policy, and user-facing error behavior.

