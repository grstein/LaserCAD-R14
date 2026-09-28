# LCV-153 - Agent remembers earlier turns of the conversation

- **Status**: Specified
- **Depends on**: LCV-142, LCV-143, LCV-145
- **Implementation**: -

## Problem

`src/app/agent_worker.rs::drive_turn` sends only `[system, user]`, so every turn starts blank:
"make it 10 mm wider" fails because the model never saw what "it" was. Best practice (pi's
coding agent, DeepSeek's multi-turn guide) replays the whole turn, tool calls included, as an
append-only list so provider prefix caches hit (DeepSeek bills cached input ~50x cheaper).

## Stories

- As an operator, I want the agent to remember this conversation so short follow-ups work.

## Acceptance criteria

1. WHEN a turn ends with a final text THE SYSTEM SHALL append that turn's messages to the
   memory in order: user, each assistant `tool_calls` message with all its tool results, and
   the final assistant text.
2. WHEN a turn starts THE SYSTEM SHALL send `[system, memory…, user(prompt)]`, with the
   system prompt resolved at turn start (LCV-143); memory holds no system message.
3. WHILE memory is not trimmed and the system prompt is unchanged THE SYSTEM SHALL send
   each request's messages as a byte-identical prefix of the next turn's first request.
4. THE SYSTEM SHALL never store `reasoning_content` or an image part; an image appears only
   as LCV-145's elision text.
5. IF a turn fails (transport, step budget, `FenceStopped`, no content) THEN THE SYSTEM SHALL
   append its user message, every whole tool-call batch that completed, and an assistant
   text `Turn stopped: <error>.`; a batch missing any result is dropped whole.
6. IF a turn is cancelled THEN THE SYSTEM SHALL append its user message and an assistant
   text `Turn cancelled by the operator.`
7. WHEN a turn starts and the document changed since the last memory entry (any commit,
   undo, redo, or File > New/Open/Open Recent) THE SYSTEM SHALL prefix the user message
   with `[The drawing changed since your last turn; re-read indices with query_entities.]`; memory is kept.
8. THE SYSTEM SHALL estimate tokens as UTF-8 bytes / 4 over every message's text and
   tool-call arguments, and cap memory at half of `Settings::agent_context_tokens`
   (default 128 000, clamped 8 000..=2 000 000).
9. WHEN memory exceeds the cap at turn start THE SYSTEM SHALL, until it is at or below half
   the cap: first replace the content of tool results outside the newest turn, oldest
   first, with `[old tool result elided]`; then drop the oldest whole turns. The newest turn
   is always kept.
10. THE SYSTEM SHALL keep memory in `AgentState`, pass a clone in `TurnConfig`, and receive
    the turn's new messages in the terminal event; the worker keeps no state between turns.
11. THE SYSTEM SHALL start with empty memory (`App::default()` and start-up) and never
    persist it; with empty memory the first request is exactly `[system, user]`.

## Out of scope

- LLM-summary compaction (pi's `/compact`): an extra paid call and failure path, and the
  document, not the chat, is the source of truth; `query_entities` re-reads it for free.
- Replaying `reasoning_content`. DeepSeek thinking mode with tools asks for it back; the
  wire does not parse it today, within a turn either. A separate spec if needed.
- Persisting or naming conversations; clearing is LCV-150; real token counting.

## Open questions

None. User decision 2026-09-28: `agent_context_tokens` is one integer field in Agent Settings; no memory-size indicator in the panel.
