//! CircleTool: two-click center→radius circle drawing (LCV-046).
//!
//! First click sets the center; second click commits a [`CreateCircle`] command
//! when `r >= EPSILON`. Preview geometry is returned by [`Tool::preview`] and
//! painted amber by the render pipeline each frame.
//!
//! MUST NOT import `eframe` or `rfd`.

use crate::app::App;
use crate::cmdline::ToolInput;
use crate::document::{commands::CreateCircle, Document, Entity, History};
use crate::geometry::{Circle, Vec2, EPSILON};
use crate::tools::Tool;

/// Internal FSM state of [`CircleTool`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum CircleState {
    /// Awaiting first click to anchor the center.
    #[default]
    Idle,
    /// Center anchored; awaiting radius click.
    WaitingRadius { center: Vec2 },
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

    /// The R14 prompt table (LCV-111 AC 17). CIRCLE had no override before
    /// this demand and inherited [`Tool::name`], so it showed no phase.
    fn status_text(&self) -> &'static str {
        match self.state {
            CircleState::Idle => "CIRCLE Specify center point:",
            CircleState::WaitingRadius { .. } => "CIRCLE Specify radius:",
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

    /// The base point relative and direct-distance input resolve against
    /// (LCV-111 AC 5): the centre, once it is fixed.
    ///
    /// Adding this also turns F8 ortho on for the radius pick, which is
    /// R14-correct — an axis-aligned radius still commits a circle.
    fn anchor(&self) -> Option<Vec2> {
        match self.state {
            CircleState::WaitingRadius { center } => Some(center),
            CircleState::Idle => None,
        }
    }

    /// CIRCLE is the one tool that overrides the `Distance` arm (ADR 0003
    /// §B3): while awaiting a radius, a bare number **is** the radius and no
    /// direction is required. That is what makes a mouse-free session work —
    /// `c` ⏎ `50,50` ⏎ `25` ⏎ commits with `last_cursor_world == None`.
    ///
    /// A non-positive radius is refused: nothing is committed and the tool
    /// stays in `WaitingRadius` so the operator can retype.
    fn on_command_input(
        &mut self,
        input: ToolInput,
        doc: &mut Document,
        history: &mut History,
    ) -> bool {
        match input {
            ToolInput::Distance { value_mm, .. } => match self.state {
                CircleState::WaitingRadius { center } => {
                    if value_mm <= EPSILON {
                        return false;
                    }
                    self.on_pointer_down(center + Vec2::new(value_mm, 0.0), false, doc, history);
                    true
                }
                CircleState::Idle => false,
            },
            ToolInput::Point(p) => {
                self.on_pointer_down(p, false, doc, history);
                true
            }
        }
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

    /// LCV-111 AC 3 — a typed centre + a typed radius point is exactly a
    /// pair of clicks.
    #[test]
    fn command_point_acts_exactly_like_a_click() {
        let (mut typed, mut typed_doc, mut typed_h) = make();
        assert!(typed.on_command_input(
            ToolInput::Point(Vec2::new(50.0, 50.0)),
            &mut typed_doc,
            &mut typed_h
        ));
        assert!(typed.on_command_input(
            ToolInput::Point(Vec2::new(75.0, 50.0)),
            &mut typed_doc,
            &mut typed_h
        ));

        let (mut clicked, mut clicked_doc, mut clicked_h) = make();
        clicked.on_pointer_down(
            Vec2::new(50.0, 50.0),
            false,
            &mut clicked_doc,
            &mut clicked_h,
        );
        clicked.on_pointer_down(
            Vec2::new(75.0, 50.0),
            false,
            &mut clicked_doc,
            &mut clicked_h,
        );

        assert_eq!(typed_doc.entities, clicked_doc.entities);
        assert_eq!(typed.status_text(), clicked.status_text());
    }

    /// LCV-111 AC 4 — a bare distance while awaiting a radius **is** the
    /// radius, with no direction and no cursor.
    #[test]
    fn circle_distance_is_the_radius() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(50.0, 50.0), false, &mut doc, &mut h);
        let consumed = t.on_command_input(
            ToolInput::Distance {
                value_mm: 25.0,
                along: None,
            },
            &mut doc,
            &mut h,
        );
        assert!(consumed);
        assert_eq!(doc.entity_count(), 1);
        match doc.entities[0] {
            Entity::Circle(c) => {
                assert_eq!(c.center, Vec2::new(50.0, 50.0));
                assert!((c.r - 25.0).abs() < 1e-9, "radius must be exactly 25 mm");
            }
            _ => panic!("expected Entity::Circle"),
        }
        assert_eq!(t.state, CircleState::Idle);
    }

    /// LCV-111 AC 4 — a non-positive radius is refused: nothing commits and
    /// the tool stays in `WaitingRadius`.
    #[test]
    fn circle_rejects_a_non_positive_radius() {
        for value_mm in [0.0, -5.0, EPSILON] {
            let (mut t, mut doc, mut h) = make();
            t.on_pointer_down(Vec2::new(50.0, 50.0), false, &mut doc, &mut h);
            let consumed = t.on_command_input(
                ToolInput::Distance {
                    value_mm,
                    along: None,
                },
                &mut doc,
                &mut h,
            );
            assert!(!consumed, "{value_mm} must be refused");
            assert_eq!(doc.entity_count(), 0);
            assert_eq!(
                t.state,
                CircleState::WaitingRadius {
                    center: Vec2::new(50.0, 50.0)
                }
            );
        }
    }

    /// LCV-111 AC 4 — in `Idle` a `Distance` means nothing: CIRCLE needs a
    /// centre before a radius.
    #[test]
    fn circle_distance_is_ignored_while_idle() {
        let (mut t, mut doc, mut h) = make();
        let consumed = t.on_command_input(
            ToolInput::Distance {
                value_mm: 25.0,
                along: Some(Vec2::new(25.0, 0.0)),
            },
            &mut doc,
            &mut h,
        );
        assert!(!consumed);
        assert_eq!(doc.entity_count(), 0);
        assert_eq!(t.state, CircleState::Idle);
    }

    /// LCV-111 AC 5 — `anchor()` follows the last fixed point: the centre.
    #[test]
    fn anchor_follows_the_last_fixed_point() {
        let (mut t, mut doc, mut h) = make();
        assert_eq!(t.anchor(), None);
        t.on_pointer_down(Vec2::new(3.0, 4.0), false, &mut doc, &mut h);
        assert_eq!(t.anchor(), Some(Vec2::new(3.0, 4.0)));
        t.cancel();
        assert_eq!(t.anchor(), None);
    }

    /// LCV-111 AC 17 — the R14 prompt in both phases.
    #[test]
    fn status_text_follows_the_phase() {
        let (mut t, mut doc, mut h) = make();
        assert_eq!(t.status_text(), "CIRCLE Specify center point:");
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        assert_eq!(t.status_text(), "CIRCLE Specify radius:");
    }
}
