//! Document model: entity enum, schema, commands, history stack, selection.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`. All entity mutation goes through
//! the `Command` trait + history stack; tools and the agent build commands and
//! call `App::commit`.
//!
//! Submodules arrive with demands LCV-020 .. LCV-027.

mod bed;
mod check;
pub mod commands;
pub mod entity;
pub mod history;
pub mod layer;
pub mod schema;
pub mod selection;
pub mod state;

pub use bed::outside_bed;
pub use check::{CheckReport, Finding, GAP_MM, check_drawing};
pub use commands::{
    AddLayer, Command, CompositeCommand, CopyEntities, CreateArc, CreateCircle, CreateLine,
    DeleteEntities, DeleteLayer, EditLayer, ExtendEntity, MoveEntities, NoOpCommand,
    SelectionCommand, SetBedSize, SetCurrentLayer, SetEntityLayers, TransformEntities, TrimEntity,
};
pub use entity::{Entity, SCHEMA_VERSION};
pub use history::{HISTORY_DEPTH, History};
pub use layer::{Layer, LayerError, LayerId, file_key, name_key};
pub use selection::Selection;
pub use state::Document;
pub use state::ids::EntityId;
