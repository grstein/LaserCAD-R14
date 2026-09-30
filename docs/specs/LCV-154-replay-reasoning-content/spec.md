# LCV-154 — Replay reasoning_content for thinking models

- **Status**: Done
- **Depends on**: LCV-153
- **Implementation**: 03675de..aee9c3b

## Problem

`src/agent/wire.rs` ignores the `reasoning_content` field of assistant messages
(`AssistantMessage` and `ChatMessage` have no such field). DeepSeek's thinking mode requires it
to be sent back on assistant messages that carry tool calls, within a turn and across turns
(https://api-docs.deepseek.com/guides/thinking_mode), and not on plain-text turns. Thinking
models therefore lose their reasoning in multi-step tool use. Found during LCV-153 research.

## Stories

- As an operator using a thinking model, I want tool-using turns to keep the model's reasoning
  so that long drawing jobs stay coherent.

## Acceptance criteria

1. WHEN a response's assistant message carries a string `reasoning_content` THE SYSTEM SHALL
   parse it.
2. WHEN the loop appends an assistant turn with tool calls and a `reasoning_content` THE SYSTEM
   SHALL send that `reasoning_content` verbatim on that message in every later request of the
   turn.
3. WHEN memory (LCV-153) replays a whole tool-call batch in a later turn THE SYSTEM SHALL send
   the same `reasoning_content` on its assistant message.
4. THE SYSTEM SHALL NOT send `reasoning_content` on a plain-text assistant message (a final
   reply, live or replayed).
5. IF a response has no `reasoning_content`, or it is `null`, THEN THE SYSTEM SHALL serialise
   that assistant message with no `reasoning_content` key, byte-identical to today.
6. THE SYSTEM SHALL count `reasoning_content` bytes in `memory::estimate_tokens`, so memory
   trimming keeps respecting the context budget.

## Out of scope

- Displaying reasoning in the AI panel.
- Provider-specific switches or settings.

## Open questions

- None. Decided (self-approved per user goal): replay the field whenever the provider sent it,
  and only then; providers that never send it see no change (AC 5), so no provider list or
  setting is needed.
