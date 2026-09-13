//! MoveTool: two-click move of selected entities (LCV-049).
//!
//! ## State machine
//!
//! ```text
//! Idle ──press (non-empty sel)──► WaitingDest { base, cursor, snapshots }
//! Idle ──press (empty sel)────► Idle (no-op)
//!
//! WaitingDest ──move──────────► WaitingDest (cursor updated)
//! WaitingDest ──press (|delta| > EPSILON)──► Idle (commit MoveEntities)
//! WaitingDest ──press (|delta| ≤ EPSILON)──► WaitingDest (zero-delta guard)
//!
//! Any ──Escape──► Idle (cancel; tool stays MOVE)
//! ```
//!
//! After a successful commit, `take_successor()` returns
//! `Some(Box<SelectTool>)` exactly once (single-shot flag).
//!
//! MUST NOT import `eframe` or `rfd`. Introduced by demand LCV-049.

use crate::app::App;
use crate::cmdline::ToolInput;
use crate::document::{Document, Entity, History, MoveEntities};
use crate::geometry::{Vec2, EPSILON};
use crate::tools::{SelectTool, Tool};

// ---------------------------------------------------------------------------
// Internal state machine
// ---------------------------------------------------------------------------

/// Internal state of [`MoveTool`].
enum MoveState {
    /// Waiting for the user to click the base point.
    Idle,
    /// Base point set; waiting for the destination click.
    WaitingDest {
        /// The first click position (base point).
        base: Vec2,
        /// Current cursor position; updated by `on_pointer_move`.
        cursor: Vec2,
        /// Cloned snapshots of the selected entities captured on the first
        /// click, used to render the preview ghost without touching `doc`.
        snapshots: Vec<Entity>,
    },
}

// ---------------------------------------------------------------------------
// MoveTool
// ---------------------------------------------------------------------------

/// Two-click MOVE tool: pick a base point, then pick a destination; commits
/// one [`MoveEntities`] command and hands control back to [`SelectTool`].
///
/// Entities to move must be selected **before** the tool activates. The
/// selection is not cleared after a successful move (AutoCAD R14 behavior).
pub struct MoveTool {
    state: MoveState,
    /// `true` after a successful commit; consumed (cleared) by the first call
    /// to `take_successor()`.
    pending_successor: bool,
}

impl Default for MoveTool {
    fn default() -> Self {
        Self {
            state: MoveState::Idle,
            pending_successor: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Tool trait
// ---------------------------------------------------------------------------

impl Tool for MoveTool {
    fn name(&self) -> &'static str {
        "MOVE"
    }

    /// The R14 prompt table (LCV-111 AC 17).
    fn status_text(&self) -> &'static str {
        match &self.state {
            MoveState::Idle => "MOVE Specify base point:",
            MoveState::WaitingDest { .. } => "MOVE Specify destination point:",
        }
    }

    /// The base point, once it is fixed (LCV-111 AC 5). This is what makes
    /// the classic `m` ⏎ `0,0` ⏎ `@10,0` ⏎ "move it 10 mm right" work.
    ///
    /// Adding it also turns F8 ortho on for the destination pick, which is
    /// R14-correct — an axis-constrained move still commits.
    fn anchor(&self) -> Option<Vec2> {
        match &self.state {
            MoveState::WaitingDest { base, .. } => Some(*base),
            MoveState::Idle => None,
        }
    }

    /// First click: snapshot selection → enter `WaitingDest`.
    /// Second click: compute delta, commit `MoveEntities`, → `Idle`.
    fn on_pointer_down(
        &mut self,
        pos: Vec2,
        _shift: bool,
        doc: &mut Document,
        history: &mut History,
    ) {
        match &self.state {
            MoveState::Idle => {
                // AC#3: no-op when nothing is selected.
                if doc.selection.is_empty() {
                    return;
                }
                // Capture entity snapshots while we have `&mut Document`.
                let snapshots: Vec<Entity> =
                    doc.selection.iter().map(|i| doc.entities[i]).collect();
                self.state = MoveState::WaitingDest {
                    base: pos,
                    cursor: pos,
                    snapshots,
                };
            }
            MoveState::WaitingDest { base, .. } => {
                let base = *base;
                let delta = pos - base;
                // AC#5: discard zero-delta clicks.
                if delta.length() <= EPSILON {
                    return;
                }
                // Commit the move.
                let indices: Vec<usize> = doc.selection.iter().collect();
                history.commit(Box::new(MoveEntities::new(indices, delta)), doc);
                self.state = MoveState::Idle;
                self.pending_successor = true;
            }
        }
    }

    /// Update the live cursor in `WaitingDest`; no-op in `Idle`.
    fn on_pointer_move(&mut self, pos: Vec2, _doc: &mut Document) {
        if let MoveState::WaitingDest { cursor, .. } = &mut self.state {
            *cursor = pos;
        }
    }

    /// Always a no-op (LCV-049 AC#8).
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

    /// Returns translated ghost copies of the selection snapshots while in
    /// `WaitingDest`; empty in `Idle`.
    fn preview(&self) -> Vec<Entity> {
        match &self.state {
            MoveState::Idle => vec![],
            MoveState::WaitingDest {
                base,
                cursor,
                snapshots,
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

    /// Reset to `Idle`, clear snapshot cache, clear the successor flag.
    fn cancel(&mut self) {
        self.state = MoveState::Idle;
        self.pending_successor = false;
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

    /// Single-shot: returns `Some(SelectTool)` exactly once after a successful
    /// commit; subsequent calls return `None` until the next commit.
    fn take_successor(&mut self) -> Option<Box<dyn Tool>> {
        if self.pending_successor {
            self.pending_successor = false;
            Some(Box::new(SelectTool::default()))
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Document, History};
    use crate::geometry::{Line, Vec2, EPSILON};

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Build a doc with one line (0,0)→(10,0) and select entity 0.
    fn doc_with_line_selected() -> (Document, History) {
        let mut doc = Document::default();
        doc.entities.push(Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
        )));
        doc.selection.add(0);
        (doc, History::default())
    }

    fn first_click(tool: &mut MoveTool, pos: Vec2, doc: &mut Document, hist: &mut History) {
        tool.on_pointer_down(pos, false, doc, hist);
    }

    fn move_cursor(tool: &mut MoveTool, pos: Vec2, doc: &mut Document) {
        tool.on_pointer_move(pos, doc);
    }

    fn second_click(tool: &mut MoveTool, pos: Vec2, doc: &mut Document, hist: &mut History) {
        tool.on_pointer_down(pos, false, doc, hist);
    }

    fn is_idle(tool: &MoveTool) -> bool {
        matches!(tool.state, MoveState::Idle)
    }

    fn is_waiting_dest(tool: &MoveTool) -> bool {
        matches!(tool.state, MoveState::WaitingDest { .. })
    }

    // -----------------------------------------------------------------------
    // AC tests
    // -----------------------------------------------------------------------

    /// AC#1 — `name()` returns exactly `"MOVE"`.
    #[test]
    fn name_is_move() {
        assert_eq!(MoveTool::default().name(), "MOVE");
    }

    /// AC#2 / LCV-111 AC 17, AC 18 — the R14 idle prompt.
    #[test]
    fn idle_status_text() {
        let tool = MoveTool::default();
        assert_eq!(tool.status_text(), "MOVE Specify base point:");
    }

    /// AC#2 / LCV-111 AC 17, AC 18 — the R14 destination prompt.
    #[test]
    fn waiting_dest_status_text() {
        let mut tool = MoveTool::default();
        let (mut doc, mut hist) = doc_with_line_selected();
        first_click(&mut tool, Vec2::new(0.0, 0.0), &mut doc, &mut hist);
        assert_eq!(tool.status_text(), "MOVE Specify destination point:");
    }

    /// AC#3 — empty selection: no-op, stays `Idle`, no command pushed.
    #[test]
    fn idle_empty_selection_no_commit() {
        let mut tool = MoveTool::default();
        let mut doc = Document::default();
        let mut hist = History::default();
        tool.on_pointer_down(Vec2::new(1.0, 1.0), false, &mut doc, &mut hist);
        assert!(!hist.can_undo());
        assert!(is_idle(&tool));
    }

    /// AC#3 — non-empty selection: first click enters `WaitingDest`.
    #[test]
    fn idle_nonzero_selection_enters_waiting() {
        let mut tool = MoveTool::default();
        let (mut doc, mut hist) = doc_with_line_selected();
        first_click(&mut tool, Vec2::new(0.0, 0.0), &mut doc, &mut hist);
        assert!(is_waiting_dest(&tool));
    }

    /// AC#7 — `preview()` is empty in `Idle`.
    #[test]
    fn preview_empty_in_idle() {
        assert!(MoveTool::default().preview().is_empty());
    }

    /// AC#6 — `preview()` returns translated ghost while in `WaitingDest`.
    #[test]
    fn preview_ghost_in_waiting_dest() {
        let mut tool = MoveTool::default();
        let (mut doc, mut hist) = doc_with_line_selected();
        first_click(&mut tool, Vec2::new(0.0, 0.0), &mut doc, &mut hist);
        move_cursor(&mut tool, Vec2::new(5.0, 5.0), &mut doc);

        let pv = tool.preview();
        assert_eq!(pv.len(), 1);
        match pv[0] {
            Entity::Line(l) => {
                assert!(l.p1.approx_eq(Vec2::new(5.0, 5.0), EPSILON));
                assert!(l.p2.approx_eq(Vec2::new(15.0, 5.0), EPSILON));
            }
            _ => panic!("expected a Line ghost"),
        }
    }

    /// AC#5 — second click commits a `MoveEntities` and returns to `Idle`.
    #[test]
    fn second_click_commits_move() {
        let mut tool = MoveTool::default();
        let (mut doc, mut hist) = doc_with_line_selected();
        first_click(&mut tool, Vec2::new(0.0, 0.0), &mut doc, &mut hist);
        second_click(&mut tool, Vec2::new(3.0, 4.0), &mut doc, &mut hist);

        assert!(hist.can_undo());
        assert!(is_idle(&tool));
        match doc.entities[0] {
            Entity::Line(l) => {
                assert!(l.p1.approx_eq(Vec2::new(3.0, 4.0), EPSILON));
            }
            _ => panic!("expected a Line"),
        }
    }

    /// AC#5 — zero-delta second click: no commit, stays `WaitingDest`.
    #[test]
    fn zero_delta_stays_waiting() {
        let mut tool = MoveTool::default();
        let (mut doc, mut hist) = doc_with_line_selected();
        first_click(&mut tool, Vec2::new(5.0, 5.0), &mut doc, &mut hist);
        second_click(&mut tool, Vec2::new(5.0, 5.0), &mut doc, &mut hist);
        assert!(!hist.can_undo());
        assert!(is_waiting_dest(&tool));
    }

    /// AC#9 — `cancel()` clears preview and returns to `Idle`.
    #[test]
    fn cancel_clears_preview() {
        let mut tool = MoveTool::default();
        let (mut doc, mut hist) = doc_with_line_selected();
        first_click(&mut tool, Vec2::new(0.0, 0.0), &mut doc, &mut hist);
        tool.cancel();
        assert!(tool.preview().is_empty());
        assert!(is_idle(&tool));
    }

    /// AC#9 — Escape (`on_key`) calls cancel; tool name is still `"MOVE"`.
    #[test]
    fn escape_key_stays_in_move_tool() {
        let mut tool = MoveTool::default();
        let (mut doc, mut hist) = doc_with_line_selected();
        first_click(&mut tool, Vec2::new(0.0, 0.0), &mut doc, &mut hist);
        let mut app = crate::app::App::default();
        tool.on_key(egui::Key::Escape, &mut app);
        assert_eq!(tool.name(), "MOVE");
        assert!(is_idle(&tool));
    }

    /// AC#10 — first call to `take_successor` after commit returns `Some("Select")`.
    #[test]
    fn take_successor_after_commit_returns_select() {
        let mut tool = MoveTool::default();
        let (mut doc, mut hist) = doc_with_line_selected();
        first_click(&mut tool, Vec2::new(0.0, 0.0), &mut doc, &mut hist);
        second_click(&mut tool, Vec2::new(3.0, 4.0), &mut doc, &mut hist);
        let successor = tool.take_successor();
        assert!(successor.is_some());
        assert_eq!(successor.unwrap().name(), "Select");
    }

    /// AC#10 — `take_successor` is single-shot: second call returns `None`.
    #[test]
    fn take_successor_is_single_shot() {
        let mut tool = MoveTool::default();
        let (mut doc, mut hist) = doc_with_line_selected();
        first_click(&mut tool, Vec2::new(0.0, 0.0), &mut doc, &mut hist);
        second_click(&mut tool, Vec2::new(3.0, 4.0), &mut doc, &mut hist);
        let _ = tool.take_successor(); // consume
        assert!(tool.take_successor().is_none());
    }

    /// AC#10 — fresh tool: `take_successor` returns `None`.
    #[test]
    fn take_successor_fresh_is_none() {
        assert!(MoveTool::default().take_successor().is_none());
    }

    /// AC#14 — `MoveTool` is object-safe.
    #[test]
    fn object_safe() {
        let _: Box<dyn Tool> = Box::new(MoveTool::default());
    }

    /// Undo after a committed move restores original positions.
    #[test]
    fn undo_restores_position() {
        let mut tool = MoveTool::default();
        let (mut doc, mut hist) = doc_with_line_selected();
        first_click(&mut tool, Vec2::new(0.0, 0.0), &mut doc, &mut hist);
        second_click(&mut tool, Vec2::new(3.0, 4.0), &mut doc, &mut hist);
        hist.undo(&mut doc);
        match doc.entities[0] {
            Entity::Line(l) => {
                assert!(l.p1.approx_eq(Vec2::new(0.0, 0.0), EPSILON));
                assert!(l.p2.approx_eq(Vec2::new(10.0, 0.0), EPSILON));
            }
            _ => panic!("expected a Line"),
        }
    }

    /// AC#11 — `SelectTool` does not override `take_successor`; default
    /// impl returns `None`.
    #[test]
    fn tool_trait_default_take_successor() {
        let mut tool = crate::tools::SelectTool::default();
        assert!(tool.take_successor().is_none());
    }

    /// LCV-111 AC 3 — typed base and destination points are exactly clicks:
    /// the same entity moves and the same successor is queued.
    #[test]
    fn command_point_acts_exactly_like_a_click() {
        let mut typed = MoveTool::default();
        let (mut typed_doc, mut typed_h) = doc_with_line_selected();
        assert!(typed.on_command_input(
            ToolInput::Point(Vec2::new(0.0, 0.0)),
            &mut typed_doc,
            &mut typed_h
        ));
        assert!(typed.on_command_input(
            ToolInput::Point(Vec2::new(10.0, 0.0)),
            &mut typed_doc,
            &mut typed_h
        ));

        let mut clicked = MoveTool::default();
        let (mut clicked_doc, mut clicked_h) = doc_with_line_selected();
        clicked.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut clicked_doc, &mut clicked_h);
        clicked.on_pointer_down(
            Vec2::new(10.0, 0.0),
            false,
            &mut clicked_doc,
            &mut clicked_h,
        );

        assert_eq!(typed_doc.entities, clicked_doc.entities);
        assert_eq!(typed_h.len(), 1);
        assert!(is_idle(&typed));
        assert!(
            typed.take_successor().is_some(),
            "MOVE hands back to SELECT"
        );
    }

    /// LCV-111 AC 5 — `anchor()` follows the last fixed point: the base.
    #[test]
    fn anchor_follows_the_last_fixed_point() {
        let mut tool = MoveTool::default();
        let (mut doc, mut hist) = doc_with_line_selected();
        assert_eq!(tool.anchor(), None, "idle MOVE has no anchor");
        first_click(&mut tool, Vec2::new(2.0, 3.0), &mut doc, &mut hist);
        assert_eq!(tool.anchor(), Some(Vec2::new(2.0, 3.0)));
        tool.cancel();
        assert_eq!(tool.anchor(), None);
    }
}
