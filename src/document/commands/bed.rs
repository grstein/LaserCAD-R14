//! The [`SetBedSize`] command: change [`Document::bed_mm`] through the history
//! stack (LCV-114).
//!
//! The bed is document state, so AGENTS.md §"State and mutation" applies to it
//! exactly as it applies to entities: the Bed size… dialog builds this command
//! and commits it, which both gives the operator Ctrl+Z after a fat-fingered
//! `1300` and dirties the document through the one existing path
//! (`History::revision()`, ADR 0002 §B) instead of adding a second writer.
//!
//! Entities and selection are untouched in both directions: changing the bed
//! moves the frame, never the drawing (LCV-114 §Out of scope).
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use super::Command;
use crate::document::Document;

/// Replace [`Document::bed_mm`], remembering the previous value for undo.
///
/// The caller is responsible for having run the new size through
/// [`clamp_bed_mm`](crate::util::clamp_bed_mm): the command stores whatever it
/// is given, because an import that must be rejected is rejected before it
/// reaches here (LCV-114 AC 9).
#[derive(Debug)]
pub struct SetBedSize {
    /// The bed size to install, `[width, height]` in millimetres.
    new_bed_mm: [f64; 2],
    /// The bed size observed at `do_` time, restored by `undo`. `None` before
    /// the first `do_`, which makes `undo` a no-op — the same double-undo
    /// safety the other commands carry.
    previous_bed_mm: Option<[f64; 2]>,
}

impl SetBedSize {
    /// Build a [`SetBedSize`] that installs `new_bed_mm`.
    pub fn new(new_bed_mm: [f64; 2]) -> Self {
        Self {
            new_bed_mm,
            previous_bed_mm: None,
        }
    }

    /// The bed size this command installs.
    pub fn new_bed_mm(&self) -> [f64; 2] {
        self.new_bed_mm
    }
}

impl Command for SetBedSize {
    fn do_(&mut self, doc: &mut Document) {
        self.previous_bed_mm = Some(doc.bed_mm);
        doc.bed_mm = self.new_bed_mm;
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(previous) = self.previous_bed_mm.take() {
            doc.bed_mm = previous;
        }
    }

    fn label(&self) -> &str {
        "Set Bed Size"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Entity;
    use crate::geometry::{Line, Vec2};

    fn doc_with_a_line() -> Document {
        let mut doc = Document::default();
        doc.entities.push(Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
        )));
        doc
    }

    /// LCV-114 AC 12 — do_ installs the new bed, undo restores the old one,
    /// and the entity count is unchanged in both directions.
    #[test]
    fn set_bed_size_round_trips_through_undo() {
        let mut doc = doc_with_a_line();
        let before = doc.bed_mm;
        let mut cmd = SetBedSize::new([300.0, 180.0]);

        cmd.do_(&mut doc);
        assert_eq!(doc.bed_mm, [300.0, 180.0]);
        assert_eq!(doc.entity_count(), 1);

        cmd.undo(&mut doc);
        assert_eq!(doc.bed_mm, before);
        assert_eq!(doc.entity_count(), 1);
    }

    /// LCV-114 AC 12 — redo works: `do_` after `undo` re-installs the bed,
    /// which is how [`crate::document::History::redo`] replays a command.
    #[test]
    fn set_bed_size_survives_a_redo_cycle() {
        let mut doc = Document::default();
        let mut cmd = SetBedSize::new([128.0, 128.0]);
        cmd.do_(&mut doc);
        cmd.undo(&mut doc);
        assert_eq!(doc.bed_mm, [400.0, 400.0]);
        cmd.do_(&mut doc);
        assert_eq!(doc.bed_mm, [128.0, 128.0]);
    }

    /// A second `undo` without an intervening `do_` is a no-op rather than a
    /// re-application of a stale capture.
    #[test]
    fn double_undo_is_a_no_op() {
        let mut doc = Document::default();
        let mut cmd = SetBedSize::new([300.0, 180.0]);
        cmd.do_(&mut doc);
        cmd.undo(&mut doc);
        cmd.undo(&mut doc);
        assert_eq!(doc.bed_mm, [400.0, 400.0]);
    }

    /// LCV-114 AC 12 — the selection is untouched in both directions.
    #[test]
    fn set_bed_size_leaves_the_selection_alone() {
        let mut doc = doc_with_a_line();
        doc.selection.set(0..1);
        let selected: Vec<usize> = doc.selection.iter().collect();

        let mut cmd = SetBedSize::new([300.0, 180.0]);
        cmd.do_(&mut doc);
        assert_eq!(doc.selection.iter().collect::<Vec<_>>(), selected);
        cmd.undo(&mut doc);
        assert_eq!(doc.selection.iter().collect::<Vec<_>>(), selected);
    }

    /// The label is what `Edit > Undo` shows.
    #[test]
    fn label_is_set_bed_size() {
        assert_eq!(SetBedSize::new([1.0, 1.0]).label(), "Set Bed Size");
    }
}
