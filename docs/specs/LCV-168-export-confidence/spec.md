# LCV-168 — Save and export confidence

- **Status**: Draft
- **Depends on**: LCV-156
- **Implementation**: -

## Problem

Save and export succeed silently. `io/file_actions.rs::action_save` and `action_save_as` report
failures through the error dialog, but a successful write says nothing. The operator has to
trust the title bar to know that the file is on disk.

Geometry outside the bed is also exported without a word. LaserGRBL then either clips it or
moves the laser off the work area. LCV-156 AC 10 already lists the per-layer files written by
`File > Export Layers`. This spec covers what is left: Save feedback and the out-of-bed
warning.

## Stories

- As an operator, I want Save to tell me which file it wrote, so that I know what LaserGRBL will
  open.
- As an operator, I want a warning when any geometry is outside the bed, before I export and
  burn a job that the machine cannot reach.

## Direction

- After a successful Save or Save As, the command dock shows an info message with the file name
  and the bed size (`Saved part.svg (400 × 400 mm)`), in the LCV-165 info style.
- When Save or Export Layers writes a drawing with any entity partly outside
  `Document::bed_mm`, the message becomes a warning that names how many entities are outside.
  The file is still written. The export contract (AGENTS.md §SVG export) is unchanged.
- Out-of-bed entities are not recoloured on the canvas. The bed border and the Zoom All view
  (LCV-166) show them.
- DESIGN.md §7 and §9 are updated in this spec's last task.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Blocking or clipping out-of-bed geometry on export.
- A pre-flight dialog or an export report window.
- Machine origin or work offset settings (LaserGRBL owns these).

## Open questions

- Should the warning run on autosave too? Proposal: no. Autosave is crash recovery, not a job.
- Should a warning count entities, or also select them so that the operator can find them?
- Until LCV-165 lands, which colour should the info message use?
