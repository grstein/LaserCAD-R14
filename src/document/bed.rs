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

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;
    use crate::document::{AddLayer, Command};
    use crate::geometry::{Arc, Circle, Line, Vec2};

    const BED: [f64; 2] = [400.0, 297.5];

    fn line(x1: f64, y1: f64, x2: f64, y2: f64) -> Entity {
        Entity::Line(Line::new(Vec2::new(x1, y1), Vec2::new(x2, y2)))
    }

    /// AC 2 — geometry well inside the bed is not outside.
    #[test]
    fn outside_bed_inside_is_false() {
        assert!(!outside_bed(&line(10.0, 10.0, 100.0, 200.0), BED));
        let circle = Entity::Circle(Circle::new(Vec2::new(200.0, 100.0), 50.0));
        assert!(!outside_bed(&circle, BED));
    }

    /// AC 2 — touching an edge, or past it by less than `EPSILON`, is inside.
    #[test]
    fn outside_bed_on_edge_within_epsilon_is_false() {
        assert!(!outside_bed(&line(0.0, 0.0, 400.0, 297.5), BED));
        let e = EPSILON / 2.0;
        assert!(!outside_bed(&line(-e, -e, 400.0 + e, 297.5 + e), BED));
    }

    /// AC 2 — past any one side by 2·EPSILON is outside.
    #[test]
    fn outside_bed_past_each_side_is_true() {
        let d = 2.0 * EPSILON;
        assert!(outside_bed(&line(-d, 10.0, 50.0, 10.0), BED), "left");
        assert!(outside_bed(&line(10.0, -d, 10.0, 50.0), BED), "bottom");
        assert!(
            outside_bed(&line(10.0, 10.0, 400.0 + d, 10.0), BED),
            "right"
        );
        assert!(outside_bed(&line(10.0, 10.0, 10.0, 297.5 + d), BED), "top");
    }

    /// AC 2 — an arc whose endpoints are inside but whose bulge crosses the
    /// top edge is outside.
    #[test]
    fn outside_bed_arc_bulge_past_top_is_true() {
        let arc = Arc::new(Vec2::new(200.0, 292.5), 10.0, 0.0, PI, true);
        assert!(outside_bed(&Entity::Arc(arc), BED));
        let low = Arc::new(Vec2::new(200.0, 292.5), 10.0, PI, 2.0 * PI, true);
        assert!(!outside_bed(&Entity::Arc(low), BED));
    }

    /// AC 2 — the count honours the layer filter.
    #[test]
    fn outside_bed_count_filters_by_layer() {
        let mut doc = Document::with_bed(BED);
        let cut = doc.current_layer();
        let mut add = AddLayer::new("Off", [9, 9, 9], false);
        add.do_(&mut doc);
        let off = add.id().expect("allocated by do_");
        doc.push_entity(line(-5.0, 10.0, 50.0, 10.0), cut);
        doc.push_entity(line(10.0, 10.0, 50.0, 10.0), cut);
        doc.push_entity(line(10.0, 10.0, 10.0, 300.0), off);
        assert_eq!(doc.outside_bed_count(|_| true), 2);
        assert_eq!(doc.outside_bed_count(|l| l == cut), 1);
        assert_eq!(doc.outside_bed_count(|l| l == off), 1);
        assert_eq!(doc.outside_bed_count(|_| false), 0);
    }
}
