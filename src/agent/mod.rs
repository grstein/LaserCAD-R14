//! Optional LLM agent: HTTP transport, wire types, tool registry, multi-turn
//! loop, chat panel and settings dialog.
//!
//! Containment rules (AGENTS.md §Purity rule, ADR 0007 §D8): only `panel.rs`
//! and `settings_ui.rs` may import `egui`, no file here may import `eframe` or
//! `rfd`, and only `transport.rs` may import `reqwest`.

pub mod wire;
pub use wire::{AssistantMessage, ChatMessage, ChatResponse, Choice, ToolCall, ToolCallFunction};

pub mod settings_ui;
pub mod transport;
pub use transport::{chat_completion, TransportError};

pub mod tools;
pub use tools::{dispatch_tool_call, tool_definitions, ToolCallError};

pub mod loop_;
pub use loop_::{
    clamp_step_budget, run_agent_turn, AgentError, AGENT_STEP_BUDGET_DEFAULT,
    AGENT_STEP_BUDGET_MAX, AGENT_STEP_BUDGET_MIN,
};

pub mod panel;
pub use panel::{draw_agent_panel, AgentPanelMsg};
