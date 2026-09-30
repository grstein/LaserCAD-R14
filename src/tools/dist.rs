//! DistTool: the DIST query — two points in, one result line out (LCV-159).
//!
//! ## State machine
//!
//! ```text
//! Idle ──press──► WaitingSecond { first }
//! WaitingSecond ──press──► Idle (message + successor SELECT)
//! Any ──Escape──► Idle (no message)
//! ```
//!
//! A query: it never touches `Document` or `History`. The result travels
//! through [`Tool::take_message`] and the hand-over to SELECT through
//! [`Tool::take_successor`]; the app drains both in that order.
//!
//! MUST NOT import `eframe` or `rfd`.

use crate::app::App;
use crate::cmdline::ToolInput;
use crate::document::{Document, Entity, History};
use crate::geometry::Vec2;
use crate::tools::{SelectTool, Tool};
use std::borrow::Cow;

/// Internal state of [`DistTool`].
#[derive(Debug, Clone, Copy, PartialEq)]
enum DistState {
    /// Waiting for the first point.
    Idle,
    /// First point fixed; it is the anchor for `@` and polar input.
    WaitingSecond {
        /// The first point, world mm.
        first: Vec2,
    },
}

/// DIST: reports the distance, angle, ΔX and ΔY between two points, then
/// hands back to SELECT.
#[derive(Debug)]
pub struct DistTool {
    state: DistState,
    /// The result line, until the app drains it.
    message: Option<String>,
    /// Set with the result; drained by `take_successor`.
    done: bool,
}

impl Default for DistTool {
    fn default() -> Self {
        Self {
            state: DistState::Idle,
            message: None,
            done: false,
        }
    }
}

impl Tool for DistTool {
    fn name(&self) -> &'static str {
        "DIST"
    }

    fn status_text(&self) -> Cow<'_, str> {
        match self.state {
            DistState::Idle => "DIST Specify first point:".into(),
            DistState::WaitingSecond { .. } => "DIST Specify second point:".into(),
        }
    }

    /// The first point, once given: `@dx,dy` and `@d<a` are relative to it.
    fn anchor(&self) -> Option<Vec2> {
        match self.state {
            DistState::WaitingSecond { first } => Some(first),
            DistState::Idle => None,
        }
    }

    fn on_pointer_down(
        &mut self,
        pos: Vec2,
        _shift: bool,
        _doc: &mut Document,
        _history: &mut History,
    ) {
        match self.state {
            DistState::Idle => self.state = DistState::WaitingSecond { first: pos },
            DistState::WaitingSecond { first } => {
                self.message = Some(report(first, pos));
                self.done = true;
                self.state = DistState::Idle;
            }
        }
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

    fn on_key(&mut self, key: egui::Key, _app: &mut App) {
        if key == egui::Key::Escape {
            self.cancel();
        }
    }

    fn preview(&self) -> Vec<Entity> {
        vec![]
    }

    fn cancel(&mut self) {
        self.state = DistState::Idle;
        self.message = None;
        self.done = false;
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

    fn take_successor(&mut self) -> Option<Box<dyn Tool>> {
        if std::mem::take(&mut self.done) {
            Some(Box::new(SelectTool::default()))
        } else {
            None
        }
    }

    fn take_message(&mut self) -> Option<String> {
        self.message.take()
    }
}

/// The DIST line for `first → second`: mm and degrees with 3 decimals, the
/// angle CCW from +X in [0, 360).
fn report(first: Vec2, second: Vec2) -> String {
    let d = second - first;
    let angle = d.y.atan2(d.x).to_degrees().rem_euclid(360.0);
    format!(
        "Distance = {}, Angle = {}°, Delta X = {}, Delta Y = {}",
        fixed3(d.length()),
        degrees3(angle),
        fixed3(d.x),
        fixed3(d.y)
    )
}

/// `v` with 3 decimals, never `-0.000`.
fn fixed3(v: f64) -> String {
    let s = format!("{v:.3}");
    if s == "-0.000" { "0.000".to_owned() } else { s }
}

/// An angle in [0, 360) with 3 decimals: a value that rounds up to 360 is 0.
fn degrees3(a: f64) -> String {
    let s = fixed3(a);
    if s == "360.000" {
        "0.000".to_owned()
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Line;

    fn click(tool: &mut DistTool, p: Vec2, doc: &mut Document, hist: &mut History) {
        tool.on_pointer_down(p, false, doc, hist);
    }

    /// AC6 — the two prompts, and the first point is the anchor.
    #[test]
    fn prompts_and_anchor_follow_the_state() {
        let mut tool = DistTool::default();
        let (mut doc, mut hist) = (Document::default(), History::default());
        assert_eq!(tool.name(), "DIST");
        assert_eq!(tool.status_text(), "DIST Specify first point:");
        assert_eq!(tool.anchor(), None);
        click(&mut tool, Vec2::new(1.0, 2.0), &mut doc, &mut hist);
        assert_eq!(tool.status_text(), "DIST Specify second point:");
        assert_eq!(tool.anchor(), Some(Vec2::new(1.0, 2.0)));
    }

    /// AC7 — the second point yields one message and a SELECT successor,
    /// both single-shot, and the drawing and history stay untouched.
    #[test]
    fn second_point_reports_and_hands_back_to_select() {
        let mut tool = DistTool::default();
        let mut doc = Document::default();
        doc.push_current(Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(5.0, 0.0),
        )));
        let mut hist = History::default();
        let typed =
            tool.on_command_input(ToolInput::Point(Vec2::new(0.0, 0.0)), &mut doc, &mut hist);
        assert!(typed);
        assert_eq!(tool.take_message(), None);
        assert!(tool.take_successor().is_none());
        click(&mut tool, Vec2::new(30.0, 40.0), &mut doc, &mut hist);
        assert_eq!(
            tool.take_message().as_deref(),
            Some("Distance = 50.000, Angle = 53.130°, Delta X = 30.000, Delta Y = 40.000")
        );
        assert_eq!(tool.take_message(), None, "single-shot");
        assert_eq!(tool.take_successor().map(|t| t.name()), Some("Select"));
        assert!(tool.take_successor().is_none(), "single-shot");
        assert_eq!(doc.entity_count(), 1);
        assert_eq!(hist.len(), 0);
    }

    /// AC7 — negative deltas, angle wrap into [0, 360), no `-0.000`.
    #[test]
    fn report_formats_signs_and_wraps_the_angle() {
        let o = Vec2::new(0.0, 0.0);
        let a = 45f64.to_radians();
        assert_eq!(
            report(o, Vec2::new(10.0 * a.cos(), -10.0 * a.sin())),
            "Distance = 10.000, Angle = 315.000°, Delta X = 7.071, Delta Y = -7.071"
        );
        assert_eq!(
            report(o, Vec2::new(-5.0, 0.0)),
            "Distance = 5.000, Angle = 180.000°, Delta X = -5.000, Delta Y = 0.000"
        );
        assert_eq!(
            report(o, Vec2::new(0.0, 2.0)),
            "Distance = 2.000, Angle = 90.000°, Delta X = 0.000, Delta Y = 2.000"
        );
        assert_eq!(
            report(o, o),
            "Distance = 0.000, Angle = 0.000°, Delta X = 0.000, Delta Y = 0.000"
        );
        assert_eq!(fixed3(-0.0), "0.000");
        assert_eq!(fixed3(-0.0004), "0.000");
        assert_eq!(degrees3(0.0), "0.000");
        assert_eq!(degrees3(359.9996), "0.000");
        assert_ne!(degrees3(359.9995), "360.000");
    }

    /// AC6 — Escape drops the first point and leaves nothing to report.
    #[test]
    fn escape_cancels() {
        let mut tool = DistTool::default();
        let (mut doc, mut hist) = (Document::default(), History::default());
        click(&mut tool, Vec2::new(1.0, 1.0), &mut doc, &mut hist);
        tool.on_key(egui::Key::Escape, &mut App::default());
        assert_eq!(tool.status_text(), "DIST Specify first point:");
        assert_eq!(tool.anchor(), None);
        assert_eq!(tool.take_message(), None);
        assert!(tool.take_successor().is_none());
    }
}
