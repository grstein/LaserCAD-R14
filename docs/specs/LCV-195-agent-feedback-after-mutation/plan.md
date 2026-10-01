# LCV-195 — Plan

## Approach

The setting is a new `Settings::agent_feedback_after_changes: bool`, `#[serde(default)]`, off.

The loop gains one non-step rendezvous, `Dispatch::Feedback`, of the same kind as
`AuthorizeUpload`, `Note` and `Replied`:

- It is asked once after the last call of a batch that ran to its end and was not fenced, before
  `steps_left_line`.
- An over-budget batch dispatches nothing and asks nothing.
- The worker maps it to `AgentAction::Feedback`, and `agent_poll.rs::answer_act` answers it outside
  the fence and the tally (AC5).

`src/app/agent_feedback.rs::feedback` decides on the UI thread, from the live document (ADR 0007
§D1):

- If the setting is off, or `turn.tally.applied` has not moved since the turn's last feedback
  (`TurnState::fed_at`), it answers `Ok("")`, so the loop appends nothing (AC4, AC6).
- Otherwise it answers `Ok(summary)`, where summary is the `Drawing now: …` sentence plus the
  `check_drawing(..).lines()` summary lines joined with `; `.
- If the drawing is non-empty and both capture opt-ins are on live, it answers
  `Observed{text: summary + "\n" + capture text, png}` from `agent_capture::capture(app,
  &CaptureFrame::Drawing)`. A capture refusal falls back to the text alone.

The loop appends `"\n" + text` when the text is non-empty. An `Observed` answer goes through the
existing image path: the label is `canvas image for tool call <last id>`, the id goes into `shown`,
and the per-image `Note` and `AuthorizeUpload` follow (AC3).

Wording:

- `Drawing now: 3 entities, X 0.000..40.000 mm, Y 0.000..20.000 mm. CHECK: 2 open ends`
- `Drawing now: 1 entity, …` (singular, as the CHECK report pluralises)
- `Drawing now: 0 entities. CHECK: no problems found.`

The `<check>` part takes the summary lines only (those starting `CHECK:`), not the finding lines.

## Touches

- `src/agent/loop_.rs`: `Dispatch::Feedback` and the doc on `agent_loop`.
- `src/agent/loop_/batch.rs` (new, T1 seam): the batch body moved out of `agent_loop`, plus the
  feedback append.
- `src/agent/bridge/action.rs`: the `AgentAction::Feedback` variant and its `tool_name` arm (none:
  it is not a tool).
- `src/app/agent_worker.rs::drive_turn`: the `Dispatch::Feedback` arm.
- `src/app/agent_poll.rs::answer_act`: the `Feedback` arm, which is not a step.
- `src/app/agent_feedback.rs` (new): `feedback`, `summary`. `src/app/mod.rs`: `mod`.
- `src/app/agent_turn.rs::TurnState`: `fed_at: u32`.
- `src/app/agent_capture.rs::allowed`: becomes `pub(crate)`.
- `src/io/settings.rs::Settings`: `agent_feedback_after_changes`.
- `src/agent/settings_ui.rs::draw_agent_settings`: the checkbox and its hint, under the canvas
  toggles. The copy constants move to `src/agent/settings_ui/copy.rs` (T2 seam, no egui).
- `AGENTS.md`: the purity list gains `loop_/batch.rs` (kernel-pure) and `settings_ui/copy.rs`
  (no egui).
- ADRs: ADR 0007, see "ADR amendment" below.

## ADR amendment

T15 appends this to ADR 0007's header as the next free `Amended (n)`:

> **Amended (n)**: <date> — LCV-195: one more non-step rendezvous, `Dispatch::Feedback`, asked
> once after a tool-call batch that ran to its end unfenced, before the steps-left line. The UI
> thread answers from the live document. `Ok("")` adds nothing. A non-empty text is appended to
> the batch's last tool result. `Observed` also attaches its image under the last call's id,
> through ADR 0011's upload check. It is not a step, is not fenced and is not counted. The worker
> still holds no document state (§D1).

## Risks

- LOC cap:
  - `loop_.rs` is at 249. This change adds ~15 lines and LCV-197 adds ~20, which would cross 270.
    T1 first moves the batch body (~60 lines) to `loop_/batch.rs`.
  - `settings_ui.rs` is at 267. The checkbox adds ~8 lines, so T2 first moves the ~40 lines of copy
    constants to `settings_ui/copy.rs`.
  - `agent_turn.rs` 250 → 253. `agent_poll.rs` 220 → ~224. `agent_capture.rs` 168: only the
    visibility changes.
- Mutation testing: **yes** (`src/agent/loop_*`). Targets: the `!fenced` and last-call guards, the
  empty-text skip, and the `fed_at` comparison.
- Ordering: the feedback must sit before `Steps left`, which stays the last line. A T5 test pins it
  on the exact string.
- Same id twice: if the last call was itself `capture_canvas`, two images ride under one call id,
  each with its own Note. This is accepted, and T10 pins that both images are labelled and both
  notes appear.
- Parallel designs: LCV-198..200 (another fork) may also touch `loop_.rs`. Rebase onto whichever
  lands first. The T1 seam is the shared point.
