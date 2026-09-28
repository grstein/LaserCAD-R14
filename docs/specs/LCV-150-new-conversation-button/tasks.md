# LCV-150 — Tasks

Sequenced after LCV-153 (memory field on `AgentState`, memory copy in `TurnConfig`).

- [x] T1 [AC2] [AC3] Test: unit tests `clear_conversation` empties `chat` and the LCV-153 memory and is a no-op while `busy`, other fields untouched (files: src/app/agent_state.rs)
- [x] T2 [AC2] [AC3] Implement `AgentState::clear_conversation` (transcript + memory) with doc comment; amend module doc (files: src/app/agent_state.rs)
- [x] T3 [P] [AC4] Test: with 2 remembered pairs, `clear_conversation` then a turn start builds a `TurnConfig` whose memory is empty (files: src/app/agent_turn.rs)
- [x] T4 [AC1] [AC2] [AC3] [AC4] [AC5] [AC6] [AC7] Test: new `lcv150_new_conversation` module — painted label in panel, idle click clears transcript and memory, busy click does not, next-frame re-enable, document/history/turn untouched, 800×600 reachability with 200 rows and while busy (files: tests/it/lcv150_new_conversation.rs, tests/it/main.rs)
- [x] T5 [AC1] [AC3] [AC6] [AC7] Add the `New Conversation` header button gated on `!agent.busy`, calling `clear_conversation`; LCV-150 module-doc paragraph (files: src/agent/panel.rs)
- [x] T6 CHANGELOG line: New Conversation button clears the agent transcript and its memory (files: CHANGELOG.md)
