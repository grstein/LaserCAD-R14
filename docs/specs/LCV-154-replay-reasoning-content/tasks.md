# LCV-154 — Tasks

- [x] T1 [AC1] [AC5] Test in the wire test module: a response message with
  `"reasoning_content":"R"` parses to `Some("R")`; missing and `null` parse to `None`; a
  tool-call `ChatMessage` with `with_reasoning(None)` serialises byte-identical to the
  pre-LCV-154 literal, and with `Some("R")` ends in `,"reasoning_content":"R"}`
  (files: src/agent/wire.rs)
- [x] T2 [AC1] [AC5] Add `reasoning_content` to `AssistantMessage` and (last) to `ChatMessage`,
  `None` in every constructor, `ChatMessage::with_reasoning`; add `reasoning_content: None` to
  struct-literal fixtures. The one task over 3 files, all edits mechanical except `wire.rs`
  (files: src/agent/wire.rs, src/agent/memory.rs, src/agent/loop_/tests.rs,
  src/app/agent_worker/tests.rs)
- [x] T3 [AC2] Test: batch 1 carries `reasoning_content` `R1`, batch 2 none, then text; the
  assistant message of batch 1 carries `R1` verbatim in requests 2 and 3, batch 2's has no
  `reasoning_content` key; an overrun "not run" batch keeps its field
  (files: src/agent/loop_/tests.rs)
- [x] T4 [AC3] [AC4] Rewrite `reasoning_content_never_reaches_the_batches` as
  `reasoning_content_rides_with_its_tool_call_batch`: the batch's tool-call message keeps
  `R1`; the recorded closing text (a reply that also had `reasoning_content`) has no key;
  turn 2's first request carries `R1` on the replayed message, verbatim
  (files: src/app/agent_worker/tests.rs)
- [x] T5 [AC2] [AC3] `agent_loop` pushes the tool-call turn with
  `.with_reasoning(message.reasoning_content)` (files: src/agent/loop_.rs)
- [x] T6 [AC6] Test: `estimate_tokens` of a tool-call message with 8 bytes of arguments and
  8 bytes of `reasoning_content` is 4; `Memory::trim` counts the field toward the cap
  (files: src/agent/memory.rs)
- [x] T7 [AC6] `message_bytes` adds `reasoning_content` bytes; `estimate_tokens` doc names it
  (files: src/agent/memory.rs)
- [ ] T8 Mutation: `MUTANTS_TARGET_DIR=/tmp/mutants-agent scripts/mutants.sh <base>`; a test
  for every missed mutant (files: the test file of the survivor)
- [ ] T9 Docs: LCV-153 AC 4 "Amended by LCV-154" note; ADR 0007 §D16 amendment (next free
  number) (files: docs/specs/LCV-153-multi-turn-agent-memory/spec.md,
  docs/adr/0007-agent-turn-mutates-the-live-document.md)
- [ ] T10 CHANGELOG line: thinking models keep their reasoning across tool calls and turns
  (files: CHANGELOG.md)
