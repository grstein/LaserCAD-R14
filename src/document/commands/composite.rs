//! [`CompositeCommand`] — *n* commands that undo as one (LCV-122).
//!
//! One agent turn may commit hundreds of actions; reversing that sentence must
//! cost one `Ctrl+Z`, not hundreds. ADR 0007 §D12 settles *how*: each action
//! still applies individually and immediately — the operator watches the
//! geometry appear — into a flat group held beside the undo stack, and
//! [`History::end_group`](crate::document::History::end_group) seals that
//! group into one of these. Nothing is re-run and the document does not
//! change, so the revision counter does not move either.
//!
//! The load-bearing detail is **order**: `do_` runs the children forward and
//! `undo` runs them in reverse. A sequence like *create A, delete A* only
//! round-trips if the delete is undone (re-inserting A) before the create is
//! (removing it again); undoing forward would restore the wrong entity, or the
//! right one at the wrong index.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use super::Command;
use crate::document::Document;

/// A [`Command`] that runs a list of child commands as a single history entry.
///
/// Round-trip invariant (`src/document/commands/mod.rs` §"Round-trip
/// invariant"): `do_` then `undo` restores the pre-`do_` document exactly, and
/// so does a `do_ → undo → do_ → undo` cycle, provided every child satisfies
/// it — which is what makes redo (a second `do_`) safe.
pub struct CompositeCommand {
    /// The children, in the order they were originally committed.
    commands: Vec<Box<dyn Command>>,
    /// What the history-aware UI shows for this entry, e.g.
    /// `"Agent: draw a square"`.
    label: String,
}

impl CompositeCommand {
    /// Wrap `commands` — **in the order they were committed** — under `label`.
    ///
    /// The caller owns the ordering contract: `commands[0]` must be the
    /// oldest. `History::end_group` is the only production caller and it
    /// preserves commit order by construction.
    pub fn new(commands: Vec<Box<dyn Command>>, label: impl Into<String>) -> Self {
        Self {
            commands,
            label: label.into(),
        }
    }
}

impl Command for CompositeCommand {
    /// Run every child forward, oldest first — the order they were committed.
    fn do_(&mut self, doc: &mut Document) {
        for command in self.commands.iter_mut() {
            command.do_(doc);
        }
    }

    /// Undo every child **in reverse**, newest first. See the module docs for
    /// why forward order corrupts a create/delete pair.
    fn undo(&mut self, doc: &mut Document) {
        for command in self.commands.iter_mut().rev() {
            command.undo(doc);
        }
    }

    fn label(&self) -> &str {
        &self.label
    }
}

impl std::fmt::Debug for CompositeCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CompositeCommand {{ label: {:?}, commands: {} }}",
            self.label,
            self.commands.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{CreateCircle, CreateLine, DeleteEntities, Entity};
    use crate::geometry::{Circle, Line, Vec2};

    fn line(y: f64) -> Line {
        Line::new(Vec2::new(0.0, y), Vec2::new(10.0, y))
    }

    fn circle() -> Circle {
        Circle::new(Vec2::new(5.0, 5.0), 2.0)
    }

    /// A pre-existing entity, so a composite that clobbers the document
    /// around itself is visible.
    fn doc_with_one_line() -> Document {
        let mut doc = Document::default();
        doc.entities.push(Entity::Line(line(99.0)));
        doc
    }

    /// The order-sensitive three-command fixture AC 11 asks for: create,
    /// create, then delete the entity that was already there. Undone in
    /// reverse it round-trips; undone forward it removes the wrong index.
    fn create_create_delete() -> Vec<Box<dyn Command>> {
        vec![
            Box::new(CreateLine::new(line(0.0))),
            Box::new(CreateCircle::new(circle())),
            Box::new(DeleteEntities::new(vec![0])),
        ]
    }

    /// AC 11 — `label()` returns the stored label verbatim.
    #[test]
    fn composite_label_is_the_stored_string() {
        let composite = CompositeCommand::new(vec![], "Agent: draw a square");
        assert_eq!(composite.label(), "Agent: draw a square");
    }

    /// AC 11 — `do_` runs the children **forward**. Run in reverse the same
    /// three commands leave `[circle, line]`; forward they leave
    /// `[line, circle]`, so this pins the direction rather than the contents.
    #[test]
    fn composite_do_runs_children_forward() {
        let mut doc = doc_with_one_line();
        let mut composite = CompositeCommand::new(create_create_delete(), "three");
        composite.do_(&mut doc);
        assert_eq!(
            doc.entities,
            vec![Entity::Line(line(0.0)), Entity::Circle(circle())],
        );
    }

    /// AC 11 — the round-trip invariant on the order-sensitive sequence, run
    /// twice (`do_ → undo → do_ → undo`) because redo calls `do_` again.
    ///
    /// This is the test mutation (a) has to break: with `undo` running forward
    /// the second child's captured index is past the end of the document and
    /// the removal panics, which fails this test by name.
    #[test]
    fn composite_roundtrip_is_order_sensitive_and_repeatable() {
        let mut doc = doc_with_one_line();
        let before = doc.entities.clone();

        let mut composite = CompositeCommand::new(create_create_delete(), "three");
        for cycle in 0..2 {
            composite.do_(&mut doc);
            assert_eq!(
                doc.entities,
                vec![Entity::Line(line(0.0)), Entity::Circle(circle())],
                "cycle {cycle}: forward order"
            );
            composite.undo(&mut doc);
            assert_eq!(doc.entities, before, "cycle {cycle}: round-trip");
        }
    }

    /// AC 11's demonstration: undone **forward**, a delete/create sequence
    /// does not restore the document. Run against a local forward-order
    /// runner over the same children, so the assertion is about the order
    /// itself and not about `CompositeCommand`'s contents.
    ///
    /// The fixture deletes first so the forward undo produces a wrong document
    /// rather than an out-of-range panic — a silent corruption is the failure
    /// mode worth exhibiting.
    #[test]
    fn undoing_forward_would_corrupt_the_document() {
        let mut doc = doc_with_one_line();
        let before = doc.entities.clone();

        let mut children: Vec<Box<dyn Command>> = vec![
            Box::new(DeleteEntities::new(vec![0])),
            Box::new(CreateLine::new(line(0.0))),
            Box::new(CreateCircle::new(circle())),
        ];
        for command in children.iter_mut() {
            command.do_(&mut doc);
        }
        // The mutant: undo oldest-first instead of newest-first.
        for command in children.iter_mut() {
            command.undo(&mut doc);
        }
        assert_ne!(
            doc.entities, before,
            "a forward-order undo must not round-trip, or AC 11 pins nothing"
        );
    }

    /// A single-child composite is still a valid command — `end_group`
    /// refuses to build one, but nothing here depends on that.
    #[test]
    fn composite_of_one_roundtrips() {
        let mut doc = Document::default();
        let mut composite =
            CompositeCommand::new(vec![Box::new(CreateLine::new(line(1.0)))], "one");
        composite.do_(&mut doc);
        assert_eq!(doc.entities.len(), 1);
        composite.undo(&mut doc);
        assert!(doc.entities.is_empty());
    }

    /// An empty composite is a no-op in both directions.
    #[test]
    fn empty_composite_is_a_noop() {
        let mut doc = Document::default();
        doc.entities.push(Entity::Line(line(0.0)));
        let mut composite = CompositeCommand::new(vec![], "none");
        composite.do_(&mut doc);
        composite.undo(&mut doc);
        assert_eq!(doc.entities.len(), 1);
    }

    /// `Debug` prints the label and the child count without requiring
    /// `Box<dyn Command>: Debug`.
    #[test]
    fn composite_debug_names_label_and_count() {
        let composite = CompositeCommand::new(create_create_delete(), "three");
        let rendered = format!("{composite:?}");
        assert!(
            rendered.contains("three") && rendered.contains('3'),
            "{rendered}"
        );
    }
}
