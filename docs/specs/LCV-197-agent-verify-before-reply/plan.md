# LCV-197 — Plan

## Approach

**UI-side state.** `src/app/agent_verify.rs::VerifyState { unverified, reminded }` lives on
`TurnState`, and `agent_poll.rs::answer_act` updates it:

- An applied action that moves the revision sets `unverified`.
- A verification action (`Measure`, `CheckDrawing`, `CaptureCanvas`, `QueryEntities`) clears it,
  but only when its answer is neither refused nor fenced. A refused `measure` verified nothing.
- LCV-195's `Feedback` is not an action call, so it never clears the flag.

**The rendezvous.** The loop gains one non-step `Dispatch::VerifyDue`. It follows the
`AuthorizeUpload` convention: `Ok` means yes, anything else means no. The UI says yes only when
`unverified && !reminded`. When it does, it sets `reminded` and pushes the `note` row
`Asked the agent to verify its work.` (AC4).

**The loop's text-reply branch.** The reply goes through `verify_or_end` in the new
`loop_/verify.rs`. The rendezvous is asked only if all three hold:

- the loop has not reminded yet (it keeps its own flag, AC5);
- a step is left (AC6);
- the reply did not come through `last_word`, so a fenced turn never asks (AC6).

On yes, the loop pushes the assistant text and the user message `VERIFY_REMINDER`, then continues.
On no, the turn ends with the text as today.

**What follows from the existing code.**

- Cancel and failure propagate as today, through `?`.
- The next send's `Replied` counts the extra model reply (LCV-193).
- `memory::whole_batches` drops the text-only assistant message and the reminder after it, since
  the span starts with an assistant message that has no tool calls. Memory keeps neither.

**Prompt.** Two short paragraphs are added to `DEFAULT_PROMPT`: the checklist (AC1) and the
pass/fail reply style (AC2).

**Dependencies.**

- LCV-195's `loop_/batch.rs` seam (T1 there). If 197 lands first, T1 below does that seam itself.
- LCV-194, for the `Measure` action variant.
- LCV-190, for `CheckDrawing`.

## Touches

- `src/agent/loop_.rs`: the `Dispatch::VerifyDue` variant, and the text branch calls
  `verify::verify_or_end`.
- `src/agent/loop_/verify.rs` (new, kernel-pure): `VERIFY_REMINDER` (verbatim AC3 text) and
  `verify_or_end`.
- `src/agent/bridge/action.rs`: the `AgentAction::VerifyDue` variant.
- `src/app/agent_worker.rs::drive_turn`: the arm.
- `src/app/agent_verify.rs` (new): `VerifyState::{after, due}` and `is_verification(&AgentAction)`.
- `src/app/mod.rs`: `mod`.
- `src/app/agent_turn.rs::TurnState`: `verify: VerifyState`.
- `src/app/agent_poll.rs::answer_act`: the `VerifyDue` arm, plus `turn.verify.after(...)` around
  `apply_fenced`.
- `src/agent/prompt.rs::DEFAULT_PROMPT`: the checklist and reply-style paragraphs.
- `AGENTS.md`: the purity list gains `loop_/verify.rs`.
- ADRs: ADR 0007, see "ADR amendment" below.

## ADR amendment

T12 appends this to ADR 0007's header as the next free `Amended (n)`:

> **Amended (n)**: <date> — LCV-197: one more non-step rendezvous, `Dispatch::VerifyDue`.
>
> - **When it is asked.** At most once per turn, on a text-only reply. It is asked only while a
>   step is left and never after a fence stop (§D14).
> - **How the UI answers.** It says yes when an action applied in this turn is not followed by an
>   answered verification call (`measure`, `check_drawing`, `capture_canvas`,
>   `query_entities`).
> - **What a yes does.** The worker appends the model's text and one fixed, code-side user
>   message asking it to verify. The turn then continues instead of ending.
> - **What stays the same.** The reminder grants nothing and is not a step. The flat group (§D12)
>   and the fence (§D14) are unchanged. Memory (§D16) keeps neither the interim reply nor the
>   reminder.

## Risks

- LOC cap:
  - `loop_.rs` stays ≤270 only with LCV-195's T1 seam. The logic lives in `loop_/verify.rs`
    (~40), so `loop_.rs` grows by ~6.
  - `agent_turn.rs` 253 → 255.
  - `agent_poll.rs` ~224 → ~232.
  - `bridge/action.rs` is about 240 after 194/195. If it passes 270, move the non-tool variants'
    docs into `bridge/action/ops.rs`; T4 checks this.
- Mutation testing: **yes**. Targets:
  - the four guards: reminded, a step left, the fence path, and refused verification;
  - `is_verification`'s list;
  - the flag set and clear order.
- Loop hazard: a model that answers the reminder with tool calls again resumes the turn. The step
  budget still bounds it, and a later text reply ends the turn (AC5). T5 pins this.
- Memory leak of the reminder text: T9 pins that memory after a reminded turn contains neither the
  reminder nor the interim reply.
