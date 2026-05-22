//! SelectTool: point-pick and window/crossing box selection (LCV-042).
//!
//! ## State machine
//!
//! ```text
//! Idle ──press──► MaybeDragging { press_pos }
//!                      │ move > DRAG_THRESHOLD_MM
//!                      ▼
//!                 Dragging { press_pos }   ──release──► Idle (box commit)
//!
//! MaybeDragging ──release──► Idle (point-pick commit)
//! Idle/Any ──Escape──► Idle (clear selection)
//! ```
//!
//! Hit-testing helpers live in [`hit`] (distance, pick, box predicates).
//! MUST NOT import `eframe` or `rfd`. Introduced by demand LCV-042.

pub(crate) mod hit;

use crate::app::App;
use crate::document::{Document, Entity, History, SelectionCommand};
use crate::geometry::Vec2;
use crate::tools::Tool;

/// How far the cursor must move from the initial press to enter drag mode.
const DRAG_THRESHOLD_MM: f64 = 2.0;

// ---------------------------------------------------------------------------
// Internal state machine
// ---------------------------------------------------------------------------

/// Internal state of [`SelectTool`].
#[derive(Debug, Clone, Copy, PartialEq)]
enum SelectState {
    /// No button held; cursor is free.
    Idle,
    /// Left button held but not yet moved past the drag threshold.
    MaybeDragging { press_pos: Vec2 },
    /// Left button held and cursor moved past the drag threshold.
    Dragging { press_pos: Vec2 },
}

// ---------------------------------------------------------------------------
// SelectTool
// ---------------------------------------------------------------------------

/// Full-featured selection tool: point pick, window box, and crossing box.
///
/// Stored as the default active tool in [`crate::tools::ToolManager`].
#[derive(Debug)]
pub struct SelectTool {
    state: SelectState,
    /// Most-recently-reported cursor position, used to render the live drag
    /// box via [`SelectTool::preview`].
    cursor_pos: Vec2,
}

impl Default for SelectTool {
    fn default() -> Self {
        Self {
            state: SelectState::Idle,
            cursor_pos: Vec2::new(0.0, 0.0),
        }
    }
}

// ---------------------------------------------------------------------------
// Tool trait implementation
// ---------------------------------------------------------------------------

impl Tool for SelectTool {
    fn name(&self) -> &'static str {
        "Select"
    }

    fn on_pointer_down(
        &mut self,
        pos: Vec2,
        _shift: bool,
        _doc: &mut Document,
        _history: &mut History,
    ) {
        self.cursor_pos = pos;
        self.state = SelectState::MaybeDragging { press_pos: pos };
    }

    fn on_pointer_move(&mut self, pos: Vec2, _doc: &mut Document) {
        self.cursor_pos = pos;
        if let SelectState::MaybeDragging { press_pos } = self.state {
            if (pos - press_pos).length() > DRAG_THRESHOLD_MM {
                self.state = SelectState::Dragging { press_pos };
            }
        }
    }

    fn on_pointer_up(&mut self, pos: Vec2, shift: bool, doc: &mut Document, history: &mut History) {
        match self.state {
            SelectState::MaybeDragging { .. } => {
                let current_sel: Vec<usize> = doc.selection.iter().collect();
                let new_indices = hit::pick_resolve(pos, shift, &doc.entities, current_sel);
                history.commit(Box::new(SelectionCommand::new(new_indices)), doc);
                self.state = SelectState::Idle;
            }
            SelectState::Dragging { press_pos } => {
                let rect = crate::geometry::Rect::new(press_pos, pos);
                let is_window = press_pos.x < pos.x;
                let new_indices: Vec<usize> = doc
                    .entities
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| {
                        if is_window {
                            hit::entity_in_window(e, &rect)
                        } else {
                            hit::entity_in_crossing(e, &rect)
                        }
                    })
                    .map(|(i, _)| i)
                    .collect();
                history.commit(Box::new(SelectionCommand::new(new_indices)), doc);
                self.state = SelectState::Idle;
            }
            SelectState::Idle => {}
        }
    }

    fn on_key(&mut self, key: egui::Key, app: &mut App) {
        match key {
            egui::Key::Escape => {
                self.cancel();
                app.history.commit(
                    Box::new(SelectionCommand::new(Vec::<usize>::new())),
                    &mut app.document,
                );
            }
            egui::Key::Delete | egui::Key::Backspace => {
                if !app.document.selection.is_empty() {
                    let indices: Vec<usize> = app.document.selection.iter().collect();
                    app.history.commit(
                        Box::new(crate::document::DeleteEntities::new(indices)),
                        &mut app.document,
                    );
                    app.history.commit(
                        Box::new(SelectionCommand::new(Vec::<usize>::new())),
                        &mut app.document,
                    );
                }
            }
            _ => {}
        }
    }

    fn preview(&self) -> Vec<Entity> {
        if let SelectState::Dragging { press_pos } = self.state {
            hit::box_preview(press_pos, self.cursor_pos)
        } else {
            vec![]
        }
    }

    fn cancel(&mut self) {
        self.state = SelectState::Idle;
    }
}

// ---------------------------------------------------------------------------
// Tests — state machine and trait surface (behavioral tests in tests/)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::History;

    fn press(tool: &mut SelectTool, pos: Vec2) {
        let mut doc = crate::document::Document::default();
        let mut hist = History::default();
        tool.on_pointer_down(pos, false, &mut doc, &mut hist);
    }

    fn slide(tool: &mut SelectTool, pos: Vec2) {
        tool.on_pointer_move(pos, &mut crate::document::Document::default());
    }

    /// AC#4 — `name()` returns exactly `"Select"`.
    #[test]
    fn name_is_select() {
        assert_eq!(SelectTool::default().name(), "Select");
    }

    /// AC#15 — `preview()` is empty in Idle.
    #[test]
    fn preview_idle_is_empty() {
        assert!(SelectTool::default().preview().is_empty());
    }

    /// AC#15 — `preview()` is empty in MaybeDragging.
    #[test]
    fn preview_maybe_dragging_is_empty() {
        let mut tool = SelectTool::default();
        press(&mut tool, Vec2::new(0.0, 0.0));
        assert!(tool.preview().is_empty());
    }

    /// AC#2 — `SelectTool` is object-safe.
    #[test]
    fn is_object_safe() {
        let _b: Box<dyn Tool> = Box::new(SelectTool::default());
        let t = SelectTool::default();
        let _r: &dyn Tool = &t;
    }

    /// AC#1 — all seven Tool methods are reachable through `SelectTool`.
    #[test]
    fn all_seven_methods_callable() {
        let mut tool = SelectTool::default();
        let mut doc = crate::document::Document::default();
        let mut hist = History::default();
        let mut app = crate::app::App::default();
        let _ = tool.name();
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut hist);
        tool.on_pointer_move(Vec2::new(1.0, 1.0), &mut doc);
        tool.on_pointer_up(Vec2::new(2.0, 2.0), false, &mut doc, &mut hist);
        tool.on_key(egui::Key::Escape, &mut app);
        let _ = tool.preview();
        tool.cancel();
    }

    /// `on_pointer_down` enters MaybeDragging with the press position.
    #[test]
    fn pointer_down_enters_maybe_dragging() {
        let mut tool = SelectTool::default();
        press(&mut tool, Vec2::new(7.0, 3.0));
        assert_eq!(
            tool.state,
            SelectState::MaybeDragging {
                press_pos: Vec2::new(7.0, 3.0)
            }
        );
    }

    /// Moving > `DRAG_THRESHOLD_MM` transitions to Dragging.
    #[test]
    fn move_past_threshold_enters_dragging() {
        let mut tool = SelectTool::default();
        press(&mut tool, Vec2::new(0.0, 0.0));
        slide(&mut tool, Vec2::new(3.0, 0.0)); // 3 mm > 2 mm
        assert!(matches!(tool.state, SelectState::Dragging { .. }));
    }

    /// Moving < `DRAG_THRESHOLD_MM` stays in MaybeDragging.
    #[test]
    fn move_below_threshold_stays_maybe_dragging() {
        let mut tool = SelectTool::default();
        press(&mut tool, Vec2::new(0.0, 0.0));
        slide(&mut tool, Vec2::new(1.0, 0.0)); // 1 mm < 2 mm
        assert!(matches!(tool.state, SelectState::MaybeDragging { .. }));
    }

    /// Release resets state to Idle.
    #[test]
    fn pointer_up_resets_to_idle() {
        let mut tool = SelectTool::default();
        let mut doc = crate::document::Document::default();
        let mut hist = History::default();
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut hist);
        slide(&mut tool, Vec2::new(5.0, 5.0));
        tool.on_pointer_up(Vec2::new(5.0, 5.0), false, &mut doc, &mut hist);
        assert_eq!(tool.state, SelectState::Idle);
    }

    /// `cancel()` resets state to Idle.
    #[test]
    fn cancel_resets_idle() {
        let mut tool = SelectTool::default();
        press(&mut tool, Vec2::new(0.0, 0.0));
        slide(&mut tool, Vec2::new(5.0, 5.0));
        tool.cancel();
        assert_eq!(tool.state, SelectState::Idle);
    }

    /// `preview()` returns 4 line entities while Dragging.
    #[test]
    fn preview_four_lines_while_dragging() {
        let mut tool = SelectTool::default();
        press(&mut tool, Vec2::new(0.0, 0.0));
        slide(&mut tool, Vec2::new(10.0, 10.0));
        let pv = tool.preview();
        assert_eq!(pv.len(), 4);
        assert!(pv.iter().all(|e| matches!(e, Entity::Line(_))));
    }

    /// `preview()` is empty after `cancel()`.
    #[test]
    fn preview_empty_after_cancel() {
        let mut tool = SelectTool::default();
        press(&mut tool, Vec2::new(0.0, 0.0));
        slide(&mut tool, Vec2::new(10.0, 10.0));
        tool.cancel();
        assert!(tool.preview().is_empty());
    }
}
