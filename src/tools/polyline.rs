//! PolylineTool: connected line segments drawn with click-click interaction.
//!
//! First click sets p1; second click commits `CreateLine(p1, p2)` and chains
//! (new p1 = p2). Degenerate clicks (`|p2−p1| ≤ EPSILON`) are discarded.
//! Escape or Enter resets to idle. MUST NOT import `eframe` or `rfd`.
//! Introduced by demand LCV-044.

use crate::app::App;
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

/// Click-click connected-segment drawing tool (LCV-044).
///
/// One click places p1; the next click commits the segment and chains — the
/// committed endpoint becomes the new p1 so segments connect end-to-end.
/// Escape or Enter resets to `Idle`.
#[derive(Debug, Default)]
pub struct PolylineTool {
    state: State,
}

impl PolylineTool {
    /// Construct a new `PolylineTool` in the idle state.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Tool for PolylineTool {
    fn name(&self) -> &'static str {
        "PLINE"
    }

    fn status_text(&self) -> &'static str {
        match self.state {
            State::Idle => "PLINE: Click to set start point",
            State::WaitingSecondPoint { .. } => {
                "PLINE: Click to set next point  |  Enter/Esc to finish"
            }
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
                // Chain: the committed endpoint becomes the new p1.
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::History;

    fn make() -> (PolylineTool, crate::document::Document, History) {
        (
            PolylineTool::new(),
            crate::document::Document::default(),
            History::default(),
        )
    }

    // ── AC: name and default state ──────────────────────────────────────

    #[test]
    fn name_is_pline() {
        let t = PolylineTool::new();
        assert_eq!(t.name(), "PLINE");
    }

    #[test]
    fn idle_preview_is_empty() {
        let t = PolylineTool::new();
        assert!(t.preview().is_empty());
    }

    #[test]
    fn idle_status_contains_start() {
        let t = PolylineTool::new();
        assert!(t.status_text().starts_with("PLINE:"));
        assert!(t.status_text().contains("start"));
    }

    // ── AC: first click ─────────────────────────────────────────────────

    #[test]
    fn first_click_transitions_to_waiting() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(1.0, 2.0), false, &mut doc, &mut h);
        assert!(t.status_text().contains("next"));
        assert!(t.preview().is_empty()); // cursor == p1 still
    }

    #[test]
    fn first_click_no_entity_committed() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(3.0, 4.0), false, &mut doc, &mut h);
        assert_eq!(doc.entity_count(), 0);
        assert!(!h.can_undo());
    }

    // ── AC: rubber-band preview ─────────────────────────────────────────

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
    fn move_while_idle_is_noop() {
        let (mut t, mut doc, _h) = make();
        t.on_pointer_move(Vec2::new(99.0, 99.0), &mut doc);
        assert!(t.preview().is_empty());
    }

    // ── AC: degenerate guard ────────────────────────────────────────────

    #[test]
    fn degenerate_second_click_ignored() {
        let (mut t, mut doc, mut h) = make();
        let p = Vec2::new(5.0, 5.0);
        t.on_pointer_down(p, false, &mut doc, &mut h);
        t.on_pointer_down(p, false, &mut doc, &mut h);
        assert_eq!(doc.entity_count(), 0);
        assert!(!h.can_undo());
    }

    // ── AC: commit and chain ────────────────────────────────────────────

    #[test]
    fn second_click_commits_and_chains() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        t.on_pointer_down(Vec2::new(10.0, 0.0), false, &mut doc, &mut h);
        assert_eq!(doc.entity_count(), 1);
        assert!(h.can_undo());
        // Still waiting — chained to p2 as new p1.
        assert!(t.status_text().contains("next"));
    }

    #[test]
    fn chain_draws_second_segment() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        t.on_pointer_down(Vec2::new(10.0, 0.0), false, &mut doc, &mut h);
        t.on_pointer_down(Vec2::new(10.0, 10.0), false, &mut doc, &mut h);
        assert_eq!(doc.entity_count(), 2);
    }

    #[test]
    fn three_segment_chain() {
        let (mut t, mut doc, mut h) = make();
        for (x, y) in [(0.0, 0.0), (5.0, 0.0), (5.0, 5.0), (0.0, 5.0)] {
            t.on_pointer_down(Vec2::new(x, y), false, &mut doc, &mut h);
        }
        assert_eq!(doc.entity_count(), 3);
    }

    // ── AC: cancel / Enter / Escape ─────────────────────────────────────

    #[test]
    fn escape_resets_to_idle() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        t.on_pointer_move(Vec2::new(5.0, 5.0), &mut doc);
        assert!(!t.preview().is_empty());
        let mut app = crate::app::App::default();
        t.on_key(egui::Key::Escape, &mut app);
        assert!(t.preview().is_empty());
        assert!(t.status_text().contains("start"));
    }

    #[test]
    fn enter_resets_to_idle() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        let mut app = crate::app::App::default();
        t.on_key(egui::Key::Enter, &mut app);
        assert!(t.preview().is_empty());
        assert!(t.status_text().contains("start"));
    }

    #[test]
    fn cancel_from_idle_is_noop() {
        let mut t = PolylineTool::new();
        t.cancel();
        assert!(t.preview().is_empty());
        assert!(t.status_text().contains("start"));
    }

    // ── AC: object-safety ───────────────────────────────────────────────

    #[test]
    fn object_safe() {
        let _: Box<dyn Tool> = Box::new(PolylineTool::new());
    }
}
