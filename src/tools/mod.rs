//! Tools: `Tool` trait, `ToolManager`, one file per drawing or modify tool.
//!
//! Tools never mutate `Document` directly — they construct a `Command` and call
//! `App::commit`. State machines stay inside individual tool files.
//!
//! Submodules arrive with demands LCV-040 .. LCV-054.
