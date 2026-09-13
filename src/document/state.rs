//! Top-level [`Document`] value: the in-memory representation of the drawing
//! the operator is editing — every line, circle, and arc they have placed plus
//! whatever they currently have selected.
//!
//! Filename note: this file is `state.rs` (not `document.rs`) so the module
//! path resolves to `document::state::Document` rather than the noisy
//! `document::document::Document`. The parent [`crate::document`] `mod.rs`
//! re-exports the public surface so external callers reach it as
//! `lasercad::document::Document`.
//!
//! [`Selection`] lives in [`crate::document::selection`] (concretized by
//! LCV-027, replacing the LCV-021 placeholder that originally sat in this
//! file). `Document` continues to own a `pub selection: Selection` field.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`. The document model is part of
//! the pure-Rust kernel.
//!
//! Introduced by demand LCV-021.

use crate::document::{Entity, Selection};
use crate::geometry::Vec2;
use crate::util::{DEFAULT_BED_HEIGHT_MM, DEFAULT_BED_WIDTH_MM};

/// The drawing the operator is editing.
///
/// Owns every placed [`Entity`] plus the current [`Selection`]. The `entities`
/// field is `pub` for now because LCV-022's `Command` trait has not landed
/// yet; once it does, all mutation flows through commands and the history
/// stack (per `AGENTS.md` §"State and mutation"). Visibility may tighten to
/// `pub(crate)` in a later demand if it proves necessary.
///
/// Deliberately not `Copy` and not `Clone`: a full document clone would be a
/// silent O(n) cost, and no current consumer needs one. A future demand can
/// add `Clone` when it has a concrete reason.
#[derive(Debug)]
pub struct Document {
    /// Placed geometric entities, in insertion order. The index inside this
    /// vector is the entity's addressable handle until a stable-id demand
    /// argues otherwise.
    pub entities: Vec<Entity>,
    /// Current selection state. Concretized by LCV-027 — see
    /// [`crate::document::selection`] for the type definition.
    pub selection: Selection,
    /// Machine bed size in millimetres, `[width, height]` (LCV-114).
    ///
    /// The **single owner** of the current bed: it is written into the SVG
    /// header on export, read back on import, carried in the autosave
    /// envelope, and changed only through
    /// [`SetBedSize`](crate::document::SetBedSize) so Ctrl+Z reaches it.
    /// `bed_mm[1]` is the axis `crate::util::flip_y` mirrors around, which is
    /// why no export or import path may substitute a constant for it.
    pub bed_mm: [f64; 2],
}

/// A blank document on a [`DEFAULT_BED_WIDTH_MM`] × [`DEFAULT_BED_HEIGHT_MM`]
/// bed.
///
/// Written by hand rather than derived (LCV-114 AC 3): `#[derive(Default)]`
/// would produce a `[0.0, 0.0]` bed, i.e. a zero mirror axis and a degenerate
/// SVG canvas. `File > New` overwrites this with the operator's configured
/// default (`Settings::default_bed_mm`); this value is what a `Document`
/// constructed in code or recovered from a pre-LCV-114 autosave envelope
/// starts at.
impl Default for Document {
    fn default() -> Self {
        Self {
            entities: Vec::new(),
            selection: Selection::default(),
            bed_mm: [DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM],
        }
    }
}

impl Document {
    /// Axis-aligned bounding box union over every entity, in `(min, max)`
    /// mm-space order.
    ///
    /// Returns `None` when [`Document::entities`] is empty — the natural
    /// signal for "no extents yet", consumed by zoom-extents (LCV-031) and
    /// SVG export (LCV-056) to fall back to a default view.
    ///
    /// Otherwise the bbox is the component-wise `min` / `max` of every
    /// entity's own [`Entity::bbox`].
    pub fn bounds(&self) -> Option<(Vec2, Vec2)> {
        let mut iter = self.entities.iter();
        let first = iter.next()?;
        let (mut min, mut max) = first.bbox();
        for entity in iter {
            let (emin, emax) = entity.bbox();
            min = Vec2::new(min.x.min(emin.x), min.y.min(emin.y));
            max = Vec2::new(max.x.max(emax.x), max.y.max(emax.y));
        }
        Some((min, max))
    }

    /// Convenience wrapper for `self.entities.len()`. Used by the status bar
    /// (Phase 6) and tests; keeps callers from reaching through the `pub`
    /// field for a single number.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Circle, Line, EPSILON};

    fn bbox_approx_eq(a: (Vec2, Vec2), b: (Vec2, Vec2)) -> bool {
        a.0.approx_eq(b.0, EPSILON) && a.1.approx_eq(b.1, EPSILON)
    }

    /// AC#1 — `Document` constructs via a struct literal over its public
    /// fields, proving the public shape.
    #[test]
    fn document_struct_literal_construction() {
        let doc = Document {
            entities: Vec::new(),
            selection: Selection::default(),
            bed_mm: [DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM],
        };
        assert_eq!(doc.entity_count(), 0);
    }

    /// AC#2 — `Selection::default()` compiles and yields a value.
    #[test]
    fn selection_default_constructs() {
        let _ = Selection::default();
    }

    /// AC#3 — a default document carries no entities.
    #[test]
    fn default_document_is_empty() {
        let doc = Document::default();
        assert!(doc.entities.is_empty());
        assert_eq!(doc.entity_count(), 0);
    }

    /// LCV-114 AC 3 — a blank document starts on the default bed. The
    /// derived `Default` this replaced would have produced `[0.0, 0.0]`.
    #[test]
    fn document_default_bed_is_400_square() {
        let doc = Document::default();
        assert_eq!(doc.bed_mm, [400.0, 400.0]);
        assert_eq!(doc.bed_mm, [DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM]);
    }

    /// LCV-114 AC 3 — `..Default::default()` still fills the bed in, which is
    /// the shape `src/io/file_actions.rs` and `src/io/autosave.rs` use.
    #[test]
    fn document_struct_update_syntax_fills_the_bed() {
        let doc = Document {
            entities: Vec::new(),
            ..Document::default()
        };
        assert_eq!(doc.bed_mm, [DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM]);
    }

    /// AC#4 — `bounds()` over an empty document returns `None`.
    #[test]
    fn default_document_bounds_is_none() {
        let doc = Document::default();
        assert!(doc.bounds().is_none());
    }

    /// AC#5 — a single-line document's bounds match the line's own bbox.
    #[test]
    fn single_line_bounds() {
        let mut doc = Document::default();
        let line = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 5.0));
        doc.entities.push(Entity::Line(line));
        let bounds = doc.bounds().expect("non-empty document has bounds");
        assert!(bbox_approx_eq(
            bounds,
            (Vec2::new(0.0, 0.0), Vec2::new(10.0, 5.0))
        ));
    }

    /// AC#6 — bounds over a line + circle is the component-wise union of the
    /// two bboxes.
    #[test]
    fn union_bounds_over_two_entities() {
        let mut doc = Document::default();
        doc.entities.push(Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 5.0),
        )));
        doc.entities
            .push(Entity::Circle(Circle::new(Vec2::new(20.0, 20.0), 3.0)));
        let bounds = doc.bounds().expect("non-empty document has bounds");
        assert!(bbox_approx_eq(
            bounds,
            (Vec2::new(0.0, 0.0), Vec2::new(23.0, 23.0))
        ));
    }

    /// AC#7 — `entity_count()` mirrors `entities.len()` for 0, 1, 3 entities.
    #[test]
    fn entity_count_matches_vec_len() {
        let mut doc = Document::default();
        assert_eq!(doc.entity_count(), 0);

        doc.entities.push(Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
        )));
        assert_eq!(doc.entity_count(), 1);
        assert_eq!(doc.entity_count(), doc.entities.len());

        doc.entities
            .push(Entity::Circle(Circle::new(Vec2::new(5.0, 5.0), 2.0)));
        doc.entities.push(Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 1.0),
        )));
        assert_eq!(doc.entity_count(), 3);
        assert_eq!(doc.entity_count(), doc.entities.len());
    }
}
