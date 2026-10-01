# LCV-184 — Visual refresh: theme, status bar and dock

- **Status**: Done
- **Depends on**: LCV-183
- **Implementation**: 0fdace8..dab5680

## Problem

The chrome is egui's stock dark theme with three colours changed (`ui/theme.rs::apply_theme`),
and it looks like a debug tool. Chrome colour literals are scattered instead of named tokens
(DESIGN.md §3, gap F4); windows keep egui's drop shadow, which §1.8 forbids; rounding, borders,
scrollbars and widget fills are egui defaults. SNAP, GRID and ORTHO are plain `selectable_label`s
(`ui/statusbar.rs::draw_statusbar`), so on and off look alike, the segments have no separators
and the coordinates use the proportional font. The dock prompt is one run of text: verb, request
and options look the same (`ui/command_line.rs`). User decision 2026-09-30: strongly R14, with a
modern, KISS refresh — flat, one border weight, one radius, no gradients, shadows or animation.

## Stories

- As an operator, I want a calm, consistent dark UI in which I see at a glance which modes are
  on and can read the command verb and its options in the prompt at once.

## Acceptance criteria

1. THE SYSTEM SHALL define every chrome colour as a named constant in `ui/theme.rs` matching a
   DESIGN.md §3 token, and no `ui/` file SHALL build a `Color32` from a literal outside it.
2. WHEN `apply_theme` runs THE SYSTEM SHALL set window and popup shadows to none, a 1 pt border
   in gray 64 on windows and menus, 3 pt widget rounding, 4 pt window and menu rounding, and
   explicit inactive, hovered and active widget fills from the tokens.
3. THE SYSTEM SHALL use the `accent` token (#4fa3e0) only for foreground: focus rings, the text of
   an active mode pill and the prompt verb; no fill SHALL use it (amends LCV-071 AC 4; the
   selection fill is `fill.selected`).
4. WHILE a mode (SNAP, GRID, ORTHO) is on THE SYSTEM SHALL paint its status-bar pill with the
   `fill.selected` fill and `accent` text; WHILE it is off, no fill, a 1 pt outline and
   `text.muted` text. A click SHALL still toggle it (amends LCV-116's `selectable_label` form).
5. THE SYSTEM SHALL paint a 1 pt vertical separator between adjacent status-bar segments, and the
   coordinates in egui's built-in monospace font.
6. WHEN the dock shows a prompt of the form `VERB  request [Options] <default>:` THE SYSTEM SHALL
   paint the verb in `accent`, the request in `text.primary`, and `[Options]` and `<default>` in
   `text.muted`; a prompt without that form SHALL be painted in `text.primary`.
7. THE SYSTEM SHALL frame the command editor row with a 1 pt border that becomes `accent` while
   the editor has keyboard focus.
8. THE SYSTEM SHALL paint the LCV-183 rail buttons with the same rounding and state fills as
   AC 2, with the active tool in `fill.selected`.
9. WHEN the window is 800×600, 1024×600 or 1280×800 THE SYSTEM SHALL meet the DESIGN.md §2
   budgets (no clipped menu, status-bar segment or rail button).
10. THE SYSTEM SHALL record tokens, radii, pills and prompt styling in DESIGN.md §1, §3, §5, §7.

## Out of scope

- A light theme or a theme switcher (DESIGN.md §12); custom fonts.
- Canvas colours: grid, snap glyph, origin and layer contrast (LCV-164).
- Severities and prompt grammar (LCV-165); wording, `status.error`, autosave state (LCV-167).
- Icons in the status bar or the dock.

## Open questions

- None. Decided (self-approved per user goal): this spec owns the chrome tokens and the LCV-071
  AC 4 amendment because it lands before LCV-167; pills amend LCV-116; coordinates use the
  built-in monospace font; the prompt verb is `accent` (foreground only, small text).
