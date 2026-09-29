//! Primitive-creation commands: [`CreateLine`], [`CreateCircle`], [`CreateArc`],
//! [`CreateEntities`].
//!
//! The single-entity commands (`CreateLine`, `CreateCircle`, `CreateArc`) each
//! append exactly one [`crate::document::Entity`] inside `do_`, capture the
//! resulting index, and remove the entity at that index inside `undo`.
//!
//! [`CreateEntities`] appends an arbitrary `Vec<Entity>` atomically — all
//! entities land in one `history.commit` so a single Ctrl+Z reverses the whole
//! batch. It records the tail-index of the vector before appending; `undo`
//! truncates back to that position. The history stack's LIFO guarantee ensures
//! no other entity was appended between `do_` and `undo`.
//!
//! `captured_index: Option<usize>` starts `None`, becomes `Some(i)` after
//! `do_`, and returns to `None` after `undo` (via [`Option::take`]). A second
//! `undo` on an already-undone command is therefore a no-op rather than a
//! corruption that would remove an unrelated entity. Redo flows back through
//! `do_`, which re-captures a fresh index at the new end of the vector.
//!
//! Per [`super::Command`], `cmd.do_(doc); cmd.undo(doc);` leaves `doc`
//! bit-equivalent (`PartialEq`-equal entities, same order, same selection) to
//! the pre-`do_` state.
//!
//! Each command lands its entities on one layer (LCV-156): the one given to
//! `on_layer`, else the current layer at the first `do_`, kept for redo.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`. Introduced by demand LCV-023.
//! [`CreateEntities`] introduced by demand LCV-045.

use super::Command;
use crate::document::{Document, Entity, LayerId};
use crate::geometry::{Arc, Circle, Line};

/// Append a [`Line`] to a [`Document`].
///
/// `do_` pushes `Entity::Line(self.line)` and records the resulting index;
/// `undo` removes the entity at that index and clears the capture so a
/// double-`undo` is a no-op.
#[derive(Debug)]
pub struct CreateLine {
    /// The line that `do_` will append to the document.
    pub line: Line,
    /// Index at which `do_` placed the entity; `None` before the first `do_`
    /// or after `undo`. Module-private to prevent caller tampering.
    captured_index: Option<usize>,
    /// Layer the entity lands on; `None` = the current layer at first `do_`.
    layer: Option<LayerId>,
}

impl CreateLine {
    /// Build a [`CreateLine`] for `line`. `captured_index` starts `None`.
    pub fn new(line: Line) -> Self {
        Self {
            line,
            captured_index: None,
            layer: None,
        }
    }

    /// Put the entity on `layer` instead of the current layer.
    pub fn on_layer(mut self, layer: LayerId) -> Self {
        self.layer = Some(layer);
        self
    }
}

impl Command for CreateLine {
    fn do_(&mut self, doc: &mut Document) {
        let layer = *self.layer.get_or_insert(doc.current_layer());
        doc.push_entity(Entity::Line(self.line), layer);
        self.captured_index = Some(doc.entities.len() - 1);
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(i) = self.captured_index.take() {
            doc.remove_entity(i);
        }
    }

    fn label(&self) -> &str {
        "Create Line"
    }
}

/// Append a [`Circle`] to a [`Document`]. Same shape as [`CreateLine`] over
/// [`Entity::Circle`].
#[derive(Debug)]
pub struct CreateCircle {
    /// The circle that `do_` will append to the document.
    pub circle: Circle,
    /// Index at which `do_` placed the entity; cleared on `undo`.
    captured_index: Option<usize>,
    /// Layer the entity lands on; `None` = the current layer at first `do_`.
    layer: Option<LayerId>,
}

impl CreateCircle {
    /// Build a [`CreateCircle`] for `circle`. `captured_index` starts `None`.
    pub fn new(circle: Circle) -> Self {
        Self {
            circle,
            captured_index: None,
            layer: None,
        }
    }

    /// Put the entity on `layer` instead of the current layer.
    pub fn on_layer(mut self, layer: LayerId) -> Self {
        self.layer = Some(layer);
        self
    }
}

impl Command for CreateCircle {
    fn do_(&mut self, doc: &mut Document) {
        let layer = *self.layer.get_or_insert(doc.current_layer());
        doc.push_entity(Entity::Circle(self.circle), layer);
        self.captured_index = Some(doc.entities.len() - 1);
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(i) = self.captured_index.take() {
            doc.remove_entity(i);
        }
    }

    fn label(&self) -> &str {
        "Create Circle"
    }
}

/// Append an [`Arc`] to a [`Document`]. Same shape as [`CreateLine`] over
/// [`Entity::Arc`].
#[derive(Debug)]
pub struct CreateArc {
    /// The arc that `do_` will append to the document.
    pub arc: Arc,
    /// Index at which `do_` placed the entity; cleared on `undo`.
    captured_index: Option<usize>,
    /// Layer the entity lands on; `None` = the current layer at first `do_`.
    layer: Option<LayerId>,
}

impl CreateArc {
    /// Build a [`CreateArc`] for `arc`. `captured_index` starts `None`.
    pub fn new(arc: Arc) -> Self {
        Self {
            arc,
            captured_index: None,
            layer: None,
        }
    }

    /// Put the entity on `layer` instead of the current layer.
    pub fn on_layer(mut self, layer: LayerId) -> Self {
        self.layer = Some(layer);
        self
    }
}

impl Command for CreateArc {
    fn do_(&mut self, doc: &mut Document) {
        let layer = *self.layer.get_or_insert(doc.current_layer());
        doc.push_entity(Entity::Arc(self.arc), layer);
        self.captured_index = Some(doc.entities.len() - 1);
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(i) = self.captured_index.take() {
            doc.remove_entity(i);
        }
    }

    fn label(&self) -> &str {
        "Create Arc"
    }
}

/// Append a batch of [`Entity`] values to a [`Document`] as one atomic
/// operation (LCV-045).
///
/// `do_` records the pre-append vector length as `captured_start`, then
/// pushes every entity in order. `undo` truncates the vector back to
/// `captured_start`. The history stack's LIFO guarantee means no other
/// entity can have been appended between `do_` and `undo`, so `truncate`
/// exactly reverses the append.
///
/// A double-`undo` is a no-op: `captured_start` is cleared by
/// [`Option::take`], so the second call finds `None` and does nothing.
///
/// Redo works by calling `do_` a second time, which re-captures a fresh
/// `captured_start` at the new tail of the vector and pushes all entities
/// again.
#[derive(Debug)]
pub struct CreateEntities {
    /// The entities that `do_` will append, in order.
    pub entities: Vec<Entity>,
    /// Vector length before `do_` pushed; `None` before the first `do_`
    /// or after `undo`. Module-private to prevent caller tampering.
    captured_start: Option<usize>,
    /// Layer the batch lands on; `None` = the current layer at first `do_`.
    layer: Option<LayerId>,
}

impl CreateEntities {
    /// Build a [`CreateEntities`] for the given `entities`.
    /// `captured_start` starts `None`.
    pub fn new(entities: Vec<Entity>) -> Self {
        Self {
            entities,
            captured_start: None,
            layer: None,
        }
    }

    /// Put the whole batch on `layer` instead of the current layer.
    pub fn on_layer(mut self, layer: LayerId) -> Self {
        self.layer = Some(layer);
        self
    }
}

impl Command for CreateEntities {
    fn do_(&mut self, doc: &mut Document) {
        // Re-capture on redo: clear any prior start so LIFO truncate is safe.
        self.captured_start = Some(doc.entities.len());
        let layer = *self.layer.get_or_insert(doc.current_layer());
        for &entity in &self.entities {
            doc.push_entity(entity, layer);
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(start) = self.captured_start.take() {
            doc.truncate_entities(start);
        }
    }

    fn label(&self) -> &str {
        "Place Text"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Vec2;
    use core::f64::consts::FRAC_PI_2;

    /// AC#1 — the three commands are reachable from
    /// `crate::document::commands::{CreateLine, CreateCircle, CreateArc}`.
    #[test]
    fn create_module_re_exports_three_commands() {
        use crate::document::commands::{CreateArc, CreateCircle, CreateLine};
        let _ = CreateLine::new(Line::new(Vec2::default(), Vec2::default()));
        let _ = CreateCircle::new(Circle::new(Vec2::default(), 1.0));
        let _ = CreateArc::new(Arc::new(Vec2::default(), 1.0, 0.0, 1.0, true));
    }

    /// AC#2 — constructors start with `captured_index: None`. Inferred by
    /// calling `undo` on a freshly-constructed command and observing no
    /// document mutation (`Option::take` on `None` is a no-op).
    #[test]
    fn create_constructors_capture_index_none() {
        let mut doc = Document::default();
        let seed = Line::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0));
        doc.push_current(Entity::Line(seed));

        let mut cmd_line = CreateLine::new(Line::new(Vec2::default(), Vec2::new(1.0, 0.0)));
        let mut cmd_circle = CreateCircle::new(Circle::new(Vec2::default(), 1.0));
        let mut cmd_arc = CreateArc::new(Arc::new(Vec2::default(), 1.0, 0.0, FRAC_PI_2, true));
        cmd_line.undo(&mut doc);
        cmd_circle.undo(&mut doc);
        cmd_arc.undo(&mut doc);

        assert_eq!(doc.entities.len(), 1);
        assert_eq!(doc.entities[0], Entity::Line(seed));
    }

    /// AC#3 — `CreateLine` round-trip on an empty document.
    #[test]
    fn create_line_roundtrip() {
        let mut doc = Document::default();
        let line = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        let mut cmd = CreateLine::new(line);
        cmd.do_(&mut doc);
        assert_eq!(doc.entities.len(), 1);
        assert_eq!(doc.entities[0], Entity::Line(line));
        cmd.undo(&mut doc);
        assert!(doc.entities.is_empty());
        assert_eq!(doc.entity_count(), 0);
    }

    /// AC#4 — `CreateCircle` round-trip on an empty document.
    #[test]
    fn create_circle_roundtrip() {
        let mut doc = Document::default();
        let circle = Circle::new(Vec2::new(5.0, 5.0), 3.0);
        let mut cmd = CreateCircle::new(circle);
        cmd.do_(&mut doc);
        assert_eq!(doc.entities.len(), 1);
        assert_eq!(doc.entities[0], Entity::Circle(circle));
        cmd.undo(&mut doc);
        assert!(doc.entities.is_empty());
        assert_eq!(doc.entity_count(), 0);
    }

    /// AC#5 — `CreateArc` round-trip with the canonical quarter arc.
    #[test]
    fn create_arc_roundtrip() {
        let mut doc = Document::default();
        let arc = Arc::new(Vec2::default(), 1.0, 0.0, FRAC_PI_2, true);
        let mut cmd = CreateArc::new(arc);
        cmd.do_(&mut doc);
        assert_eq!(doc.entities.len(), 1);
        assert_eq!(doc.entities[0], Entity::Arc(arc));
        cmd.undo(&mut doc);
        assert!(doc.entities.is_empty());
        assert_eq!(doc.entity_count(), 0);
    }

    /// AC#6 — two `CreateLine`s push to the end in order; LIFO undo clears.
    #[test]
    fn two_creates_then_lifo_undo() {
        let mut doc = Document::default();
        let line_a = Line::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
        let line_b = Line::new(Vec2::new(2.0, 0.0), Vec2::new(3.0, 0.0));
        let mut cmd_a = CreateLine::new(line_a);
        let mut cmd_b = CreateLine::new(line_b);

        cmd_a.do_(&mut doc);
        cmd_b.do_(&mut doc);
        assert_eq!(doc.entities.len(), 2);
        assert_eq!(doc.entities[0], Entity::Line(line_a));
        assert_eq!(doc.entities[1], Entity::Line(line_b));

        cmd_b.undo(&mut doc);
        cmd_a.undo(&mut doc);
        assert!(doc.entities.is_empty());
        assert_eq!(doc.entity_count(), 0);
    }

    /// AC#7 — exact label strings.
    #[test]
    fn create_labels_exact() {
        let cmd_line = CreateLine::new(Line::new(Vec2::default(), Vec2::new(1.0, 0.0)));
        let cmd_circle = CreateCircle::new(Circle::new(Vec2::default(), 1.0));
        let cmd_arc = CreateArc::new(Arc::new(Vec2::default(), 1.0, 0.0, FRAC_PI_2, true));
        let cmd_batch = CreateEntities::new(vec![]);
        assert_eq!(cmd_line.label(), "Create Line");
        assert_eq!(cmd_circle.label(), "Create Circle");
        assert_eq!(cmd_arc.label(), "Create Arc");
        assert_eq!(cmd_batch.label(), "Place Text");
    }

    /// AC#8 — every create command is object-safe.
    #[test]
    fn create_commands_are_object_safe() {
        let _: Box<dyn Command> =
            Box::new(CreateLine::new(Line::new(Vec2::default(), Vec2::default())));
        let _: Box<dyn Command> = Box::new(CreateCircle::new(Circle::new(Vec2::default(), 1.0)));
        let _: Box<dyn Command> = Box::new(CreateArc::new(Arc::new(
            Vec2::default(),
            1.0,
            0.0,
            1.0,
            true,
        )));
        let _: Box<dyn Command> = Box::new(CreateEntities::new(vec![]));
    }

    /// `captured_index.take()` makes a double `undo` a no-op rather than a
    /// corruption that would mis-target a neighbouring entity.
    #[test]
    fn double_undo_is_a_noop() {
        let mut doc = Document::default();
        let seed = Line::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0));
        doc.push_current(Entity::Line(seed));
        let line = Line::new(Vec2::new(2.0, 0.0), Vec2::new(3.0, 0.0));
        let mut cmd = CreateLine::new(line);

        cmd.do_(&mut doc);
        assert_eq!(doc.entities.len(), 2);
        cmd.undo(&mut doc);
        assert_eq!(doc.entities.len(), 1);
        cmd.undo(&mut doc);
        assert_eq!(doc.entities.len(), 1);
        assert_eq!(doc.entities[0], Entity::Line(seed));
    }

    // ── CreateEntities tests ────────────────────────────────────────────

    /// LCV-045 — empty batch is a no-op for both `do_` and `undo`.
    #[test]
    fn create_entities_empty_batch_noop() {
        let mut doc = Document::default();
        let mut cmd = CreateEntities::new(vec![]);
        cmd.do_(&mut doc);
        assert_eq!(doc.entity_count(), 0);
        cmd.undo(&mut doc);
        assert_eq!(doc.entity_count(), 0);
    }

    /// LCV-045 — two-entity round-trip: `do_` appends both; `undo` removes both.
    #[test]
    fn create_entities_two_entity_roundtrip() {
        let mut doc = Document::default();
        let line_a = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        let line_b = Line::new(Vec2::new(10.0, 0.0), Vec2::new(10.0, 5.0));
        let mut cmd = CreateEntities::new(vec![Entity::Line(line_a), Entity::Line(line_b)]);

        cmd.do_(&mut doc);
        assert_eq!(doc.entity_count(), 2);
        assert_eq!(doc.entities[0], Entity::Line(line_a));
        assert_eq!(doc.entities[1], Entity::Line(line_b));

        cmd.undo(&mut doc);
        assert_eq!(doc.entity_count(), 0);
    }

    /// LCV-045 — existing entities before the batch are preserved on undo.
    #[test]
    fn create_entities_preserves_prior_entities_on_undo() {
        let mut doc = Document::default();
        let seed = Line::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
        doc.push_current(Entity::Line(seed));

        let new_line = Line::new(Vec2::new(5.0, 0.0), Vec2::new(6.0, 0.0));
        let mut cmd = CreateEntities::new(vec![Entity::Line(new_line)]);

        cmd.do_(&mut doc);
        assert_eq!(doc.entity_count(), 2);

        cmd.undo(&mut doc);
        assert_eq!(doc.entity_count(), 1);
        assert_eq!(doc.entities[0], Entity::Line(seed));
    }

    /// LCV-045 — double undo is a no-op (captured_start is cleared by take).
    #[test]
    fn create_entities_double_undo_is_noop() {
        let mut doc = Document::default();
        let seed = Line::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
        doc.push_current(Entity::Line(seed));

        let extra = Line::new(Vec2::new(2.0, 0.0), Vec2::new(3.0, 0.0));
        let mut cmd = CreateEntities::new(vec![Entity::Line(extra)]);

        cmd.do_(&mut doc);
        assert_eq!(doc.entity_count(), 2);
        cmd.undo(&mut doc);
        assert_eq!(doc.entity_count(), 1);
        cmd.undo(&mut doc); // second undo must be a no-op
        assert_eq!(doc.entity_count(), 1);
        assert_eq!(doc.entities[0], Entity::Line(seed));
    }

    /// LCV-045 — redo (second `do_`) re-appends all entities.
    #[test]
    fn create_entities_redo_reappends() {
        let mut doc = Document::default();
        let line = Line::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
        let mut cmd = CreateEntities::new(vec![Entity::Line(line)]);

        cmd.do_(&mut doc);
        assert_eq!(doc.entity_count(), 1);
        cmd.undo(&mut doc);
        assert_eq!(doc.entity_count(), 0);
        cmd.do_(&mut doc); // redo
        assert_eq!(doc.entity_count(), 1);
        assert_eq!(doc.entities[0], Entity::Line(line));
    }

    // ── LCV-048 named acceptance-criterion tests ─────────────────────────

    /// LCV-048 AC#1 — constructor is reachable; starts with no captured start.
    #[test]
    fn create_entities_constructor_reachable() {
        let _ = CreateEntities::new(vec![]);
        let l = Line::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
        let _ = CreateEntities::new(vec![Entity::Line(l)]);
    }

    /// LCV-048 AC#2 — two-line do/undo round-trip.
    #[test]
    fn create_entities_roundtrip_two_lines() {
        let mut doc = Document::default();
        let l1 = Line::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
        let l2 = Line::new(Vec2::new(2.0, 0.0), Vec2::new(3.0, 0.0));
        let mut cmd = CreateEntities::new(vec![Entity::Line(l1), Entity::Line(l2)]);
        cmd.do_(&mut doc);
        assert_eq!(doc.entities.len(), 2);
        assert_eq!(doc.entities[0], Entity::Line(l1));
        assert_eq!(doc.entities[1], Entity::Line(l2));
        cmd.undo(&mut doc);
        assert!(doc.entities.is_empty());
    }

    /// LCV-048 AC#3 — undo preserves pre-existing entities.
    #[test]
    fn create_entities_undo_does_not_remove_preexisting() {
        let mut doc = Document::default();
        let seed = Line::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
        doc.push_current(Entity::Line(seed));
        let l1 = Line::new(Vec2::new(2.0, 0.0), Vec2::new(3.0, 0.0));
        let l2 = Line::new(Vec2::new(4.0, 0.0), Vec2::new(5.0, 0.0));
        let mut cmd = CreateEntities::new(vec![Entity::Line(l1), Entity::Line(l2)]);
        cmd.do_(&mut doc);
        assert_eq!(doc.entity_count(), 3);
        cmd.undo(&mut doc);
        assert_eq!(doc.entity_count(), 1);
        assert_eq!(doc.entities[0], Entity::Line(seed));
    }

    /// LCV-048 AC#4 — double undo is a no-op (no panic, no corruption).
    #[test]
    fn create_entities_double_undo_is_noop_lcv048() {
        let mut doc = Document::default();
        let l = Line::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
        let mut cmd = CreateEntities::new(vec![Entity::Line(l)]);
        cmd.do_(&mut doc);
        assert_eq!(doc.entity_count(), 1);
        cmd.undo(&mut doc);
        assert_eq!(doc.entity_count(), 0);
        cmd.undo(&mut doc); // second undo — must not panic or corrupt
        assert_eq!(doc.entity_count(), 0);
    }

    /// LCV-048 AC#5 — label is the exact string `"Place Text"`.
    #[test]
    fn create_entities_label_exact() {
        assert_eq!(CreateEntities::new(vec![]).label(), "Place Text");
    }

    /// LCV-048 AC#6 — `CreateEntities` is object-safe.
    #[test]
    fn create_entities_object_safe() {
        let _: Box<dyn Command> = Box::new(CreateEntities::new(vec![]));
    }
}
