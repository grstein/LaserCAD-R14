//! Tools: `Tool` trait, `ToolManager`, one file per drawing or modify tool.
//!
//! Tools never mutate `Document` directly — they construct a `Command` and call
//! `App::commit`. State machines stay inside individual tool files.
//!
//! Submodules arrive with demands LCV-040 .. LCV-054.

pub mod manager;
pub mod select;
pub mod tool;

pub use manager::ToolManager;
pub use select::SelectTool;
pub use tool::Tool;

/// Module witness for the skeleton test. Will retire once all tools land.
pub const MODULE: &str = "tools";
