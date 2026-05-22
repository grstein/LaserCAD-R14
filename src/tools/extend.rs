//! ExtendTool stub — activates when the user presses `X` (LCV-070).
//!
//! Full extend behaviour (pick entity end, commit `ExtendEntity`) will be
//! wired in LCV-051. This file exists so the shortcut dispatch table can
//! construct a `Box<dyn Tool>` with `name() == "EXTEND"` without waiting for
//! LCV-051. MUST NOT import `eframe` or `rfd`.

use crate::app::App;
use crate::document::{Document, Entity, History};
use crate::geometry::Vec2;
use crate::tools::Tool;

/// Extend-tool stub. Full implementation lands with LCV-051.
#[derive(Debug, Default)]
pub struct ExtendTool;

impl Tool for ExtendTool {
    fn name(&self) -> &'static str {
        "EXTEND"
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
    fn name_is_extend() {
        assert_eq!(ExtendTool.name(), "EXTEND");
    }
}
