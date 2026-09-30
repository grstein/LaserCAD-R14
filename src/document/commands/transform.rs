//! [`TransformEntities`]: replace entities in place with their image under a
//! [`Transform`], or append the images and keep the sources. Backs the
//! `RotateTool` and the agent's `rotate_entity` (LCV-158) and MIRROR
//! (LCV-181); SCALE (LCV-182) reuses it.
//!
//! In place, `do_` snapshots the originals before replacing them; `undo`
//! writes the snapshot back, so the round trip is bit-exact even though a
//! rotation by an arbitrary angle is not exactly invertible in `f64`.
//! Replacing in place keeps every entity's index, layer and selection
//! untouched. With `keep_source`, `do_` appends each image on its source's
//! layer and `undo` truncates back to the captured length, as
//! `CopyEntities` does; the sources and the selection are untouched.
//!
//! Out-of-range indices are an invariant violation; `debug_assert!` traps it
//! in debug. Callers build `indices` from valid entities only.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use super::Command;
use crate::document::{Document, Entity};
use crate::geometry::Transform;

/// Map a set of entities through a [`Transform`], in place.
/// Empty `indices` is a valid no-op for both `do_` and `undo`.
#[derive(Debug)]
pub struct TransformEntities {
    /// Indices into [`Document::entities`] to transform. Caller ensures each
    /// index is in range and unique.
    pub indices: Vec<usize>,
    /// The transform applied to each indexed entity.
    pub transform: Transform,
    /// `true` appends the images and keeps the sources; `false` replaces the
    /// sources in place.
    pub keep_source: bool,
    /// `(index, original)` pairs captured by the last in-place `do_`.
    originals: Vec<(usize, Entity)>,
    /// Entity count before the last keep-source `do_`; `undo` truncates to it.
    len_before: usize,
}

impl TransformEntities {
    /// Build a [`TransformEntities`] that maps `indices` through `transform`.
    pub fn new(indices: Vec<usize>, transform: Transform) -> Self {
        Self {
            indices,
            transform,
            keep_source: false,
            originals: Vec::new(),
            len_before: 0,
        }
    }

    /// The same command, appending the images and keeping the sources when
    /// `keep` is `true`.
    pub fn with_keep_source(mut self, keep: bool) -> Self {
        self.keep_source = keep;
        self
    }
}

impl Command for TransformEntities {
    fn do_(&mut self, doc: &mut Document) {
        self.originals.clear();
        self.len_before = doc.entities.len();
        if self.keep_source {
            for &i in &self.indices {
                debug_assert!(i < self.len_before, "TransformEntities: index OOR");
                let image = doc.entities[i].transformed(&self.transform);
                let layer = doc.entity_layer(i).unwrap_or_else(|| doc.current_layer());
                doc.push_entity(image, layer);
            }
            return;
        }
        for &i in &self.indices {
            debug_assert!(i < doc.entities.len(), "TransformEntities: index OOR");
            let original = doc.entities[i];
            doc.entities[i] = original.transformed(&self.transform);
            self.originals.push((i, original));
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if self.keep_source {
            doc.truncate_entities(self.len_before);
            return;
        }
        while let Some((i, original)) = self.originals.pop() {
            doc.entities[i] = original;
        }
    }

    fn label(&self) -> &str {
        match self.transform {
            Transform::Rotate { .. } => "Rotate Entities",
            Transform::Mirror { .. } => "Mirror Entities",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::commands::AddLayer;
    use crate::document::{History, LayerId};
    use crate::geometry::{Arc, Circle, EPSILON, Line, Vec2};
    use core::f64::consts::FRAC_PI_2;

    /// Line 0 and arc 2 on the default layer, circle 1 on `Engrave`; 0 and 1
    /// selected.
    fn doc() -> (Document, LayerId) {
        let mut doc = Document::default();
        doc.push_current(Entity::Line(Line::new(
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 0.0),
        )));
        AddLayer::new("Engrave", [0, 0, 255], true).do_(&mut doc);
        let engrave = doc.layer_by_name("Engrave").unwrap().id;
        doc.push_entity(
            Entity::Circle(Circle::new(Vec2::new(3.0, 3.0), 1.0)),
            engrave,
        );
        doc.push_current(Entity::Arc(Arc::new(
            Vec2::new(0.3, 0.7),
            0.5,
            0.1,
            1.3,
            true,
        )));
        doc.selection.add(0);
        doc.selection.add(1);
        (doc, engrave)
    }

    /// An angle whose rotation does not round-trip exactly in `f64`.
    fn rotation() -> Transform {
        Transform::Rotate {
            base: Vec2::new(0.1, 0.2),
            angle: 0.7,
        }
    }

    /// AC7 — `do_` replaces the indexed entities in place; the others, the
    /// layers and the selection stay as they were.
    #[test]
    fn do_replaces_in_place_keeping_layer_and_selection() {
        let (mut doc, engrave) = doc();
        let before = doc.entities.clone();
        let selection = doc.selection.clone();
        let t = Transform::Rotate {
            base: Vec2::new(0.0, 0.0),
            angle: FRAC_PI_2,
        };
        TransformEntities::new(vec![0, 1], t).do_(&mut doc);
        assert_eq!(doc.entity_count(), 3);
        match doc.entities[0] {
            Entity::Line(l) => {
                assert!(l.p1.approx_eq(Vec2::new(0.0, 1.0), EPSILON));
                assert!(l.p2.approx_eq(Vec2::new(0.0, 2.0), EPSILON));
            }
            other => panic!("variant changed: {other:?}"),
        }
        assert_eq!(doc.entities[1], before[1].transformed(&t));
        assert_eq!(doc.entities[2], before[2]);
        assert_eq!(doc.entity_layer(1), Some(engrave));
        assert_eq!(doc.entity_layer(0), Some(doc.current_layer()));
        assert_eq!(doc.selection, selection);
    }

    /// AC7 — undo restores the originals bit-exact; redo gives the same
    /// result as the first `do_`.
    #[test]
    fn undo_is_bit_exact_and_redo_is_identical() {
        let (mut doc, _) = doc();
        let before = doc.entities.clone();
        let mut cmd = TransformEntities::new(vec![0, 1, 2], rotation());
        cmd.do_(&mut doc);
        let after = doc.entities.clone();
        assert_ne!(after, before);
        cmd.undo(&mut doc);
        assert_eq!(doc.entities, before);
        cmd.do_(&mut doc);
        assert_eq!(doc.entities, after);
        cmd.undo(&mut doc);
        assert_eq!(doc.entities, before);
    }

    /// AC7 — committed through `History`, a rotation is one undo step.
    #[test]
    fn history_records_one_step() {
        let (mut doc, _) = doc();
        let before = doc.entities.clone();
        let mut history = History::default();
        history.commit(
            Box::new(TransformEntities::new(vec![0, 1, 2], rotation())),
            &mut doc,
        );
        assert_eq!(history.len(), 1);
        assert!(history.undo(&mut doc));
        assert_eq!(doc.entities, before);
        assert!(!history.can_undo());
    }

    /// Empty indices are a no-op; the label names the edit.
    #[test]
    fn empty_indices_noop_and_label() {
        let (mut doc, _) = doc();
        let before = doc.entities.clone();
        let mut cmd = TransformEntities::new(Vec::new(), rotation());
        cmd.do_(&mut doc);
        assert_eq!(doc.entities, before);
        cmd.undo(&mut doc);
        assert_eq!(doc.entities, before);
        let boxed: Box<dyn Command> = Box::new(cmd);
        assert_eq!(boxed.label(), "Rotate Entities");
    }

    /// Across the vertical line x = 5.
    fn mirror() -> Transform {
        Transform::Mirror {
            a: Vec2::new(5.0, 0.0),
            b: Vec2::new(5.0, 1.0),
        }
    }

    /// LCV-181 AC6 — keep-source appends each image on its source's layer,
    /// leaves the sources and the selection alone, and undo/redo round-trips.
    #[test]
    fn keep_source_appends_on_source_layers_and_undoes() {
        let (mut doc, engrave) = doc();
        let before = doc.entities.clone();
        let selection = doc.selection.clone();
        let t = mirror();
        let mut cmd = TransformEntities::new(vec![0, 1], t).with_keep_source(true);
        cmd.do_(&mut doc);
        assert_eq!(doc.entity_count(), 5);
        assert_eq!(doc.entities[..3], before[..]);
        assert_eq!(doc.entities[3], before[0].transformed(&t));
        assert_eq!(doc.entities[4], before[1].transformed(&t));
        assert_eq!(doc.entity_layer(3), Some(doc.current_layer()));
        assert_eq!(doc.entity_layer(4), Some(engrave));
        assert_eq!(doc.selection, selection);
        let after = doc.entities.clone();
        cmd.undo(&mut doc);
        assert_eq!(doc.entities, before);
        assert_eq!(doc.entity_layer(1), Some(engrave));
        cmd.do_(&mut doc);
        assert_eq!(doc.entities, after);
        assert_eq!(doc.entity_layer(4), Some(engrave));
    }

    /// LCV-181 AC7 — without keep-source the images replace the sources in
    /// place, on the same indices and layers.
    #[test]
    fn erase_source_replaces_in_place() {
        let (mut doc, engrave) = doc();
        let before = doc.entities.clone();
        let t = mirror();
        TransformEntities::new(vec![0, 1], t).do_(&mut doc);
        assert_eq!(doc.entity_count(), 3);
        assert_eq!(doc.entities[0], before[0].transformed(&t));
        assert_eq!(doc.entities[1], before[1].transformed(&t));
        assert_eq!(doc.entity_layer(1), Some(engrave));
    }

    /// LCV-181 AC9 — a keep-source mirror is one undo step labelled as such.
    #[test]
    fn keep_source_history_records_one_step() {
        let (mut doc, _) = doc();
        let before = doc.entities.clone();
        let mut history = History::default();
        let cmd = TransformEntities::new(vec![0, 1, 2], mirror()).with_keep_source(true);
        assert_eq!((&cmd as &dyn Command).label(), "Mirror Entities");
        history.commit(Box::new(cmd), &mut doc);
        assert_eq!(history.len(), 1);
        assert_eq!(doc.entity_count(), 6);
        assert!(history.undo(&mut doc));
        assert_eq!(doc.entities, before);
        assert!(!history.can_undo());
    }
}
