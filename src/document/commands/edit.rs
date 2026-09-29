//! Editing commands: [`DeleteEntities`] and [`MoveEntities`]. Back the
//! `DeleteTool` (LCV-052), `MoveTool` (LCV-049), and the agent CAD registry
//! (LCV-078).
//!
//! `DeleteEntities::do_` sorts `indices` descending so each `Vec::remove(i)`
//! does not invalidate later indices, capturing `(i, entity)` pairs in that
//! order. `undo` `pop`s the capture (ascending order) and `Vec::insert(i, e)`s
//! each back into place — low first means higher slots are not shifted. After
//! undo the capture is empty, so a double-undo is a no-op.
//!
//! `MoveEntities::do_` translates each indexed entity by `self.delta`; `undo`
//! by `-self.delta`. Pure `f64` addition is exactly invertible, so the
//! round-trip is bit-stable within `EPSILON`.
//!
//! Out-of-range indices are an invariant violation; `debug_assert!` traps it
//! in debug. Callers (selection-driven tools) build `indices` from valid
//! entities only.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`. Introduced by demand LCV-024.

use super::Command;
use crate::document::{Document, Entity, LayerId};
use crate::geometry::Vec2;

/// Remove a set of entities from a [`Document`] and remember them for undo.
/// Empty `indices` is a valid no-op for both `do_` and `undo`.
#[derive(Debug)]
pub struct DeleteEntities {
    /// Indices into [`Document::entities`] to remove. Caller ensures each
    /// index is in range and unique.
    pub indices: Vec<usize>,
    /// Captured `(original_index, entity, layer)` triples in removal order (descending
    /// by original index). `undo` drains via `pop`, yielding ascending order.
    captured_entities: Vec<(usize, Entity, LayerId)>,
}

impl DeleteEntities {
    /// Build a [`DeleteEntities`] for the given `indices`.
    pub fn new(indices: Vec<usize>) -> Self {
        Self {
            indices,
            captured_entities: Vec::new(),
        }
    }
}

impl Command for DeleteEntities {
    fn do_(&mut self, doc: &mut Document) {
        // Clear any prior capture so redo does not double-capture.
        self.captured_entities.clear();
        // Sort descending so `Vec::remove(i)` does not shift later indices.
        let mut sorted: Vec<usize> = self.indices.clone();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        for i in sorted {
            debug_assert!(i < doc.entities.len(), "DeleteEntities: index OOR");
            let (removed, layer) = doc.remove_entity(i);
            self.captured_entities.push((i, removed, layer));
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        // `pop` yields ascending original-index order — the right order for
        // `Vec::insert(i, e)`. After the loop the capture is empty.
        while let Some((i, entity, layer)) = self.captured_entities.pop() {
            doc.insert_entity(i, entity, layer);
        }
    }

    fn label(&self) -> &str {
        "Delete Entities"
    }
}

/// Translate a set of entities in a [`Document`] by a fixed `delta` (mm).
/// Empty `indices` is a valid no-op for both `do_` and `undo`.
#[derive(Debug)]
pub struct MoveEntities {
    /// Indices into [`Document::entities`] to translate. Caller ensures each
    /// index is in range and unique (a duplicated index translates twice).
    pub indices: Vec<usize>,
    /// Translation vector in mm.
    pub delta: Vec2,
}

impl MoveEntities {
    /// Build a [`MoveEntities`] that translates `indices` by `delta`.
    pub fn new(indices: Vec<usize>, delta: Vec2) -> Self {
        Self { indices, delta }
    }
}

impl Command for MoveEntities {
    fn do_(&mut self, doc: &mut Document) {
        for &i in &self.indices {
            debug_assert!(i < doc.entities.len(), "MoveEntities: index OOR");
            doc.entities[i].translate(self.delta);
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        let inverse = -self.delta;
        for &i in &self.indices {
            debug_assert!(i < doc.entities.len(), "MoveEntities: index OOR");
            doc.entities[i].translate(inverse);
        }
    }

    fn label(&self) -> &str {
        "Move Entities"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Arc, Circle, Line, EPSILON};
    use core::f64::consts::FRAC_PI_2;

    fn three_entity_doc() -> (Document, Line, Circle, Arc) {
        let (la, cb, ac) = (
            unit_line(),
            Circle::new(Vec2::new(5.0, 5.0), 2.0),
            Arc::new(Vec2::new(10.0, 0.0), 1.5, 0.0, FRAC_PI_2, true),
        );
        let doc = doc_with(vec![Entity::Line(la), Entity::Circle(cb), Entity::Arc(ac)]);
        (doc, la, cb, ac)
    }
    fn line_at(e: &Entity) -> Line {
        match e {
            Entity::Line(l) => *l,
            _ => panic!(),
        }
    }
    fn circle_at(e: &Entity) -> Circle {
        match e {
            Entity::Circle(c) => *c,
            _ => panic!(),
        }
    }
    fn doc_with(entities: Vec<Entity>) -> Document {
        let mut doc = Document::default();
        entities.into_iter().for_each(|e| doc.push_current(e));
        doc
    }
    fn unit_line() -> Line {
        Line::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0))
    }

    /// AC#1 — constructors reachable with documented signatures.
    #[test]
    fn edit_module_defines_delete_and_move() {
        let _ = DeleteEntities::new(vec![0, 1]);
        let _ = MoveEntities::new(vec![0], Vec2::new(1.0, 2.0));
    }

    /// AC#2 — both types re-exported via `commands::{...}`.
    #[test]
    fn commands_re_exports_delete_and_move() {
        use crate::document::commands::{DeleteEntities, MoveEntities};
        let _ = DeleteEntities::new(vec![0]);
        let _ = MoveEntities::new(vec![0], Vec2::default());
    }

    /// AC#4 — delete-then-undo on a 3-entity document restores all three.
    #[test]
    fn delete_all_three_then_undo_restores_order() {
        let (mut doc, line_a, circle_b, arc_c) = three_entity_doc();
        let mut cmd = DeleteEntities::new(vec![0, 1, 2]);
        cmd.do_(&mut doc);
        assert_eq!(doc.entity_count(), 0);
        cmd.undo(&mut doc);
        assert_eq!(doc.entity_count(), 3);
        assert_eq!(doc.entities[0], Entity::Line(line_a));
        assert_eq!(doc.entities[1], Entity::Circle(circle_b));
        assert_eq!(doc.entities[2], Entity::Arc(arc_c));
    }

    /// AC#5 — non-contiguous indices [0, 2] from [a, b, c] remove a and c
    /// (the canonical "leaves middle entity alone" case); undo restores all
    /// three at their original indices.
    #[test]
    fn delete_non_contiguous_indices_then_undo() {
        let (mut doc, line_a, circle_b, arc_c) = three_entity_doc();
        let mut cmd = DeleteEntities::new(vec![0, 2]);
        cmd.do_(&mut doc);
        assert_eq!(doc.entity_count(), 1);
        assert_eq!(doc.entities[0], Entity::Circle(circle_b));
        cmd.undo(&mut doc);
        assert_eq!(doc.entity_count(), 3);
        assert_eq!(doc.entities[0], Entity::Line(line_a));
        assert_eq!(doc.entities[1], Entity::Circle(circle_b));
        assert_eq!(doc.entities[2], Entity::Arc(arc_c));
    }

    /// AC#6 — empty-indices delete is a no-op for both `do_` and `undo`.
    #[test]
    fn delete_empty_indices_is_noop() {
        let line_a = unit_line();
        let circle_b = Circle::new(Vec2::new(5.0, 5.0), 2.0);
        let mut doc = doc_with(vec![Entity::Line(line_a), Entity::Circle(circle_b)]);

        let mut cmd = DeleteEntities::new(Vec::new());
        cmd.do_(&mut doc);
        assert_eq!(doc.entity_count(), 2);
        cmd.undo(&mut doc);
        assert_eq!(
            doc.entities,
            vec![Entity::Line(line_a), Entity::Circle(circle_b)]
        );
    }

    /// AC#7 — move round-trip on a single line, within `EPSILON`.
    #[test]
    fn move_line_roundtrip() {
        let line = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        let mut doc = doc_with(vec![Entity::Line(line)]);

        let mut cmd = MoveEntities::new(vec![0], Vec2::new(3.0, 4.0));
        cmd.do_(&mut doc);
        let after = line_at(&doc.entities[0]);
        assert!(after.p1.approx_eq(Vec2::new(3.0, 4.0), EPSILON));
        assert!(after.p2.approx_eq(Vec2::new(13.0, 4.0), EPSILON));

        cmd.undo(&mut doc);
        let restored = line_at(&doc.entities[0]);
        assert!(restored.p1.approx_eq(line.p1, EPSILON));
        assert!(restored.p2.approx_eq(line.p2, EPSILON));
    }

    /// AC#8 — move applies to multiple entities; undo restores each.
    #[test]
    fn move_multiple_entities_roundtrip() {
        let line = unit_line();
        let circle = Circle::new(Vec2::new(10.0, 10.0), 2.0);
        let mut doc = doc_with(vec![Entity::Line(line), Entity::Circle(circle)]);

        let mut cmd = MoveEntities::new(vec![0, 1], Vec2::new(5.0, 0.0));
        cmd.do_(&mut doc);
        let l = line_at(&doc.entities[0]);
        let c = circle_at(&doc.entities[1]);
        assert!(l.p1.approx_eq(Vec2::new(5.0, 0.0), EPSILON));
        assert!(l.p2.approx_eq(Vec2::new(6.0, 0.0), EPSILON));
        assert!(c.center.approx_eq(Vec2::new(15.0, 10.0), EPSILON));
        assert!((c.r - 2.0).abs() < EPSILON);

        cmd.undo(&mut doc);
        let l = line_at(&doc.entities[0]);
        let c = circle_at(&doc.entities[1]);
        assert!(l.p1.approx_eq(line.p1, EPSILON));
        assert!(l.p2.approx_eq(line.p2, EPSILON));
        assert!(c.center.approx_eq(circle.center, EPSILON));
        assert!((c.r - circle.r).abs() < EPSILON);
    }

    /// AC#9 — empty-indices move is a no-op.
    #[test]
    fn move_empty_indices_is_noop() {
        let line = unit_line();
        let mut doc = doc_with(vec![Entity::Line(line)]);
        let mut cmd = MoveEntities::new(Vec::new(), Vec2::new(99.0, 99.0));
        cmd.do_(&mut doc);
        assert_eq!(doc.entities[0], Entity::Line(line));
        cmd.undo(&mut doc);
        assert_eq!(doc.entities[0], Entity::Line(line));
    }

    /// AC#10 — exact label strings.
    #[test]
    fn edit_labels_exact() {
        assert_eq!(DeleteEntities::new(vec![0]).label(), "Delete Entities");
        assert_eq!(
            MoveEntities::new(vec![0], Vec2::default()).label(),
            "Move Entities"
        );
    }

    /// AC#11 — both commands are object-safe (`Box<dyn Command>`).
    #[test]
    fn edit_commands_are_object_safe() {
        let _: Box<dyn Command> = Box::new(DeleteEntities::new(vec![0]));
        let _: Box<dyn Command> = Box::new(MoveEntities::new(vec![0], Vec2::default()));
    }

    /// Double-undo is a no-op: the second undo finds nothing to insert.
    #[test]
    fn delete_double_undo_is_a_noop() {
        let (mut doc, line_a, circle_b, arc_c) = three_entity_doc();
        let mut cmd = DeleteEntities::new(vec![0, 1, 2]);
        cmd.do_(&mut doc);
        cmd.undo(&mut doc);
        assert_eq!(doc.entity_count(), 3);
        cmd.undo(&mut doc);
        assert_eq!(doc.entity_count(), 3);
        assert_eq!(doc.entities[0], Entity::Line(line_a));
        assert_eq!(doc.entities[1], Entity::Circle(circle_b));
        assert_eq!(doc.entities[2], Entity::Arc(arc_c));
    }
}
