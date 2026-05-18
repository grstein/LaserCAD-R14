//! Document model: entity enum, schema, commands, history stack, selection.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`. All entity mutation goes through
//! the `Command` trait + history stack; tools and the agent build commands and
//! call `App::commit`.
//!
//! Submodules arrive with demands LCV-020 .. LCV-027.

pub mod commands;
pub mod entity;
pub mod history;
pub mod schema;
pub mod selection;
pub mod state;

pub use commands::{
    Command, CreateArc, CreateCircle, CreateLine, DeleteEntities, MoveEntities, NoOpCommand,
    SelectionCommand,
};
pub use entity::{Entity, SCHEMA_VERSION};
pub use history::{History, HISTORY_DEPTH};
pub use selection::Selection;
pub use state::Document;
