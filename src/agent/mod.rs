//! Optional LLM agent: command-line classifier, HTTP transport, tool registry,
//! multi-turn loop, settings dialog.
//!
//! `agent::classifier` is part of the kernel (pure routing on text) and MUST
//! NOT import `egui`/`eframe`/`rfd`. The rest of `agent` may use them.
//!
//! Submodules arrive with demands LCV-075 .. LCV-080.

pub mod settings_ui;
pub mod transport;
pub use transport::{chat_completion, Message, TransportError};

pub const MODULE: &str = "agent";

pub mod tools;
pub use tools::{dispatch_tool_call, tool_definitions, ToolCallError};

pub mod loop_;
pub use loop_::{run_agent_turn, AgentError, MAX_TOOL_CALLS_PER_TURN};
