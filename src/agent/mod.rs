//! Optional LLM agent: HTTP transport, wire types, tool registry, multi-turn
//! loop, command-line routing, chat panel and settings dialog.
//!
//! Containment rules (AGENTS.md §Purity rule, ADR 0007 §D8): only `panel.rs`
//! and `settings_ui.rs` may import `egui`, no file here may import `eframe` or
//! `rfd`, and only `transport.rs` may import `reqwest`.
//!
//! Since LCV-122 one more rule joins them (ADR 0007 §D1): **no file here holds
//! document state.** `Document`, `History`, `Vec<Entity>` and `Arc<Mutex<_>>`
//! appear nowhere under `src/agent/`, with **no exceptions**. LCV-122 carried
//! one — `panel.rs`, which built a throwaway `Document`/`History` pair for
//! every turn — and LCV-123 deleted it: the turn now runs against the
//! operator's real document on the UI thread (`src/app/agent_turn.rs`), and
//! this directory never names one. `tests/it/repo/bridge_scans.rs` enforces
//! that, and asserts its exception list **empty**; re-opening it needs an ADR.

pub mod wire;
pub use wire::{AssistantMessage, ChatMessage, ChatResponse, Choice, ToolCall, ToolCallFunction};

pub mod settings_ui;
pub use settings_ui::{AgentSettingsFrame, SYSTEM_PROMPT_ID, draw_agent_settings};

pub mod transport;
pub use transport::{TransportError, chat_completion};

pub mod bridge;
pub use bridge::{AgentAction, AgentEvent, AgentOutcome};

pub mod classifier;
pub use classifier::{Route, classify};

pub mod drawing;
pub use drawing::DrawingItem;

pub mod tools;
pub use tools::{ToolCallError, parse_tool_call, tool_definitions};

pub mod prompt;
pub use prompt::DEFAULT_PROMPT;

pub mod loop_;
pub use loop_::{
    AGENT_STEP_BUDGET_DEFAULT, AGENT_STEP_BUDGET_MAX, AGENT_STEP_BUDGET_MIN, AgentError,
    clamp_step_budget,
};
// The loop is driven from `src/app/agent_worker.rs`, which owns the turn
// (ADR 0007 §D8). Re-exported here so that file needs no deep path.
pub(crate) use loop_::agent_loop;

pub mod memory;
pub use memory::{Memory, TurnEnd};

pub mod panel;
pub use panel::draw_agent_panel;
