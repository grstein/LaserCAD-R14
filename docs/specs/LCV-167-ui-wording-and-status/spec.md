# LCV-167 — UI wording, one AI name and status bar polish

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

- **Casing.** `Save As…` and `Fit to Bed` sit next to `Bed size…` and `Keyboard shortcuts…`
  (`ui/menubar.rs`).
- **AI feature name.** The feature has four names. The toolbar says `Agent`
  (`ui/toolbar.rs::AGENT_TOGGLE_LABEL`), the panel heading says `AI Assistant`
  (`agent/panel.rs`), the window is `Agent Settings` (`app/panels.rs`) but the menu item is
  `Agent settings…`, and the dock says `AI` (`ui/command_destination.rs`). Messages point to
  "Help > Agent settings" (`app/cmdline.rs::AGENT_UNAVAILABLE`, `agent/transport.rs`).
- **Coordinates.** Space padding (`ui/statusbar.rs::format_coords`, `{:>6.2}`) shifts the
  digits when a sign appears or a value passes 1000 mm.
- **Colour.** Agent errors are `Color32::RED` (`agent/panel.rs`), 3.8:1 on the panel. The
  LCV-071 accent `#4fa3e0` never shipped; as a fill under `#d0d0d0` text it measures 1.8:1.
- **Autosave.** A failed write is dropped silently (`app/autosave.rs::record_autosave_outcome`);
  the status bar keeps saying `○ autosaved`. Toolbar buttons show their key only on hover.

## Stories

- As an operator, I want one name and one casing per thing, steady coordinates, and to know when
  autosave stops working.

## Direction

- Casing per DESIGN.md §9: Title Case for menus, buttons, window titles and the toolbar (so
  LCV-156's `Export layers` becomes `Export Layers`); sentence case for everything else.
- One AI name: **AI Assistant** for the feature (panel heading, window title prefix), **AI** as
  the short form (toolbar toggle, dock destination, `/ai`), and **AI Settings** for the
  settings window and its menu item. Messages point to "Help > AI Settings…".
- Coordinates are padded with figure spaces (U+2007) to a fixed width, including the sign.
- Add a `status.error` token `#ff6b6b` (5.5:1) for errors in the panel and dock. The LCV-071
  accent becomes a foreground-only token.
- The status bar shows a fourth autosave state, `autosave failed`, in `status.error`, until the
  next write succeeds.
- Toolbar labels show their key (`Line  L`) within the LCV-140 width cap. Seams for `plan.md`:
  `ui/statusbar.rs` 256 LOC, `app/mod.rs` 295 LOC.
- DESIGN.md §3, §4 and §9 are updated in this spec's last task.

## Acceptance criteria

To be written by /specify.

## Out of scope

- The preset badge (LCV-156 AC 15 removes it) and the layer dropdown (LCV-156 AC 5).
- Translating the UI; renaming code identifiers (`agent/`, `AgentAction`).

## Open questions

- ✱ LCV-116 product decision 3 fixes three autosave states "do not re-open". The fourth state
  reopens it.
- ✱ LCV-071 AC 4 sets `selection.bg_fill` to `#4fa3e0`, but `ui/theme.rs::apply_theme` never
  did. This spec amends that AC to foreground-only use and keeps egui's `#005c80` fill.
- Undo labels read `Agent: …` (`app/agent_turn.rs`); become `AI: …`? Keep the `TOOL_COLOR` and
  `warn_fg_color` names that LCV-125 tests scan.
