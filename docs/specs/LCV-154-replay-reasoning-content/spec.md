# LCV-154 — Replay reasoning_content for thinking models

- **Status**: Draft
- **Depends on**: LCV-153
- **Implementation**: -

## Problem

`src/agent/wire.rs` ignores the `reasoning_content` field of assistant messages. DeepSeek's
thinking mode requires that field to be sent back on assistant messages that carry tool calls,
within a turn and across turns (https://api-docs.deepseek.com/guides/thinking_mode), while it
must not be sent back on plain-text turns. Thinking models may therefore behave worse in
multi-step tool use. Found during LCV-153 research; deferred past 1.0 by the user on 2026-09-28.

## Stories

- As an operator using a thinking model, I want tool-using turns to keep the model's reasoning
  so that long drawing jobs stay coherent.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Displaying reasoning in the agent panel.

## Open questions

- Which providers besides DeepSeek need the field replayed, and does any reject it?
