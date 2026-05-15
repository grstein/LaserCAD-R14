# LaserCAD R14 Backlog

This backlog is ordered for AI implementation agents. Only `Ready` items should be picked for coding without further product refinement.

## Ready

| Priority | Demand | Why Now |
| --- | --- | --- |
| P1 | [LCD-001 - Export SVG compatibility fixtures](demands/LCD-001-export-svg-compatibility-fixtures.md) | Export is the product handoff to LaserGRBL and needs regression evidence. |
| P1 | [LCD-006 - English-Native Bundle, UI Strings, and Source Comments](demands/LCD-006-english-native-bundle-and-source.md) | Distributed bundle still leaks pt-BR through Tauri metadata, `<html lang>`, README, and kernel JSDoc; contradicts the global / AI-friendly positioning. |
| P1 | [LCD-007 - English-Native Documentation Cleanup](demands/LCD-007-english-native-documentation-cleanup.md) | `docs/` is bilingual and `specs/` documents the pre-migration architecture; agents read instructions that no longer match the code. |
| P1 | [LCD-005 - Normal File Menu With SVG Workflow](demands/LCD-005-normal-file-menu-svg-workflow.md) | Users need the basic CAD document workflow without a native format or three competing save actions. |
| P1 | [LCD-002 - Document command line behavior gaps](demands/LCD-002-document-command-line-behavior-gaps.md) | Keyboard-first flow is core to the product and must stay explicit for future agents. |
| P2 | [LCD-008 - Adopt English-Only Convention](demands/LCD-008-adopt-english-only-convention.md) | Without a CLAUDE.md rule change, the next contribution reintroduces pt-BR and reverts LCD-006/007. |
| P2 | [LCD-004 - Offset line and circle by distance](demands/LCD-004-offset-line-circle-by-distance.md) | Offset supports common laser clearance/spacing work without broad curve-offset complexity. |

## Needs Refinement

| Priority | Demand | Missing Decision |
| --- | --- | --- |
| P3 | [LCD-003 - Import simple SVG as editable geometry](demands/LCD-003-import-simple-svg-editable-geometry.md) | Third-party SVG import still needs tighter constraints; LaserCAD SVG roundtrip is covered by LCD-005. |

## Rejected Or Deferred Themes

| Theme | Decision |
| --- | --- |
| Rich SVG effects, fills, masks, filters | Rejected for product core because LaserGRBL compatibility requires plain geometry. |
| Decorative dashboards or landing pages | Rejected because the drawing viewport is the product surface. |
| General illustration features | Deferred unless tied to precise laser-cutting geometry. |
| Complex material/machine database | Deferred until export workflow proves a concrete need beyond SVG presets. |
| Native project format (`.lasercad`, JSON, ZIP) | Deferred; SVG remains the only user-visible file format until LaserCAD-authored SVG roundtrip proves insufficient. |
