# LCV-156 — Layers with one export file per layer

- **Status**: Planned
- **Depends on**: none
- **Implementation**: -

## Problem

LaserGRBL applies one speed and power setting to each imported file. Today the export preset is
set once for the whole document (`File > Export preset`), so all entities end up in a single
cut/mark/engrave group. A job that mixes cutting and engraving has to be split by hand.
The user chose LightBurn-style layers on 2026-09-29.

## Stories

- As an operator, I want named, colored layers with each entity on one of them, so that I can see
  which parts are cut and which are engraved.
- As an operator, I want one LaserGRBL-ready file per layer, so that each file gets its own speed
  and power setting in LaserGRBL.

## Acceptance criteria

1. WHEN a new document is created THE SYSTEM SHALL give it exactly one layer, named `Cut`, colored
   `#ff0000`, with Output on, and make it the current layer.
2. WHEN a tool or the agent creates an entity THE SYSTEM SHALL put it on the current layer.
3. THE SYSTEM SHALL draw every entity in its layer's color; selection and preview keep their own colors.
4. WHEN the operator runs `LAYER`/`LA` or opens `Layers…` THE SYSTEM SHALL show a dialog that lists
   every layer and can add, rename and recolor layers, toggle Output, set the current layer and
   move the selection to a layer.
5. THE SYSTEM SHALL show the current layer in a status-bar dropdown that can also change it.
6. IF a rename or new layer would duplicate a name (compared case-insensitively after sanitising
   it for file names) or another layer's color THEN THE SYSTEM SHALL refuse the change and say why.
7. IF the operator deletes a layer that still has entities, or deletes the last layer, THEN THE
   SYSTEM SHALL refuse and say why.
8. WHEN layers or entity membership change THE SYSTEM SHALL record the change as a `Command`, so
   one Ctrl+Z undoes it.
9. WHEN the drawing is saved THE SYSTEM SHALL write one `<g>` per layer, in layer order, carrying
   the layer's name, color and Output flag; opening the file SHALL restore the layers, their order,
   the entity membership and the current layer.
10. WHEN the operator runs `File > Export layers` on a saved drawing THE SYSTEM SHALL write
    `<mother>-<layer>.svg` in the mother file's folder for every layer that has Output on and
    has entities. Each file holds only that layer's geometry and follows the LaserGRBL rules
    (bed header, Y flip, `fill="none"`, arcs as `A`, stroke width 0.1 mm). Existing files are
    overwritten, and the status bar lists the files written.
11. IF the drawing has never been saved THEN `Export layers` SHALL ask the operator to save first
    and write nothing.
12. IF no layer has both Output on and entities THEN THE SYSTEM SHALL write nothing and say so.
13. THE SYSTEM SHALL keep layers across autosave and restore.
14. WHEN the agent lists entities THE SYSTEM SHALL report each entity's layer; creation tools
    SHALL accept an optional existing layer name, and an unknown name SHALL be refused.
15. THE SYSTEM SHALL remove `File > Export preset` and the preset badge in the status bar.

## Out of scope

- Speed, power and passes per layer (LaserGRBL owns these).
- Show/hide per layer, nested layers, blocks.
- Migrating v0.2 files. Geometry outside a layer group loads onto the default layer.
- Agent tools that create, rename or delete layers.

## Open questions
