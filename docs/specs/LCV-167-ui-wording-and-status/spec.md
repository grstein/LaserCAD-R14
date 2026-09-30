# LCV-167 — UI wording, one AI name and status bar polish

- **Status**: Specified
- **Depends on**: LCV-183
- **Implementation**: -

## Problem

Casing is mixed (`Save As…` and `Fit to Bed` next to `Bed size…`, `Export layers` and
`Keyboard shortcuts…` in `ui/menubar.rs`). The AI feature has four names: `Agent` on the rail
(`ui/toolbar.rs::AGENT_TOGGLE_LABEL`), `AI Assistant` in the panel (`agent/panel.rs`),
`Agent Settings` as window (`app/panels.rs`) but `Agent settings…` as menu item, `AI` in the dock
(`ui/command_destination.rs`); messages point to "Help > Agent settings" and undo labels read
`Agent: …` (`app/agent_turn.rs`). Coordinates use space padding (`ui/statusbar.rs::format_coords`),
so digits jump when a sign appears. Agent errors are `Color32::RED` (3.8:1). A failed autosave is
dropped silently (`app/autosave.rs::record_autosave_outcome`): the bar keeps saying `○ autosaved`.

## Stories

- As an operator, I want one name and one casing per thing, steady coordinates, and to know when
  autosave stops working.

## Acceptance criteria

1. THE SYSTEM SHALL label every menu item, button, window title and rail tooltip in Title Case
   (DESIGN.md §9), including `Export Layers`, `Bed Size…`, `Keyboard Shortcuts…`, and the
   `Bed Size` and `Keyboard Shortcuts` window titles; all other text stays sentence case.
2. THE SYSTEM SHALL name the settings window `AI Settings` and its Help menu item `AI Settings…`.
3. THE SYSTEM SHALL use `AI` as the rail toggle's short name with tooltip `AI Assistant`, keep
   `AI Assistant` as the panel heading and `AI` as the dock destination label.
4. WHEN a message points the operator to the settings (unavailable agent, 401, timeout, capture
   disabled) THE SYSTEM SHALL say `Help > AI Settings…`.
5. WHEN an AI turn commits THE SYSTEM SHALL label its undo step `AI: <prompt>`.
6. THE SYSTEM SHALL pad each coordinate in `format_coords` with figure spaces (U+2007) to a fixed
   width that includes the sign, so `(1, -1)` and `(-1234.5, 9999.99)` give strings of equal
   char count.
7. THE SYSTEM SHALL define one `status.error` token `#ff6b6b` (≥4.5:1 on the panel fill) and
   paint AI panel errors and dock errors in it; `Color32::RED` is no longer used.
8. THE SYSTEM SHALL keep egui's default selection fill (`#005c80`) and use the LCV-071 accent
   `#4fa3e0` only as a foreground colour (amends LCV-071 AC 4).
9. WHEN an autosave write fails THE SYSTEM SHALL show `× autosave failed` in `status.error` in the
   status bar until the next write succeeds, which restores `○ autosaved` (a fourth state;
   amends LCV-116 product decision 3).
10. THE SYSTEM SHALL record the tokens, names and casing in DESIGN.md §3, §4 and §9 in this
    spec's last task.

## Out of scope

- Rail tooltip content (name, key, command word): LCV-183.
- The layer dropdown (LCV-156 AC 5); translating the UI.
- Renaming code identifiers (`agent/`, `AgentAction`, `TOOL_COLOR`, `warn_fg_color`).

## Open questions

- None. Decided (self-approved per user goal): the fourth autosave state reopens LCV-116
  decision 3; LCV-071 AC 4 is amended to foreground-only; undo labels become `AI: …`.
