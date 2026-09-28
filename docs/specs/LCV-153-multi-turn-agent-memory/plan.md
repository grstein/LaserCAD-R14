# LCV-153 — Plan

## Approach

Memory is UI-side data: `AgentState::memory: agent::memory::Memory`, a `Vec` of turns (each a
`Vec<ChatMessage>`), so AC 9 can drop whole turns and never mistake LCV-145's image `user`
message for a turn start. All policy (record, whole batches, estimate, trim, clamp, texts) is in a
new kernel-pure `src/agent/memory.rs`. The UI owns each turn's user message
(`TurnState::user`, prefixed per AC 7); the worker only reports the tool batches after it. At
turn start `arm_with_limit` trims (AC 9) and builds `TurnState::user`; `start_turn` clones the
flattened memory into `TurnConfig::memory`; `drive_turn` sends `[system, memory…, user]`
(`agent_loop` is untouched: it already appends only). The worker returns the whole batches after
the user message with images elided. `Done`/`Failed` carry them (AC 10). Every exit goes through
`end_turn`, which takes a `memory::TurnEnd`. After `finish_turn` it appends the record and
stamps `(History::id(), revision)`. Cancel and a lost worker build their record UI-side with no
batches, because no event arrives. No thread, channel, dependency or `Document` access is added.

## Touches

- `src/agent/memory.rs` (new) — `Memory { turns }` with `push_turn`, `flatten`, `is_empty`,
  `clear` (for LCV-150); `TurnEnd::{Done{text}, Stopped{error}, Cancelled}`;
  `turn_record(user, batches, end)`; `whole_batches(&[ChatMessage])` drops a trailing batch that
  is missing a result; `estimate_tokens` = sum of UTF-8 bytes of the text content and tool-call
  `arguments` of every message, then `/ 4` (floor, once);
  `clamp_context_tokens` (8 000..=2 000 000, default 128 000); `trim(&mut self, context_tokens)`
  (cap = ctx/2, target = cap/2; phase 1 elides tool results outside the last turn, oldest
  first; phase 2 drops the oldest turn while > 1 turn). Constants `ELIDED_TOOL_RESULT`,
  `CANCELLED_TEXT`, `DRAWING_CHANGED_PREFIX`; prefixed user = `format!("{PREFIX}\n{prompt}")`.
- `src/agent/mod.rs` — `pub mod memory;` + re-exports; `src/agent/wire.rs` — `ChatMessage::assistant(text)`.
- `src/agent/bridge.rs` — `AgentEvent::Done(String, Vec<ChatMessage>)`,
  `Failed(String, Vec<ChatMessage>)`; `AgentEvent::done(text)` / `failed(err)` (empty batches)
  keep test call sites to one token.
- `src/app/agent_worker.rs` — `TurnConfig::memory: Vec<ChatMessage>` (`Debug`: `memory_len`;
  `Eq` derive dropped unless LCV-145 made `ChatMessage: Eq`). `run_agent_turn`/`drive_turn`
  return `(Result<String, AgentError>, Vec<ChatMessage>)`. The batches are
  `whole_batches(&messages[mem+2..])`, then LCV-145's `replace_images`.
- `src/app/agent_turn.rs` — `TurnState::user: String`; `arm_with_limit` calls
  `agent_memory::begin`, which trims and returns the prefixed text. The chat row stays the raw
  prompt. `start_turn` arms first, then `config_for(app)`, which reads `turn.user` and clones the
  memory; the thread match forwards the batches.
- `src/app/agent_memory.rs` (new) — `begin(app, prompt) -> String`; `record(app, end,
  batches)`: `turn_record` → `push_turn` → stamp `agent.memory_mark`.
- `src/app/agent_poll.rs::end_turn` — gains `(TurnEnd, Vec<ChatMessage>)`, and `record` runs
  after `finish_turn`. The lost exits (3/4) pass `Stopped{AGENT_LOST_MESSAGE}` and `cancel_turn`
  passes `Cancelled`.
- `src/app/agent_state.rs` — `memory: Memory`, `memory_mark: Option<(u64, u64)>`; `app/mod.rs` `mod` line.
- `src/document/history.rs` — `History::id()`: a process-unique `u64` from a static
  `AtomicU64`, assigned in `with_depth`. File > New/Open/Open Recent assign a fresh `History`,
  so a revision that resets to 0 is still detected; `io/` is untouched (§D12).
- `src/io/settings.rs` — `#[serde(default)] agent_context_tokens: u32` (128 000).
- `src/agent/settings_ui.rs` — one "Context tokens" integer field under the step budget.
- `AGENTS.md` §Purity — `memory.rs` joins the kernel-pure list (lcv128 scan).
- `tests/it/lcv153_agent_memory.rs` (new, `mod` in `tests/it/main.rs`). ADRs: 0007 amendment (9), see ARCHITECT?.

## Risks

- ARCHITECT? ADR 0007 §D3 fixes `Done(String)`/`Failed(String)`, §D8 lists every agent file
  and §D13 lists the `TurnConfig` fields. LCV-153 changes all three: the terminal events carry
  the batches as `Vec<ChatMessage>`, `TurnConfig` gains `memory`, and the new files are
  `src/agent/memory.rs` (kernel-pure) and `src/app/agent_memory.rs`. `end_turn` gains memory
  work (§D11: the same single exit, no new exit), and `History` gains `id()`. Does this need
  amendment (9), and does a flattened `Vec<ChatMessage>` clone in `TurnConfig` keep §D1?
  Proposed answer: yes, amend §D3/§D8/§D13 (no reversal). §D1 holds: memory is conversation,
  not document state, and it crosses by value once per turn in both directions.
- LOC cap (impl, now → after 144/145 → after 153): `agent_turn.rs` 235 → ~237 → ~252;
  `agent_poll.rs` 228 → ~240 → ~250; `agent_worker.rs` 164 → ~185 → ~205; `loop_.rs`
  166 → ~233 → +0; `wire.rs` 160 → ~230 → ~235; `agent_state.rs` 47 → 47 → ~55 (LCV-150
  ~67); `settings.rs` 174 → ~184 → ~192; `settings_store.rs` 120 → +0; `settings_ui.rs`
  222 → ~245 → ~258; `history.rs` 256 → ~264; `app/mod.rs` 292 → 294 → 295. Seams:
  `agent_memory.rs` already keeps memory glue out of `agent_turn`/`agent_poll`. If
  `settings_ui.rs` passes 270, one `u32_row` helper serves both the budget and context rows
  (no new egui file, §D8). If `agent_turn.rs` passes 270, `config_for`/`turn_config` move to
  `agent_worker.rs` next to `TurnConfig`.
- Mutation testing: **yes**. Targets: `memory.rs` (trim order, `>` vs `>=`, `/4`, newest
  kept, `whole_batches`), `drive_turn`'s slice offset, `agent_memory::begin`'s mark comparison
  and `History::id`.
- AC 3 against LCV-145: the request that carried an image is elided after the send, so it is
  not a prefix of the next turn. The AC 3 test covers text-only turns and asserts that the
  elided form is the prefix (open question to the user).
- The 38 `AgentEvent::Done/Failed(` sites move to `done()`/`failed()` first (T7–T9); every commit compiles.
