//! CopyTool: base point, then any number of second points, each placing a
//! translated copy of the selection (LCV-157).
//!
//! ## State machine
//!
//! ```text
//! Idle ──press (non-empty sel)──► WaitingSecond { base, cursor, indices, snapshots }
//! Idle ──press (empty sel)────► Idle (no-op)
//!
//! WaitingSecond ──move──────────► WaitingSecond (cursor updated)
//! WaitingSecond ──press (|delta| > EPSILON)──► WaitingSecond (commit CopyEntities)
//! WaitingSecond ──press (|delta| ≤ EPSILON)──► WaitingSecond (zero-delta guard)
//!
//! Any ──Escape / Enter──► Idle (end the run; tool stays COPY)
//! ```
//!
//! The source indices are captured with the base point. Copies are only ever
//! appended, so those indices stay valid for the whole run, unless an undo
//! reaches past the base point: then the next point cancels the run. No successor: COPY
//! stays armed, like PLINE.
//!
//! MUST NOT import `eframe` or `rfd`.

use crate::app::App;
use crate::cmdline::ToolInput;
use crate::document::{CopyEntities, Document, Entity, History};
use crate::geometry::{EPSILON, Vec2};
use crate::tools::Tool;

/// Internal state of [`CopyTool`].
#[derive(Debug)]
enum CopyState {
    /// Waiting for the base point.
    Idle,
    /// Base point fixed; each second point places one copy.
    WaitingSecond {
        /// The base point.
        base: Vec2,
        /// Current cursor position; updated by `on_pointer_move`.
        cursor: Vec2,
        /// The source indices, captured with the base point.
        indices: Vec<usize>,
        /// The source entities, captured with the base point, for the preview.
        snapshots: Vec<Entity>,
    },
}

/// Multiple COPY: pick a base point, then each further point commits one
/// [`CopyEntities`] and keeps the same base point and source set.
///
/// Entities to copy must be selected **before** the tool activates.
#[derive(Debug)]
pub struct CopyTool {
    state: CopyState,
}

impl Default for CopyTool {
    fn default() -> Self {
        Self {
            state: CopyState::Idle,
        }
    }
}

impl Tool for CopyTool {
    fn name(&self) -> &'static str {
        "COPY"
    }

    fn status_text(&self) -> &'static str {
        match &self.state {
            CopyState::Idle => "COPY Specify base point:",
            CopyState::WaitingSecond { .. } => "COPY Specify second point:",
        }
    }

    /// The base point, once fixed: `@dx,dy` is relative to it (AC3).
    fn anchor(&self) -> Option<Vec2> {
        match &self.state {
            CopyState::WaitingSecond { base, .. } => Some(*base),
            CopyState::Idle => None,
        }
    }

    /// First point: capture the selection. Each later point: place one copy.
    fn on_pointer_down(
        &mut self,
        pos: Vec2,
        _shift: bool,
        doc: &mut Document,
        history: &mut History,
    ) {
        match &self.state {
            CopyState::Idle => {
                // AC2: nothing selected, nothing to copy.
                if doc.selection.is_empty() {
                    return;
                }
                let indices: Vec<usize> = doc.selection.iter().collect();
                let snapshots = indices.iter().map(|&i| doc.entities[i]).collect();
                self.state = CopyState::WaitingSecond {
                    base: pos,
                    cursor: pos,
                    indices,
                    snapshots,
                };
            }
            CopyState::WaitingSecond {
                base,
                indices,
                snapshots,
                ..
            } => {
                // An undo past the base point can remove or shift the sources.
                if !sources_intact(doc, indices, snapshots) {
                    self.cancel();
                    return;
                }
                let delta = pos - *base;
                // AC7: a zero delta places nothing.
                if delta.length() <= EPSILON {
                    return;
                }
                // AC5/AC6: one undo step per placement; the state is kept.
                let cmd = CopyEntities::new(indices.clone(), delta);
                history.commit(Box::new(cmd), doc);
            }
        }
    }

    fn on_pointer_move(&mut self, pos: Vec2, _doc: &mut Document) {
        if let CopyState::WaitingSecond { cursor, .. } = &mut self.state {
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

    /// AC8: Escape or a blank Enter ends the run; the tool stays COPY.
    fn on_key(&mut self, key: egui::Key, _app: &mut App) {
        if matches!(key, egui::Key::Escape | egui::Key::Enter) {
            self.cancel();
        }
    }

    /// The sources translated to the cursor (AC3).
    fn preview(&self) -> Vec<Entity> {
        match &self.state {
            CopyState::Idle => vec![],
            CopyState::WaitingSecond {
                base,
                cursor,
                snapshots,
                ..
            } => {
                let offset = *cursor - *base;
                snapshots
                    .iter()
                    .map(|e| {
                        let mut ghost = *e;
                        ghost.translate(offset);
                        ghost
                    })
                    .collect()
            }
        }
    }

    fn cancel(&mut self) {
        self.state = CopyState::Idle;
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

/// Do the sources captured with the base point still sit, unchanged, at
/// their indices? An undo past the base point can remove or shift them; MOVE
/// and COPY then drop back to the base-point prompt instead of editing a
/// stale index.
pub(crate) fn sources_intact(doc: &Document, indices: &[usize], snapshots: &[Entity]) -> bool {
    indices.len() == snapshots.len()
        && indices
            .iter()
            .zip(snapshots)
            .all(|(&i, s)| doc.entities.get(i) == Some(s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Line;

    /// A doc with one line (0,0)→(10,0), selected.
    fn doc_with_line_selected() -> (Document, History) {
        let mut doc = Document::default();
        doc.push_current(Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
        )));
        doc.selection.add(0);
        (doc, History::default())
    }

    fn click(tool: &mut CopyTool, pos: Vec2, doc: &mut Document, hist: &mut History) {
        tool.on_pointer_down(pos, false, doc, hist);
    }

    fn line_at(e: &Entity) -> Line {
        match e {
            Entity::Line(l) => *l,
            _ => panic!("expected a Line"),
        }
    }

    /// AC1/AC3 — the two prompts.
    #[test]
    fn prompts_follow_the_state() {
        let mut tool = CopyTool::default();
        assert_eq!(tool.name(), "COPY");
        assert_eq!(tool.status_text(), "COPY Specify base point:");
        let (mut doc, mut hist) = doc_with_line_selected();
        click(&mut tool, Vec2::new(0.0, 0.0), &mut doc, &mut hist);
        assert_eq!(tool.status_text(), "COPY Specify second point:");
    }

    /// AC2 — an empty selection leaves the drawing unchanged and the tool at
    /// the base-point prompt.
    #[test]
    fn empty_selection_is_a_no_op() {
        let mut tool = CopyTool::default();
        let (mut doc, mut hist) = doc_with_line_selected();
        doc.selection.clear();
        click(&mut tool, Vec2::new(1.0, 1.0), &mut doc, &mut hist);
        assert_eq!(doc.entity_count(), 1);
        assert!(!hist.can_undo());
        assert_eq!(tool.status_text(), "COPY Specify base point:");
        assert_eq!(tool.anchor(), None);
    }

    /// AC3 — the base point is the anchor and the preview follows the cursor.
    #[test]
    fn base_point_is_the_anchor_and_preview_follows_the_cursor() {
        let mut tool = CopyTool::default();
        let (mut doc, mut hist) = doc_with_line_selected();
        assert!(tool.preview().is_empty());
        click(&mut tool, Vec2::new(2.0, 3.0), &mut doc, &mut hist);
        assert_eq!(tool.anchor(), Some(Vec2::new(2.0, 3.0)));
        tool.on_pointer_move(Vec2::new(7.0, 8.0), &mut doc);
        let pv = tool.preview();
        assert_eq!(pv.len(), 1);
        assert!(line_at(&pv[0]).p1.approx_eq(Vec2::new(5.0, 5.0), EPSILON));
        assert!(line_at(&pv[0]).p2.approx_eq(Vec2::new(15.0, 5.0), EPSILON));
    }

    /// AC5 — after a placement COPY stays at the second-point prompt with
    /// the same base point; each point places another copy.
    #[test]
    fn stays_armed_after_each_placement() {
        let mut tool = CopyTool::default();
        let (mut doc, mut hist) = doc_with_line_selected();
        click(&mut tool, Vec2::new(0.0, 0.0), &mut doc, &mut hist);
        click(&mut tool, Vec2::new(10.0, 0.0), &mut doc, &mut hist);
        assert_eq!(tool.status_text(), "COPY Specify second point:");
        assert_eq!(tool.anchor(), Some(Vec2::new(0.0, 0.0)));
        click(&mut tool, Vec2::new(0.0, 5.0), &mut doc, &mut hist);
        assert_eq!(doc.entity_count(), 3);
        assert_eq!(hist.len(), 2);
        assert!(
            line_at(&doc.entities[0])
                .p1
                .approx_eq(Vec2::default(), EPSILON)
        );
        assert!(
            line_at(&doc.entities[1])
                .p1
                .approx_eq(Vec2::new(10.0, 0.0), EPSILON)
        );
        assert!(
            line_at(&doc.entities[2])
                .p1
                .approx_eq(Vec2::new(0.0, 5.0), EPSILON)
        );
        assert!(tool.take_successor().is_none(), "COPY stays armed");
    }

    /// AC5 — the source set is the one captured with the base point, even
    /// if the selection changes during the run.
    #[test]
    fn source_set_is_captured_with_the_base_point() {
        let mut tool = CopyTool::default();
        let (mut doc, mut hist) = doc_with_line_selected();
        click(&mut tool, Vec2::new(0.0, 0.0), &mut doc, &mut hist);
        doc.selection.clear();
        click(&mut tool, Vec2::new(10.0, 0.0), &mut doc, &mut hist);
        assert_eq!(doc.entity_count(), 2);
    }

    /// AC7 — a second point equal to the base point places nothing.
    #[test]
    fn zero_delta_places_nothing() {
        let mut tool = CopyTool::default();
        let (mut doc, mut hist) = doc_with_line_selected();
        click(&mut tool, Vec2::new(5.0, 5.0), &mut doc, &mut hist);
        click(&mut tool, Vec2::new(5.0, 5.0), &mut doc, &mut hist);
        assert_eq!(doc.entity_count(), 1);
        assert!(!hist.can_undo());
        assert_eq!(tool.status_text(), "COPY Specify second point:");
    }

    /// AC8 — Enter and Escape end the run; the placed copies stay and the
    /// tool is back at the base-point prompt.
    #[test]
    fn enter_and_escape_end_the_run() {
        for key in [egui::Key::Enter, egui::Key::Escape] {
            let mut tool = CopyTool::default();
            let (mut doc, mut hist) = doc_with_line_selected();
            click(&mut tool, Vec2::new(0.0, 0.0), &mut doc, &mut hist);
            click(&mut tool, Vec2::new(10.0, 0.0), &mut doc, &mut hist);
            tool.on_key(key, &mut App::default());
            assert_eq!(tool.name(), "COPY");
            assert_eq!(tool.status_text(), "COPY Specify base point:");
            assert!(tool.preview().is_empty());
            assert_eq!(doc.entity_count(), 2, "{key:?} keeps the copies");
        }
    }

    /// ADR 0003 §B3 — typed points act exactly like clicks.
    #[test]
    fn command_point_acts_like_a_click() {
        let mut tool = CopyTool::default();
        let (mut doc, mut hist) = doc_with_line_selected();
        assert!(tool.on_command_input(ToolInput::Point(Vec2::default()), &mut doc, &mut hist));
        assert!(tool.on_command_input(ToolInput::Point(Vec2::new(4.0, 0.0)), &mut doc, &mut hist));
        assert_eq!(doc.entity_count(), 2);
    }

    /// Regression: undo past the run start (the sources' creation and
    /// selection) and then a second point must not panic or copy a stale
    /// index; COPY drops back to the base-point prompt.
    #[test]
    fn undo_past_the_run_start_cancels_instead_of_panicking() {
        use crate::document::{CreateLine, SelectionCommand};
        let mut doc = Document::default();
        let mut hist = History::default();
        let l = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        hist.commit(Box::new(CreateLine::new(l)), &mut doc);
        hist.commit(Box::new(SelectionCommand::new([0usize])), &mut doc);
        let mut tool = CopyTool::default();
        click(&mut tool, Vec2::new(0.0, 0.0), &mut doc, &mut hist);
        click(&mut tool, Vec2::new(10.0, 0.0), &mut doc, &mut hist);
        for _ in 0..3 {
            assert!(hist.undo(&mut doc));
        }
        assert_eq!(doc.entity_count(), 0);
        click(&mut tool, Vec2::new(20.0, 0.0), &mut doc, &mut hist);
        assert_eq!(doc.entity_count(), 0);
        assert_eq!(tool.status_text(), "COPY Specify base point:");
        assert!(hist.can_redo(), "nothing was committed");
    }

    /// Regression: the copies of a multi-entity selection are appended in
    /// ascending source order, whatever order the selection was built in.
    #[test]
    fn copies_are_appended_in_ascending_source_order() {
        let mut doc = Document::default();
        for i in 0..32 {
            let y = f64::from(i);
            doc.push_current(Entity::Line(Line::new(
                Vec2::new(0.0, y),
                Vec2::new(1.0, y),
            )));
        }
        for i in (0..32).rev() {
            doc.selection.add(i);
        }
        let mut hist = History::default();
        let mut tool = CopyTool::default();
        click(&mut tool, Vec2::new(0.0, 0.0), &mut doc, &mut hist);
        click(&mut tool, Vec2::new(100.0, 0.0), &mut doc, &mut hist);
        for i in 0..32u32 {
            let copy = line_at(&doc.entities[32 + i as usize]);
            assert_eq!(copy.p1, Vec2::new(100.0, f64::from(i)), "copy {i}");
        }
    }
}
