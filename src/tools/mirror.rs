//! MirrorTool: reflect the selection across a picked line (LCV-181).
//!
//! ## State machine
//!
//! ```text
//! FirstPoint ──press (non-empty sel)──► SecondPoint { a, cursor, indices, snapshots }
//! FirstPoint ──press (empty sel)────► FirstPoint (no-op)
//!
//! SecondPoint ──move─────────────────► SecondPoint (cursor updated)
//! SecondPoint ──press (b ≠ a)────────► Confirm { transform, indices, snapshots }
//! SecondPoint ──press (b = a)────────► SecondPoint (ignored)
//!
//! Confirm ──raw "", n, no──► FirstPoint (commit, keep sources)
//! Confirm ──raw y, yes─────► FirstPoint (commit, replace sources)
//! Confirm ──raw other──────► Confirm (refused)
//! Confirm ──answer (sources undone/shifted)──► FirstPoint (cancel, no commit)
//!
//! Any ──Escape──► FirstPoint (cancel; tool stays MIRROR)
//! ```
//!
//! At `Confirm` the tool asks for raw input (ADR 0003 §D, the TEXT
//! precedent), so the Yes/No answer reaches [`Tool::on_raw_input`] unparsed
//! and `n` is never read as a command word. After a commit,
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

/// Internal state of [`MirrorTool`].
#[derive(Debug)]
enum MirrorState {
    /// Waiting for the first point of the mirror line.
    FirstPoint,
    /// First point fixed; waiting for the second.
    SecondPoint {
        /// The first point of the mirror line.
        a: Vec2,
        /// Current cursor position; updated by `on_pointer_move`.
        cursor: Vec2,
        /// The selected indices, captured with the first point.
        indices: Vec<usize>,
        /// Copies of the selected entities, for the preview and the
        /// undo-past-the-first-point guard.
        snapshots: Vec<Entity>,
    },
    /// Mirror line fixed; waiting for the erase-source answer.
    Confirm {
        /// The mirror across the fixed line.
        transform: Transform,
        /// The selected indices, captured with the first point.
        indices: Vec<usize>,
        /// Copies of the selected entities, as in `SecondPoint`.
        snapshots: Vec<Entity>,
    },
}

/// MIRROR: pick two points of a mirror line, then answer whether to erase
/// the sources; commits one [`TransformEntities`] and hands control back to
/// [`SelectTool`].
///
/// Entities must be selected before the tool activates. The selection is
/// kept on the same indices: the sources after a keep-source mirror, the
/// mirrored geometry after an erase-source one.
#[derive(Debug)]
pub struct MirrorTool {
    state: MirrorState,
    /// `true` after a commit; cleared by the first `take_successor()`.
    pending_successor: bool,
}

impl Default for MirrorTool {
    fn default() -> Self {
        Self {
            state: MirrorState::FirstPoint,
            pending_successor: false,
        }
    }
}

/// The answer to the erase-source prompt: `Some(true)` erases, `Some(false)`
/// keeps (blank Enter is the default No), `None` is not an answer.
fn erase_answer(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => Some(true),
        "" | "n" | "no" => Some(false),
        _ => None,
    }
}

impl Tool for MirrorTool {
    fn name(&self) -> &'static str {
        "MIRROR"
    }

    /// The R14 prompts (AC1, AC3, AC5).
    fn status_text(&self) -> Cow<'_, str> {
        match &self.state {
            MirrorState::FirstPoint => "MIRROR  Specify first point of mirror line:".into(),
            MirrorState::SecondPoint { .. } => {
                "MIRROR  Specify second point of mirror line:".into()
            }
            MirrorState::Confirm { .. } => "MIRROR  Erase source objects? [Yes/No] <N>:".into(),
        }
    }

    /// The first point while picking the second, so `@` and polar input are
    /// relative to it.
    fn anchor(&self) -> Option<Vec2> {
        match &self.state {
            MirrorState::SecondPoint { a, .. } => Some(*a),
            _ => None,
        }
    }

    /// Raw only at the Yes/No prompt (ADR 0003 §D).
    fn wants_raw_input(&self) -> bool {
        matches!(self.state, MirrorState::Confirm { .. })
    }

    /// First click fixes the first point (no-op on an empty selection, AC2);
    /// second click fixes the line unless it coincides with the first (AC4).
    fn on_pointer_down(
        &mut self,
        pos: Vec2,
        _shift: bool,
        doc: &mut Document,
        _history: &mut History,
    ) {
        match &mut self.state {
            MirrorState::FirstPoint => {
                if doc.selection.is_empty() {
                    return;
                }
                let indices: Vec<usize> = doc.selection.iter().collect();
                let snapshots = indices.iter().map(|&i| doc.entities[i]).collect();
                self.state = MirrorState::SecondPoint {
                    a: pos,
                    cursor: pos,
                    indices,
                    snapshots,
                };
            }
            MirrorState::SecondPoint {
                a,
                indices,
                snapshots,
                ..
            } => {
                let transform = Transform::Mirror { a: *a, b: pos };
                if transform.is_identity() {
                    return;
                }
                self.state = MirrorState::Confirm {
                    transform,
                    indices: std::mem::take(indices),
                    snapshots: std::mem::take(snapshots),
                };
            }
            MirrorState::Confirm { .. } => {}
        }
    }

    /// Update the live cursor in `SecondPoint`; no-op otherwise.
    fn on_pointer_move(&mut self, pos: Vec2, _doc: &mut Document) {
        if let MirrorState::SecondPoint { cursor, .. } = &mut self.state {
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

    /// The selection mirrored across first point → cursor (AC3), or across
    /// the fixed line at the Yes/No prompt; the snapshots unmirrored while
    /// the cursor sits on the first point; empty before the first point.
    fn preview(&self) -> Vec<Entity> {
        let (transform, snapshots) = match &self.state {
            MirrorState::FirstPoint => return vec![],
            MirrorState::SecondPoint {
                a,
                cursor,
                snapshots,
                ..
            } => (Transform::Mirror { a: *a, b: *cursor }, snapshots),
            MirrorState::Confirm {
                transform,
                snapshots,
                ..
            } => (*transform, snapshots),
        };
        snapshots
            .iter()
            .map(|e| e.transformed(&transform))
            .collect()
    }

    /// Reset to `FirstPoint` and clear the successor flag.
    fn cancel(&mut self) {
        self.state = MirrorState::FirstPoint;
        self.pending_successor = false;
    }

    /// A point is a click; anything else is refused.
    fn on_command_input(
        &mut self,
        input: ToolInput,
        doc: &mut Document,
        history: &mut History,
    ) -> bool {
        match input.as_point() {
            Some(p) if !matches!(self.state, MirrorState::Confirm { .. }) => {
                self.on_pointer_down(p, false, doc, history);
                true
            }
            _ => false,
        }
    }

    /// The Yes/No answer (AC5–AC7): commits one [`TransformEntities`],
    /// appending the mirrored copies for No and replacing the sources for
    /// Yes. Any other text is refused and the prompt stays; sources changed
    /// by an undo since the first point cancel the run.
    fn on_raw_input(&mut self, raw: &str, doc: &mut Document, history: &mut History) -> bool {
        let MirrorState::Confirm {
            transform,
            indices,
            snapshots,
        } = &self.state
        else {
            return false;
        };
        let Some(erase) = erase_answer(raw) else {
            return false;
        };
        if !sources_intact(doc, indices, snapshots) {
            self.cancel();
            return true;
        }
        let cmd = TransformEntities::new(indices.clone(), *transform).with_keep_source(!erase);
        history.commit(Box::new(cmd), doc);
        self.state = MirrorState::FirstPoint;
        self.pending_successor = true;
        true
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

    /// A line (10,0)→(20,5) and an arc, both selected.
    fn doc_selected() -> (Document, History) {
        let mut doc = Document::default();
        doc.push_current(Entity::Line(Line::new(
            Vec2::new(10.0, 0.0),
            Vec2::new(20.0, 5.0),
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

    /// Across the Y axis.
    fn across_y() -> Transform {
        Transform::Mirror {
            a: Vec2::new(0.0, 0.0),
            b: Vec2::new(0.0, 10.0),
        }
    }

    /// Picks (0,0) then (0,10): the tool is at the Yes/No prompt.
    fn at_confirm(tool: &mut MirrorTool, doc: &mut Document, h: &mut History) {
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, doc, h);
        tool.on_pointer_down(Vec2::new(0.0, 10.0), false, doc, h);
    }

    /// AC1/AC3/AC5 — the three R14 prompts; the first point is the anchor;
    /// raw input only at the Yes/No prompt.
    #[test]
    fn prompts_anchor_and_raw_mode() {
        let mut tool = MirrorTool::default();
        let (mut doc, mut h) = doc_selected();
        assert_eq!(tool.name(), "MIRROR");
        assert_eq!(
            tool.status_text(),
            "MIRROR  Specify first point of mirror line:"
        );
        assert!(!tool.wants_raw_input());
        tool.on_pointer_down(Vec2::new(1.0, 2.0), false, &mut doc, &mut h);
        assert_eq!(
            tool.status_text(),
            "MIRROR  Specify second point of mirror line:"
        );
        assert_eq!(tool.anchor(), Some(Vec2::new(1.0, 2.0)));
        assert!(!tool.wants_raw_input());
        tool.on_pointer_down(Vec2::new(1.0, 5.0), false, &mut doc, &mut h);
        assert_eq!(
            tool.status_text(),
            "MIRROR  Erase source objects? [Yes/No] <N>:"
        );
        assert_eq!(tool.anchor(), None);
        assert!(tool.wants_raw_input());
    }

    /// AC2 — with nothing selected a pick leaves the drawing unchanged.
    #[test]
    fn empty_selection_is_a_noop() {
        let mut tool = MirrorTool::default();
        let (mut doc, mut h) = doc_selected();
        doc.selection.clear();
        let before = doc.entities.clone();
        at_confirm(&mut tool, &mut doc, &mut h);
        assert!(!tool.on_raw_input("y", &mut doc, &mut h));
        assert_eq!(doc.entities, before);
        assert!(!h.can_undo());
        assert_eq!(
            tool.status_text(),
            "MIRROR  Specify first point of mirror line:"
        );
    }

    /// AC3 — after the first point the preview is the selection mirrored
    /// across first point → cursor.
    #[test]
    fn preview_follows_the_cursor_line() {
        let mut tool = MirrorTool::default();
        let (mut doc, mut h) = doc_selected();
        assert!(tool.preview().is_empty());
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        let preview = tool.preview();
        assert_eq!(preview.len(), 2);
        assert!(
            doc.entities.iter().all(|e| preview.contains(e)),
            "unmirrored"
        );
        tool.on_pointer_move(Vec2::new(0.0, 3.0), &mut doc);
        let preview = tool.preview();
        assert_eq!(preview.len(), 2);
        for e in &doc.entities {
            assert!(preview.contains(&e.transformed(&across_y())), "{e:?}");
        }
    }

    /// AC4 — a second point on the first (within `EPSILON`) is ignored.
    #[test]
    fn coincident_second_point_is_ignored() {
        let mut tool = MirrorTool::default();
        let (mut doc, mut h) = doc_selected();
        tool.on_pointer_down(Vec2::new(3.0, 3.0), false, &mut doc, &mut h);
        let near = Vec2::new(3.0, 3.0 + EPSILON / 2.0);
        assert!(tool.on_command_input(ToolInput::Point(near), &mut doc, &mut h));
        assert_eq!(
            tool.status_text(),
            "MIRROR  Specify second point of mirror line:"
        );
        assert!(!h.can_undo());
    }

    /// AC6/AC9 — blank, `n` and `no` append the mirrored copies and keep
    /// the sources and the selection, as one undo step, then hand back to
    /// SELECT.
    #[test]
    fn no_keeps_sources() {
        for answer in ["", "  ", "n", "No", "NO"] {
            let mut tool = MirrorTool::default();
            let (mut doc, mut h) = doc_selected();
            let before = doc.entities.clone();
            let selection = doc.selection.clone();
            at_confirm(&mut tool, &mut doc, &mut h);
            assert!(tool.on_raw_input(answer, &mut doc, &mut h), "{answer:?}");
            assert_eq!(doc.entity_count(), 4, "{answer:?}");
            assert_eq!(doc.entities[..2], before[..]);
            for e in &before {
                assert!(doc.entities[2..].contains(&e.transformed(&across_y())));
            }
            assert_eq!(doc.selection, selection);
            assert_eq!(h.len(), 1);
            assert_eq!(tool.take_successor().map(|t| t.name()), Some("Select"));
            assert!(tool.take_successor().is_none());
        }
    }

    /// AC7/AC9 — `y` and `yes` replace the sources in place as one undo step.
    #[test]
    fn yes_replaces_sources() {
        for answer in ["y", "Yes", " YES "] {
            let mut tool = MirrorTool::default();
            let (mut doc, mut h) = doc_selected();
            let before = doc.entities.clone();
            at_confirm(&mut tool, &mut doc, &mut h);
            assert!(tool.on_raw_input(answer, &mut doc, &mut h), "{answer:?}");
            let want: Vec<Entity> = before.iter().map(|e| e.transformed(&across_y())).collect();
            assert_eq!(doc.entities, want, "{answer:?}");
            assert_eq!(h.len(), 1);
            assert!(tool.take_successor().is_some());
        }
    }

    /// Other text is refused and the prompt stays; a point is refused there.
    #[test]
    fn other_answers_are_refused() {
        let mut tool = MirrorTool::default();
        let (mut doc, mut h) = doc_selected();
        at_confirm(&mut tool, &mut doc, &mut h);
        for answer in ["maybe", "yess", "1,1"] {
            assert!(!tool.on_raw_input(answer, &mut doc, &mut h), "{answer:?}");
        }
        let p = ToolInput::Point(Vec2::new(1.0, 1.0));
        assert!(!tool.on_command_input(p, &mut doc, &mut h));
        assert!(!h.can_undo());
        assert!(tool.wants_raw_input());
    }

    /// Escape at the Yes/No prompt cancels and the tool stays MIRROR.
    #[test]
    fn escape_at_confirm_cancels() {
        let mut tool = MirrorTool::default();
        let (mut doc, mut h) = doc_selected();
        at_confirm(&mut tool, &mut doc, &mut h);
        let mut app = App::default();
        tool.on_key(egui::Key::Escape, &mut app);
        assert_eq!(tool.name(), "MIRROR");
        assert!(!tool.wants_raw_input());
        assert!(tool.preview().is_empty());
        assert!(!h.can_undo());
    }

    /// An undo past the first point cancels instead of mirroring stale
    /// indices.
    #[test]
    fn changed_sources_cancel() {
        let mut tool = MirrorTool::default();
        let (mut doc, mut h) = doc_selected();
        at_confirm(&mut tool, &mut doc, &mut h);
        doc.truncate_entities(1);
        let before = doc.entities.clone();
        assert!(tool.on_raw_input("y", &mut doc, &mut h));
        assert_eq!(doc.entities, before);
        assert!(!h.can_undo());
        assert!(!tool.wants_raw_input());
    }
}
