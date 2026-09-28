# LCV-150 — Plan

## Approach

Sequenced after LCV-153, which gives `AgentState` a conversation memory (the pairs sent
before each new prompt) next to the display-only `agent.chat`, and copies it into
`TurnConfig` at turn start. "New Conversation" empties both; with the memory empty the next
turn sends exactly `[system, user]` (LCV-153 AC 9), so AC4 needs no worker change.

- One method `AgentState::clear_conversation(&mut self)` empties `chat` and the LCV-153
  memory, and does nothing while `busy` (defence in depth under the disabled button). It
  touches no other field: `input_draft`, `turn`, `rx` and `panel_open` stay put. A turn is
  never in flight when it runs, so no in-flight `TurnConfig` copy can outlive the clear.
- One `small_button("New Conversation")` in the panel header row of `draw_agent_panel`, added
  right-to-left after the `×` button, via `ui.add_enabled(!app.agent.busy, …)`. The enabled
  state is read from `busy` in the same frame it is drawn, so AC6 needs no extra wiring.
- The header sits above the transcript `ScrollArea`, so it is never scrolled away (AC7).
- The panel still "renders and reports" (ADR 0007 §D8): the click is one call, no thread, no
  `Document`/`History`, no `start_turn`.

## Touches

- `src/app/agent_state.rs::AgentState::clear_conversation` — new `pub` method with doc
  comment; clears `chat` and LCV-153's memory field (name fixed by LCV-153's plan); module
  doc "Pure data, no behaviour" amended to name this one guarded method.
- `src/agent/panel.rs::draw_agent_panel` — header button; module doc gains an LCV-150 paragraph.
- `src/app/agent_turn.rs` (tests only) — after a clear, the `TurnConfig` `start_turn` builds
  carries an empty memory. The empty-memory → `["system", "user"]` half is LCV-153 AC 9's
  test in `src/app/agent_worker.rs`; not duplicated.
- `tests/it/lcv150_new_conversation.rs` (new) + `tests/it/main.rs` — paint and pointer tests
  using `tests/harness/paint.rs` and the ADR 0002 §A4 rule 3 click convention (helpers
  `click_events`/`locate` copied locally from `lcv141_agent_panel_width_and_settings.rs`).
- `CHANGELOG.md` — Unreleased line.
- ADRs: none.

## AC → test map

- AC1 painted run "New Conversation" inside the agent panel rect (paint harness).
- AC2 unit: `clear_conversation` on 3 rows + 2 memory pairs → both empty; pointer click on the
  painted button → both empty.
- AC3 / AC6 pointer: click while `busy == true` leaves `chat` intact; set `busy = false`, the very
  next frame's click clears it. Unit: `clear_conversation` while busy is a no-op.
- AC4 painted transcript after the click has no row text; with 2 remembered pairs, a clear, then
  a turn start: the built `TurnConfig` memory is empty (so LCV-153 AC 9 sends `[system, user]`).
- AC5 after the click: `history` revision and undo depth unchanged, entity count unchanged,
  `agent.busy == false`, `agent.rx.is_none()`, no new `user` row.
- AC7 at 800×600 with a 200-row transcript and with `busy == true`: the button's painted rect is
  inside the panel rect and inside the screen, and a click on it (when idle) clears.

## Risks

- Header width at 800×600: panel ceiling is 266.67 pt; heading + `×` + button must fit. The AC7
  test uses `lcv141`'s containment/non-overlap check. Seam if it does not fit: a
  `ui.horizontal` row of its own directly under the header (still above the `ScrollArea`),
  no other change.
- LOC cap: `panel.rs` 240 → ~250; `agent_state.rs` 47 → ~60. No seam needed.
- Mutation testing: no (not `src/agent/` kernel logic, export or `History`).
- Ordering: LCV-153 must land first (its memory field and `TurnConfig` copy). If 150 were built
  first, `clear_conversation` would clear `chat` only and T1–T3 would lose their memory half.
