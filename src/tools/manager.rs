//! ToolManager: owner of the active tool, routes pointer and key events.
//!
//! `ToolManager` holds exactly one active tool as `Box<dyn Tool>`. It delegates
//! pointer and keyboard events to the active tool and exposes the tool's preview
//! geometry to the render pipeline.
//!
//! Switching tools (via `set_tool`) calls `cancel()` on the old tool before
//! replacing it — ensures in-progress state is cleaned up.
//!
//! MUST NOT import `eframe` or `rfd`. Introduced by demand LCV-040.

use crate::app::App;
use crate::document::Entity;
use crate::geometry::Vec2;
use crate::tools::Tool;

/// Owner of the active tool, routes pointer and key events.
///
/// Holds exactly one active tool as `Box<dyn Tool>`. Delegates all pointer
/// and keyboard events to the active tool.
pub struct ToolManager {
    /// The currently active tool. Never `None` — the app always has a tool.
    active: Box<dyn Tool>,
}

impl ToolManager {
    /// Construct a `ToolManager` with the given initial tool.
    ///
    /// # Example
    /// ```
    /// use lasercad::tools::{ToolManager, SelectTool};
    ///
    /// let manager = ToolManager::new(Box::new(SelectTool::default()));
    /// assert_eq!(manager.active_tool_name(), "Select");
    /// ```
    pub fn new(initial: Box<dyn Tool>) -> Self {
        Self { active: initial }
    }

    /// Replace the active tool. Calls `cancel()` on the old tool before
    /// switching to ensure in-progress state is cleaned up.
    ///
    /// # Example
    /// ```
    /// use lasercad::tools::{ToolManager, SelectTool};
    ///
    /// let mut manager = ToolManager::new(Box::new(SelectTool::default()));
    /// manager.set_tool(Box::new(SelectTool::default()));
    /// assert_eq!(manager.active_tool_name(), "Select");
    /// ```
    pub fn set_tool(&mut self, tool: Box<dyn Tool>) {
        self.active.cancel();
        self.active = tool;
    }

    /// Name of the currently active tool (delegates to `active.name()`).
    pub fn active_tool_name(&self) -> &'static str {
        self.active.name()
    }

    /// Route a pointer-down event to the active tool.
    pub fn handle_pointer_down(&mut self, pos: Vec2, app: &mut App) {
        self.active.on_pointer_down(pos, app);
    }

    /// Route a pointer-move event to the active tool.
    pub fn handle_pointer_move(&mut self, pos: Vec2, app: &mut App) {
        self.active.on_pointer_move(pos, app);
    }

    /// Route a pointer-up event to the active tool.
    pub fn handle_pointer_up(&mut self, pos: Vec2, app: &mut App) {
        self.active.on_pointer_up(pos, app);
    }

    /// Route a keyboard event to the active tool.
    pub fn handle_key(&mut self, key: egui::Key, app: &mut App) {
        self.active.on_key(key, app);
    }

    /// Get the preview geometry from the active tool. Returns an empty vector
    /// if the tool has no in-progress preview.
    pub fn preview(&self) -> Vec<Entity> {
        self.active.preview()
    }
}

impl Default for ToolManager {
    /// Default `ToolManager` starts with a `SelectTool` (LCV-040 AC#6).
    fn default() -> Self {
        Self::new(Box::new(crate::tools::SelectTool))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::SelectTool;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// AC#3, AC#6 — `ToolManager` can be constructed with `SelectTool` and
    /// reports its name correctly.
    #[test]
    fn tool_manager_new_with_select_tool() {
        let manager = ToolManager::new(Box::new(SelectTool));
        assert_eq!(manager.active_tool_name(), "Select");
    }

    /// AC#6 — `ToolManager::default()` starts with `SelectTool`.
    #[test]
    fn tool_manager_default_has_select_tool() {
        let manager = ToolManager::default();
        assert_eq!(manager.active_tool_name(), "Select");
    }

    /// AC#14 — `set_tool` calls `cancel()` on the old tool before replacing it.
    #[test]
    fn tool_manager_set_tool_cancels_old() {
        // Build a mock tool that tracks whether `cancel()` was called.
        #[derive(Clone)]
        struct MockTool {
            cancelled: Rc<RefCell<bool>>,
        }
        impl Tool for MockTool {
            fn name(&self) -> &'static str {
                "Mock"
            }
            fn on_pointer_down(&mut self, _pos: Vec2, _app: &mut App) {}
            fn on_pointer_move(&mut self, _pos: Vec2, _app: &mut App) {}
            fn on_pointer_up(&mut self, _pos: Vec2, _app: &mut App) {}
            fn on_key(&mut self, _key: egui::Key, _app: &mut App) {}
            fn preview(&self) -> Vec<Entity> {
                vec![]
            }
            fn cancel(&mut self) {
                *self.cancelled.borrow_mut() = true;
            }
        }

        let cancelled_flag = Rc::new(RefCell::new(false));
        let mock = MockTool {
            cancelled: cancelled_flag.clone(),
        };

        let mut manager = ToolManager::new(Box::new(mock));
        assert_eq!(manager.active_tool_name(), "Mock");
        assert!(!*cancelled_flag.borrow(), "cancel not called yet");

        manager.set_tool(Box::new(SelectTool));
        assert!(
            *cancelled_flag.borrow(),
            "cancel should have been called on old tool"
        );
        assert_eq!(manager.active_tool_name(), "Select");
    }
}
