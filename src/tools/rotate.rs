//! RotateTool: rotate the selection about a base point (LCV-158).
//!
//! ## State machine
//!
//! ```text
//! Idle ──press (non-empty sel)──► WaitingAngle { base, cursor, indices, snapshots }
//! Idle ──press (empty sel)────► Idle (no-op)
//!
//! WaitingAngle ──move──────────► WaitingAngle (cursor updated)
//! WaitingAngle ──press / typed degrees (angle ≠ 0 mod 2π)──► Idle (commit TransformEntities)
//! WaitingAngle ──press / typed degrees (angle ≡ 0)──► WaitingAngle (zero-angle guard)
//! WaitingAngle ──press (sources undone/shifted)──► Idle (cancel, no commit)
//!
//! Any ──Escape──► Idle (cancel; tool stays ROTATE)
//! ```
//!
//! A picked point gives the angle of the vector base → point, CCW from +X; a
//! typed number is the angle in degrees, CCW positive. Degrees exist only at
//! that input edge. After a commit, `take_successor()` returns `SelectTool`
//! once, as MOVE does.
//!
//! MUST NOT import `eframe` or `rfd`.

use crate::app::App;
use crate::cmdline::ToolInput;
use crate::document::{Document, Entity, History, TransformEntities};
use crate::geometry::{EPSILON, Transform, Vec2};
use crate::tools::copy::sources_intact;
use crate::tools::{SelectTool, Tool};
use std::borrow::Cow;

/// Internal state of [`RotateTool`].
#[derive(Debug)]
enum RotateState {
    /// Waiting for the base point.
    Idle,
    /// Base point fixed; waiting for the rotation angle.
    WaitingAngle {
        /// The base point, the fixed point of the rotation.
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

/// ROTATE: pick a base point, then pick or type an angle; commits one
/// [`TransformEntities`] and hands control back to [`SelectTool`].
///
/// Entities must be selected before the tool activates; the selection is
/// kept after the rotation (R14 behavior).
#[derive(Debug)]
pub struct RotateTool {
    state: RotateState,
    /// `true` after a commit; cleared by the first `take_successor()`.
    pending_successor: bool,
    /// A typed value's refusal line (LCV-165 AC7); drained by `take_message()`.
    message: Option<String>,
}

impl Default for RotateTool {
    fn default() -> Self {
        Self {
            state: RotateState::Idle,
            pending_successor: false,
            message: None,
        }
    }
}

/// The line shown when a typed value is refused (LCV-165 AC7).
const REFUSAL: &str = "Rotation angle must be a finite number.";

/// The angle of `p − base`, CCW from +X; `None` when the two coincide.
fn angle_to(base: Vec2, p: Vec2) -> Option<f64> {
    let d = p - base;
    (d.length() > EPSILON).then(|| d.y.atan2(d.x))
}

impl RotateTool {
    /// Rotate the captured sources by `angle` radians about the base point.
    /// A zero turn commits nothing and keeps prompting (AC8); sources changed
    /// by an undo since the base point cancel the run.
    fn rotate_by(&mut self, angle: f64, doc: &mut Document, history: &mut History) {
        let RotateState::WaitingAngle {
            base,
            indices,
            snapshots,
            ..
        } = &self.state
        else {
            return;
        };
        if !sources_intact(doc, indices, snapshots) {
            self.cancel();
            return;
        }
        let transform = Transform::Rotate { base: *base, angle };
        if transform.is_identity() {
            return;
        }
        let cmd = TransformEntities::new(indices.clone(), transform);
        history.commit(Box::new(cmd), doc);
        self.state = RotateState::Idle;
        self.pending_successor = true;
    }
}

impl Tool for RotateTool {
    fn name(&self) -> &'static str {
        "ROTATE"
    }

    /// The R14 prompts (AC1, AC3).
    fn status_text(&self) -> Cow<'_, str> {
        match &self.state {
            RotateState::Idle => "ROTATE  Specify base point:".into(),
            RotateState::WaitingAngle { .. } => "ROTATE  Specify rotation angle:".into(),
        }
    }

    /// The base point once fixed, so `@` and polar input are relative to it.
    fn anchor(&self) -> Option<Vec2> {
        match &self.state {
            RotateState::WaitingAngle { base, .. } => Some(*base),
            RotateState::Idle => None,
        }
    }

    /// First click fixes the base point (no-op on an empty selection, AC2);
    /// second click rotates by the angle base → click (AC5).
    fn on_pointer_down(
        &mut self,
        pos: Vec2,
        _shift: bool,
        doc: &mut Document,
        history: &mut History,
    ) {
        match &self.state {
            RotateState::Idle => {
                if doc.selection.is_empty() {
                    return;
                }
                let indices: Vec<usize> = doc.selection.iter().collect();
                let snapshots = indices.iter().map(|&i| doc.entities[i]).collect();
                self.state = RotateState::WaitingAngle {
                    base: pos,
                    cursor: pos,
                    indices,
                    snapshots,
                };
            }
            RotateState::WaitingAngle { base, .. } => {
                if let Some(angle) = angle_to(*base, pos) {
                    self.rotate_by(angle, doc, history);
                }
            }
        }
    }

    /// Update the live cursor in `WaitingAngle`; no-op in `Idle`.
    fn on_pointer_move(&mut self, pos: Vec2, _doc: &mut Document) {
        if let RotateState::WaitingAngle { cursor, .. } = &mut self.state {
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

    /// The selection rotated by the angle base → cursor (AC3); the snapshots
    /// unrotated while the cursor sits on the base point; empty in `Idle`.
    fn preview(&self) -> Vec<Entity> {
        match &self.state {
            RotateState::Idle => vec![],
            RotateState::WaitingAngle {
                base,
                cursor,
                snapshots,
                ..
            } => {
                let angle = angle_to(*base, *cursor).unwrap_or(0.0);
                let transform = Transform::Rotate { base: *base, angle };
                snapshots
                    .iter()
                    .map(|e| e.transformed(&transform))
                    .collect()
            }
        }
    }

    /// Reset to `Idle` and clear the successor flag.
    fn cancel(&mut self) {
        self.state = RotateState::Idle;
        self.pending_successor = false;
    }

    /// A point is a click. At the angle prompt a bare number is the angle in
    /// degrees, CCW positive (AC4); elsewhere it is refused.
    fn on_command_input(
        &mut self,
        input: ToolInput,
        doc: &mut Document,
        history: &mut History,
    ) -> bool {
        match (input, &self.state) {
            (ToolInput::Distance { value_mm, .. }, RotateState::WaitingAngle { .. }) => {
                if !value_mm.is_finite() {
                    self.message = Some(REFUSAL.to_owned());
                    return false;
                }
                self.rotate_by(value_mm.to_radians(), doc, history);
                true
            }
            _ => match input.as_point() {
                Some(p) => {
                    self.on_pointer_down(p, false, doc, history);
                    true
                }
                None => false,
            },
        }
    }

    /// Single-shot: the refusal of the last typed value (LCV-165 AC7).
    fn take_message(&mut self) -> Option<String> {
        self.message.take()
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
    use crate::geometry::{Arc, Line};
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

    fn typed(tool: &mut RotateTool, input: ToolInput, doc: &mut Document, h: &mut History) {
        assert!(tool.on_command_input(input, doc, h), "{input:?} refused");
    }

    fn degrees(value_mm: f64) -> ToolInput {
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
        let mut tool = RotateTool::default();
        let (mut doc, mut h) = doc_selected();
        assert_eq!(tool.name(), "ROTATE");
        assert_eq!(tool.status_text(), "ROTATE  Specify base point:");
        assert_eq!(tool.anchor(), None);
        tool.on_pointer_down(Vec2::new(1.0, 2.0), false, &mut doc, &mut h);
        assert_eq!(tool.status_text(), "ROTATE  Specify rotation angle:");
        assert_eq!(tool.anchor(), Some(Vec2::new(1.0, 2.0)));
    }

    /// AC2 — with nothing selected a pick leaves the drawing unchanged.
    #[test]
    fn empty_selection_is_a_noop() {
        let mut tool = RotateTool::default();
        let (mut doc, mut h) = doc_selected();
        doc.selection.clear();
        let before = doc.entities.clone();
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        tool.on_pointer_down(Vec2::new(0.0, 5.0), false, &mut doc, &mut h);
        assert_eq!(doc.entities, before);
        assert!(!h.can_undo());
        assert_eq!(tool.status_text(), "ROTATE  Specify base point:");
    }

    /// AC3 — the preview is the selection rotated by base → cursor.
    #[test]
    fn preview_follows_the_cursor_angle() {
        let mut tool = RotateTool::default();
        let (mut doc, mut h) = doc_selected();
        assert!(tool.preview().is_empty());
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        assert_eq!(tool.preview(), doc.entities, "cursor on base: unrotated");
        tool.on_pointer_move(Vec2::new(0.0, 3.0), &mut doc);
        let t = Transform::Rotate {
            base: Vec2::new(0.0, 0.0),
            angle: FRAC_PI_2,
        };
        let want: Vec<Entity> = doc.entities.iter().map(|e| e.transformed(&t)).collect();
        assert_eq!(tool.preview(), want);
    }

    /// AC4/AC7 — a typed 90 turns the selection CCW about the base as one
    /// undo step, keeps the selection, and hands back to SELECT.
    #[test]
    fn typed_degrees_rotate_ccw() {
        let mut tool = RotateTool::default();
        let (mut doc, mut h) = doc_selected();
        let selection = doc.selection.clone();
        typed(
            &mut tool,
            ToolInput::Point(Vec2::new(0.0, 0.0)),
            &mut doc,
            &mut h,
        );
        typed(&mut tool, degrees(90.0), &mut doc, &mut h);
        let l = line_at(&doc);
        assert!(l.p1.approx_eq(Vec2::new(0.0, 10.0), EPSILON), "{l:?}");
        assert!(l.p2.approx_eq(Vec2::new(0.0, 20.0), EPSILON), "{l:?}");
        assert_eq!(h.len(), 1);
        assert_eq!(doc.selection, selection);
        assert_eq!(tool.status_text(), "ROTATE  Specify base point:");
        assert_eq!(tool.take_successor().map(|t| t.name()), Some("Select"));
        assert!(tool.take_successor().is_none());
    }

    /// AC5 — a picked point rotates by the angle of base → point.
    #[test]
    fn picked_point_gives_the_angle() {
        let mut tool = RotateTool::default();
        let (mut doc, mut h) = doc_selected();
        tool.on_pointer_down(Vec2::new(10.0, 0.0), false, &mut doc, &mut h);
        tool.on_pointer_down(Vec2::new(7.0, 0.0), false, &mut doc, &mut h);
        let l = line_at(&doc);
        assert!(l.p1.approx_eq(Vec2::new(10.0, 0.0), EPSILON), "{l:?}");
        assert!(l.p2.approx_eq(Vec2::new(0.0, 0.0), EPSILON), "{l:?}");
        assert_eq!(h.len(), 1);
    }

    /// AC8 — 0, 360, −720 and a pick on +X commit nothing; the tool keeps
    /// prompting for the angle.
    #[test]
    fn zero_angle_commits_nothing() {
        let mut tool = RotateTool::default();
        let (mut doc, mut h) = doc_selected();
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        for value in [0.0, 360.0, -720.0] {
            typed(&mut tool, degrees(value), &mut doc, &mut h);
        }
        tool.on_pointer_down(Vec2::new(4.0, 0.0), false, &mut doc, &mut h);
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        assert!(!h.can_undo());
        assert_eq!(tool.status_text(), "ROTATE  Specify rotation angle:");
        assert!(tool.take_successor().is_none());
    }

    /// A number at the base-point prompt is refused; Escape cancels and the
    /// tool stays ROTATE.
    #[test]
    fn number_at_base_refused_and_escape_cancels() {
        let mut tool = RotateTool::default();
        let (mut doc, mut h) = doc_selected();
        assert!(!tool.on_command_input(degrees(90.0), &mut doc, &mut h));
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        let mut app = App::default();
        tool.on_key(egui::Key::Escape, &mut app);
        assert_eq!(tool.name(), "ROTATE");
        assert!(tool.preview().is_empty());
        assert_eq!(tool.anchor(), None);
    }

    /// LCV-165 AC7 — a non-finite angle is refused with ROTATE's own line,
    /// handed out once through `take_message`; nothing is committed.
    #[test]
    fn non_finite_angle_leaves_its_refusal() {
        let mut tool = RotateTool::default();
        let (mut doc, mut h) = doc_selected();
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        for value in [f64::NAN, f64::INFINITY] {
            assert!(!tool.on_command_input(degrees(value), &mut doc, &mut h));
            assert_eq!(
                tool.take_message().as_deref(),
                Some("Rotation angle must be a finite number.")
            );
            assert_eq!(tool.take_message(), None, "single-shot");
        }
        assert!(!h.can_undo());
        assert_eq!(tool.status_text(), "ROTATE  Specify rotation angle:");
    }

    /// An undo past the base point cancels instead of rotating stale indices.
    #[test]
    fn changed_sources_cancel() {
        let mut tool = RotateTool::default();
        let (mut doc, mut h) = doc_selected();
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        doc.truncate_entities(1);
        let before = doc.entities.clone();
        typed(&mut tool, degrees(45.0), &mut doc, &mut h);
        assert_eq!(doc.entities, before);
        assert!(!h.can_undo());
        assert_eq!(tool.anchor(), None);
    }
}
