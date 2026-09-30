# LCV-166 — Menu icons and shortcut column, Zoom All/Extents and Ctrl+A

- **Status**: Specified
- **Depends on**: LCV-183
- **Implementation**: -

## Problem

Shortcuts are pasted into menu labels with a tab (`"New\tCtrl+N"`, `"Grid\tF7"` in
`ui/menubar.rs`); egui renders the tab as a gap, so they do not line up. `Open Recent ▶` adds a
hand-drawn arrow on top of egui's own. The View menu has Zoom In, Zoom Out and Fit to Bed but no
Zoom Extents (only `F`/`Ctrl+0`, `app/viewport.rs::handle_zoom_extents`), and nothing shows the
bed and stray geometry at once. `Edit > Select All` has no key, and `Ctrl+A` is not in the
ADR 0002 §A6 gate table. User decision 2026-09-30: menu rows get icons from the LCV-183 set.

## Stories

- As an operator, I want each shortcut in an aligned column, so that I learn the keys from the menu.
- As an operator, I want Zoom Extents and Zoom All in the View menu.
- As an operator, I want Ctrl+A to select everything.

## Acceptance criteria

1. THE SYSTEM SHALL paint every menu row as `icon slot | label | shortcut`, with the shortcut
   right-aligned in one column per menu and no tab character in any label.
2. THE SYSTEM SHALL paint the LCV-183 icon in the icon slot of every Tools-menu row, and icons from
   the same set for File (New, Open, Save), Edit (Undo, Redo, Delete) and View (Zoom In, Zoom Out,
   Zoom Extents, Fit to Bed); a row with no icon SHALL keep an empty slot of the same width.
3. THE SYSTEM SHALL paint the Grid, Snap and Ortho check marks in the icon slot, with `F7`, `F3`
   and `F8` in the shortcut column.
4. THE SYSTEM SHALL show no hand-drawn arrow in a submenu label (`Open Recent`, `Object snap`).
5. THE SYSTEM SHALL order the View menu as Zoom In, Zoom Out, Zoom Extents (`F`), Zoom All,
   Fit to Bed, a separator, then Grid, Snap, Object snap and Ortho (amends LCV-116 AC 18/19).
6. WHEN the operator picks `View > Zoom Extents` THE SYSTEM SHALL frame the drawing extents exactly
   as `F` does.
7. WHEN the operator picks `View > Zoom All` THE SYSTEM SHALL frame the union of the bed and the
   drawing extents; with no entities it SHALL frame the bed.
8. WHEN the operator presses `Ctrl+A` while the command line does not have focus THE SYSTEM SHALL
   select all entities through the same path as `Edit > Select All` (one undo step).
9. WHILE the command line has focus THE SYSTEM SHALL leave `Ctrl+A` to the text field.
10. THE SYSTEM SHALL list `Ctrl+A` in the ADR 0002 §A6 gate table (global commands) and record
    the menu row layout in DESIGN.md §7 and §8 in this spec's last task.

## Out of scope

- Alt+letter mnemonics (egui 0.29 has none); a customisable shortcut table.
- Icons for rows with no rail tool or view action (Export layers, Bed size, Help).
- New menus or reordering beyond AC 5 (R14 order; `Format` stays after View).

## Open questions

- None. Decided (self-approved per user goal): Ctrl+A does not fire while the command line has
  focus; Zoom All and Fit to Bed both stay; the `ui/menubar.rs` split (287 LOC) is a plan.md seam.
