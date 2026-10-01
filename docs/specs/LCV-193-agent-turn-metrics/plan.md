# LCV-193 — Plan

## Approach

The counts live on the UI side, in the turn's `TurnState`. Every exit goes through `agent_poll::end_turn`
(ADR 0007 §D11), including an operator cancel, where the worker says nothing. A new kernel-pure
`src/agent/metrics.rs::TurnMetrics` (`Copy`, `Default`, `PartialEq`: `steps`, `applied`, `refused`, `repeated`,
`captures`, `replies`, all `u32`) replaces `TurnState::steps` and `TurnState::applied`, so there is one record,
not two. `TurnMetrics::note()` renders the AC1 line.

- **Steps, refused, repeated (AC1–AC3).** The `Act` arm of `poll_agent_rx` moves into
  `agent_poll::answer_act(app, &action) -> AgentOutcome` with no behaviour change. For each step `Act` it calls
  `TurnMetrics::step(&outcome, repeated)`. `refused` counts `Refused` and `Fenced` outcomes. `repeated` counts the
  LCV-192 repeat answers, which are a subset of `refused`, recognised by `repeat.rs::is_repeat(reason)` on
  `AgentAction::Malformed`. `applied` stays in `apply_fenced`, gated on the revision moving, so it is the same
  number the undo note reads.
- **Replies and captures (AC1–AC3).** Only the worker sees a completion. After each successful `send_fn`,
  `send_images` sends one non-step `Dispatch::Replied { captures }`, where `captures` is the number of image parts
  that request carried with upload authorised (0 when withheld). `drive_turn` maps it to
  `AgentAction::Replied { captures }`. `answer_act` answers it `Ok("")`: not a step, no row, never fenced, the same
  class as `AuthorizeUpload` (ADR 0011 item 10). A failed request is not a reply and sends nothing.
- **The note (AC1, AC4).** `end_turn` pushes `("note", tally.note())` after `finish_turn`, so it is always the last
  row of the turn, with zero counts when nothing happened. The undo note keeps its own rule and stays silent at
  zero. The metrics row never reaches the model: memory records batches, not chat rows (§D16).
- **Tests (AC3).** `answer_act` makes a scripted turn synchronous. `drive_turn`, with a stub `send_fn` and
  `ask = |a| Ok(answer_act(&mut app, &a))` on an `App` armed by `arm_turn`, runs a known call sequence with no
  thread and no socket. The test then reads `app.agent.turn.tally`, sends `Done` and polls once for the note.

## Touches

- `src/agent/metrics.rs` (new, kernel-pure): `TurnMetrics`, `step`, `note`. `src/agent/mod.rs`: `pub use`.
  AGENTS.md purity list.
- `src/agent/loop_.rs` (or `loop_/images.rs`, see Risks): `Dispatch::Replied { captures }` in `send_images`.
- `src/agent/wire.rs`: `has_image` becomes `image_count` (callers compare `> 0`), so there is no LOC growth.
- `src/agent/bridge/action.rs`: `AgentAction::Replied { captures: u32 }` and its `tool_name` arm (LCV-192).
- `src/agent/repeat.rs` (LCV-192): `is_repeat(reason) -> bool` next to the repeat text, so there is one literal.
- `src/app/agent_worker.rs::drive_turn`: the `Replied` mapping.
- `src/app/agent_poll.rs`: `answer_act`, the tally in the `Act` path, and the note in `end_turn`.
  `src/app/agent_turn.rs::TurnState`: `tally` replaces `steps` and `applied`. `src/agent/panel.rs`: the progress
  row reads `tally.steps`, and the role table's `note` row cites LCV-193.
- ADRs: amend ADR 0007 (11): §D11 says every exit ends with the metrics note, §D13 says `Replied` is a non-step
  rendezvous, and §D8 gets a `metrics.rs` row. ADR 0011 item 10: `captures` counts authorised images of a request
  that got a reply. No new ADR and no architect.

## Decisions (self-approved per user goal)

- The line uses the AC template word for word, with plurals fixed (`1 steps`), so it has one format for grep and for
  comparing models.
- `steps` is the progress row's number (every step `Act`, malformed and fenced included). An LCV-189 overrun batch is
  answered by the loop, not dispatched, so it counts in neither `steps` nor `refused`.
- `captures sent` counts images that rode a request the model answered. Withheld or failed uploads don't count,
  even when LCV-187's note already said `sent`.
- The tally is readable in `app.agent.turn.tally` until the next turn is armed. It is not persisted, logged or shown
  anywhere else.

## Risks

- LOC cap: `loop_.rs` is at 257, plus LCV-187's `Dispatch::Note`. If it passes 270, T3 first moves `send_images` and
  `last_word` to `src/agent/loop_/images.rs` (LCV-187's named seam, kernel-pure, AGENTS.md list), unless LCV-187
  already did. `agent_poll.rs` is at 247, reaches about 262 with this work and LCV-187, and moves `end_turn`,
  `finish_turn` and `undo_note` to `app/agent_poll/turn_end.rs` if it passes 270. `bridge/action.rs` is at about 255
  after LCV-186/187/190/191/192 and gains 4. `wire.rs` (266) stays flat.
- Mutation testing: yes, for `src/agent/` and `agent_poll`: the `step` classification, repeated counted as refused
  too, `captures` 0 when withheld, no `Replied` on a failed send, and `Replied` staying out of `steps`.
- Test churn:
  - Exhaustive `Dispatch` matches in `loop_/tests.rs` and the `asks` counters and `seen` vectors in
    `agent_worker/tests.rs` now also see `Replied`.
  - `turn.steps` and `turn.applied` appear in 16 places in 9 files (a mechanical rename, done in one task so every
    commit compiles).
  - Transcripts that assert the last row or the row count get one more `note` row.
- Overlap: LCV-187 (`send_images`, `drive_turn`, `agent_poll` note push), LCV-192 (`drive_turn` repeat guard,
  `repeat.rs`, `tool_name`) and LCV-186/190/191 (`bridge/action.rs`). This spec runs last of the six.
