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
    pub fn entity_id(&self, index: usize) -> Option<EntityId> {
        self.entity_ids.get(index).copied()
    }

    /// The current index of the entity with `id`, if it is live.
    pub fn index_of(&self, id: EntityId) -> Option<usize> {
        self.entity_ids.iter().position(|&e| e == id)
    }

    /// Debug-check that `id` was handed out before and is not live, so
    /// putting it back cannot duplicate an id.
    pub(super) fn debug_restorable(&self, id: EntityId) {
        debug_assert!(id.0 < self.next_id, "restored id {id} never handed out");
        debug_assert!(self.index_of(id).is_none(), "restored id {id} is live");
    }

    /// Take the next id; the counter never goes back.
    pub(super) fn fresh_id(&mut self) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        id
    }
}
