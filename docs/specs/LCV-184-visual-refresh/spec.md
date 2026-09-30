# LCV-184 — Visual refresh: theme, status bar and dock

- **Status**: Draft
- **Depends on**: LCV-183
- **Implementation**: -

## Problem

The chrome is egui's stock dark theme with three colours changed (`ui/theme.rs::apply_theme`).
It looks like a debug tool, not a CAD:

- **Theme.** Colour literals are scattered through the chrome, not kept as named tokens
  (DESIGN.md §3, gap → F4). Windows keep egui's drop shadow, which §1.8 forbids. Widget rounding,
  borders and scrollbars are egui defaults, set nowhere on purpose.
- **Status bar.** SNAP, GRID and ORTHO are plain `selectable_label`s
  (`ui/statusbar.rs::draw_statusbar`), so on and off are hard to tell apart at a glance. The
  segments have no separators, and the coordinates use the proportional font.
- **Dock.** The prompt row is one run of text. The command verb, the request and the options
  all look the same (`ui/command_line.rs`).

User decision 2026-09-30: the UI stays strongly based on AutoCAD R14, but gets a modern, KISS
refresh.

## Stories

- As an operator, I want a calm, consistent dark UI, so that the geometry stands out and the
  chrome never distracts.
- As an operator, I want to see at a glance which modes are on.
- As an operator, I want to read the command verb and its options in the prompt at once.

## Direction

- **Modern KISS**: flat surfaces, one border weight, one corner radius, consistent spacing. No
  gradients, shadows, animation, transparency effects or custom fonts. The R14 layout, prompts,
  keys and glyphs stay.
- **Theme** (`ui/theme.rs`): every chrome colour is a named constant for a DESIGN.md §3 token
  (this closes F4 for the chrome). Rounding: 3 pt for widgets, 4 pt for windows and menus.
  Window and popup shadows are off, and each has a 1 pt border in gray 64. Scrollbars are thin.
  Widget fills for inactive, hovered and active states are written out as tokens, not left to
  egui's defaults. `accent` (#4fa3e0) is foreground-only: focus rings, the active mode text, and
  the prompt verb.
- **Status bar**: SNAP, GRID and ORTHO are pills. When a mode is on, the pill has `fill.selected`
  and `text.primary`; when it is off, it has no fill and uses `text.muted`, with a 1 pt outline
  so that it still reads as a button. Segments are divided by thin vertical separators.
  Coordinates use monospace, which also keeps the digits from shifting (LCV-167 pads them).
- **Dock prompt**: the verb is shown in `accent`, the request in `text.primary`, and
  `[Options]` and `<default>` in `text.muted`. The grammar itself is LCV-165. The editor row gets
  a 1 pt frame and a focus ring in `accent`.
- **Rail**: the 32 pt icon buttons from LCV-183 use the same rounding and state fills.
- Budgets (DESIGN.md §2) are re-measured at 800×600, 1024×600 and 1280×800.
- DESIGN.md §1, §3, §5 and §7 are updated in this spec's last task.

## Acceptance criteria

To be written by /specify.

## Out of scope

- A light theme or a theme switcher (DESIGN.md §12).
- Canvas colours: the grid, snap glyph, origin and layer contrast belong to LCV-164.
- Message severities and prompt grammar (LCV-165); the wording, the `status.error` token and the
  autosave state (LCV-167).
- Icons in the status bar or the dock.

## Open questions

- ✱ LCV-071 AC 4 (the accent as a selection fill) is amended by LCV-167. This spec relies on
  that amendment: coordinate the order, or move the token work here.
- ✱ LCV-116 pins the status bar's mode toggles as `selectable_label`s. The pill form amends it.
- Monospace coordinates (Hack) next to proportional labels: acceptable, or keep the proportional
  font and rely on LCV-167's figure spaces?
- Does the dock verb in `accent` compete with the canvas? The alternative is `text.primary` in
  the strong style.
