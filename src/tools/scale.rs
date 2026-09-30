//! ScaleTool: scale the selection uniformly about a base point (LCV-182).
//!
//! ## State machine
//!
//! ```text
//! Idle ──press (non-empty sel)──► WaitingFactor { base, cursor, indices, snapshots }
//! Idle ──press (empty sel)────► Idle (no-op)
//!
//! WaitingFactor ──move──────────► WaitingFactor (cursor updated)
//! WaitingFactor ──press / typed factor (> 0, ≠ 1)──► Idle (commit TransformEntities)
//! WaitingFactor ──press / typed factor (≈ 1)──► WaitingFactor (identity guard)
//! WaitingFactor ──typed factor (≤ 0, not finite) / point on base──► WaitingFactor (refused)
//! WaitingFactor ──press (sources undone/shifted)──► Idle (cancel, no commit)
//!
//! Any ──Escape──► Idle (cancel; tool stays SCALE)
//! ```
//!
//! A picked point gives the factor as its distance from the base point in
//! mm; a typed number is the factor itself. After a commit,
//! `take_successor()` returns `SelectTool` once, as ROTATE does.
//!
//! MUST NOT import `eframe` or `rfd`.

use crate::app::App;
use crate::cmdline::ToolInput;
use crate::document::{Document, Entity, History, TransformEntities};
use crate::geometry::{Transform, Vec2};
use crate::tools::copy::sources_intact;
use crate::tools::{SelectTool, Tool};
use std::borrow::Cow;

/// Internal state of [`ScaleTool`].
#[derive(Debug)]
enum ScaleState {
    /// Waiting for the base point.
    Idle,
    /// Base point fixed; waiting for the scale factor.
    WaitingFactor {
        /// The base point, the fixed point of the scale.
        base: Vec2,
        /// Current cursor position; updated by `on_pointer_move`.
        cursor: Vec2,
        /// The selected indices, captured with the base point.
        indices: Vec<usize>,
        /// Copies of the selected entities, for the preview and the
        /// undo-past-the-base-point guard.
        snapshots: Vec<Entity>,
    },
}

/// SCALE: pick a base point, then pick or type a factor; commits one
/// [`TransformEntities`] and hands control back to [`SelectTool`].
///
/// Entities must be selected before the tool activates; the selection is
/// kept after the scale (R14 behavior).
#[derive(Debug)]
pub struct ScaleTool {
    state: ScaleState,
    /// `true` after a commit; cleared by the first `take_successor()`.
    pending_successor: bool,
}

impl Default for ScaleTool {
    fn default() -> Self {
        Self {
            state: ScaleState::Idle,
            pending_successor: false,
        }
    }
}

/// `true` for a factor SCALE accepts: finite and positive (AC5).
fn accepted(factor: f64) -> bool {
    factor.is_finite() && factor > 0.0
}

impl ScaleTool {
    /// The factor a pick at `p` gives: its distance from the base point.
    /// `None` in `Idle`.
    fn picked_factor(&self, p: Vec2) -> Option<f64> {
        match &self.state {
            ScaleState::WaitingFactor { base, .. } => Some(base.distance(p)),
            ScaleState::Idle => None,
        }
    }

    /// Scale the captured sources by `factor` about the base point. Returns
    /// `false` for a refused factor (AC5); a factor of 1 commits nothing and
    /// keeps prompting (AC6); sources changed by an undo since the base point
    /// cancel the run.
    fn scale_by(&mut self, factor: f64, doc: &mut Document, history: &mut History) -> bool {
        if !accepted(factor) {
            return false;
        }
        let ScaleState::WaitingFactor {
            base,
            indices,
            snapshots,
            ..
        } = &self.state
        else {
            return false;
        };
        if !sources_intact(doc, indices, snapshots) {
            self.cancel();
            return true;
        }
        let transform = Transform::Scale {
            base: *base,
            factor,
        };
        if transform.is_identity() {
            return true;
        }
        let cmd = TransformEntities::new(indices.clone(), transform);
        history.commit(Box::new(cmd), doc);
        self.state = ScaleState::Idle;
        self.pending_successor = true;
        true
    }
}

impl Tool for ScaleTool {
    fn name(&self) -> &'static str {
        "SCALE"
    }

    /// The R14 prompts (AC1, AC3).
    fn status_text(&self) -> Cow<'_, str> {
        match &self.state {
            ScaleState::Idle => "SCALE Specify base point:".into(),
            ScaleState::WaitingFactor { .. } => "SCALE Specify scale factor:".into(),
        }
    }

    /// The base point once fixed, so `@` and polar input are relative to it.
    fn anchor(&self) -> Option<Vec2> {
        match &self.state {
            ScaleState::WaitingFactor { base, .. } => Some(*base),
            ScaleState::Idle => None,
        }
    }

    /// First click fixes the base point (no-op on an empty selection, AC2);
    /// second click scales by the distance base → click (AC4). A click on
    /// the base point gives factor 0 and is ignored (AC5).
    fn on_pointer_down(
        &mut self,
        pos: Vec2,
        _shift: bool,
        doc: &mut Document,
        history: &mut History,
    ) {
        match self.picked_factor(pos) {
            Some(factor) => {
                self.scale_by(factor, doc, history);
            }
            None => {
                if doc.selection.is_empty() {
                    return;
                }
                let indices: Vec<usize> = doc.selection.iter().collect();
                let snapshots = indices.iter().map(|&i| doc.entities[i]).collect();
                self.state = ScaleState::WaitingFactor {
                    base: pos,
                    cursor: pos,
                    indices,
                    snapshots,
                };
            }
        }
    }

    /// Update the live cursor in `WaitingFactor`; no-op in `Idle`.
    fn on_pointer_move(&mut self, pos: Vec2, _doc: &mut Document) {
        if let ScaleState::WaitingFactor { cursor, .. } = &mut self.state {
            *cursor = pos;
        }
    }

    /// Always a no-op.
    fn on_pointer_up(
        &mut self,
        _pos: Vec2,
        _shift: bool,
        _doc: &mut Document,
        _history: &mut History,
    ) {
    }

    /// Escape → `cancel()`. Does **not** switch the active tool.
    fn on_key(&mut self, key: egui::Key, _app: &mut App) {
        if key == egui::Key::Escape {
            self.cancel();
        }
    }

    /// The selection scaled by the distance base → cursor (AC3); the
    /// snapshots unscaled while the cursor sits on the base point; empty in
    /// `Idle`.
    fn preview(&self) -> Vec<Entity> {
        match &self.state {
            ScaleState::Idle => vec![],
            ScaleState::WaitingFactor {
                base,
                cursor,
                snapshots,
                ..
            } => {
                let factor = base.distance(*cursor);
                if !accepted(factor) {
                    return snapshots.clone();
                }
                let transform = Transform::Scale {
                    base: *base,
                    factor,
                };
                snapshots
                    .iter()
                    .map(|e| e.transformed(&transform))
                    .collect()
            }
        }
    }

    /// Reset to `Idle` and clear the successor flag.
    fn cancel(&mut self) {
        self.state = ScaleState::Idle;
        self.pending_successor = false;
    }

    /// A point is a click, except that a point on the base is refused. At
    /// the factor prompt a bare number is the factor (AC4); zero, negative
    /// or non-finite is refused (AC5); elsewhere a number is refused.
    fn on_command_input(
        &mut self,
        input: ToolInput,
        doc: &mut Document,
        history: &mut History,
    ) -> bool {
        match (input, &self.state) {
            (ToolInput::Distance { value_mm, .. }, ScaleState::WaitingFactor { .. }) => {
                self.scale_by(value_mm, doc, history)
            }
            _ => match input.as_point() {
                Some(p) => match self.picked_factor(p) {
                    Some(factor) => self.scale_by(factor, doc, history),
                    None => {
                        self.on_pointer_down(p, false, doc, history);
                        true
                    }
                },
                None => false,
            },
        }
    }

    /// Single-shot: `Some(SelectTool)` once after a commit.
    fn take_successor(&mut self) -> Option<Box<dyn Tool>> {
        if self.pending_successor {
            self.pending_successor = false;
            Some(Box::new(SelectTool::default()))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Arc, EPSILON, Line};
    use core::f64::consts::FRAC_PI_2;

    /// A line (10,0)→(20,0) and an arc, both selected.
    fn doc_selected() -> (Document, History) {
        let mut doc = Document::default();
        doc.push_current(Entity::Line(Line::new(
            Vec2::new(10.0, 0.0),
            Vec2::new(20.0, 0.0),
        )));
        doc.push_current(Entity::Arc(Arc::new(
            Vec2::new(5.0, 5.0),
            2.0,
            0.0,
            FRAC_PI_2,
            true,
        )));
        doc.selection.add(0);
        doc.selection.add(1);
        (doc, History::default())
    }

    fn typed(tool: &mut ScaleTool, input: ToolInput, doc: &mut Document, h: &mut History) {
        assert!(tool.on_command_input(input, doc, h), "{input:?} refused");
    }

    fn number(value_mm: f64) -> ToolInput {
        ToolInput::Distance {
            value_mm,
            along: None,
        }
    }

    fn line_at(doc: &Document) -> Line {
        match doc.entities[0] {
            Entity::Line(l) => l,
            other => panic!("expected a line, got {other:?}"),
        }
    }

    /// AC1/AC3 — the two R14 prompts; the base becomes the anchor.
    #[test]
    fn prompts_and_anchor() {
        let mut tool = ScaleTool::default();
        let (mut doc, mut h) = doc_selected();
        assert_eq!(tool.name(), "SCALE");
        assert_eq!(tool.status_text(), "SCALE Specify base point:");
        assert_eq!(tool.anchor(), None);
        tool.on_pointer_down(Vec2::new(1.0, 2.0), false, &mut doc, &mut h);
        assert_eq!(tool.status_text(), "SCALE Specify scale factor:");
        assert_eq!(tool.anchor(), Some(Vec2::new(1.0, 2.0)));
    }

    /// AC2 — with nothing selected a pick leaves the drawing unchanged.
    #[test]
    fn empty_selection_is_a_noop() {
        let mut tool = ScaleTool::default();
        let (mut doc, mut h) = doc_selected();
        doc.selection.clear();
        let before = doc.entities.clone();
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        tool.on_pointer_down(Vec2::new(0.0, 5.0), false, &mut doc, &mut h);
        assert_eq!(doc.entities, before);
        assert!(!h.can_undo());
        assert_eq!(tool.status_text(), "SCALE Specify base point:");
    }

    /// AC3 — the preview is the selection scaled by the distance base →
    /// cursor, and unscaled while the cursor sits on the base.
    #[test]
    fn preview_follows_the_cursor_distance() {
        let mut tool = ScaleTool::default();
        let (mut doc, mut h) = doc_selected();
        assert!(tool.preview().is_empty());
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        assert_eq!(tool.preview(), doc.entities, "cursor on base: unscaled");
        tool.on_pointer_move(Vec2::new(0.0, 3.0), &mut doc);
        let t = Transform::Scale {
            base: Vec2::new(0.0, 0.0),
            factor: 3.0,
        };
        let want: Vec<Entity> = doc.entities.iter().map(|e| e.transformed(&t)).collect();
        assert_eq!(tool.preview(), want);
    }

    /// AC4/AC7 — a typed 2 doubles the selection about the base as one undo
    /// step, keeps the selection, and hands back to SELECT.
    #[test]
    fn typed_factor_scales() {
        let mut tool = ScaleTool::default();
        let (mut doc, mut h) = doc_selected();
        let selection = doc.selection.clone();
        typed(
            &mut tool,
            ToolInput::Point(Vec2::new(0.0, 0.0)),
            &mut doc,
            &mut h,
        );
        typed(&mut tool, number(2.0), &mut doc, &mut h);
        let l = line_at(&doc);
        assert!(l.p1.approx_eq(Vec2::new(20.0, 0.0), EPSILON), "{l:?}");
        assert!(l.p2.approx_eq(Vec2::new(40.0, 0.0), EPSILON), "{l:?}");
        match doc.entities[1] {
            Entity::Arc(a) => {
                assert!(a.center.approx_eq(Vec2::new(10.0, 10.0), EPSILON));
                assert_eq!(a.r, 4.0);
                assert_eq!((a.start_angle, a.end_angle, a.ccw), (0.0, FRAC_PI_2, true));
            }
            other => panic!("expected an arc, got {other:?}"),
        }
        assert_eq!(h.len(), 1);
        assert_eq!(doc.selection, selection);
        assert_eq!(tool.status_text(), "SCALE Specify base point:");
        assert_eq!(tool.take_successor().map(|t| t.name()), Some("Select"));
        assert!(tool.take_successor().is_none());
    }

    /// AC4 — a picked point scales by its distance from the base.
    #[test]
    fn picked_point_gives_the_factor() {
        let mut tool = ScaleTool::default();
        let (mut doc, mut h) = doc_selected();
        tool.on_pointer_down(Vec2::new(10.0, 0.0), false, &mut doc, &mut h);
        tool.on_pointer_down(Vec2::new(10.0, 0.5), false, &mut doc, &mut h);
        let l = line_at(&doc);
        assert!(l.p1.approx_eq(Vec2::new(10.0, 0.0), EPSILON), "{l:?}");
        assert!(l.p2.approx_eq(Vec2::new(15.0, 0.0), EPSILON), "{l:?}");
        assert_eq!(h.len(), 1);
    }

    /// AC5 — zero, negative and non-finite factors and a typed point on the
    /// base are refused; a click on the base is ignored; the tool keeps
    /// prompting for the factor.
    #[test]
    fn bad_factors_are_refused() {
        let mut tool = ScaleTool::default();
        let (mut doc, mut h) = doc_selected();
        tool.on_pointer_down(Vec2::new(1.0, 1.0), false, &mut doc, &mut h);
        for value in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(!tool.on_command_input(number(value), &mut doc, &mut h));
        }
        let on_base = ToolInput::Point(Vec2::new(1.0, 1.0));
        assert!(!tool.on_command_input(on_base, &mut doc, &mut h));
        tool.on_pointer_down(Vec2::new(1.0, 1.0), false, &mut doc, &mut h);
        assert!(!h.can_undo());
        assert_eq!(tool.status_text(), "SCALE Specify scale factor:");
    }

    /// AC6 — a factor of 1, typed or picked, commits nothing; the tool keeps
    /// prompting for the factor.
    #[test]
    fn factor_one_commits_nothing() {
        let mut tool = ScaleTool::default();
        let (mut doc, mut h) = doc_selected();
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        typed(&mut tool, number(1.0), &mut doc, &mut h);
        typed(&mut tool, number(1.0 + EPSILON / 2.0), &mut doc, &mut h);
        tool.on_pointer_down(Vec2::new(0.0, 1.0), false, &mut doc, &mut h);
        assert!(!h.can_undo());
        assert_eq!(tool.status_text(), "SCALE Specify scale factor:");
        assert!(tool.take_successor().is_none());
    }

    /// A number at the base-point prompt is refused; Escape cancels and the
    /// tool stays SCALE.
    #[test]
    fn number_at_base_refused_and_escape_cancels() {
        let mut tool = ScaleTool::default();
        let (mut doc, mut h) = doc_selected();
        assert!(!tool.on_command_input(number(2.0), &mut doc, &mut h));
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        let mut app = App::default();
        tool.on_key(egui::Key::Escape, &mut app);
        assert_eq!(tool.name(), "SCALE");
        assert!(tool.preview().is_empty());
        assert_eq!(tool.anchor(), None);
    }

    /// An undo past the base point cancels instead of scaling stale indices.
    #[test]
    fn changed_sources_cancel() {
        let mut tool = ScaleTool::default();
        let (mut doc, mut h) = doc_selected();
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        doc.truncate_entities(1);
        let before = doc.entities.clone();
        typed(&mut tool, number(2.0), &mut doc, &mut h);
        assert_eq!(doc.entities, before);
        assert!(!h.can_undo());
        assert_eq!(tool.anchor(), None);
    }
}
