# LCV-166 — Menu shortcut column, Zoom All/Extents and Ctrl+A

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

The menus are how operators learn the keyboard, but today they teach it poorly:

- **Shortcuts.** They are pasted into the label with a tab (`"New\tCtrl+N"`,
  `"Grid\tF7"` in `ui/menubar.rs`). egui renders the tab as a gap, not a column, so the
  shortcuts do not line up.
- **Submenus.** Two submenus add a hand-drawn arrow on top of egui's own (`"Open Recent ▶"`,
  `"Export preset ▸"`), so each shows two arrows. The last of these goes away with LCV-156 AC 15.
- **View menu.** It has Zoom In, Zoom Out and Fit to Bed, but not Zoom Extents. Zoom Extents only
  exists as `F`/`Ctrl+0` (`app/viewport.rs::handle_zoom_extents`). Nothing shows the bed and
  any geometry outside it at once.
- **Select All.** `Edit > Select All` has no key. `Ctrl+A` is not in the ADR 0002 §A6 gate table.

## Stories

- As an operator, I want each menu item's shortcut in an aligned column, so that I can learn
  the keys by reading the menu.
- As an operator, I want Zoom Extents and Zoom All in the View menu, so that I can find
  geometry that has left the bed.
- As an operator, I want Ctrl+A to select everything.

## Direction

- Shortcuts go in a right-aligned column: `egui::Button::shortcut_text` for buttons, and a small
  custom row for the Grid/Snap/Ortho checkboxes, which have no `shortcut_text`. Drop the
  hand-drawn submenu arrows.
- View menu: `Zoom In`, `Zoom Out`, `Zoom Extents  F`, `Zoom All`, `Fit to Bed`, a separator,
  then the three mode checkboxes. Zoom All follows R14: it fits the bed and the drawing extents
  together.
- `Ctrl+A` selects all visible entities through the same path as `Edit > Select All`. It joins
  the ADR 0002 §A6 gate table in the global-commands class.
- The Tools menu shows each tool's key in the same column (`toolbar::TOOLS`).
- DESIGN.md §7 and §8 are updated in this spec's last task.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Menu mnemonics (Alt+letter underlines). egui 0.29 has no support for them.
- A customisable shortcut table.
- New menus or menu reordering (order stays File Edit View Tools Help).

## Open questions

- ✱ LCV-116 AC 18/19 pin the View menu's labels (`Grid\tF7`, …) and its contents. This spec
  amends both.
- ✱ ADR 0002 §A6: `Ctrl+A` is a new global-command key. Does it fire while the command line has
  focus? Proposal: no. A focused text field keeps Ctrl+A for "select all text".
- Zoom All vs Fit to Bed: keep both, or let Zoom All replace Fit to Bed?
- Seam: `ui/menubar.rs` is at 288 implementation LOC. The View/Edit menus probably need to move
  to a sibling file (`plan.md`).
