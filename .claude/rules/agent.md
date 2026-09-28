---
paths:
  - "src/agent/**"
  - "src/app/agent_*.rs"
---
# Agent area rules (ADR 0007)

- Containment is in AGENTS.md §Purity rule and is scanned by `tests/it/lcv128_normative_enumerations.rs`:
  a new file under `src/agent/` must be named in that section or the gate fails.
- A turn runs on a plain `std::thread` (`src/app/agent_turn.rs::start_turn`) calling blocking
  `reqwest` in `src/agent/transport.rs::chat_completion`; results return over `std::sync::mpsc`,
  polled once per frame in `src/app/agent_poll.rs`. No `tokio`.
- The worker holds no `Document`, `History`, entity snapshot or `Arc<Mutex<_>>`. It asks the UI thread
  to apply one `AgentAction` at a time and waits for the real outcome. Adding `Clone` to `Document` is a blocker.
- One turn = one flat history group = one Ctrl+Z (§D12). The fence is `History::revision()` +
  `History::group_open()` (§D14). Budgets: `u32`, clamped at the read site (§D13).
- `agent.busy` is set only in `agent_turn::arm_turn` and cleared only in `agent_poll::end_turn`,
  which every terminal path tail-calls (§D11). A repaint guard that can latch `true` is a blocker.
- `panel.rs` renders and reports; spawning a turn is app-side wiring.
