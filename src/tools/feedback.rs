//! Styled canvas feedback a tool returns from [`Tool::feedback`] (ADR 0013).
//!
//! A [`Mark`] is plain, egui-free data: the app maps each variant to a render
//! primitive in `app/viewport/paint.rs::paint`, so `render/` never sees it.
//! The styles are closed: a fifth form needs an ADR 0013 amendment.
//!
//! [`FeedbackGate`] is the Esc mute of ADR 0013 §4, owned by
//! [`super::ToolManager`] (the LCV-163 plan's LOC-cap seam).
//!
//! [`Tool::feedback`]: super::Tool::feedback

use crate::document::Entity;
use crate::geometry::Vec2;

/// One piece of canvas feedback, in the style the tool wants it painted.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Mark {
    /// Solid `preview` stroke: rubber band, window selection box.
    Preview(Entity),
    /// Dashed `preview` stroke: crossing selection box (LCV-163 AC 2).
    Dashed(Entity),
    /// `doc.entities[i]` in its layer colour at the `hover` width (AC 3).
    Hover(usize),
    /// Dashed `danger` stroke: what TRIM or ERASE will remove (AC 4, AC 5).
    Danger(Entity),
}

/// Esc mutes the hover and danger feedback until the pointer moves
/// (LCV-163 AC 9, ADR 0013 §4): Esc stores the last `Move` point, and the
/// cursor handed to [`super::Tool::feedback`] is `None` while it equals that
/// point. A `Move` to a different point clears the mute.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct FeedbackGate {
    last_move: Option<Vec2>,
    muted_at: Option<Vec2>,
}

impl FeedbackGate {
    /// Record a pointer `Move` to `pos`; a different point unmutes.
    pub(crate) fn on_move(&mut self, pos: Vec2) {
        self.last_move = Some(pos);
        if self.muted_at.is_some_and(|m| m != pos) {
            self.muted_at = None;
        }
    }

    /// Esc: mute at the last `Move` point.
    pub(crate) fn on_escape(&mut self) {
        self.muted_at = self.last_move;
    }

    /// The cursor the tool sees: `None` while muted at `cursor`.
    pub(crate) fn cursor(&self, cursor: Option<Vec2>) -> Option<Vec2> {
        cursor.filter(|c| self.muted_at != Some(*c))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Document, History};
    use crate::geometry::Vec2;
    use crate::tools::{LineTool, Tool};

    /// ADR 0013 §1 — a tool that does not override `feedback` returns its
    /// `preview()` unchanged, each entity wrapped as `Mark::Preview`.
    #[test]
    fn default_feedback_wraps_preview_as_preview_marks() {
        let mut tool = LineTool::default();
        let mut doc = Document::default();
        let mut hist = History::default();
        tool.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut doc, &mut hist);
        tool.on_pointer_move(Vec2::new(10.0, 5.0), &mut doc);
        let preview = tool.preview();
        assert!(!preview.is_empty(), "fixture must have a rubber band");
        let marks = tool.feedback(&doc, Some(Vec2::new(10.0, 5.0)));
        let expected: Vec<Mark> = preview.into_iter().map(Mark::Preview).collect();
        assert_eq!(marks, expected);
        assert_eq!(tool.feedback(&doc, None), expected, "cursor is ignored");
    }

    /// LCV-163 AC 9 — Esc mutes the cursor at the last move point; the same
    /// point stays muted, a different one unmutes.
    #[test]
    fn escape_mutes_until_the_pointer_moves() {
        let (a, b) = (Vec2::new(1.0, 2.0), Vec2::new(1.5, 2.0));
        let mut gate = FeedbackGate::default();
        gate.on_move(a);
        assert_eq!(gate.cursor(Some(a)), Some(a), "unmuted by default");
        gate.on_escape();
        assert_eq!(gate.cursor(Some(a)), None, "muted after Esc");
        gate.on_move(a);
        assert_eq!(gate.cursor(Some(a)), None, "same point stays muted");
        gate.on_move(b);
        assert_eq!(gate.cursor(Some(b)), Some(b), "a move unmutes");
        assert_eq!(gate.cursor(Some(a)), Some(a), "and the mute is gone");
        assert_eq!(gate.cursor(None), None, "off the canvas stays None");
    }

    /// Esc before any move mutes nothing.
    #[test]
    fn escape_without_a_move_mutes_nothing() {
        let mut gate = FeedbackGate::default();
        gate.on_escape();
        let p = Vec2::new(0.0, 0.0);
        assert_eq!(gate.cursor(Some(p)), Some(p));
    }
}
