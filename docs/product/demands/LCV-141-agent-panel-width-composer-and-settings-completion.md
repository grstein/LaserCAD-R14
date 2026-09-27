# LCV-141 - Agent panel stays within the right third

- **Status**: Done
- **Phase**: 12
- **Depends on**: LCV-125, LCV-129, LCV-132
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: implementer-rust — eff5c93, b703fdd, 2a2e447, 92f2367, 77b316b, 4bb53eb

## Problem

`src/app/panels.rs::draw_agent_side_panel` builds the agent panel as
`egui::SidePanel::right("agent_panel").resizable(true).default_width(300.0)`
— a default and a drag-resizable width, but no `.max_width()` at all. Nothing
stops the panel (by drag, or by a wide screen's default) from consuming most
of the drawing surface, and nothing in `src/agent/panel.rs` bounds its
children independently: the transcript's reserved height is a flat constant
(`ui.available_height() - 60.0`) computed before the code checks
`app.agent_busy`, so the "Thinking… / Cancel" row that only appears while
busy is never accounted for in that reservation — the composer row it sits
above can be pushed toward, or past, the panel's clipped bottom edge exactly
when an operator most wants to reach Cancel.

Separately, the Agent Settings window (`src/agent/settings_ui.rs`,
`src/app/panels.rs::agent_settings_dialog`) already saves on close — by the
`×` button or programmatically — but has no visible control that says so and
no button other than `×` to close it; an operator has no in-dialog cue that
their edits are live and will persist the moment they dismiss the window.

## Scope

- A hard ceiling on the agent panel's width: at most one third of the whole
  application window's width, enforced on every frame regardless of how the
  current width was reached (default, drag, or a shrunk window).
- Bounding the panel's children so long transcript content or the busy
  Cancel row cannot push the composer out of the visible, clipped area.
- Two small additions to the existing Agent Settings window: an explicit
  "Done" button using the exact same close-and-persist path as `×`, and one
  sentence stating that changes apply immediately and persist on close.
- Wrapping the Agent Settings body in a bounded `ScrollArea`, matching the
  pattern ADR 0009 already established for the shortcuts dialog, so that if
  the form grows in a later demand it scrolls instead of pushing Done off
  the bottom of the window.

## Out of scope

Full-screen chat, a persisted width preference (the ceiling recomputes from
the live window size every frame; nothing about width is written to
`Settings`), docking infrastructure, new agent capabilities, or turning
Agent Settings into an Apply/Cancel transaction (it stays live-edit,
persist-on-close — Done is a second way to trigger the same close, not a
different semantics).

## Acceptance criteria

1. The agent panel's actual outer rectangle — read via
   `egui::containers::panel::PanelState::load(ctx, egui::Id::new("agent_panel"))`,
   not inferred from painted content — never exceeds
   `ctx.screen_rect().width() / 3.0`: the whole application window's width,
   not `CentralPanel`'s remaining width after the toolbar and this panel
   already claim their share.
2. The ceiling is recomputed from `ctx.screen_rect()` on every frame (egui's
   `SidePanel` re-clamps its persisted width against `width_range` on every
   `show`, whether or not a drag is in progress — see
   `egui-0.29.1/src/containers/panel.rs::show_inside_dyn`), so it holds on
   the very first frame, after a width the operator dragged wide in a bigger
   window is carried into a smaller one, during an active resize drag, and
   after the window itself shrinks. Default width is `min(300.0, ceiling)`;
   no minimum-width setting may exceed the ceiling.
3. At a `ctx.screen_rect().width()` of `800.0`, the ceiling is `266.666...`
   (not `300.0`), measured to within `0.5` logical point.
4. The transcript's reserved scroll height accounts for whether the busy
   "Thinking… / Cancel" row will render *this* frame, not a flat constant
   that assumes it never does; Send/Cancel and the composer row are sized
   and reserved before the transcript scroll area claims the remaining
   space, in both the busy and non-busy case.
5. With a single unbroken 300-character transcript entry and, separately,
   with 200 short transcript entries, every one of the input field, Send
   button, the busy Cancel button and the `×` close button remains fully
   inside the panel's clip rect and does not overlap another control, in
   both the busy and non-busy state. Closing the panel (`×`) restores the
   canvas to the full width the panel previously subtracted.
6. The Agent Settings window shows one sentence stating that changes apply
   immediately and persist when the window closes, and a "Done" button next
   to the four existing fields; clicking Done calls the exact same
   close-and-persist path `src/app/panels.rs::agent_settings_dialog`
   already runs for `×` (same `persist_settings()` call, same
   `was_open && !app.agent_settings_open` guard — not a second, parallel
   persistence path). The existing key-masking (`password(true)`) and the
   always-visible plaintext-storage warning (`PLAINTEXT_KEY_WARNING`) are
   unchanged.
7. The Agent Settings body sits inside a bounded `egui::ScrollArea`; at
   `800x600` all four existing rows, both explanatory sentences and the Done
   button are reachable — either all visible at once or reachable by
   scrolling within the dialog's own bounds — never clipped with no way to
   reach them. A growth probe (a synthetic fifth field added only inside the
   test) that pushes the form's content height past the window's available
   height proves the `ScrollArea`, not window growth, absorbs the overflow
   and Done stays reachable.
8. This demand changes none of: prompt submission, turn cancellation, undo
   behavior, the panel's default-closed state, the settings persistence
   schema, or the repaint policy (the three-site count in
   `src/app/viewport.rs::every_repaint_request_in_src_is_conditional` is
   unchanged).

## Expected tests

- AC 1-3: real-`App` frames at application widths of `800`, `1024` and
  `1280`, reading `PanelState::load(ctx, ...).rect.width()` — including a
  case that first drags/sets the panel wide at `1280` and then re-renders at
  `800` to prove the remembered width is reclamped, and a case that resizes
  the window down mid-session. Assert the numeric ratio, not merely that a
  button is still visible.
- AC 4-5: `tests/harness/paint.rs`-based containment tests with full
  200-entry and 300-character-unbroken transcripts, in both `agent_busy`
  states, plus real Send/Cancel/`×` clicks through the pointer-click
  convention (ADR 0002 §A4 rule 3).
- AC 6: a real click on Done and a real click on `×`, each against a
  test-owned `settings_path`, asserting identical persisted bytes and
  identical `App` state afterward; painted-text assertions for the new
  sentence, the existing masking and the existing warning.
- AC 7: a bounded-scrolling test at `800x600` with the shipped four-field
  form (today: fits, Done reachable) and the growth-probe variant described
  in AC 7 (synthetic extra field: still reachable only via the `ScrollArea`).
- AC 8: re-run of the existing LCV-125/LCV-129 regressions
  (`tests/lcv125_agent_panel_and_settings.rs`,
  `tests/lcv129_agent_timeout_and_cancel.rs`) and
  `tests/lcv120_idle_repaint.rs` / `every_repaint_request_in_src_is_conditional`
  unmodified.

## Open questions

None. The one-third ceiling is a hard requirement, not an approximate
default, and every other AC follows from bounding the panel's existing
children rather than adding new ones.

## Notes

Width belongs in `src/app/panels.rs::draw_agent_side_panel`; child layout in
`src/agent/panel.rs`; settings content in `src/agent/settings_ui.rs`.

Gotcha: `egui::SidePanel::default_width()` widens `width_range.max` via
`.at_least(default_width)` if the default exceeds the existing max, and
`.max_width()` narrows unconditionally. Call `.max_width(ceiling)` after
`.default_width(...)` each frame (or pass a pre-clamped default), or a
naively-ordered builder call can silently widen the ceiling back out.

`tests/harness/paint.rs::painted_runs_at` already asserts
`pixels_per_point == 1.0` as of LCV-136 (implemented first in this drive's
order); this demand's tests rely on that guard rather than re-adding it.
