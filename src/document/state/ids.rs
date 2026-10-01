//! Stable entity ids (LCV-188, ADR 0014): one id per entity, kept in lockstep
//! with `Document::entities`, never reused during an app run, never saved.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use crate::document::Document;

/// An entity's id for the app run, printed `e<N>` (N ≥ 1). Survives every
/// other edit; in-place edits keep it; undo and redo restore it (ADR 0014).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityId(pub u64);

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "e{}", self.0)
    }
}

impl Document {
    /// The id of entity `index`, if the index is in range.
    pub fn entity_id(&self, _index: usize) -> Option<EntityId> {
        None
    }

    /// The current index of the entity with `id`, if it is live.
    pub fn index_of(&self, _id: EntityId) -> Option<usize> {
        None
    }
}
