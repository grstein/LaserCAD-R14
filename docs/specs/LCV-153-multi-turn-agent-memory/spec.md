# LCV-153 - Agent remembers earlier turns of the conversation

- **Status**: Draft
- **Depends on**: LCV-142, LCV-143
- **Implementation**: -

## Problem

`src/app/agent_worker.rs::run_agent_turn` sends only the system prompt and the new prompt,
so every turn starts blank. "Now make it 10 mm wider" or "undo that and use arcs" fail
because the model has no idea what "it" or "that" was; the operator must restate the whole
job each turn. The transcript on screen suggests a conversation the model never sees.

## Stories

- As an operator, I want the agent to remember what we said earlier in this conversation so
  that I can refine a drawing in short follow-up requests.

## Acceptance criteria

1. WHEN a turn ends with a final assistant text THE SYSTEM SHALL record the pair (that turn's
   user prompt, that final text) in the conversation memory.
2. WHEN a turn starts THE SYSTEM SHALL send `[system, u1, a1, …, uN, aN, user(prompt)]`: the
   turn-start system prompt (LCV-143), then the remembered pairs oldest first, then the new
   prompt. Memory holds no system message; an edited prompt applies from the next turn.
3. THE SYSTEM SHALL carry only plain text: no tool calls, tool results, image parts
   (LCV-145), `error`/`note`/`refused`/`tool` transcript rows or step counts.
4. IF a turn ends in an error (transport, step budget, `FenceStopped`, no content) or is
   cancelled THEN THE SYSTEM SHALL record nothing from that turn.
5. WHILE the remembered pairs exceed 20 pairs or 32 KiB of text THE SYSTEM SHALL drop the
   oldest whole pair first; the newest pair is kept even if it alone exceeds the byte cap.
6. WHEN the document is replaced (File > New, Open, Open Recent) THE SYSTEM SHALL clear the
   memory, add a `note` row `Agent memory cleared: the drawing was replaced.`, and record
   nothing from a turn that was in flight at that moment.
7. WHEN the fence trips and the turn still ends with a final text THE SYSTEM SHALL record the
   pair as in AC 1 and keep the earlier memory.
8. THE SYSTEM SHALL keep the memory UI-side next to the transcript and hand the worker a
   turn-start copy inside `TurnConfig`; the worker keeps no state between turns and the
   memory is never persisted (empty at start-up and in `App::default()`).
9. WHEN the memory is empty THE SYSTEM SHALL send exactly `[system, user]`, as today.

## Out of scope

- Persisting conversations across restarts, named sessions, export.
- Summarising or compressing old turns; token counting against a model's context size.
- A setting for the bounds. Clearing on demand is LCV-150's New Conversation button.

## Open questions (recommended default in brackets)

1. Carry tool calls and results? [No: indices go stale between turns and batches are large;
   the model re-reads with `query_entities`.]
2. Record a failed or cancelled turn's prompt? [No: a lone user message without a reply
   confuses the next turn; the operator retypes.]
3. Bound: 20 pairs and 32 KiB, fixed constants? [Yes, both, oldest pair first.]
4. Clear memory on File > New/Open? [Yes, with a `note` row; the transcript stays.]
5. Clear memory on a fence trip? [No: the conversation is still valid; the prompt already
   makes the model re-read before touching indices.]
6. Show memory state in the panel (e.g. "remembering 5 turns")? [No for this spec.]
