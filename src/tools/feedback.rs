//! Styled canvas feedback a tool returns from [`Tool::feedback`] (ADR 0013).
//!
//! A [`Mark`] is plain, egui-free data: the app maps each variant to a render
//! primitive in `app/viewport/paint.rs::paint`, so `render/` never sees it.
//! The styles are closed: a fifth form needs an ADR 0013 amendment.
//!
//! [`Tool::feedback`]: super::Tool::feedback

use crate::document::Entity;

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
}
