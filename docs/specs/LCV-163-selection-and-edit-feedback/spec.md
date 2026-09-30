# LCV-163 — Selection and edit feedback

- **Status**: Specified
- **Depends on**: LCV-156, LCV-162, LCV-160
- **Implementation**: -

## Problem

The canvas shows what a click did only after it is done. Window and crossing boxes are the same
four amber lines (`tools/select/hit.rs::box_preview`), nothing marks the entity under the
cursor, and TRIM and ERASE have no preview (`tools/trim.rs::preview`,
`tools/delete.rs::preview` return `vec![]`). The preview amber `#ffdc64`, snap `#ffa000` and
warning `#ff8f00` are hard to tell apart. With layers (LCV-156) entity colour belongs to the
layer, so state must be shown by form (dash, thickness, halo) first and by hue second.

## Stories

- As an operator, I want to see before I release whether my box selects inside or crossing.
- As an operator, I want the entity my click will hit to be highlighted in a dense drawing.
- As an operator, I want TRIM and ERASE to show what they will remove before I click.

## Acceptance criteria

1. WHILE a left-to-right selection box is dragged THE SYSTEM SHALL paint its outline solid.
2. WHILE a right-to-left selection box is dragged THE SYSTEM SHALL paint its outline dashed.
3. WHILE a tool waits for an entity pick (Select idle, TRIM, EXTEND) and an entity lies within
   the pickbox (LCV-162) THE SYSTEM SHALL paint that entity with a stroke thicker than 1 pt in
   its own layer colour; WHEN no entity is in the pickbox THE SYSTEM SHALL paint no highlight.
4. WHILE TRIM is active and the pickbox is over a trimmable piece of a line, circle or arc THE
   SYSTEM SHALL paint exactly that piece dashed in the `danger` token.
5. WHILE ERASE is active with a non-empty selection and the pointer is over the canvas THE SYSTEM
   SHALL paint every selected entity dashed in the `danger` token.
6. WHEN the click follows a hover of AC 3–5 THE SYSTEM SHALL affect exactly the highlighted
   entity or piece (preview and result agree).
7. THE SYSTEM SHALL paint the canvas in this order: background, bed fill, grid, bed border and
   outside overlay, entities, selection halo, hover highlight, preview (boxes, danger pieces,
   rubber-band), snap glyph, crosshair (amends LCV-137 AC 1).
8. THE SYSTEM SHALL give `danger` and `hover` a single home each in `render/palette.rs`, with
   ≥3:1 contrast on the bed and a hue distinct from `preview`, `snap` and `status.warning`
   (DESIGN.md §3 rules).
9. WHEN the operator presses Esc or the pointer leaves the canvas THE SYSTEM SHALL paint no hover
   highlight and no danger preview on the next frame.
10. THE SYSTEM SHALL record the new canvas elements in DESIGN.md §3 (tokens) and §6 (paint order
    and element table) in this spec's last task.

## Out of scope

- Grips and grip editing (rejected in DESIGN.md §12).
- Selection cycling between overlapping entities.
- Changing the selection halo (`render/selection.rs`) beyond keeping it readable on layer colours.
- The seam for styled preview (styled preview items vs. a separate highlight query): /design,
  with the `architect`.
- A fill tint on selection boxes (outline form only).
- EXTEND's added-segment preview keeps the `preview` amber; only its picked entity gets AC 3.

## Open questions

- None.
