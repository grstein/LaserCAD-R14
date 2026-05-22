//! MoveTool stub — activates when the user presses `M` (LCV-070).
//!
//! Full move behaviour (select + drag + commit `MoveEntities`) will be wired
//! in LCV-049. This file exists so the shortcut dispatch table can construct a
//! `Box<dyn Tool>` with `name() == "MOVE"` without waiting for LCV-049.
//! MUST NOT import `eframe` or `rfd`.

use crate::app::App;
use crate::document::{Document, Entity, History};
use crate::geometry::Vec2;
use crate::tools::Tool;

/// Move-tool stub. Full implementation lands with LCV-049.
#[derive(Debug, Default)]
pub struct MoveTool;

impl Tool for MoveTool {
    fn name(&self) -> &'static str {
        "MOVE"
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
    fn name_is_move() {
        assert_eq!(MoveTool.name(), "MOVE");
    }
}
