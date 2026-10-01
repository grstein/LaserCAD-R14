//! Geometry outside the machine bed (LCV-168).
//!
//! An entity is *outside the bed* when its exact bounding box leaves
//! `[0, w] × [0, h]` of [`Document::bed_mm`] by more than [`EPSILON`] on any
//! side. Save and Export Layers count such entities to warn the operator;
//! nothing here blocks, clips or selects them.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use super::{Document, Entity, LayerId};
use crate::geometry::EPSILON;

/// `true` when `entity`'s bounding box extends past `[0, w] × [0, h]` of
/// `bed_mm` (`[w, h]`, mm) by more than [`EPSILON`]. Touching an edge within
/// `EPSILON` is inside; an arc's bulge counts because its bbox includes its
/// cardinal extremes.
pub fn outside_bed(entity: &Entity, bed_mm: [f64; 2]) -> bool {
    let (min, max) = entity.bbox();
    min.x < -EPSILON
        || min.y < -EPSILON
        || max.x > bed_mm[0] + EPSILON
        || max.y > bed_mm[1] + EPSILON
}

impl Document {
    /// How many entities on layers for which `on` returns `true` are
    /// [`outside_bed`] of this document's bed.
    pub fn outside_bed_count(&self, on: impl Fn(LayerId) -> bool) -> usize {
        self.entities
            .iter()
            .enumerate()
            .filter(|(i, e)| self.entity_layer(*i).is_some_and(&on) && outside_bed(e, self.bed_mm))
            .count()
    }
}
