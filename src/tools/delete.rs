//! DeleteTool (ERASE): deletes the current selection on pointer-down or Delete/Backspace key.
//!
//! ## Behaviour
//!
//! * `name()` returns `"ERASE"`.
//! * `on_pointer_down` — if the selection is non-empty: commit `DeleteEntities`
//!   for the selected indices then commit `SelectionCommand` (empty) to clear
//!   the selection through the history stack so both operations are undoable
//!   together. If the selection is empty the event is a no-op.
//! * `on_key(Delete | Backspace)` — same two-commit sequence via
//!   `app.document` and `app.history`.
//! * All other events are no-ops. The tool is stateless (unit struct).
//!
//! The primary UX path for most users is Delete/Backspace *while* `SelectTool`
//! is active (handled in [`crate::tools::select`]). `DeleteTool` is a
//! dedicated ERASE mode for toolbar-driven workflows.
//!
//! MUST NOT import `eframe` or `rfd`. Introduced by demand LCV-052.

use crate::app::App;
use crate::document::{DeleteEntities, Document, Entity, History, SelectionCommand};
use crate::geometry::Vec2;
use crate::tools::Tool;

// ---------------------------------------------------------------------------
// DeleteTool
// ---------------------------------------------------------------------------

/// Stateless erase tool: deletes selected entities on pointer-down or key press.
///
/// Two commands are committed in sequence so both the deletion *and* the
/// resulting selection-clear participate in the undo stack atomically:
///
/// 1. `DeleteEntities` — removes the entities at the selected indices.
/// 2. `SelectionCommand::new([])` — clears the (now stale) selection.
///
/// Ctrl+Z undoes in reverse order (clear first, then re-insert), restoring
/// the previous state exactly.
#[derive(Debug, Default)]
pub struct DeleteTool;

/// Commit the two-step delete+clear sequence to `history`/`doc`.
///
/// Extracted so both `on_pointer_down` and `on_key` share the exact same
/// code path without duplication.
fn commit_delete(doc: &mut Document, history: &mut History) {
    if doc.selection.is_empty() {
        return;
    }
    let indices: Vec<usize> = doc.selection.iter().collect();
    history.commit(Box::new(DeleteEntities::new(indices)), doc);
    history.commit(Box::new(SelectionCommand::new(Vec::<usize>::new())), doc);
}

impl Tool for DeleteTool {
    fn name(&self) -> &'static str {
        "ERASE"
    }

    fn on_pointer_down(
        &mut self,
        _pos: Vec2,
        _shift: bool,
        doc: &mut Document,
        history: &mut History,
    ) {
        commit_delete(doc, history);
    }

    fn on_pointer_move(&mut self, _pos: Vec2, _doc: &mut Document) {}

    fn on_pointer_up(
        &mut self,
        _pos: Vec2,
        _shift: bool,
        _doc: &mut Document,
        _history: &mut History,
    ) {
    }

    fn on_key(&mut self, key: egui::Key, app: &mut App) {
        if matches!(key, egui::Key::Delete | egui::Key::Backspace) {
            commit_delete(&mut app.document, &mut app.history);
        }
    }

    fn preview(&self) -> Vec<Entity> {
        vec![]
    }

    fn cancel(&mut self) {}
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{History, SelectionCommand};
    use crate::geometry::{Line, Vec2};

    fn doc_with_line() -> Document {
        let mut doc = Document::default();
        let line = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        doc.push_current(Entity::Line(line));
        doc
    }

    /// AC#1 — `name()` returns exactly `"ERASE"`.
    #[test]
    fn name_is_erase() {
        assert_eq!(DeleteTool.name(), "ERASE");
    }

    /// AC#2 — object safety: `DeleteTool` stores as `Box<dyn Tool>`.
    #[test]
    fn is_object_safe() {
        let _b: Box<dyn Tool> = Box::new(DeleteTool);
    }

    /// AC#3 — `preview()` is always empty.
    #[test]
    fn preview_always_empty() {
        assert!(DeleteTool.preview().is_empty());
    }

    /// AC#4 — `cancel()` is a no-op (doesn't panic or corrupt state).
    #[test]
    fn cancel_is_noop() {
        let mut tool = DeleteTool;
        tool.cancel();
    }

    /// AC#5 — `on_pointer_down` with a non-empty selection deletes the
    /// entity and clears the selection.
    #[test]
    fn pointer_down_with_selection_deletes_entity() {
        let mut doc = doc_with_line();
        let mut hist = History::default();
        // Select entity at index 0.
        hist.commit(Box::new(SelectionCommand::new([0usize])), &mut doc);
        assert_eq!(doc.entity_count(), 1);
        assert!(!doc.selection.is_empty());

        let mut tool = DeleteTool;
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut hist);

        assert_eq!(doc.entity_count(), 0, "entity should be deleted");
        assert!(doc.selection.is_empty(), "selection should be cleared");
    }

    /// AC#6 — `on_pointer_down` with empty selection is a no-op.
    #[test]
    fn pointer_down_with_empty_selection_is_noop() {
        let mut doc = doc_with_line();
        let mut hist = History::default();

        let mut tool = DeleteTool;
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut hist);

        assert_eq!(
            doc.entity_count(),
            1,
            "entity must survive when nothing selected"
        );
        assert!(doc.selection.is_empty());
    }

    /// AC#7 — after `on_pointer_down` deletes, `hist.undo` restores the entity.
    #[test]
    fn pointer_down_delete_is_undoable() {
        let mut doc = doc_with_line();
        let mut hist = History::default();
        hist.commit(Box::new(SelectionCommand::new([0usize])), &mut doc);

        let mut tool = DeleteTool;
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut hist);
        assert_eq!(doc.entity_count(), 0);

        // Undo the selection-clear.
        hist.undo(&mut doc);
        // Undo the DeleteEntities.
        hist.undo(&mut doc);

        assert_eq!(
            doc.entity_count(),
            1,
            "entity should be restored after undo"
        );
        // Selection is restored to {0} by the undo chain.
        assert!(doc.selection.is_selected(0));
    }

    /// AC#8 — two entities selected: both are removed after `on_pointer_down`.
    #[test]
    fn pointer_down_deletes_multiple_entities() {
        let mut doc = Document::default();
        let line = Line::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
        doc.push_current(Entity::Line(line));
        doc.push_current(Entity::Line(line));
        let mut hist = History::default();
        hist.commit(Box::new(SelectionCommand::new([0usize, 1])), &mut doc);

        let mut tool = DeleteTool;
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut hist);

        assert_eq!(doc.entity_count(), 0);
        assert!(doc.selection.is_empty());
    }
}
