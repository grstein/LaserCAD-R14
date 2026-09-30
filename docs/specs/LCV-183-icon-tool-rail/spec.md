# LCV-183 — Icon tool rail in two columns

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

The tool rail is a column of sixteen text buttons plus `Agent` (`ui/toolbar.rs::TOOLS`,
`selectable_label`). Words are slow to scan: the operator reads every label to find Trim or
Mirror, while an AutoCAD user finds them by shape. The labels also set the rail's width
(`app/panels.rs::toolbar_width`, up to 120 pt), and sixteen rows plus separators do not fit the
height at 800×600, so the rail scrolls (LCV-140 AC 2).

User decision 2026-09-30: the rail uses AutoCAD-style icons. This reverses DESIGN.md §1.8 and
§12, where icons were rejected (LCV-065/066/140).

## Stories

- As an operator who knows AutoCAD, I want to recognise each tool by its R14 icon, so that I
  find it without reading.
- As an operator, I want the rail to take less width and never scroll at 800×600, so that the
  canvas gets the space.
- As a new operator, I want hovering a tool to show its name, key and command word, so that I
  learn the keyboard from the rail.

## Direction

- **Icons are painted vectors.** One small function per icon draws egui `Painter` primitives
  (lines, polylines, circles, arcs, rects) into a square rect. No image files, no icon font, no
  new dependency. Icons are crisp at any `pixels_per_point`, take their colour from the theme,
  and can be tested on painted shapes (`tests/harness/paint.rs`).
- **Style**: modern flat line icons that keep the R14 metaphors. 20 pt glyph in a 32 pt square
  button, 1.5 pt stroke in `text.primary`, pixel-centred. A point marker (small filled square)
  shows where the tool asks for a pick, as in R14 (the Line icon has two endpoints, the Arc icon
  three points).
- **Metaphors** (R14 Draw / Modify / Inquiry toolbars):

  | Tool | Icon |
  |---|---|
  | Select | pointer arrow |
  | Line | diagonal segment, two endpoint markers |
  | Polyline | zigzag with vertex markers |
  | Rect | rectangle, two opposite corner markers |
  | Circle | circle with centre marker |
  | Arc | arc through three point markers |
  | Text | stroked capital `A` (drawn with lines, not a font glyph) |
  | Move | four-way arrow cross |
  | Copy | two offset squares, the front one solid |
  | Rotate | square and a curved arrow about a base marker |
  | Mirror | a shape, a dashed axis, the reflected shape |
  | Scale | small square inside a large one, diagonal arrow |
  | Trim | cutting edge crossing a segment, cut part dashed |
  | Extend | boundary line, segment with an arrow reaching it |
  | Delete | eraser |
  | Dist | dimension line `|←→|` |

- **Layout**: two columns, as R14 docked its Draw and Modify toolbars side by side. The left
  column holds the draw group (Select … Text), the right column the modify group (Move … Dist).
  The AI toggle sits below both columns, after a separator. The rail is about 76 pt wide and
  about 330 pt tall, so it fits 800×600 with no scrolling. The `ScrollArea` stays as the fallback
  for shorter windows (LCV-140 AC 2).
- **States**: the active tool uses `fill.selected`; hover uses egui's hover fill. There is no
  text on the button.
- **Tooltip**: name, key and command word, e.g. `Line — L · LINE`, `Rotate — ROTATE`
  (sentence case per DESIGN.md §9). It is derived from `TOOLS` and the command words, not from a
  second table.
- `TOOLS` stays the single source: each `ToolEntry` gains its icon function. The icon set lives
  in a new `ui/icons/` module (one file per group, under the 300-LOC cap) that LCV-166 reuses
  for menu rows.
- The AI toggle gets an icon too (see open questions). Its name follows LCV-167.
- DESIGN.md §1, §2, §7 and §12 are updated in this spec's last task.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Icons in menus (LCV-166 adds them, using this icon set).
- Floating, docking or user-configurable toolbars; a ribbon; a toolbar with text beside the icons.
- Icon files (SVG/PNG), `egui_extras` image loaders, icon fonts.
- Themes or icon colour settings.

## Open questions

- ✱ LCV-104's acceptance table and LCV-140 AC 2/3 set the rail's text labels, its width from
  the label text and the `Agent` label. This spec amends them.
- ✱ DESIGN.md §1.8 and §12 reject icons. This spec reverses that (user decision 2026-09-30).
- AI toggle: an icon (a speech bubble or a sparkle) or the short text `AI` in the same 32 pt
  square? The text is clearer, but it breaks the rule that every button is an icon.
- Should the rail show an optional group caption (`Draw`, `Modify`) above each column, or only
  the separator? Captions cost about 16 pt of height.
