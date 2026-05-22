//! TrimTool stub — activates when the user presses `T` (LCV-070).
//!
//! Full trim behaviour (pick overlap, commit `TrimEntity`) will be wired in
//! LCV-050. This file exists so the shortcut dispatch table can construct a
//! `Box<dyn Tool>` with `name() == "TRIM"` without waiting for LCV-050.
//! MUST NOT import `eframe` or `rfd`.

use crate::app::App;
use crate::document::{Document, Entity, History};
use crate::geometry::Vec2;
use crate::tools::Tool;

/// Trim-tool stub. Full implementation lands with LCV-050.
#[derive(Debug, Default)]
pub struct TrimTool;

impl Tool for TrimTool {
    fn name(&self) -> &'static str {
        "TRIM"
    }

    fn on_pointer_down(
        &mut self,
        _pos: Vec2,
        _shift: bool,
        _doc: &mut Document,
        _history: &mut History,
    ) {
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

    fn on_key(&mut self, _key: egui::Key, _app: &mut App) {}

    fn preview(&self) -> Vec<Entity> {
        vec![]
    }

    fn cancel(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_is_trim() {
        assert_eq!(TrimTool.name(), "TRIM");
    }
}
