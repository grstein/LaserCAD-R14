//! Document model: entity enum, schema, commands, history stack, selection.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`. All entity mutation goes through
//! the `Command` trait + history stack; tools and the agent build commands and
//! call `App::commit`.
//!
//! Submodules arrive with demands LCV-020 .. LCV-027.

pub const MODULE: &str = "document";
