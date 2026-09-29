//! Top-level [`Document`] value: every entity the operator placed, the layers
//! they sit on (LCV-156, ADR 0012), the selection and the bed size.
//! File is `state.rs` so the path reads `document::state::Document`.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`. Introduced by demand LCV-021.

use crate::document::layer::{check_fields, name_key};
use crate::document::{Entity, Layer, LayerError, LayerId, Selection};
use crate::geometry::Vec2;
use crate::util::{DEFAULT_BED_HEIGHT_MM, DEFAULT_BED_WIDTH_MM};

/// The drawing the operator is editing.
///
/// Owns every placed [`Entity`], the layers they sit on (LCV-156, ADR 0012)
/// and the current [`Selection`]. `entities` is `pub` for reads; its length
/// only changes through [`Document::push_entity`], [`Document::insert_entity`],
/// [`Document::remove_entity`] and [`Document::truncate_entities`], which keep
/// the private per-entity layer vector in lockstep.
///
/// Invariants of the private layer state: at least one layer; ids, name keys
/// ([`name_key`]) and colors unique; `current_layer` exists; one layer id per
/// entity, each existing.
///
/// Not `Copy`, not `Clone` (ADR 0007).
#[derive(Debug)]
pub struct Document {
    /// Placed geometric entities, in insertion order. The index inside this
    /// vector is the entity's addressable handle. Mutate its length only
    /// through the `Document` entity mutators.
    pub entities: Vec<Entity>,
    /// Current selection state (LCV-027).
    pub selection: Selection,
    /// Machine bed size in millimetres, `[width, height]` (LCV-114).
    ///
    /// The **single owner** of the current bed: written into the SVG header,
    /// read back on import, carried in the autosave envelope, and changed
    /// only through [`SetBedSize`](crate::document::SetBedSize).
    /// `bed_mm[1]` is the axis `crate::util::flip_y` mirrors around.
    pub bed_mm: [f64; 2],
    layers: Vec<Layer>,
    current_layer: LayerId,
    entity_layers: Vec<LayerId>,
}

/// A blank document on a [`DEFAULT_BED_WIDTH_MM`] × [`DEFAULT_BED_HEIGHT_MM`]
/// bed with the single default `Cut` layer current (LCV-156 AC 1).
impl Default for Document {
    fn default() -> Self {
        Self::with_bed([DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM])
    }
}

impl Document {
    /// A blank document on `bed_mm` with the default `Cut` layer current.
    pub fn with_bed(bed_mm: [f64; 2]) -> Self {
        let cut = Layer::default_cut();
        Self {
            entities: Vec::new(),
            selection: Selection::default(),
            bed_mm,
            current_layer: cut.id,
            layers: vec![cut],
            entity_layers: Vec::new(),
        }
    }

    /// A validated document from its parts (open, import, autosave restore).
    /// Refuses layer sets that break the invariants, and entity/membership
    /// vectors of different lengths or naming a missing layer.
    pub fn from_parts(
        bed_mm: [f64; 2],
        layers: Vec<Layer>,
        current_layer: LayerId,
        entities: Vec<Entity>,
        entity_layers: Vec<LayerId>,
    ) -> Result<Document, LayerError> {
        let mut doc = Self::with_bed(bed_mm);
        doc.layers.clear();
        for layer in layers {
            if doc.layer(layer.id).is_some() {
                return Err(LayerError::DuplicateName(layer.name));
            }
            check_fields(&doc.layers, None, &layer.name, layer.color)?;
            doc.layers.push(layer);
        }
        if doc.layers.is_empty() {
            return Err(LayerError::LastLayer);
        }
        let known = |id: LayerId| doc.layer(id).is_some();
        if !known(current_layer) {
            return Err(LayerError::UnknownLayer(format!("#{}", current_layer.0)));
        }
        if entities.len() != entity_layers.len() {
            return Err(LayerError::UnknownLayer("(entity without a layer)".into()));
        }
        if let Some(id) = entity_layers.iter().find(|&&id| !known(id)) {
            return Err(LayerError::UnknownLayer(format!("#{}", id.0)));
        }
        doc.current_layer = current_layer;
        doc.entities = entities;
        doc.entity_layers = entity_layers;
        Ok(doc)
    }

    /// Axis-aligned bounding box union over every entity, `(min, max)` in mm;
    /// `None` for an empty document.
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

    /// Convenience wrapper for `self.entities.len()`.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// The layers, in display and file order.
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// The layer new entities land on.
    pub fn current_layer(&self) -> LayerId {
        self.current_layer
    }

    /// The layer with `id`, if any.
    pub fn layer(&self, id: LayerId) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    /// The layer whose name has the same [`name_key`] as `name`, if any.
    pub fn layer_by_name(&self, name: &str) -> Option<&Layer> {
        let key = name_key(name);
        self.layers.iter().find(|l| name_key(&l.name) == key)
    }

    /// The layer entity `index` sits on, if the index is in range.
    pub fn entity_layer(&self, index: usize) -> Option<LayerId> {
        self.entity_layers.get(index).copied()
    }

    /// Entity `index`'s layer color; the first layer's color when the index
    /// has no membership (never for a document kept in lockstep).
    pub fn layer_color(&self, index: usize) -> [u8; 3] {
        // Invariant: a document always has at least one layer.
        let first = self.layers[0].color;
        self.entity_layer(index)
            .and_then(|id| self.layer(id))
            .map_or(first, |l| l.color)
    }

    /// How many entities sit on layer `id`.
    pub fn layer_entity_count(&self, id: LayerId) -> usize {
        self.entity_layers.iter().filter(|&&l| l == id).count()
    }

    /// The id a new layer gets: one past the largest in use.
    pub fn next_layer_id(&self) -> LayerId {
        let max = self.layers.iter().map(|l| l.id.0).max();
        LayerId(max.map_or(0, |m| m + 1))
    }

    /// Append `entity` on layer `layer`.
    pub fn push_entity(&mut self, entity: Entity, layer: LayerId) {
        debug_assert!(self.layer(layer).is_some(), "push_entity: unknown layer");
        self.entities.push(entity);
        self.entity_layers.push(layer);
        self.debug_lockstep();
    }

    /// Append `entity` on the current layer.
    pub fn push_current(&mut self, entity: Entity) {
        self.push_entity(entity, self.current_layer);
    }

    /// Insert `entity` at `index` on layer `layer`.
    pub fn insert_entity(&mut self, index: usize, entity: Entity, layer: LayerId) {
        debug_assert!(self.layer(layer).is_some(), "insert_entity: unknown layer");
        self.entities.insert(index, entity);
        self.entity_layers.insert(index, layer);
        self.debug_lockstep();
    }

    /// Remove and return entity `index` with its layer.
    pub fn remove_entity(&mut self, index: usize) -> (Entity, LayerId) {
        let entity = self.entities.remove(index);
        let layer = self.entity_layers.remove(index);
        self.debug_lockstep();
        (entity, layer)
    }

    /// Keep only the first `len` entities.
    pub fn truncate_entities(&mut self, len: usize) {
        self.entities.truncate(len);
        self.entity_layers.truncate(len);
        self.debug_lockstep();
    }

    /// Refuse a new layer named `name` with `color` that would break the
    /// layer invariants (AC 6).
    pub fn check_new_layer(&self, name: &str, color: [u8; 3]) -> Result<(), LayerError> {
        check_fields(&self.layers, None, name, color)
    }

    /// Refuse renaming/recoloring layer `id` to `name`/`color` (AC 6).
    pub fn check_edit_layer(
        &self,
        id: LayerId,
        name: &str,
        color: [u8; 3],
    ) -> Result<(), LayerError> {
        if self.layer(id).is_none() {
            return Err(LayerError::UnknownLayer(format!("#{}", id.0)));
        }
        check_fields(&self.layers, Some(id), name, color)
    }

    /// Refuse deleting layer `id` while it has entities or is the last (AC 7).
    pub fn check_delete_layer(&self, id: LayerId) -> Result<(), LayerError> {
        let layer = self
            .layer(id)
            .ok_or(LayerError::UnknownLayer(format!("#{}", id.0)))?;
        if self.layer_entity_count(id) > 0 {
            return Err(LayerError::NotEmpty(layer.name.clone()));
        }
        if self.layers.len() == 1 {
            return Err(LayerError::LastLayer);
        }
        Ok(())
    }

    /// Insert `layer` at display position `pos` (layer commands only).
    pub(crate) fn insert_layer(&mut self, pos: usize, layer: Layer) {
        self.layers.insert(pos.min(self.layers.len()), layer);
    }

    /// Remove layer `id`, returning it and its display position.
    pub(crate) fn remove_layer(&mut self, id: LayerId) -> Option<(usize, Layer)> {
        let pos = self.layers.iter().position(|l| l.id == id)?;
        Some((pos, self.layers.remove(pos)))
    }

    /// Replace the layer with `layer.id`, returning the old value.
    pub(crate) fn replace_layer(&mut self, layer: Layer) -> Option<Layer> {
        let slot = self.layers.iter_mut().find(|l| l.id == layer.id)?;
        Some(std::mem::replace(slot, layer))
    }

    /// Make `id` current, returning the previous current layer.
    pub(crate) fn set_current_layer(&mut self, id: LayerId) -> LayerId {
        debug_assert!(self.layer(id).is_some(), "set_current_layer: unknown layer");
        std::mem::replace(&mut self.current_layer, id)
    }

    /// Move entity `index` onto `layer`, returning its previous layer.
    pub(crate) fn set_entity_layer(&mut self, index: usize, layer: LayerId) -> LayerId {
        std::mem::replace(&mut self.entity_layers[index], layer)
    }

    fn debug_lockstep(&self) {
        debug_assert_eq!(
            self.entities.len(),
            self.entity_layers.len(),
            "layer lockstep"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Circle, EPSILON, Line};

    fn bbox_approx_eq(a: (Vec2, Vec2), b: (Vec2, Vec2)) -> bool {
        a.0.approx_eq(b.0, EPSILON) && a.1.approx_eq(b.1, EPSILON)
    }

    /// `with_bed` builds a blank document on the given bed.
    #[test]
    fn with_bed_construction() {
        let doc = Document::with_bed([100.0, 50.0]);
        assert_eq!((doc.entity_count(), doc.bed_mm), (0, [100.0, 50.0]));
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
        doc.push_current(Entity::Line(line));
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
        doc.push_current(Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 5.0),
        )));
        doc.push_current(Entity::Circle(Circle::new(Vec2::new(20.0, 20.0), 3.0)));
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

        doc.push_current(Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
        )));
        assert_eq!(doc.entity_count(), 1);
        assert_eq!(doc.entity_count(), doc.entities.len());

        doc.push_current(Entity::Circle(Circle::new(Vec2::new(5.0, 5.0), 2.0)));
        doc.push_current(Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 1.0),
        )));
        assert_eq!(doc.entity_count(), 3);
        assert_eq!(doc.entity_count(), doc.entities.len());
    }

    fn layer(id: u32, name: &str, color: [u8; 3]) -> Layer {
        let output = true;
        let (id, name) = (LayerId(id), name.to_owned());
        Layer {
            id,
            name,
            color,
            output,
        }
    }

    fn two_layers() -> Vec<Layer> {
        vec![layer(0, "Cut", [255, 0, 0]), layer(3, "Mark", [0, 0, 255])]
    }

    fn a_line() -> Entity {
        Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0)))
    }

    /// LCV-156 AC 1 — a new document has exactly one current layer `Cut`, red,
    /// Output on.
    #[test]
    fn new_document_has_one_current_cut_layer() {
        let doc = Document::default();
        assert_eq!(doc.layers(), &[Layer::default_cut()]);
        assert_eq!(doc.current_layer(), doc.layers()[0].id);
        let l = &doc.layers()[0];
        assert_eq!(
            (l.name.as_str(), l.color, l.output),
            ("Cut", [255, 0, 0], true)
        );
    }

    #[test]
    fn mutators_keep_membership_in_lockstep() {
        let mut doc = Document::from_parts([10.0; 2], two_layers(), LayerId(0), vec![], vec![])
            .expect("valid parts");
        doc.push_entity(a_line(), LayerId(3));
        doc.push_current(a_line());
        doc.insert_entity(1, a_line(), LayerId(3));
        assert_eq!(doc.entity_layer(0), Some(LayerId(3)));
        assert_eq!(doc.entity_layer(1), Some(LayerId(3)));
        assert_eq!(doc.entity_layer(2), Some(LayerId(0)));
        assert_eq!(doc.layer_color(2), [255, 0, 0]);
        assert_eq!(doc.layer_color(0), [0, 0, 255]);
        assert_eq!(doc.layer_entity_count(LayerId(3)), 2);
        assert_eq!(doc.remove_entity(0).1, LayerId(3));
        doc.truncate_entities(1);
        assert_eq!((doc.entity_count(), doc.entity_layer(1)), (1, None));
        assert_eq!(doc.next_layer_id(), LayerId(4));
    }

    #[test]
    fn from_parts_refuses_broken_layer_sets() {
        let bed = [10.0; 2];
        let parts = |layers: Vec<Layer>, current: u32, el: Vec<LayerId>| {
            let entities = vec![a_line(); el.len()];
            Document::from_parts(bed, layers, LayerId(current), entities, el).map(|_| ())
        };
        assert_eq!(parts(vec![], 0, vec![]), Err(LayerError::LastLayer));
        let dup_name = vec![layer(0, "Cut", [1, 0, 0]), layer(1, "cut", [2, 0, 0])];
        assert!(matches!(
            parts(dup_name, 0, vec![]),
            Err(LayerError::DuplicateName(_))
        ));
        let dup_color = vec![layer(0, "A", [1, 0, 0]), layer(1, "B", [1, 0, 0])];
        assert!(matches!(
            parts(dup_color, 0, vec![]),
            Err(LayerError::DuplicateColor(_))
        ));
        assert_eq!(
            parts(vec![layer(0, "#", [1, 0, 0])], 0, vec![]),
            Err(LayerError::EmptyName)
        );
        assert!(parts(two_layers(), 9, vec![]).is_err());
        assert!(parts(two_layers(), 0, vec![LayerId(7)]).is_err());
        let mismatched =
            Document::from_parts(bed, two_layers(), LayerId(0), vec![], vec![LayerId(0)]);
        assert!(mismatched.is_err());
        assert!(parts(two_layers(), 3, vec![LayerId(0), LayerId(3)]).is_ok());
    }

    /// LCV-156 AC 6 — duplicate names (by sanitised key) and colors refused.
    #[test]
    fn check_new_and_edit_refuse_duplicates() {
        let doc = Document::from_parts([10.0; 2], two_layers(), LayerId(0), vec![], vec![])
            .expect("valid parts");
        let dup = doc.check_new_layer(" mark! ", [0, 1, 0]);
        assert_eq!(dup, Err(LayerError::DuplicateName("Mark".into())));
        assert!(matches!(
            doc.check_new_layer("Engrave", [0, 0, 255]),
            Err(LayerError::DuplicateColor(_))
        ));
        assert_eq!(
            doc.check_new_layer("  ", [0, 1, 0]),
            Err(LayerError::EmptyName)
        );
        assert_eq!(doc.check_new_layer("Engrave", [0, 1, 0]), Ok(()));
        assert_eq!(
            doc.check_edit_layer(LayerId(3), "MARK", [0, 0, 255]),
            Ok(())
        );
        assert!(
            doc.check_edit_layer(LayerId(3), "cut", [0, 0, 255])
                .is_err()
        );
        assert!(doc.check_edit_layer(LayerId(9), "X", [9, 9, 9]).is_err());
    }

    /// LCV-156 AC 7 — a layer with entities, or the last layer, is not deleted.
    #[test]
    fn check_delete_refuses_non_empty_and_last() {
        let mut doc = Document::from_parts([10.0; 2], two_layers(), LayerId(0), vec![], vec![])
            .expect("valid parts");
        doc.push_entity(a_line(), LayerId(3));
        assert_eq!(
            doc.check_delete_layer(LayerId(3)),
            Err(LayerError::NotEmpty("Mark".into()))
        );
        assert_eq!(doc.check_delete_layer(LayerId(0)), Ok(()));
        assert_eq!(
            Document::default().check_delete_layer(LayerId(0)),
            Err(LayerError::LastLayer)
        );
        assert!(doc.check_delete_layer(LayerId(8)).is_err());
    }
}
