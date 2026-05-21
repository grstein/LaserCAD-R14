//! Tools: `Tool` trait, `ToolManager`, one file per drawing or modify tool.
//!
//! Tools never mutate `Document` directly — they construct a `Command` and call
//! `history.commit(cmd, doc)`. State machines stay inside individual tool files.
//!
//! Submodules arrive with demands LCV-040 .. LCV-054.

pub mod line;
pub mod manager;
pub mod pointer_event;
pub mod select;
pub mod tool;

pub use line::LineTool;
pub use manager::ToolManager;
pub use pointer_event::{PointerButton, PointerEvent};
pub use select::SelectTool;
pub use tool::Tool;

/// Module witness for the skeleton test. Will retire once all tools land.
pub const MODULE: &str = "tools";
