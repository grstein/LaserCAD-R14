//! LineTool: click-click line drawing with chain mode and live preview.
//!
//! First click sets p1; second click commits `CreateLine(p1, p2)` and chains
//! (new p1 = p2). Degenerate clicks (`|p2−p1| ≤ EPSILON`) are discarded.
//! Escape or Enter resets to idle. MUST NOT import `eframe` or `rfd`.
//! Introduced by demand LCV-043.

use crate::app::App;
use crate::cmdline::ToolInput;
use crate::document::{commands::CreateLine, Document, Entity, History};
use crate::geometry::{Line, Vec2, EPSILON};
use crate::tools::Tool;

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Idle,
    WaitingSecondPoint { p1: Vec2, cursor: Vec2 },
}

impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

/// Click-click line drawing tool (LCV-043).
///
/// One click places p1; the next click commits the line and chains — the
/// committed endpoint becomes the new p1. Escape or Enter resets to `Idle`.
#[derive(Debug, Default)]
pub struct LineTool {
    state: State,
}

impl LineTool {
    /// Construct a new `LineTool` in the idle state.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Tool for LineTool {
    fn name(&self) -> &'static str {
        "LINE"
    }

    /// The R14 prompt table (LCV-111 AC 17).
    fn status_text(&self) -> &'static str {
        match self.state {
            State::Idle => "LINE Specify first point:",
            State::WaitingSecondPoint { .. } => "LINE Specify next point (Enter to finish):",
        }
    }

    fn on_pointer_down(
        &mut self,
        pos: Vec2,
        _shift: bool,
        doc: &mut Document,
        history: &mut History,
    ) {
        match self.state {
            State::Idle => {
                self.state = State::WaitingSecondPoint {
                    p1: pos,
                    cursor: pos,
                };
            }
            State::WaitingSecondPoint { p1, .. } => {
                if (pos - p1).length() <= EPSILON {
                    return; // discard degenerate
                }
                history.commit(Box::new(CreateLine::new(Line::new(p1, pos))), doc);
                self.state = State::WaitingSecondPoint {
                    p1: pos,
                    cursor: pos,
                };
            }
        }
    }

    fn on_pointer_move(&mut self, pos: Vec2, _doc: &mut Document) {
        if let State::WaitingSecondPoint { ref mut cursor, .. } = self.state {
            *cursor = pos;
        }
    }

    fn on_pointer_up(
        &mut self,
        _pos: Vec2,
        _shift: bool,
        _doc: &mut Document,
        _history: &mut History,
    ) {
    }

    fn anchor(&self) -> Option<Vec2> {
        match self.state {
            State::WaitingSecondPoint { p1, .. } => Some(p1),
            State::Idle => None,
        }
    }

    fn on_key(&mut self, key: egui::Key, _app: &mut App) {
        if matches!(key, egui::Key::Escape | egui::Key::Enter) {
            self.cancel();
        }
    }

    fn preview(&self) -> Vec<Entity> {
        match self.state {
            State::Idle => vec![],
            State::WaitingSecondPoint { p1, cursor } => {
                if (cursor - p1).length() > EPSILON {
                    vec![Entity::Line(Line::new(p1, cursor))]
                } else {
                    vec![]
                }
            }
        }
    }

    fn cancel(&mut self) {
        self.state = State::Idle;
    }

    /// The canonical body (ADR 0003 §B3): a typed point is exactly a click at
    /// that point, so the whole typed path inherits every fix to the click
    /// path. A `Distance` with no resolved direction designates no point and
    /// is refused.
    fn on_command_input(
        &mut self,
        input: ToolInput,
        doc: &mut Document,
        history: &mut History,
    ) -> bool {
        match input.as_point() {
            Some(p) => {
                self.on_pointer_down(p, false, doc, history);
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::History;

    fn make() -> (LineTool, crate::document::Document, History) {
        (
            LineTool::new(),
            crate::document::Document::default(),
            History::default(),
        )
    }

    #[test]
    fn name_and_idle_status() {
        let t = LineTool::new();
        assert_eq!(t.name(), "LINE");
        assert!(t.preview().is_empty());
        assert_eq!(t.status_text(), "LINE Specify first point:");
    }

    #[test]
    fn first_click_transitions_to_waiting() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(1.0, 2.0), false, &mut doc, &mut h);
        assert_eq!(
            t.status_text(),
            "LINE Specify next point (Enter to finish):"
        );
        assert!(t.preview().is_empty()); // cursor == p1 still
    }

    #[test]
    fn preview_rubber_band_after_move() {
        let (mut t, mut doc, mut h) = make();
        let p1 = Vec2::new(0.0, 0.0);
        let p2 = Vec2::new(10.0, 0.0);
        t.on_pointer_down(p1, false, &mut doc, &mut h);
        t.on_pointer_move(p2, &mut doc);
        let pv = t.preview();
        assert_eq!(pv.len(), 1);
        match pv[0] {
            Entity::Line(l) => {
                assert!(l.p1.approx_eq(p1, EPSILON));
                assert!(l.p2.approx_eq(p2, EPSILON));
            }
            _ => panic!("expected Entity::Line"),
        }
    }

    #[test]
    fn degenerate_second_click_ignored() {
        let (mut t, mut doc, mut h) = make();
        let p = Vec2::new(5.0, 5.0);
        t.on_pointer_down(p, false, &mut doc, &mut h);
        t.on_pointer_down(p, false, &mut doc, &mut h);
        assert_eq!(doc.entity_count(), 0);
        assert!(!h.can_undo());
    }

    #[test]
    fn second_click_commits_and_chains() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        t.on_pointer_down(Vec2::new(10.0, 0.0), false, &mut doc, &mut h);
        assert_eq!(doc.entity_count(), 1);
        assert!(h.can_undo());
        // still chaining
        assert_eq!(
            t.status_text(),
            "LINE Specify next point (Enter to finish):"
        );
    }

    #[test]
    fn chain_draws_second_line() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        t.on_pointer_down(Vec2::new(10.0, 0.0), false, &mut doc, &mut h);
        t.on_pointer_down(Vec2::new(10.0, 10.0), false, &mut doc, &mut h);
        assert_eq!(doc.entity_count(), 2);
    }

    #[test]
    fn escape_and_enter_cancel_to_idle() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        t.on_pointer_move(Vec2::new(5.0, 5.0), &mut doc);
        assert!(!t.preview().is_empty());
        let mut app = crate::app::App::default();
        t.on_key(egui::Key::Escape, &mut app);
        assert!(t.preview().is_empty());
        assert_eq!(t.status_text(), "LINE Specify first point:");
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        t.on_key(egui::Key::Enter, &mut app);
        assert!(t.preview().is_empty());
    }

    #[test]
    fn object_safe_and_idle_cancel_noop() {
        let mut t = LineTool::new();
        t.cancel();
        assert!(t.preview().is_empty());
        let _: Box<dyn Tool> = Box::new(LineTool::new());
    }

    /// LCV-053 AC#7 — `anchor()` returns `None` when idle.
    #[test]
    fn line_tool_anchor_idle_is_none() {
        let t = LineTool::new();
        assert_eq!(t.anchor(), None);
    }

    /// LCV-053 AC#8 — `anchor()` returns `Some(p1)` after first click.
    #[test]
    fn line_tool_anchor_waiting_returns_p1() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(7.0, 4.0), false, &mut doc, &mut h);
        assert_eq!(t.anchor(), Some(Vec2::new(7.0, 4.0)));
    }

    /// LCV-053 AC#8 (precision) — anchor advances to the newly committed
    /// endpoint after a second click commits a segment and chains.
    #[test]
    fn line_tool_anchor_updates_after_second_commit() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        t.on_pointer_down(Vec2::new(10.0, 0.0), false, &mut doc, &mut h);
        assert_eq!(t.anchor(), Some(Vec2::new(10.0, 0.0)));
    }

    /// LCV-111 AC 3 — a typed point is exactly a click at that point: same
    /// committed geometry, same resulting state as a literal
    /// `on_pointer_down` pair.
    #[test]
    fn command_point_acts_exactly_like_a_click() {
        let (mut typed, mut typed_doc, mut typed_h) = make();
        assert!(typed.on_command_input(
            ToolInput::Point(Vec2::new(0.0, 0.0)),
            &mut typed_doc,
            &mut typed_h
        ));
        assert!(typed.on_command_input(
            ToolInput::Point(Vec2::new(100.0, 0.0)),
            &mut typed_doc,
            &mut typed_h
        ));

        let (mut clicked, mut clicked_doc, mut clicked_h) = make();
        clicked.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut clicked_doc, &mut clicked_h);
        clicked.on_pointer_down(
            Vec2::new(100.0, 0.0),
            false,
            &mut clicked_doc,
            &mut clicked_h,
        );

        assert_eq!(typed_doc.entities, clicked_doc.entities);
        assert_eq!(typed.anchor(), clicked.anchor());
        assert_eq!(typed.status_text(), clicked.status_text());
        assert_eq!(typed_h.len(), 1);
        match typed_doc.entities[0] {
            Entity::Line(l) => {
                assert_eq!(l.p1, Vec2::new(0.0, 0.0));
                assert_eq!(l.p2, Vec2::new(100.0, 0.0));
            }
            _ => panic!("expected Entity::Line"),
        }
    }

    /// LCV-111 AC 3 — a `Distance` the app could not resolve carries no
    /// point, so the canonical body refuses it and commits nothing.
    #[test]
    fn command_distance_without_a_direction_is_refused() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        let consumed = t.on_command_input(
            ToolInput::Distance {
                value_mm: 50.0,
                along: None,
            },
            &mut doc,
            &mut h,
        );
        assert!(!consumed);
        assert_eq!(doc.entity_count(), 0);
        assert_eq!(t.anchor(), Some(Vec2::new(0.0, 0.0)));
    }
}
