# LCV-140 - Compact R14 chrome and useful action hints

- **Status**: Draft
- **Phase**: 11
- **Depends on**: LCV-115, LCV-116, LCV-132, LCV-139
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: -

## Problem

Tool/status allocation lacks a compact-layout guarantee. Controls expose
little shortcut guidance, coordinates omit explicit units and the preset
badge does not explain its whole-file scope.

## Scope

Content-fitting text tool rail, metadata-derived hover hints, explicit mm/export
labels and contained critical status information.

## Out of scope

Ribbon, icon package, new bindings, layers, unit switching, editable preset
badge, theme overhaul or duplicate F1 headroom work.

## Acceptance criteria

1. Keep all eleven tools reachable, derive inventory/hints from `TOOLS`, and preserve active-state highlighting and settled keyboard mnemonics.
2. At default typography, the rail fits text within 120pt; group existing drawing/modify actions without duplicating tool definitions. Use scrolling only if supported height requires it.
3. Add useful tool/mode hints with existing shortcuts; invent no Select shortcut. Replace the robot-only agent toggle with short text.
4. Coordinates visibly state mm. The read-only preset badge states Export and explains that it applies to all geometry when saving SVG.
5. At 800x600, 1024x600 and 1280x800, critical mode/coordinate/export content is contained. A second compact status row may hold counts/recovery detail; total default-font status height is at most 56pt.
6. With the one-third-capped agent panel open at 800x600, combined chrome retains at least 320x300pt of canvas. Long signed coordinates and counts do not hide critical controls.
7. Hints do not mutate state; mode actions still fire once. Preset semantics and exported SVG remain unchanged.

## Expected tests

- AC 1-3: table-derived inventory, real hover content, active-state and mode click tests.
- AC 4-6: settled full-precision bounds at all sizes, agent open/closed, large signed coordinate/count fixtures and tooltip expansion.
- AC 7: unchanged document/history on hover, one-toggle assertions and LCV-115 preset roundtrips.

## Open questions

None at product level. Actual allocation failures must be measured before
changing layout; source inspection alone is not a clipping measurement.

## Notes

Primary files: `src/ui/toolbar.rs`, `src/ui/statusbar.rs`,
`src/ui/menubar.rs`, `src/app/panels.rs`. Reuse LCV-134 for F1 sizing.
