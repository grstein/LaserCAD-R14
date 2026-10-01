# LCV-183 — Icon tool rail in two columns

- **Status**: Done
- **Depends on**: none
- **Implementation**: da1cb24..7c57640

## Problem

The tool rail is a column of sixteen text buttons plus `Agent` (`ui/toolbar.rs::TOOLS`,
`selectable_label`). Words are slow to scan; an AutoCAD user finds Trim or Mirror by shape. The
labels set the rail's width (`app/panels.rs::toolbar_width`, up to 120 pt), and sixteen rows do
not fit 800×600, so the rail scrolls (LCV-140 AC 2). User decision 2026-09-30: AutoCAD-style
icons, flat and KISS, painted as vectors — no icon files or fonts (DESIGN.md §1.8, §12).

## Stories

- As an operator who knows AutoCAD, I want to recognise each tool by its R14 icon.
- As an operator, I want a narrow rail that never scrolls at 800×600, so the canvas gets the space.
- As a new operator, I want a tool's tooltip to show its name, key and command word.

## Acceptance criteria

1. THE SYSTEM SHALL draw each `TOOLS` entry as a 32 pt square button with no text, holding a
   flat line icon painted with egui vector shapes (1.5 pt stroke in the text colour) inside a
   centred 20 pt square; no image, texture or icon font is used.
2. THE SYSTEM SHALL keep R14 metaphors, one icon per tool, and no two tools SHALL paint the same
   icon: Select pointer arrow · Line segment with two end markers · Polyline zigzag with vertex
   markers · Rect with two corner markers · Circle with centre marker · Arc through three markers
   · Text stroked `A` · Move four-way arrows · Copy two offset squares · Rotate square with a
   curved arrow · Mirror shape, dashed axis, reflection · Scale small square in a large one with a
   diagonal arrow · Trim cutting edge with the cut part dashed · Extend segment with an arrow to a
   boundary · Delete eraser · Dist dimension line `|←→|`.
3. THE SYSTEM SHALL lay the rail out in two columns: the draw group (Select … Text) top-down on
   the left, the modify group (Move … Dist) top-down on the right, then a separator and the AI
   toggle below both. No group captions are shown.
4. WHILE a tool is active THE SYSTEM SHALL paint its button with the selected fill, and no other
   tool button with it.
5. WHEN the operator clicks a tool button THE SYSTEM SHALL activate that tool, as today.
6. WHEN the operator hovers a tool button THE SYSTEM SHALL show the tooltip
   `<Label> — <key> · <WORD>`, or `<Label> — <WORD>` when the tool has no key, derived from
   `TOOLS` (e.g. `Line — L · LINE`, `Rotate — ROTATE`, `Delete — E · ERASE`).
7. THE SYSTEM SHALL draw the AI toggle as a 32 pt square with the short text `AI` (LCV-167's
   short form) and the tooltip `AI Assistant`, in the selected fill while the panel is open; a
   click toggles the panel as today.
8. THE SYSTEM SHALL keep the rail at most 80 pt wide and SHALL show every tool button and the AI
   toggle without scrolling at 800×600, 1024×600 and 1280×800.
9. IF the window is too short for the rail THEN THE SYSTEM SHALL scroll it, and a wheel scroll
   SHALL bring the AI toggle into view (LCV-140 AC 2 kept).
10. THE SYSTEM SHALL record the rail in DESIGN.md §2 (width budget) and §7 (tool rail).

This spec amends LCV-104's rail labels and LCV-140 AC 2/3 (label-sized width, `Agent` text).
The Tools menu and the shortcuts dialog keep their text labels from `TOOLS`.

## Out of scope

- Icons in menus (LCV-166 reuses this icon set).
- Floating, docking or configurable toolbars; a ribbon; text beside the icons.
- Icon files (SVG/PNG), image loaders, icon fonts; themes or icon colour settings.
- Renaming the AI feature elsewhere (LCV-167).

## Open questions

- None. Decided (self-approved per user goal 2026-09-30): the AI toggle shows the text `AI`, not
  an icon (no R14 metaphor exists; LCV-167 names it `AI`); no group captions (height budget);
  tooltip word is the tool's command name from `TOOLS` (`ERASE`, `PLINE`, …).
