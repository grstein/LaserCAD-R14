# LCV-154 — Plan

## Approach

Add one optional field and pass it along. `wire.rs` gains `reasoning_content: Option<String>` on
`AssistantMessage` (parsed) and on `ChatMessage` (sent), both `#[serde(default,
skip_serializing_if = "Option::is_none")]`, so a response without the key, or with `null`,
serialises exactly as today (AC 5). `ChatMessage::with_reasoning(self, Option<String>)` sets it;
the one caller is `loop_.rs::agent_loop`, where it pushes a tool-call assistant turn, so every
later request of the turn carries it (AC 2). Memory needs no new code: `whole_batches` clones
those messages into `turn_record`, so a replayed batch keeps the field (AC 3). A plain-text
reply only ever becomes `ChatMessage::assistant(text)`, which never has the field (AC 4).
`memory.rs::message_bytes` adds the field's bytes to the estimate (AC 6).

This is small and independent of the other agent specs, so it goes **first in the agent
queue after LCV-186** (before 187, 192, 191, 190, 193, 188).

## Touches

- `src/agent/wire.rs::AssistantMessage` — new field `reasoning_content`.
- `src/agent/wire.rs::ChatMessage` — new **last** field `reasoning_content` (declared last, so
  the existing keys keep their order and bytes); every constructor sets `None`;
  new `with_reasoning`.
- `src/agent/loop_.rs::agent_loop` — `assistant_with_tool_calls(..).with_reasoning(
  message.reasoning_content)`. `message` is only partly moved by the `match`, so no clone.
- `src/agent/memory.rs::message_bytes` — count `reasoning_content` bytes; doc of
  `estimate_tokens` names it.
- Test fixtures that build `AssistantMessage`/`ChatMessage` by struct literal get
  `reasoning_content: None`: `src/agent/loop_/tests.rs` (7), `src/app/agent_worker/tests.rs`
  (`batch`, `text`), `src/agent/memory.rs` tests (`reply`).
- `src/app/agent_worker/tests.rs::reasoning_content_never_reaches_the_batches` (LCV-153 AC 4)
  is **inverted** by this spec: rewritten as the AC 3/AC 4 test.
- Docs: LCV-153 `spec.md` AC 4 gets an "Amended by LCV-154" note (tool-call assistant turns
  now store the field; plain text and images unchanged). ADR 0007 §D16 gets the next free
  amendment number: the field rides verbatim with its tool-call batch, never with plain text.
  No new ADR, no architect.

## Decisions (self-approved per user goal)

- Replay whenever the provider sent the field, and only then; no provider list, no setting
  (the spec's own resolution).
- `Option<String>`, strictly: a non-string `reasoning_content` fails the parse like any other
  malformed response. No known provider sends one; a lenient deserializer is not worth its
  lines. OpenRouter's `reasoning` key is not read (out of scope: no provider switches).
- An overrun batch answered "not run" (LCV-189) is still a tool-call assistant turn and keeps
  its `reasoning_content`.
- The memory trim never edits `reasoning_content`; only tool results are elided, as today.
  Dropping an oldest whole turn drops its reasoning with it. DeepSeek requires the text
  verbatim, so a truncated form would be worse than none.
- The existing field-order is kept: `reasoning_content` serialises after `tool_call_id`.

## Risks

- LOC cap: `wire.rs` is at 265 and ends at about 281 (two fields, `with_reasoning`, four
  constructor lines). Over 270, under 300. Seam if a later spec grows it: move
  `replace_images` and `has_image` (LCV-193 turns it into `image_count`) to `wire/images.rs`;
  `ContentPart::png` stays in `wire.rs`, the only `base64` user. `loop_.rs` (257) and
  `memory.rs` (207) grow by at most one line each.
- Overlaps: LCV-193 also edits `wire.rs` (`has_image`), `loop_.rs` and
  `agent_worker/tests.rs`; LCV-192 and 187 edit `agent_worker/tests.rs`. Going first keeps
  their rebases to fixture lines.
- Mutation testing: **yes** (`src/agent/`). `message_bytes` `+` → `-`/`*` and a dropped
  `with_reasoning` are pinned by exact-value tests (AC 6 counts, AC 2 verbatim string).
- Branches: the Specified `spec.md` lives on `ui` (f371e23); this folder carries the same text
  so the two branches merge without conflict.
