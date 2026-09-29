# LCV-163 — Selection and edit feedback

- **Status**: Draft
- **Depends on**: LCV-156
- **Implementation**: -

## Problem

The canvas does not show what a click will do until it is done:

- Window and crossing selection look the same while dragging. `tools/select/mod.rs::preview`
  returns four plain lines, drawn in the preview amber. Only the release shows which mode ran.
- Nothing highlights the entity under the cursor. Trim and Delete have no preview at all
  (`tools/trim.rs::preview`, `tools/delete.rs::preview` return `vec![]`), so the operator finds
  out what they removed only after the click.
- The preview amber `#ffdc64` (`render/preview.rs`), the snap orange `#ffa000`
  (`render/snaps.rs`) and the warning orange `#ff8f00` are hard to tell apart.

With layers (LCV-156), entity colour belongs to the layer, so these states cannot be shown by
colour alone.

## Stories

- As an operator, I want to see before I release whether my box selects inside or crossing.
- As an operator, I want the entity my click will hit to be highlighted, so that I do not pick
  the wrong one in a dense drawing.
- As an operator, I want Trim and Delete to show what they will remove before I click.

## Direction

- Window box: solid outline. Crossing box: dashed outline (R14). An optional faint fill may use a
  different tint for each mode, but the solid/dashed form is the real cue.
- Hover highlight: the pickable entity under the pickbox (LCV-162) is drawn thicker, in its own
  colour, before the click.
- Trim and Delete preview: the piece that will be removed is drawn dashed in a `danger` canvas
  token.
- State is shown by form (halo, dash, thickness, glyph) and only then by hue. That way it stays
  readable on any layer colour.
- `Tool::preview` returns bare `Entity` values with no style channel. `/design` needs the
  `architect` to pick the seam (styled preview items or a separate highlight query).
- DESIGN.md §3 and §6 are updated in this spec's last task.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Grips and grip editing (rejected in DESIGN.md §12).
- Selection cycling between overlapping entities.
- Changing the selection halo (`render/selection.rs`) beyond keeping it readable on layer colours.

## Open questions

- Should hover highlight run in every tool, or only while a pick is pending?
- Trim and Extend on arcs (LCV-160) need the same preview. Land this before or together with 160?
- ✱ LCV-137 AC 1 pins the canvas paint order. The hover highlight and the Trim/Delete preview
  must fit into it, so amend that AC here.
