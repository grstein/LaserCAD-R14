# LCV-153 — Tasks

- [x] T1 [AC10] Architect: ADR 0007 amendment (9) per plan §Risks ARCHITECT? — §D3 terminal
  payloads, §D8 rows `src/agent/memory.rs` + `src/app/agent_memory.rs`, §D13 `TurnConfig::memory`
  (files: docs/adr/0007-*.md)
- [x] T2 [AC1] [AC4] [AC5] [AC6] [AC8] [AC9] Test: `memory.rs` unit tests against stub signatures —
  `turn_record` per `TurnEnd`, `whole_batches` drops an incomplete trailing batch, `estimate_tokens`
  (sum then `/4`, arguments counted), `clamp_context_tokens` bounds/default, `trim` phase 1 (oldest
  first, newest turn untouched), phase 2, stops at ≤ cap/2, keeps a single oversized turn, no
  system message; list `memory.rs` in AGENTS.md §Purity (files: src/agent/memory.rs, src/agent/mod.rs, AGENTS.md)
- [x] T3 [AC1] [AC5] [AC6] [AC8] [AC9] Implement `memory.rs`; `ChatMessage::assistant` (files: src/agent/memory.rs, src/agent/wire.rs)
- [x] T4 [P] [AC7] Test: two fresh `History` values differ in `id()`; `id()` is stable across
  commit/undo/redo/`end_group` (files: src/document/history.rs)
- [ ] T5 [AC7] Implement `History::id` (static `AtomicU64`, set in `with_depth`) (files: src/document/history.rs)
- [ ] T6 [P] [AC8] Test then implement `Settings::agent_context_tokens` (serde default 128 000, old
  file without the key loads) and the Agent Settings integer field (harness edit persists via the
  LCV-141 Done path) (files: src/io/settings.rs, src/agent/settings_ui.rs)
- [ ] T7 [AC10] Add `AgentEvent::done(text)` / `failed(err)` constructors (old shape still) and
  migrate src test sites (files: src/agent/bridge.rs, src/agent/panel.rs, src/app/agent_turn.rs)
- [ ] T8 [AC10] Migrate integration-test sites to the constructors (files:
  tests/it/lcv123_agent_turn.rs, tests/it/lcv124_command_line_routing.rs, tests/it/lcv125_agent_panel_and_settings.rs)
- [ ] T9 [AC10] Migrate remaining sites (files: tests/it/lcv129_agent_timeout_and_cancel.rs,
  tests/it/lcv142_turn_group.rs, src/app/mod.rs)
- [ ] T10 [AC2] [AC3] [AC4] [AC5] [AC11] Test (worker, `agent_worker.rs` tests): empty memory →
  exactly `["system","user"]`; memory M → `[system, M…, user]`; turn-1 requests (serialized
  per message) are a byte-identical prefix of turn 2's first request built through
  `turn_record`; a `reasoning_content` response field never reaches the batches; failure
  batches = whole batches only (transport mid-turn, step budget, `FenceStopped`, `NoContent`)
  (files: src/app/agent_worker.rs)
- [ ] T11 [AC2] [AC5] [AC10] `TurnConfig::memory`, `drive_turn`/`run_agent_turn` tuple return,
  `AgentEvent::Done/Failed(_, Vec<ChatMessage>)`; `poll_agent_rx` destructures (files:
  src/app/agent_worker.rs, src/agent/bridge.rs, src/app/agent_poll.rs)
- [ ] T12 [AC1] [AC5] [AC6] [AC7] [AC9] [AC10] [AC11] Test (integration, `arm_turn` + events):
  Done with batches → memory = user, batches, assistant; Failed → `Turn stopped: <error>.`;
  `cancel_turn` → user + `CANCELLED_TEXT`; lost exit → user + stopped text; next turn's
  `turn.user` is prefixed after a commit, undo, redo, File > New or Open, and not prefixed when
  nothing changed or memory is empty; over-cap memory is trimmed at arm; `App::default()` memory
  is empty; the next `config_for` memory equals `memory.flatten()` (files:
  tests/it/lcv153_agent_memory.rs, tests/it/main.rs)
- [ ] T13 [AC1] [AC5] [AC6] [AC7] [AC9] `agent_memory.rs` (`begin`, `record`), `mod` line,
  `AgentState::{memory, memory_mark}` (files: src/app/agent_memory.rs, src/app/mod.rs, src/app/agent_state.rs)
- [ ] T14 [AC7] [AC9] [AC10] Wire it: `TurnState::user`, `arm_with_limit` → `begin`,
  `start_turn` arms then `config_for`, thread forwards batches; `end_turn(row, TurnEnd, batches)`
  → `record` after `finish_turn`, on all five exits (files: src/app/agent_turn.rs, src/app/agent_poll.rs)
- [ ] T15 CHANGELOG line: the agent remembers the conversation; "Context tokens" setting (files: CHANGELOG.md)
