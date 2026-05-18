//! [`SelectionCommand`]: change [`Document::selection`] through the undo
//! stack so a Ctrl+Z after a window-pick restores the previous selection.
//!
//! `do_` captures the document's current selection set, swaps in
//! `new_indices`, and stashes the previous set in `captured_old`. `undo`
//! restores the captured set into the document. A double-undo is a no-op
//! because `captured_old` is consumed by [`Option::take`]; the next `do_`
//! re-captures whatever the document holds at that moment, which makes the
//! command safe to drive through redo as well (LCV-026's redo replays
//! `do_`).
//!
//! Selection sizes are small (typically far below 100 indices), so cloning
//! the inbound `HashSet` on every `do_` is cheap and keeps the implementation
//! plain.
//!
//! Per [`super::Command`], `cmd.do_(doc); cmd.undo(doc);` leaves the document
//! `PartialEq`-equal to its pre-`do_` state — including the selection.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`. Introduced by demand LCV-027.
//!
//! [`Document::selection`]: crate::document::Document::selection

use std::collections::HashSet;

use super::Command;
use crate::document::Document;

/// Replace the document's selection atomically. The previous selection set
/// is captured inside `do_` so [`SelectionCommand::undo`] can restore it.
///
/// Construction does not look at the document; the new set is provided up
/// front. Capture happens at `do_` time so the same command instance is
/// safe to commit through [`crate::document::History`] (which calls `do_`
/// once on commit and again on redo).
#[derive(Debug)]
pub struct SelectionCommand {
    /// The selection to install when `do_` runs.
    new_indices: HashSet<usize>,
    /// The selection observed at `do_` time, restored by `undo`. `None`
    /// before the first `do_` and after `undo` consumes it via `take`.
    captured_old: Option<HashSet<usize>>,
}

impl SelectionCommand {
    /// Build a command that, when `do_`'d, replaces the document's selection
    /// with `new_indices`. Duplicates in the input collapse (it's a set).
    ///
    /// Pass an empty iterator to clear the selection through the undo stack:
    /// `SelectionCommand::new(Vec::<usize>::new())`.
    pub fn new(new_indices: impl IntoIterator<Item = usize>) -> Self {
        Self {
            new_indices: new_indices.into_iter().collect(),
            captured_old: None,
        }
    }
}

impl Command for SelectionCommand {
    fn do_(&mut self, doc: &mut Document) {
        let prev = doc.selection.replace_indices(self.new_indices.clone());
        self.captured_old = Some(prev);
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(prev) = self.captured_old.take() {
            doc.selection.replace_indices(prev);
        }
    }

    fn label(&self) -> &str {
        "Select"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(iter: impl IntoIterator<Item = usize>) -> Vec<usize> {
        let mut v: Vec<usize> = iter.into_iter().collect();
        v.sort_unstable();
        v
    }

    /// AC#9 — round-trip on an empty starting selection: `do_` installs
    /// `{2, 4, 6}`, `undo` returns to empty.
    #[test]
    fn selection_command_roundtrip_from_empty_start() {
        let mut doc = Document::default();
        let mut cmd = SelectionCommand::new([2usize, 4, 6]);
        cmd.do_(&mut doc);
        assert_eq!(sorted(doc.selection.iter()), vec![2, 4, 6]);
        cmd.undo(&mut doc);
        assert!(doc.selection.is_empty());
    }

    /// AC#10 — round-trip on a non-empty starting selection: `do_` overwrites
    /// `{0, 1}` with `{5, 7}`; `undo` restores `{0, 1}`.
    #[test]
    fn selection_command_roundtrip_from_non_empty_start() {
        let mut doc = Document::default();
        doc.selection.set([0usize, 1]);
        let mut cmd = SelectionCommand::new([5usize, 7]);
        cmd.do_(&mut doc);
        assert_eq!(sorted(doc.selection.iter()), vec![5, 7]);
        cmd.undo(&mut doc);
        assert_eq!(sorted(doc.selection.iter()), vec![0, 1]);
    }

    /// AC#11 — an empty `SelectionCommand` clears the prior selection; `undo`
    /// restores it.
    #[test]
    fn empty_selection_command_clears_then_undo_restores() {
        let mut doc = Document::default();
        doc.selection.set([3usize, 9]);
        let mut cmd = SelectionCommand::new(Vec::<usize>::new());
        cmd.do_(&mut doc);
        assert!(doc.selection.is_empty());
        cmd.undo(&mut doc);
        assert_eq!(sorted(doc.selection.iter()), vec![3, 9]);
    }

    /// AC#12 — label is exactly `"Select"`.
    #[test]
    fn selection_command_label_is_select() {
        let cmd = SelectionCommand::new([0usize]);
        assert_eq!(cmd.label(), "Select");
    }

    /// AC#13 — object-safe: `Box<dyn Command>` compiles.
    #[test]
    fn selection_command_is_object_safe() {
        let _: Box<dyn Command> = Box::new(SelectionCommand::new([0usize, 1]));
    }

    /// A second `undo` after the capture has been consumed is a no-op rather
    /// than a corruption — `Option::take` leaves `None` behind.
    #[test]
    fn double_undo_is_a_noop() {
        let mut doc = Document::default();
        doc.selection.set([0usize, 1]);
        let mut cmd = SelectionCommand::new([4usize]);
        cmd.do_(&mut doc);
        cmd.undo(&mut doc);
        assert_eq!(sorted(doc.selection.iter()), vec![0, 1]);
        cmd.undo(&mut doc);
        assert_eq!(sorted(doc.selection.iter()), vec![0, 1]);
    }

    /// Redo path: history.redo replays `do_` after `undo`. The command must
    /// re-capture the now-restored selection so a second `undo` works again.
    #[test]
    fn do_after_undo_re_captures_and_round_trips_again() {
        let mut doc = Document::default();
        doc.selection.set([1usize]);
        let mut cmd = SelectionCommand::new([7usize, 8]);

        cmd.do_(&mut doc);
        assert_eq!(sorted(doc.selection.iter()), vec![7, 8]);
        cmd.undo(&mut doc);
        assert_eq!(sorted(doc.selection.iter()), vec![1]);

        // Redo: do_ again, should still land on {7, 8} and undo should still
        // return to {1}.
        cmd.do_(&mut doc);
        assert_eq!(sorted(doc.selection.iter()), vec![7, 8]);
        cmd.undo(&mut doc);
        assert_eq!(sorted(doc.selection.iter()), vec![1]);
    }

    /// Duplicate indices in the input collapse — it's a set.
    #[test]
    fn duplicate_input_indices_collapse() {
        let mut doc = Document::default();
        let mut cmd = SelectionCommand::new([4usize, 4, 4, 5]);
        cmd.do_(&mut doc);
        assert_eq!(doc.selection.len(), 2);
        assert!(doc.selection.is_selected(4));
        assert!(doc.selection.is_selected(5));
    }
}
