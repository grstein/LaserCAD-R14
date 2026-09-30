//! SelectTool: point-pick and window/crossing box selection (LCV-042).
//!
//! ## State machine
//!
//! ```text
//! Idle ──press──► MaybeDragging { press_pos }
//!                      │ move > DRAG_THRESHOLD_PT × mm/pt
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
use crate::tools::{DRAG_THRESHOLD_PT, Mark, PICK_APERTURE_PT, Tool};
use std::borrow::Cow;

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
    /// Live zoom in mm per screen point (LCV-162); `1.0` until forwarded.
    mm_per_pt: f64,
}

impl Default for SelectTool {
    fn default() -> Self {
        Self {
            state: SelectState::Idle,
            cursor_pos: Vec2::new(0.0, 0.0),
            mm_per_pt: 1.0,
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

    /// R14's idle prompt, verbatim (LCV-111 AC 17). SELECT is the tool that
    /// is active when the operator is not in the middle of anything, so its
    /// prompt is the command line's resting state.
    fn status_text(&self) -> Cow<'_, str> {
        "Command:".into()
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
        if let SelectState::MaybeDragging { press_pos } = self.state
            && (pos - press_pos).length() > DRAG_THRESHOLD_PT * self.mm_per_pt
        {
            self.state = SelectState::Dragging { press_pos };
        }
    }

    fn on_pointer_up(&mut self, pos: Vec2, shift: bool, doc: &mut Document, history: &mut History) {
        match self.state {
            SelectState::MaybeDragging { .. } => {
                let current_sel: Vec<usize> = doc.selection.iter().collect();
                let radius = PICK_APERTURE_PT * self.mm_per_pt;
                let new_indices = hit::pick_resolve(pos, shift, &doc.entities, current_sel, radius);
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
                // Cancel the in-progress drag unconditionally, but commit the
                // clearing command only when there is a selection to clear.
                // Escape is the most-pressed key in a CAD session, and an
                // empty SelectionCommand on an empty selection is a no-op that
                // would make Ctrl+Z stop feeling like undo (LCV-105).
                self.cancel();
                if !app.document.selection.is_empty() {
                    app.history.commit(
                        Box::new(SelectionCommand::new(Vec::<usize>::new())),
                        &mut app.document,
                    );
                }
            }
            egui::Key::Delete | egui::Key::Backspace if !app.document.selection.is_empty() => {
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

    /// LCV-163 AC 1–3: while dragging, the box — solid `Preview` for a
    /// window (left to right), `Dashed` for a crossing; otherwise the entity
    /// the click would pick, as `Hover`.
    fn feedback(&self, doc: &Document, cursor: Option<Vec2>) -> Vec<Mark> {
        if let SelectState::Dragging { press_pos } = self.state {
            let style = if press_pos.x < self.cursor_pos.x {
                Mark::Preview
            } else {
                Mark::Dashed
            };
            return hit::box_preview(press_pos, self.cursor_pos)
                .into_iter()
                .map(style)
                .collect();
        }
        let radius = PICK_APERTURE_PT * self.mm_per_pt;
        cursor
            .and_then(|c| hit::pick_closest(c, &doc.entities, radius))
            .map(Mark::Hover)
            .into_iter()
            .collect()
    }

    fn cancel(&mut self) {
        self.state = SelectState::Idle;
    }

    /// An entity pick unless a box drag is under way (LCV-162 AC 5/AC 6).
    fn wants_entity_pick(&self) -> bool {
        !matches!(self.state, SelectState::Dragging { .. })
    }

    fn set_pick_scale(&mut self, mm_per_pt: f64) {
        self.mm_per_pt = mm_per_pt;
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

    /// LCV-111 AC 17 — SELECT shows R14's idle prompt, not its name.
    #[test]
    fn status_text_is_the_r14_idle_prompt() {
        assert_eq!(SelectTool::default().status_text(), "Command:");
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

    /// Moving > `DRAG_THRESHOLD_PT` at the default 1 mm/pt enters Dragging.
    #[test]
    fn move_past_threshold_enters_dragging() {
        let mut tool = SelectTool::default();
        press(&mut tool, Vec2::new(0.0, 0.0));
        slide(&mut tool, Vec2::new(3.0, 0.0)); // 3 mm > 2 mm
        assert!(matches!(tool.state, SelectState::Dragging { .. }));
    }

    /// Moving < `DRAG_THRESHOLD_PT` at the default 1 mm/pt stays MaybeDragging.
    #[test]
    fn move_below_threshold_stays_maybe_dragging() {
        let mut tool = SelectTool::default();
        press(&mut tool, Vec2::new(0.0, 0.0));
        slide(&mut tool, Vec2::new(1.0, 0.0)); // 1 mm < 2 mm
        assert!(matches!(tool.state, SelectState::MaybeDragging { .. }));
    }

    /// LCV-162 AC 9 — the drag threshold scales with the zoom: 30 mm is a
    /// 1.5 pt move at 20 mm/pt (a click), 0.15 mm a 3 pt move at 0.05.
    #[test]
    fn drag_threshold_follows_the_pick_scale() {
        let mut tool = SelectTool::default();
        tool.set_pick_scale(20.0);
        press(&mut tool, Vec2::new(0.0, 0.0));
        slide(&mut tool, Vec2::new(30.0, 0.0));
        assert!(matches!(tool.state, SelectState::MaybeDragging { .. }));
        tool.set_pick_scale(0.05);
        slide(&mut tool, Vec2::new(0.15, 0.0));
        assert!(matches!(tool.state, SelectState::Dragging { .. }));
    }

    /// LCV-162 AC 7 — the click pick radius is 5 pt at the live zoom.
    #[test]
    fn click_pick_radius_follows_the_pick_scale() {
        let mut doc = crate::document::Document::default();
        doc.entities.push(Entity::Line(crate::geometry::Line::new(
            Vec2::new(-10.0, 0.0),
            Vec2::new(10.0, 0.0),
        )));
        let mut hist = History::default();
        for (scale, y, picked) in [(0.05, 0.2, true), (0.05, 0.3, false), (20.0, 80.0, true)] {
            let mut tool = SelectTool::default();
            tool.set_pick_scale(scale);
            tool.on_pointer_down(Vec2::new(0.0, y), false, &mut doc, &mut hist);
            tool.on_pointer_up(Vec2::new(0.0, y), false, &mut doc, &mut hist);
            assert_eq!(!doc.selection.is_empty(), picked, "{scale} mm/pt, {y} mm");
        }
    }

    /// LCV-162 AC 5/AC 6 — an entity pick in Idle and MaybeDragging, not
    /// while a box drag is under way.
    #[test]
    fn wants_entity_pick_unless_dragging() {
        let mut tool = SelectTool::default();
        assert!(tool.wants_entity_pick());
        press(&mut tool, Vec2::new(0.0, 0.0));
        assert!(tool.wants_entity_pick());
        slide(&mut tool, Vec2::new(10.0, 0.0));
        assert!(!tool.wants_entity_pick());
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

    /// LCV-163 AC 1/AC 2 — the drag box is a solid `Preview` box left to
    /// right (window) and a `Dashed` box right to left (crossing).
    #[test]
    fn feedback_box_style_follows_the_drag_direction() {
        let doc = crate::document::Document::default();
        let mut tool = SelectTool::default();
        press(&mut tool, Vec2::new(0.0, 0.0));
        slide(&mut tool, Vec2::new(10.0, 10.0));
        let marks = tool.feedback(&doc, Some(Vec2::new(10.0, 10.0)));
        assert_eq!(marks.len(), 4);
        assert!(marks.iter().all(|m| matches!(m, Mark::Preview(_))));
        slide(&mut tool, Vec2::new(-10.0, 10.0));
        let marks = tool.feedback(&doc, None);
        assert_eq!(marks.len(), 4, "the box does not need the cursor");
        assert!(marks.iter().all(|m| matches!(m, Mark::Dashed(_))));
    }

    /// LCV-163 AC 3 — off a drag, the entity in the pickbox is `Hover`ed;
    /// nothing when none is in range or the cursor is `None`.
    #[test]
    fn feedback_hovers_the_closest_entity_in_the_pickbox() {
        let mut doc = crate::document::Document::default();
        for y in [0.0, 3.0] {
            doc.entities.push(Entity::Line(crate::geometry::Line::new(
                Vec2::new(-10.0, y),
                Vec2::new(10.0, y),
            )));
        }
        let tool = SelectTool::default();
        assert_eq!(
            tool.feedback(&doc, Some(Vec2::new(0.0, 2.0))),
            vec![Mark::Hover(1)]
        );
        assert!(tool.feedback(&doc, Some(Vec2::new(0.0, 9.0))).is_empty());
        assert!(tool.feedback(&doc, None).is_empty());
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

    /// LCV-105 AC#6 — Escape with nothing selected commits nothing: the undo
    /// stack must not collect one no-op `SelectionCommand` per Escape press.
    #[test]
    fn escape_with_empty_selection_commits_nothing() {
        let mut tool = SelectTool::default();
        let mut app = crate::app::App::default();
        assert!(app.document.selection.is_empty());

        tool.on_key(egui::Key::Escape, &mut app);
        tool.on_key(egui::Key::Escape, &mut app);

        assert!(
            !app.history.can_undo(),
            "Escape on an empty selection must leave the undo stack untouched"
        );
    }

    /// LCV-105 AC#7 — Escape with a live selection still clears it, exactly
    /// once: one undo restores the two selected indices.
    #[test]
    fn escape_with_selection_clears_it_once() {
        let mut tool = SelectTool::default();
        let mut app = crate::app::App::default();
        for _ in 0..3 {
            app.document
                .entities
                .push(Entity::Line(crate::geometry::Line::new(
                    Vec2::new(0.0, 0.0),
                    Vec2::new(1.0, 0.0),
                )));
        }
        app.history.commit(
            Box::new(SelectionCommand::new(vec![0usize, 2usize])),
            &mut app.document,
        );
        assert_eq!(app.document.selection.len(), 2);

        tool.on_key(egui::Key::Escape, &mut app);
        assert!(app.document.selection.is_empty(), "Escape must clear it");

        assert!(app.history.undo(&mut app.document));
        let mut restored: Vec<usize> = app.document.selection.iter().collect();
        restored.sort_unstable();
        assert_eq!(
            restored,
            vec![0usize, 2usize],
            "exactly one command was pushed, so one undo restores the selection"
        );
    }
}
