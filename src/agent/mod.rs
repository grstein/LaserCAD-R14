//! Optional LLM agent: HTTP transport, wire types, tool registry, multi-turn
//! loop, chat panel and settings dialog.
//!
//! Containment rules (AGENTS.md §Purity rule, ADR 0007 §D8): only `panel.rs`
//! and `settings_ui.rs` may import `egui`, no file here may import `eframe` or
//! `rfd`, and only `transport.rs` may import `reqwest`.
//!
//! Since LCV-122 one more rule joins them (ADR 0007 §D1): **no file here holds
//! document state.** `Document`, `History`, `Vec<Entity>` and `Arc<Mutex<_>>`
//! appear nowhere under `src/agent/` — except in `panel.rs`, whose throwaway
//! pair LCV-123 deletes. `tests/lcv122_source_scans.rs` enforces that, with the
//! exception list asserted to be exactly one file long so it cannot grow.

pub mod wire;
pub use wire::{AssistantMessage, ChatMessage, ChatResponse, Choice, ToolCall, ToolCallFunction};

pub mod settings_ui;
pub mod transport;
pub use transport::{chat_completion, TransportError};

pub mod bridge;
pub use bridge::{AgentAction, AgentEvent, AgentOutcome};

pub mod tools;
pub use tools::{parse_tool_call, tool_definitions, ToolCallError};

pub mod loop_;
pub use loop_::{
    clamp_step_budget, AgentError, AGENT_STEP_BUDGET_DEFAULT, AGENT_STEP_BUDGET_MAX,
    AGENT_STEP_BUDGET_MIN,
};
// The loop and the prompt are driven from `src/app/agent_turn.rs`, which owns
// the turn (ADR 0007 §D8). Re-exported here so that file needs no deep path.
pub(crate) use loop_::{agent_loop, AGENT_SYSTEM_PROMPT};

pub mod panel;
pub use panel::draw_agent_panel;
