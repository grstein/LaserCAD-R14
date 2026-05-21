//! SelectTool: placeholder for the real SelectTool (LCV-042).
//!
//! This stub lets the app start with a valid active tool. All methods are
//! no-ops; the full SelectTool (LCV-042) will implement box-select,
//! click-to-select, shift-click-to-toggle, etc.
//!
//! MUST NOT import `eframe` or `rfd`. Introduced by demand LCV-040.

use crate::app::App;
use crate::document::{Document, Entity, History};
use crate::geometry::Vec2;
use crate::tools::Tool;

/// Placeholder SelectTool — a no-op stub.
///
/// The full implementation (LCV-042) will handle box-select, click-to-select,
/// shift-click-to-toggle, etc. This stub exists solely so the app can start
/// with a valid active tool.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SelectTool;

impl Tool for SelectTool {
    fn name(&self) -> &'static str {
        "Select"
    }

    fn on_pointer_down(&mut self, _pos: Vec2, _doc: &mut Document, _history: &mut History) {
        // No-op: full implementation in LCV-042
    }

    fn on_pointer_move(&mut self, _pos: Vec2, _doc: &mut Document) {
        // No-op: full implementation in LCV-042
    }

    fn on_pointer_up(&mut self, _pos: Vec2, _doc: &mut Document, _history: &mut History) {
        // No-op: full implementation in LCV-042
    }

    fn on_key(&mut self, _key: egui::Key, _app: &mut App) {
        // No-op: full implementation in LCV-042
    }

    fn preview(&self) -> Vec<Entity> {
        vec![]
    }

    fn cancel(&mut self) {
        // No-op: nothing to cancel in the stub
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::History;

    /// AC#4 — `SelectTool::default().name()` returns exactly `"Select"`.
    #[test]
    fn select_tool_name_exact() {
        let tool = SelectTool;
        assert_eq!(tool.name(), "Select");
    }

    /// AC#15 — `SelectTool::default().preview()` returns an empty vector.
    #[test]
    fn select_tool_preview_is_empty() {
        let tool = SelectTool;
        assert!(tool.preview().is_empty());
    }

    /// AC#2 — SelectTool is object-safe (can be stored as `Box<dyn Tool>`).
    #[test]
    fn select_tool_is_object_safe() {
        let _boxed: Box<dyn Tool> = Box::new(SelectTool);
        let _ref: &dyn Tool = &SelectTool;
    }

    /// AC#1 — Tool trait has seven methods; call them all through SelectTool.
    #[test]
    fn tool_trait_has_seven_methods() {
        let mut tool = SelectTool;
        let mut doc = crate::document::Document::default();
        let mut history = History::default();
        let mut app = crate::app::App::default();

        // The seven methods:
        let _ = tool.name();
        tool.on_pointer_down(Vec2::new(0.0, 0.0), &mut doc, &mut history);
        tool.on_pointer_move(Vec2::new(1.0, 1.0), &mut doc);
        tool.on_pointer_up(Vec2::new(2.0, 2.0), &mut doc, &mut history);
        tool.on_key(egui::Key::Escape, &mut app);
        let _ = tool.preview();
        tool.cancel();
    }
}
