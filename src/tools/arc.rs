//! ArcTool: 3-point arc drawing tool.
//!
//! Click start → end → a point on the arc. Circumcenter is computed from a
//! 2×2 Cartesian linear system; CCW flag from the triangle's signed area.
//! Collinear inputs refuse to commit without mutating the document.
//!
//! MUST NOT import `eframe` or `rfd`. Introduced by demand LCV-047.

use crate::app::App;
use crate::cmdline::ToolInput;
use crate::document::{Document, Entity, History, commands::CreateArc};
use crate::geometry::{Arc, EPSILON, Line, Vec2};
use crate::tools::Tool;
use std::borrow::Cow;

/// Internal state for [`ArcTool`].
#[derive(Debug, Clone, Copy, PartialEq)]
enum ArcState {
    /// No points placed yet.
    Idle,
    /// First click recorded; waiting for the arc end point.
    WaitingEnd { start: Vec2 },
    /// First two clicks recorded; waiting for a point on the arc.
    WaitingMid { start: Vec2, end: Vec2 },
}

/// Compute arc parameters from three points via the circumcenter formula.
///
/// `a` and `c` are the arc endpoints; `b` is a point on the arc.
/// Returns `(center, r, start_angle, end_angle, ccw)` or `None` when
/// the points are collinear or degenerate (radius below [`EPSILON`]).
///
/// The circumcenter `(h, k)` solves the 2×2 system derived from
/// `|center−a|²=|center−b|²=|center−c|²` via Cramer's rule.
/// The cross product `(b−a)×(c−a)` doubles as the determinant and the
/// signed-area proxy that sets the `ccw` flag.
fn arc_from_3_points(a: Vec2, b: Vec2, c: Vec2) -> Option<(Vec2, f64, f64, f64, bool)> {
    let ux = b.x - a.x;
    let uy = b.y - a.y;
    let vx = c.x - a.x;
    let vy = c.y - a.y;
    let cross = ux * vy - uy * vx;
    if cross.abs() < EPSILON {
        return None;
    }
    let rhs1 = b.x * b.x - a.x * a.x + b.y * b.y - a.y * a.y;
    let rhs2 = c.x * c.x - a.x * a.x + c.y * c.y - a.y * a.y;
    let d = 2.0 * cross;
    let center = Vec2::new((rhs1 * vy - uy * rhs2) / d, (ux * rhs2 - rhs1 * vx) / d);
    let r = center.distance(a);
    if r < EPSILON {
        return None;
    }
    Some((
        center,
        r,
        (a.y - center.y).atan2(a.x - center.x),
        (c.y - center.y).atan2(c.x - center.x),
        cross > 0.0,
    ))
}

/// 3-point arc drawing tool.
///
/// Workflow: click start → click end → click a point on the arc.
/// Resolves the circumscribed circle and commits a [`CreateArc`] command.
/// Escape at any step cancels back to [`ArcState::Idle`].
#[derive(Debug)]
pub struct ArcTool {
    state: ArcState,
    cursor: Vec2,
}

impl Default for ArcTool {
    fn default() -> Self {
        Self {
            state: ArcState::Idle,
            cursor: Vec2::default(),
        }
    }
}

impl Tool for ArcTool {
    fn name(&self) -> &'static str {
        "ARC"
    }

    /// The R14 prompt table (LCV-111 AC 17). ARC had no override before this
    /// demand and inherited [`Tool::name`], so it showed no phase.
    fn status_text(&self) -> Cow<'_, str> {
        match self.state {
            ArcState::Idle => "ARC  Specify start point:".into(),
            ArcState::WaitingEnd { .. } => "ARC  Specify end point:".into(),
            ArcState::WaitingMid { .. } => "ARC  Specify point on arc:".into(),
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
            ArcState::Idle => self.state = ArcState::WaitingEnd { start: pos },
            ArcState::WaitingEnd { start } => {
                self.state = ArcState::WaitingMid { start, end: pos };
            }
            ArcState::WaitingMid { start, end } => {
                if let Some((center, r, sa, ea, ccw)) = arc_from_3_points(start, pos, end) {
                    history.commit(
                        Box::new(CreateArc::new(Arc::new(center, r, sa, ea, ccw))),
                        doc,
                    );
                    self.state = ArcState::Idle;
                }
                // Collinear: stay in WaitingMid so the user can try again.
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

    fn preview(&self) -> Vec<Entity> {
        match self.state {
            ArcState::Idle => vec![],
            ArcState::WaitingEnd { start } => {
                vec![Entity::Line(Line::new(start, self.cursor))]
            }
            ArcState::WaitingMid { start, end } => {
                match arc_from_3_points(start, self.cursor, end) {
                    Some((center, r, sa, ea, ccw)) => {
                        vec![Entity::Arc(Arc::new(center, r, sa, ea, ccw))]
                    }
                    None => vec![
                        Entity::Line(Line::new(start, self.cursor)),
                        Entity::Line(Line::new(self.cursor, end)),
                    ],
                }
            }
        }
    }

    fn cancel(&mut self) {
        self.state = ArcState::Idle;
    }

    /// The most recently fixed point (LCV-111 AC 5): the start while waiting
    /// for the end, the end while waiting for the point on the arc.
    ///
    /// Adding this also turns F8 ortho on for ARC, which is R14-correct — an
    /// axis-aligned chord still commits an arc.
    fn anchor(&self) -> Option<Vec2> {
        match self.state {
            ArcState::WaitingEnd { start } => Some(start),
            ArcState::WaitingMid { end, .. } => Some(end),
            ArcState::Idle => None,
        }
    }

    /// The canonical body (ADR 0003 §B3) — a typed point is exactly a click.
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
    use crate::geometry::EPSILON;
    use core::f64::consts::PI;

    fn doc_and_hist() -> (crate::document::Document, History) {
        (crate::document::Document::default(), History::default())
    }

    /// Pinned circumcenter: a=(1,0), b=(0,1), c=(−1,0) on the unit circle.
    /// Expected: center=(0,0), r=1, start_angle=0, end_angle=π, ccw=true.
    #[test]
    fn circumcenter_unit_circle() {
        let (center, r, sa, ea, ccw) = arc_from_3_points(
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(-1.0, 0.0),
        )
        .expect("non-collinear");
        assert!(
            center.approx_eq(Vec2::new(0.0, 0.0), 1e-9),
            "center {center:?}"
        );
        assert!((r - 1.0).abs() < 1e-9, "r={r}");
        assert!(sa.abs() < 1e-9, "sa={sa}");
        assert!((ea - PI).abs() < 1e-9, "ea={ea}");
        assert!(ccw);
    }

    /// Collinear inputs return `None`.
    #[test]
    fn collinear_returns_none() {
        assert!(
            arc_from_3_points(
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(2.0, 0.0)
            )
            .is_none()
        );
    }

    /// CW triangle sets ccw=false; circumcenter is equidistant from all three.
    #[test]
    fn cw_orientation_and_equidistance() {
        // CW ordering: a=(1,0), b=(−1,0), c=(0,1).
        let (_, _, _, _, ccw) = arc_from_3_points(
            Vec2::new(1.0, 0.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(0.0, 1.0),
        )
        .expect("non-collinear");
        assert!(!ccw);

        // Equidistance on a 3-4-5 triangle's circumscribed circle.
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(4.0, 0.0);
        let c = Vec2::new(0.0, 3.0);
        let (center, r, _, _, _) = arc_from_3_points(a, b, c).expect("non-collinear");
        for p in [a, b, c] {
            assert!((center.distance(p) - r).abs() < EPSILON);
        }
    }

    /// name() is exactly "ARC", idle preview is empty.
    #[test]
    fn name_and_idle_preview() {
        let t = ArcTool::default();
        assert_eq!(t.name(), "ARC");
        assert!(t.preview().is_empty());
    }

    /// After first click, preview is one Line; after second click with
    /// non-collinear cursor, preview is one Arc.
    #[test]
    fn preview_progresses_through_states() {
        let (mut doc, mut hist) = doc_and_hist();
        let mut t = ArcTool::default();

        t.on_pointer_down(Vec2::new(1.0, 0.0), false, &mut doc, &mut hist);
        t.on_pointer_move(Vec2::new(0.0, 0.0), &mut doc);
        assert!(matches!(t.preview()[0], Entity::Line(_)));

        t.on_pointer_down(Vec2::new(-1.0, 0.0), false, &mut doc, &mut hist);
        t.on_pointer_move(Vec2::new(0.0, 1.0), &mut doc);
        let p = t.preview();
        assert_eq!(p.len(), 1);
        assert!(matches!(p[0], Entity::Arc(_)));
    }

    /// Collinear cursor in WaitingMid produces two guide Lines.
    #[test]
    fn collinear_cursor_shows_two_lines() {
        let (mut doc, mut hist) = doc_and_hist();
        let mut t = ArcTool::default();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut hist);
        t.on_pointer_down(Vec2::new(2.0, 0.0), false, &mut doc, &mut hist);
        t.on_pointer_move(Vec2::new(1.0, 0.0), &mut doc);
        let p = t.preview();
        assert_eq!(p.len(), 2);
        assert!(p.iter().all(|e| matches!(e, Entity::Line(_))));
    }

    /// Third valid click commits Arc and resets to Idle.
    #[test]
    fn third_click_commits_and_resets() {
        let (mut doc, mut hist) = doc_and_hist();
        let mut t = ArcTool::default();
        t.on_pointer_down(Vec2::new(1.0, 0.0), false, &mut doc, &mut hist);
        t.on_pointer_down(Vec2::new(-1.0, 0.0), false, &mut doc, &mut hist);
        t.on_pointer_down(Vec2::new(0.0, 1.0), false, &mut doc, &mut hist);
        assert_eq!(doc.entity_count(), 1);
        assert!(matches!(doc.entities[0], Entity::Arc(_)));
        assert_eq!(t.state, ArcState::Idle);
    }

    /// Collinear third click does not commit; stays in WaitingMid.
    #[test]
    fn collinear_third_click_stays() {
        let (mut doc, mut hist) = doc_and_hist();
        let mut t = ArcTool::default();
        let start = Vec2::new(0.0, 0.0);
        let end = Vec2::new(2.0, 0.0);
        t.on_pointer_down(start, false, &mut doc, &mut hist);
        t.on_pointer_down(end, false, &mut doc, &mut hist);
        t.on_pointer_down(Vec2::new(1.0, 0.0), false, &mut doc, &mut hist);
        assert_eq!(doc.entity_count(), 0);
        assert_eq!(t.state, ArcState::WaitingMid { start, end });
    }

    /// Escape resets to Idle; undo/redo round-trip works.
    #[test]
    fn escape_and_undo_redo() {
        let (mut doc, mut hist) = doc_and_hist();
        let mut t = ArcTool::default();
        let mut app = crate::app::App::default();

        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut hist);
        t.on_key(egui::Key::Escape, &mut app);
        assert_eq!(t.state, ArcState::Idle);

        // Full commit → undo → redo.
        t.on_pointer_down(Vec2::new(1.0, 0.0), false, &mut doc, &mut hist);
        t.on_pointer_down(Vec2::new(-1.0, 0.0), false, &mut doc, &mut hist);
        t.on_pointer_down(Vec2::new(0.0, 1.0), false, &mut doc, &mut hist);
        assert_eq!(doc.entity_count(), 1);
        hist.undo(&mut doc);
        assert_eq!(doc.entity_count(), 0);
        hist.redo(&mut doc);
        assert_eq!(doc.entity_count(), 1);
    }

    /// Object safety: ArcTool fits in a Box<dyn Tool>.
    #[test]
    fn object_safe() {
        let _: Box<dyn Tool> = Box::new(ArcTool::default());
    }

    /// LCV-111 AC 3 — three typed points draw exactly the arc three clicks
    /// draw.
    #[test]
    fn command_point_acts_exactly_like_a_click() {
        let pts = [
            Vec2::new(1.0, 0.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(0.0, 1.0),
        ];

        let mut typed = ArcTool::default();
        let (mut typed_doc, mut typed_h) = doc_and_hist();
        for p in pts {
            assert!(typed.on_command_input(ToolInput::Point(p), &mut typed_doc, &mut typed_h));
        }

        let mut clicked = ArcTool::default();
        let (mut clicked_doc, mut clicked_h) = doc_and_hist();
        for p in pts {
            clicked.on_pointer_down(p, false, &mut clicked_doc, &mut clicked_h);
        }

        assert_eq!(typed_doc.entities, clicked_doc.entities);
        assert_eq!(typed_doc.entity_count(), 1);
        assert_eq!(typed_h.len(), 1);
        assert_eq!(typed.state, ArcState::Idle);
    }

    /// LCV-111 AC 5 — `anchor()` follows the last fixed point through both
    /// in-progress phases.
    #[test]
    fn anchor_follows_the_last_fixed_point() {
        let mut t = ArcTool::default();
        let (mut doc, mut h) = doc_and_hist();
        assert_eq!(t.anchor(), None, "idle ARC has no anchor");

        t.on_pointer_down(Vec2::new(1.0, 0.0), false, &mut doc, &mut h);
        assert_eq!(t.anchor(), Some(Vec2::new(1.0, 0.0)), "WaitingEnd → start");

        t.on_pointer_down(Vec2::new(-1.0, 0.0), false, &mut doc, &mut h);
        assert_eq!(t.anchor(), Some(Vec2::new(-1.0, 0.0)), "WaitingMid → end");

        t.cancel();
        assert_eq!(t.anchor(), None);
    }

    /// LCV-111 AC 17 — the R14 prompt in all three phases.
    #[test]
    fn status_text_follows_the_phase() {
        let mut t = ArcTool::default();
        let (mut doc, mut h) = doc_and_hist();
        assert_eq!(t.status_text(), "ARC  Specify start point:");
        t.on_pointer_down(Vec2::new(1.0, 0.0), false, &mut doc, &mut h);
        assert_eq!(t.status_text(), "ARC  Specify end point:");
        t.on_pointer_down(Vec2::new(-1.0, 0.0), false, &mut doc, &mut h);
        assert_eq!(t.status_text(), "ARC  Specify point on arc:");
    }
}
