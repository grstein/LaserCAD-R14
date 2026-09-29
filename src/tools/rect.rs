//! RectTool: two-click corner-to-corner rectangle drawing (LCV-045).
//!
//! First click anchors corner1; second click commits four [`CreateEntities`]
//! lines (bottom, right, top, left sides). Degenerate inputs — where either
//! the width or height is ≤ `EPSILON` — are silently ignored.
//!
//! Preview shows the four sides while the cursor moves after the first click.
//! Escape resets to idle.
//!
//! MUST NOT import `eframe` or `rfd`.

use crate::app::App;
use crate::cmdline::ToolInput;
use crate::document::{Document, Entity, History, commands::CreateEntities};
use crate::geometry::{EPSILON, Line, Vec2};
use crate::tools::Tool;

/// Internal FSM state of [`RectTool`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum RectState {
    /// Awaiting the first click to anchor corner1.
    #[default]
    Idle,
    /// corner1 anchored; awaiting the diagonally-opposite corner.
    WaitingSecondCorner { corner1: Vec2, cursor: Vec2 },
}

/// Two-click rectangle drawing tool (LCV-045).
///
/// First click anchors corner1; the second click commits four
/// [`Entity::Line`] values (the four sides) via a single [`CreateEntities`]
/// command so the entire rectangle is undone atomically.
///
/// Degenerate inputs — width ≤ `EPSILON` or height ≤ `EPSILON` — are
/// silently discarded. Escape resets to idle.
#[derive(Debug, Default)]
pub struct RectTool {
    state: RectState,
}

impl RectTool {
    /// Construct a new `RectTool` in the idle state.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Build the four [`Entity::Line`] sides of a rectangle from `c1` to `c2`.
///
/// Side order: bottom → right → top → left (clockwise from the
/// bottom-left corner). Every side is returned as an `Entity::Line`.
fn rect_sides(c1: Vec2, c2: Vec2) -> [Entity; 4] {
    let bl = Vec2::new(c1.x.min(c2.x), c1.y.min(c2.y));
    let tr = Vec2::new(c1.x.max(c2.x), c1.y.max(c2.y));
    let br = Vec2::new(tr.x, bl.y);
    let tl = Vec2::new(bl.x, tr.y);
    [
        Entity::Line(Line::new(bl, br)), // bottom
        Entity::Line(Line::new(br, tr)), // right
        Entity::Line(Line::new(tr, tl)), // top
        Entity::Line(Line::new(tl, bl)), // left
    ]
}

impl Tool for RectTool {
    fn name(&self) -> &'static str {
        "RECT"
    }

    /// The R14 prompt table (LCV-111 AC 17).
    fn status_text(&self) -> &'static str {
        match self.state {
            RectState::Idle => "RECT Specify first corner:",
            RectState::WaitingSecondCorner { .. } => "RECT Specify opposite corner:",
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
            RectState::Idle => {
                self.state = RectState::WaitingSecondCorner {
                    corner1: pos,
                    cursor: pos,
                };
            }
            RectState::WaitingSecondCorner { corner1, .. } => {
                let w = (pos.x - corner1.x).abs();
                let h = (pos.y - corner1.y).abs();
                if w <= EPSILON || h <= EPSILON {
                    return; // degenerate — discard
                }
                let sides = rect_sides(corner1, pos);
                history.commit(Box::new(CreateEntities::new(sides.to_vec())), doc);
                self.state = RectState::Idle;
            }
        }
    }

    fn on_pointer_move(&mut self, pos: Vec2, _doc: &mut Document) {
        if let RectState::WaitingSecondCorner { ref mut cursor, .. } = self.state {
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
        if key == egui::Key::Escape {
            self.cancel();
        }
    }

    /// Preview: the four sides of the in-progress rectangle, or empty.
    fn preview(&self) -> Vec<Entity> {
        match self.state {
            RectState::Idle => vec![],
            RectState::WaitingSecondCorner { corner1, cursor } => {
                let w = (cursor.x - corner1.x).abs();
                let h = (cursor.y - corner1.y).abs();
                if w > EPSILON && h > EPSILON {
                    rect_sides(corner1, cursor).to_vec()
                } else {
                    vec![]
                }
            }
        }
    }

    fn cancel(&mut self) {
        self.state = RectState::Idle;
    }

    /// The canonical body (ADR 0003 §B3) — a typed point is exactly a click.
    ///
    /// RECT deliberately has **no** [`Tool::anchor`] override (LCV-111
    /// product decision 4), so `@dx,dy` and direct-distance entry are never
    /// resolved for it and only the absolute form `100,50` reaches here.
    /// That is the whole exact-rectangle workflow.
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

    fn make() -> (RectTool, crate::document::Document, History) {
        (
            RectTool::new(),
            crate::document::Document::default(),
            History::default(),
        )
    }

    // ── AC: name and default state ──────────────────────────────────────

    #[test]
    fn name_is_rect() {
        assert_eq!(RectTool::new().name(), "RECT");
    }

    #[test]
    fn idle_preview_is_empty() {
        assert!(RectTool::new().preview().is_empty());
    }

    #[test]
    fn idle_status_contains_first_corner() {
        let t = RectTool::new();
        assert_eq!(t.status_text(), "RECT Specify first corner:");
    }

    // ── AC: first click ─────────────────────────────────────────────────

    #[test]
    fn first_click_transitions_to_waiting() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(1.0, 2.0), false, &mut doc, &mut h);
        assert_eq!(t.status_text(), "RECT Specify opposite corner:");
        assert_eq!(doc.entity_count(), 0);
        assert!(!h.can_undo());
    }

    #[test]
    fn first_click_preview_empty_until_move() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        // cursor == corner1 → degenerate preview → empty
        assert!(t.preview().is_empty());
    }

    // ── AC: rubber-band preview ─────────────────────────────────────────

    #[test]
    fn preview_shows_four_lines_after_move() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        t.on_pointer_move(Vec2::new(10.0, 5.0), &mut doc);
        let pv = t.preview();
        assert_eq!(pv.len(), 4, "must have exactly four preview sides");
        for e in &pv {
            assert!(
                matches!(e, Entity::Line(_)),
                "each preview entity must be a Line"
            );
        }
    }

    #[test]
    fn preview_corners_match_input() {
        let (mut t, mut doc, mut h) = make();
        let c1 = Vec2::new(0.0, 0.0);
        let c2 = Vec2::new(10.0, 5.0);
        t.on_pointer_down(c1, false, &mut doc, &mut h);
        t.on_pointer_move(c2, &mut doc);
        let pv = t.preview();
        // Collect all distinct endpoints touched by the preview lines.
        let mut pts: Vec<Vec2> = Vec::new();
        for e in &pv {
            if let Entity::Line(l) = e {
                pts.push(l.p1);
                pts.push(l.p2);
            }
        }
        // All endpoints must be corners of the axis-aligned bounding box.
        for p in &pts {
            let on_x = (p.x - c1.x).abs() < EPSILON || (p.x - c2.x).abs() < EPSILON;
            let on_y = (p.y - c1.y).abs() < EPSILON || (p.y - c2.y).abs() < EPSILON;
            assert!(on_x && on_y, "endpoint {p:?} not on rectangle corner");
        }
    }

    #[test]
    fn move_while_idle_is_noop() {
        let (mut t, mut doc, _h) = make();
        t.on_pointer_move(Vec2::new(99.0, 99.0), &mut doc);
        assert!(t.preview().is_empty());
    }

    // ── AC: degenerate guards ────────────────────────────────────────────

    #[test]
    fn zero_width_second_click_ignored() {
        let (mut t, mut doc, mut h) = make();
        let p = Vec2::new(5.0, 5.0);
        t.on_pointer_down(p, false, &mut doc, &mut h);
        // Same X → zero width
        t.on_pointer_down(Vec2::new(5.0, 15.0), false, &mut doc, &mut h);
        assert_eq!(doc.entity_count(), 0);
        assert!(!h.can_undo());
    }

    #[test]
    fn zero_height_second_click_ignored() {
        let (mut t, mut doc, mut h) = make();
        let p = Vec2::new(5.0, 5.0);
        t.on_pointer_down(p, false, &mut doc, &mut h);
        // Same Y → zero height
        t.on_pointer_down(Vec2::new(15.0, 5.0), false, &mut doc, &mut h);
        assert_eq!(doc.entity_count(), 0);
        assert!(!h.can_undo());
    }

    #[test]
    fn same_point_second_click_ignored() {
        let (mut t, mut doc, mut h) = make();
        let p = Vec2::new(5.0, 5.0);
        t.on_pointer_down(p, false, &mut doc, &mut h);
        t.on_pointer_down(p, false, &mut doc, &mut h);
        assert_eq!(doc.entity_count(), 0);
        assert!(!h.can_undo());
    }

    // ── AC: commit ──────────────────────────────────────────────────────

    #[test]
    fn second_valid_click_commits_four_lines_and_returns_idle() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        t.on_pointer_down(Vec2::new(10.0, 5.0), false, &mut doc, &mut h);
        assert_eq!(doc.entity_count(), 4, "must commit exactly 4 line entities");
        assert!(h.can_undo());
        // Tool returns to idle
        assert!(t.preview().is_empty());
        assert_eq!(t.status_text(), "RECT Specify first corner:");
    }

    #[test]
    fn all_four_committed_entities_are_lines() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        t.on_pointer_down(Vec2::new(10.0, 5.0), false, &mut doc, &mut h);
        for e in &doc.entities {
            assert!(
                matches!(e, Entity::Line(_)),
                "committed entity must be a Line"
            );
        }
    }

    #[test]
    fn committed_rect_is_undoable_in_one_step() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        t.on_pointer_down(Vec2::new(10.0, 5.0), false, &mut doc, &mut h);
        assert_eq!(doc.entity_count(), 4);
        h.undo(&mut doc);
        assert_eq!(
            doc.entity_count(),
            0,
            "single undo must remove all four sides"
        );
    }

    // ── AC: cancel / Escape ─────────────────────────────────────────────

    #[test]
    fn escape_resets_to_idle_and_clears_preview() {
        let (mut t, mut doc, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut h);
        t.on_pointer_move(Vec2::new(10.0, 5.0), &mut doc);
        assert!(!t.preview().is_empty());

        let mut app = crate::app::App::default();
        t.on_key(egui::Key::Escape, &mut app);
        assert!(t.preview().is_empty());
        assert_eq!(t.status_text(), "RECT Specify first corner:");
    }

    #[test]
    fn cancel_from_idle_is_noop() {
        let mut t = RectTool::new();
        t.cancel();
        assert!(t.preview().is_empty());
        assert_eq!(t.status_text(), "RECT Specify first corner:");
    }

    // ── AC: object safety ───────────────────────────────────────────────

    #[test]
    fn is_object_safe() {
        let _: Box<dyn Tool> = Box::new(RectTool::new());
    }

    /// LCV-111 AC 3 — a typed pair of absolute corners draws the same
    /// rectangle a pair of clicks does.
    #[test]
    fn command_point_acts_exactly_like_a_click() {
        let (mut typed, mut typed_doc, mut typed_h) = make();
        assert!(typed.on_command_input(
            ToolInput::Point(Vec2::new(0.0, 0.0)),
            &mut typed_doc,
            &mut typed_h
        ));
        assert!(typed.on_command_input(
            ToolInput::Point(Vec2::new(100.0, 50.0)),
            &mut typed_doc,
            &mut typed_h
        ));

        let (mut clicked, mut clicked_doc, mut clicked_h) = make();
        clicked.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut clicked_doc, &mut clicked_h);
        clicked.on_pointer_down(
            Vec2::new(100.0, 50.0),
            false,
            &mut clicked_doc,
            &mut clicked_h,
        );

        assert_eq!(typed_doc.entities, clicked_doc.entities);
        assert_eq!(typed_doc.entity_count(), 4, "four sides");
        assert_eq!(typed_h.len(), 1, "one undo entry for the whole box");
        assert_eq!(typed.status_text(), "RECT Specify first corner:");
    }

    /// LCV-111 AC 5 / product decision 4 — RECT must **not** override
    /// `anchor()`. Its only consumer other than the command line is the F8
    /// ortho clamp, which would pin the opposite corner to a cardinal axis
    /// from corner 1, zeroing the width or the height; `on_pointer_down`
    /// then discards the degenerate rectangle and RECT becomes undrawable by
    /// mouse while ortho is on. Deleting this test must be a deliberate act.
    #[test]
    fn rect_has_no_anchor_so_ortho_cannot_flatten_it() {
        let (mut t, mut doc, mut h) = make();
        assert_eq!(t.anchor(), None, "idle RECT has no anchor");
        t.on_pointer_down(Vec2::new(10.0, 10.0), false, &mut doc, &mut h);
        assert_eq!(
            t.anchor(),
            None,
            "RECT in WaitingSecondCorner must still report no anchor"
        );
    }
}
