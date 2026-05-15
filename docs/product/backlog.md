# LaserCAD R14 Backlog

This backlog is ordered for AI implementation agents. Only `Ready` items should be picked for coding without further product refinement.

## Ready

| Priority | Demand | Why Now |
| --- | --- | --- |
| P1 | [LCD-001 - Export SVG compatibility fixtures](demands/LCD-001-export-svg-compatibility-fixtures.md) | Export is the product handoff to LaserGRBL and needs regression evidence. |
| P1 | [LCD-002 - Document command line behavior gaps](demands/LCD-002-document-command-line-behavior-gaps.md) | Keyboard-first flow is core to the product and must stay explicit for future agents. |
| P2 | [LCD-004 - Offset line and circle by distance](demands/LCD-004-offset-line-circle-by-distance.md) | Offset supports common laser clearance/spacing work without broad curve-offset complexity. |

## Needs Refinement

| Priority | Demand | Missing Decision |
| --- | --- | --- |
| P2 | [LCD-003 - Import simple SVG as editable geometry](demands/LCD-003-import-simple-svg-editable-geometry.md) | Needs tighter constraints on supported SVG subset and failure behavior. |

## Rejected Or Deferred Themes

| Theme | Decision |
| --- | --- |
| Rich SVG effects, fills, masks, filters | Rejected for product core because LaserGRBL compatibility requires plain geometry. |
| Decorative dashboards or landing pages | Rejected because the drawing viewport is the product surface. |
| General illustration features | Deferred unless tied to precise laser-cutting geometry. |
| Complex material/machine database | Deferred until export workflow proves a concrete need beyond SVG presets. |
