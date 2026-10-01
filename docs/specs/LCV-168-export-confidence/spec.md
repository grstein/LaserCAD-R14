# LCV-168 — Save and export confidence

- **Status**: Done
- **Depends on**: LCV-156, LCV-165
- **Implementation**: 7930021..f72ebb4

## Problem

A successful Save says nothing (`io/file_actions.rs::action_save`, `action_save_as` only report
failures), so the operator trusts the title bar to know the file is on disk. Geometry outside
the bed is exported without a word; LaserGRBL then clips it or drives the laser off the work
area. LCV-156 AC 10 already lists the files `File > Export Layers` writes; this spec adds Save
feedback and the out-of-bed warning.

## Stories

- As an operator, I want Save to tell me which file it wrote, so that I know what LaserGRBL will
  open.
- As an operator, I want a warning when geometry is outside the bed, before I burn a job the
  machine cannot reach.

## Acceptance criteria

An entity is *outside the bed* when its bounding box extends past `[0, w] × [0, h]` of
`Document::bed_mm` by more than `EPSILON`.

1. WHEN Save or Save As writes the file THE SYSTEM SHALL show the info message
   `Saved <file name> (<w> × <h> mm)` in the command dock (LCV-165 info style), with the bed
   sizes in Rust's shortest `f64` form (`400`, `297.5`).
2. IF, on a successful Save or Save As, n ≥ 1 entities are outside the bed THEN THE SYSTEM SHALL
   show the message as a warning with ` — n entities outside the bed` appended (`1 entity` when
   n = 1).
3. WHEN Export Layers writes files and n ≥ 1 entities on exported layers are outside the bed THE
   SYSTEM SHALL add the warning `n entities outside the bed` after the LCV-156 file list.
4. WHILE entities are outside the bed THE SYSTEM SHALL still write every file, byte-identical to
   what the export writes today (the SVG export contract is unchanged).
5. IF Save, Save As or Export Layers fails or is cancelled THEN THE SYSTEM SHALL show no `Saved`
   message and no out-of-bed warning (the error dialog behaves as today).
6. WHEN an autosave write happens THE SYSTEM SHALL show neither message.
7. THE SYSTEM SHALL record the messages in DESIGN.md §7 and §9 in this spec's last task.

## Out of scope

- Blocking, clipping or recolouring out-of-bed geometry; selecting it for the operator.
- A pre-flight dialog or export report window.
- Machine origin or work offset settings (LaserGRBL owns these).

## Open questions

- None. Decided (self-approved per user goal): no warning on autosave (crash recovery, not a
  job); the warning counts entities and selects nothing; info style comes from LCV-165, which
  lands first.
