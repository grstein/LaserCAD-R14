//! CircleTool: two-click center→radius circle drawing (LCV-046).
//!
//! First click sets the center; second click commits a [`CreateCircle`] command
//! when `r >= EPSILON`. Preview geometry is returned by [`Tool::preview`] and
//! painted amber by the render pipeline each frame.
//!
//! MUST NOT import `eframe` or `rfd`.

use crate::app::App;
use crate::document::{commands::CreateCircle, Document, Entity, History};
use crate::geometry::{Circle, Vec2, EPSILON};
use crate::tools::Tool;

/// Internal FSM state of [`CircleTool`].
#[derive(Debug, Clone, Copy, PartialEq)]
enum CircleState {
    /// Awaiting first click to anchor the center.
    Idle,
    /// Center anchored; awaiting radius click.
    WaitingRadius { center: Vec2 },
}

impl Default for CircleState {
    fn default() -> Self {
        Self::Idle
    }
}

/// Two-click circle drawing tool: first click → center, second click → radius.
///
/// Degenerate radius clicks (`r < EPSILON`) are silently ignored. Escape
/// cancels and returns to [`CircleState::Idle`].
#[derive(Debug, Default)]
pub struct CircleTool {
    state: CircleState,
    /// Last cursor position in world mm — drives the live preview radius.
    cursor: Vec2,
}

impl Tool for CircleTool {
    fn name(&self) -> &'static str {
        "CIRCLE"
    }

    fn on_pointer_down(
        &mut self,
        pos: Vec2,
        _shift: bool,
        doc: &mut Document,
        history: &mut History,
    ) {
        match self.state {
            CircleState::Idle => {
                self.state = CircleState::WaitingRadius { center: pos };
                self.cursor = pos;
            }
            CircleState::WaitingRadius { center } => {
                let r = pos.distance(center);
                if r >= EPSILON {
                    history.commit(Box::new(CreateCircle::new(Circle::new(center, r))), doc);
                    self.state = CircleState::Idle;
                }
            }
        }
    }

    fn on_pointer_move(&mut self, pos: Vec2, _doc: &mut Document) {
        self.cursor = pos;
    }

    fn on_pointer_up(
        &mut self,
        _pos: Vec2,
        _shift: bool,
        _doc: &mut Document,
        _history: &mut History,
    ) {
    }

    fn on_key(&mut self, key: egui::Key, _app: &mut App) {
        if key == egui::Key::Escape {
            self.cancel();
        }
    }

    /// Preview: the in-progress circle while in `WaitingRadius` state, or empty.
    fn preview(&self) -> Vec<Entity> {
        match self.state {
            CircleState::Idle => vec![],
            CircleState::WaitingRadius { center } => {
                let r = self.cursor.distance(center);
                if r >= EPSILON {
                    vec![Entity::Circle(Circle::new(center, r))]
                } else {
                    vec![]
                }
            }
        }
    }

    fn cancel(&mut self) {
        self.state = CircleState::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::History;

    fn make() -> (CircleTool, crate::document::Document, History) {
        (
            CircleTool::default(),
            crate::document::Document::default(),
            History::default(),
        )
    }

    #[test]
    fn name_is_circle() {
        assert_eq!(CircleTool::default().name(), "CIRCLE");
    }

    #[test]
    fn default_idle_empty_preview() {
        let t = CircleTool::default();
        assert_eq!(t.state, CircleState::Idle);
        assert!(t.preview().is_empty());
    }

    #[test]
    fn first_click_enters_waiting_radius_no_commit() {
        let (mut t, mut doc, mut h) = make();
        let c = Vec2::new(10.0, 20.0);
        t.on_pointer_down(c, false, &mut doc, &mut h);
        assert_eq!(t.state, CircleState::WaitingRadius { center: c });
        assert_eq!(doc.entity_count(), 0);
    }

    #[test]
    fn preview_reflects_cursor_distance() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        t.on_pointer_move(Vec2::new(5.0, 0.0), &mut doc);
        let prev = t.preview();
        assert_eq!(prev.len(), 1);
        if let Entity::Circle(c) = prev[0] {
            assert!((c.r - 5.0).abs() < EPSILON);
        } else {
            panic!("expected Entity::Circle");
        }
    }

    #[test]
    fn second_click_commits_and_returns_idle() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        t.on_pointer_down(Vec2::new(3.0, 4.0), false, &mut doc, &mut h); // r = 5
        assert_eq!(t.state, CircleState::Idle);
        assert_eq!(doc.entity_count(), 1);
        if let Entity::Circle(c) = doc.entities[0] {
            assert!((c.r - 5.0).abs() < EPSILON);
        } else {
            panic!("expected Entity::Circle committed");
        }
        assert!(t.preview().is_empty());
    }

    #[test]
    fn degenerate_second_click_ignored() {
        let (mut t, mut doc, mut h) = make();
        let center = Vec2::new(1.0, 1.0);
        t.on_pointer_down(center, false, &mut doc, &mut h);
        t.on_pointer_down(center, false, &mut doc, &mut h); // r = 0
        assert_eq!(t.state, CircleState::WaitingRadius { center });
        assert_eq!(doc.entity_count(), 0);
    }

    #[test]
    fn escape_cancels_to_idle() {
        let (mut t, mut doc, mut h) = make();
        let mut app = crate::app::App::default();
        t.on_pointer_down(Vec2::new(5.0, 5.0), false, &mut doc, &mut h);
        t.on_key(egui::Key::Escape, &mut app);
        assert_eq!(t.state, CircleState::Idle);
        assert!(t.preview().is_empty());
    }

    #[test]
    fn cancel_resets_to_idle() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(2.0, 3.0), false, &mut doc, &mut h);
        t.cancel();
        assert_eq!(t.state, CircleState::Idle);
    }

    #[test]
    fn committed_circle_is_undoable() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        t.on_pointer_down(Vec2::new(10.0, 0.0), false, &mut doc, &mut h);
        assert_eq!(doc.entity_count(), 1);
        h.undo(&mut doc);
        assert_eq!(doc.entity_count(), 0);
    }

    #[test]
    fn is_object_safe() {
        let _: Box<dyn Tool> = Box::new(CircleTool::default());
    }
}
