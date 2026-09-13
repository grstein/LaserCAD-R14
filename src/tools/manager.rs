//! ToolManager: owner of the active tool, routes pointer and key events.
//!
//! Holds one active `Box<dyn Tool>`, delegates pointer / keyboard events,
//! and exposes preview geometry. `set_tool` calls `cancel()` on the old tool.
//!
//! **Dispatch paths** — `on_pointer_event` (LCV-041, preferred) takes a
//! [`PointerEvent`] + `&mut Document` + `&mut History`; `handle_pointer_*`
//! (LCV-040, legacy) extract doc/history from `&mut App` and call through.
//!
//! MUST NOT import `eframe` or `rfd`. Introduced by demand LCV-040.

use crate::app::App;
use crate::cmdline::ToolInput;
use crate::document::{Document, Entity, History};
use crate::geometry::Vec2;
use crate::tools::pointer_event::{PointerButton, PointerEvent};
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

    // ------------------------------------------------------------------
    // Unified dispatch (LCV-041)
    // ------------------------------------------------------------------

    /// Dispatch a [`PointerEvent`] to the active tool.
    ///
    /// - `Move` → [`Tool::on_pointer_move`]
    /// - `Press { button: Primary }` → [`Tool::on_pointer_down`] (shift forwarded)
    /// - `Release { button: Primary }` → [`Tool::on_pointer_up`] (shift forwarded)
    /// - Other button combinations → no-op (reserved for future demands).
    pub fn on_pointer_event(
        &mut self,
        event: &PointerEvent,
        doc: &mut Document,
        history: &mut History,
    ) {
        match event {
            PointerEvent::Move { world_pos } => {
                self.active.on_pointer_move(*world_pos, doc);
            }
            PointerEvent::Press {
                world_pos,
                button: PointerButton::Primary,
                shift,
            } => {
                self.active
                    .on_pointer_down(*world_pos, *shift, doc, history);
            }
            PointerEvent::Release {
                world_pos,
                button: PointerButton::Primary,
                shift,
            } => {
                self.active.on_pointer_up(*world_pos, *shift, doc, history);
            }
            // Secondary / Middle buttons: no-op for now.
            PointerEvent::Press { .. } | PointerEvent::Release { .. } => {}
        }
    }

    // ------------------------------------------------------------------
    // Legacy wrappers (LCV-040 — kept for backwards compatibility)
    // ------------------------------------------------------------------

    /// Route a pointer-down event to the active tool.
    ///
    /// Legacy wrapper: extracts `doc` and `history` from `app` and calls
    /// [`Tool::on_pointer_down`] with `shift: false`.
    pub fn handle_pointer_down(&mut self, pos: Vec2, app: &mut App) {
        self.active
            .on_pointer_down(pos, false, &mut app.document, &mut app.history);
    }

    /// Route a pointer-move event to the active tool.
    ///
    /// Legacy wrapper: extracts `doc` from `app` and calls
    /// [`Tool::on_pointer_move`].
    pub fn handle_pointer_move(&mut self, pos: Vec2, app: &mut App) {
        self.active.on_pointer_move(pos, &mut app.document);
    }

    /// Route a pointer-up event to the active tool.
    ///
    /// Legacy wrapper: extracts `doc` and `history` from `app` and calls
    /// [`Tool::on_pointer_up`] with `shift: false`.
    pub fn handle_pointer_up(&mut self, pos: Vec2, app: &mut App) {
        self.active
            .on_pointer_up(pos, false, &mut app.document, &mut app.history);
    }

    /// Route a keyboard event to the active tool.
    pub fn handle_key(&mut self, key: egui::Key, app: &mut App) {
        self.active.on_key(key, app);
    }

    /// Forward a typed character to the active tool (LCV-048 TextTool).
    ///
    /// No `&mut App` borrow is needed — `on_text_input` does not call back into
    /// `App`, so `std::mem::take` is unnecessary here.
    pub fn on_text_input(&mut self, ch: char) {
        self.active.on_text_input(ch);
    }

    /// Get the preview geometry from the active tool. Returns an empty vector
    /// if the tool has no in-progress preview.
    pub fn preview(&self) -> Vec<Entity> {
        self.active.preview()
    }

    /// Context-sensitive prompt text for the command-line widget (LCV-068).
    ///
    /// Delegates to `active.status_text()`. When `SelectTool` is active this
    /// returns R14's idle prompt `"Command:"`; the drawing tools return a
    /// per-phase prompt (LCV-111 AC 17).
    pub fn active_status_text(&self) -> &'static str {
        self.active.status_text()
    }

    /// Forward one resolved command-line input to the active tool (LCV-111).
    ///
    /// Called from `crate::app::submit` once the raw text has been parsed and
    /// resolved into a [`ToolInput`]. Returns the tool's verdict verbatim:
    /// `true` = consumed, `false` = refused (the caller then sets the
    /// feedback message and mutates nothing).
    pub fn on_command_input(
        &mut self,
        input: ToolInput,
        doc: &mut Document,
        history: &mut History,
    ) -> bool {
        self.active.on_command_input(input, doc, history)
    }

    /// Forward `take_successor` to the active tool.
    ///
    /// Returns `Some(t)` exactly once after the active tool requests a hand-off
    /// (e.g. [`MoveTool`](crate::tools::MoveTool) after a successful commit).
    /// Returns `None` otherwise.
    ///
    /// Introduced by demand LCV-049.
    pub fn take_successor(&mut self) -> Option<Box<dyn Tool>> {
        self.active.take_successor()
    }

    /// The active tool's anchor point for ortho / snap constraints.
    ///
    /// Returns `None` when the active tool has no live anchor (e.g. idle
    /// state or a tool without the concept of a first point). Delegates
    /// directly to [`Tool::anchor`]. Introduced by demand LCV-053.
    pub fn anchor(&self) -> Option<crate::geometry::Vec2> {
        self.active.anchor()
    }
}

impl Default for ToolManager {
    /// Default `ToolManager` starts with a `SelectTool` (LCV-040 AC#6).
    fn default() -> Self {
        Self::new(Box::new(crate::tools::SelectTool::default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::tools::pointer_event::{PointerButton, PointerEvent};
    use crate::tools::SelectTool;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// AC#3, AC#6 — `ToolManager` can be constructed with `SelectTool` and
    /// reports its name correctly.
    #[test]
    fn tool_manager_new_with_select_tool() {
        let manager = ToolManager::new(Box::new(SelectTool::default()));
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

        manager.set_tool(Box::new(SelectTool::default()));
        assert!(*cancelled_flag.borrow(), "cancel should have been called");
        assert_eq!(manager.active_tool_name(), "Select");
    }

    /// LCV-041 — `on_pointer_event` dispatches Move/Primary-Press/Release to
    /// the correct tool methods; Secondary and Middle buttons are no-ops.
    #[test]
    fn on_pointer_event_dispatches_correctly() {
        #[derive(Clone)]
        struct CountTool {
            moves: Rc<RefCell<u32>>,
            downs: Rc<RefCell<u32>>,
            ups: Rc<RefCell<u32>>,
        }
        impl Tool for CountTool {
            fn name(&self) -> &'static str {
                "Count"
            }
            fn on_pointer_down(
                &mut self,
                _: Vec2,
                _shift: bool,
                _: &mut Document,
                _: &mut History,
            ) {
                *self.downs.borrow_mut() += 1;
            }
            fn on_pointer_move(&mut self, _: Vec2, _: &mut Document) {
                *self.moves.borrow_mut() += 1;
            }
            fn on_pointer_up(&mut self, _: Vec2, _shift: bool, _: &mut Document, _: &mut History) {
                *self.ups.borrow_mut() += 1;
            }
            fn on_key(&mut self, _: egui::Key, _: &mut App) {}
            fn preview(&self) -> Vec<Entity> {
                vec![]
            }
            fn cancel(&mut self) {}
        }

        let moves = Rc::new(RefCell::new(0u32));
        let downs = Rc::new(RefCell::new(0u32));
        let ups = Rc::new(RefCell::new(0u32));
        let mut mgr = ToolManager::new(Box::new(CountTool {
            moves: moves.clone(),
            downs: downs.clone(),
            ups: ups.clone(),
        }));
        let mut doc = Document::default();
        let mut hist = History::default();
        let pos = Vec2::new(1.0, 2.0);

        // Move → on_pointer_move.
        mgr.on_pointer_event(&PointerEvent::Move { world_pos: pos }, &mut doc, &mut hist);
        assert_eq!(*moves.borrow(), 1, "move → on_pointer_move");
        assert_eq!(*downs.borrow(), 0);
        assert_eq!(*ups.borrow(), 0);

        // Primary press → on_pointer_down.
        mgr.on_pointer_event(
            &PointerEvent::Press {
                world_pos: pos,
                button: PointerButton::Primary,
                shift: false,
            },
            &mut doc,
            &mut hist,
        );
        assert_eq!(*downs.borrow(), 1, "primary press → on_pointer_down");

        // Primary release → on_pointer_up.
        mgr.on_pointer_event(
            &PointerEvent::Release {
                world_pos: pos,
                button: PointerButton::Primary,
                shift: false,
            },
            &mut doc,
            &mut hist,
        );
        assert_eq!(*ups.borrow(), 1, "primary release → on_pointer_up");

        // Secondary press → no-op.
        mgr.on_pointer_event(
            &PointerEvent::Press {
                world_pos: pos,
                button: PointerButton::Secondary,
                shift: false,
            },
            &mut doc,
            &mut hist,
        );
        assert_eq!(*downs.borrow(), 1, "secondary press must be no-op");

        // Middle release → no-op.
        mgr.on_pointer_event(
            &PointerEvent::Release {
                world_pos: pos,
                button: PointerButton::Middle,
                shift: false,
            },
            &mut doc,
            &mut hist,
        );
        assert_eq!(*ups.borrow(), 1, "middle release must be no-op");
    }

    /// LCV-068 AC#5 / LCV-111 AC 17, AC 18 — `active_status_text` returns
    /// R14's idle prompt `"Command:"` when `SelectTool` is active.
    #[test]
    fn tool_manager_active_status_text_defaults_to_name() {
        let manager = ToolManager::default();
        assert_eq!(manager.active_status_text(), "Command:");
    }

    /// LCV-068 AC#5 — `active_status_text` reflects an overridden `status_text`.
    #[test]
    fn tool_manager_active_status_text_reflects_override() {
        struct PromptTool;
        impl Tool for PromptTool {
            fn name(&self) -> &'static str {
                "Prompt"
            }
            fn status_text(&self) -> &'static str {
                "LINE: Click start point"
            }
            fn on_pointer_down(&mut self, _: Vec2, _: bool, _: &mut Document, _: &mut History) {}
            fn on_pointer_move(&mut self, _: Vec2, _: &mut Document) {}
            fn on_pointer_up(&mut self, _: Vec2, _: bool, _: &mut Document, _: &mut History) {}
            fn on_key(&mut self, _: egui::Key, _: &mut App) {}
            fn preview(&self) -> Vec<Entity> {
                vec![]
            }
            fn cancel(&mut self) {}
        }

        let manager = ToolManager::new(Box::new(PromptTool));
        assert_eq!(manager.active_status_text(), "LINE: Click start point");
    }

    /// LCV-068 AC#6 / LCV-111 AC 2 — `on_command_input` delegates the
    /// resolved [`ToolInput`] to the active tool and forwards its verdict.
    #[test]
    fn tool_manager_on_command_input_delegates() {
        struct RecordTool {
            calls: Rc<RefCell<Vec<ToolInput>>>,
        }
        impl Tool for RecordTool {
            fn name(&self) -> &'static str {
                "Record"
            }
            fn on_pointer_down(&mut self, _: Vec2, _: bool, _: &mut Document, _: &mut History) {}
            fn on_pointer_move(&mut self, _: Vec2, _: &mut Document) {}
            fn on_pointer_up(&mut self, _: Vec2, _: bool, _: &mut Document, _: &mut History) {}
            fn on_key(&mut self, _: egui::Key, _: &mut App) {}
            fn preview(&self) -> Vec<Entity> {
                vec![]
            }
            fn cancel(&mut self) {}
            fn on_command_input(
                &mut self,
                input: ToolInput,
                _doc: &mut Document,
                _history: &mut History,
            ) -> bool {
                self.calls.borrow_mut().push(input);
                true
            }
        }

        let calls = Rc::new(RefCell::new(Vec::<ToolInput>::new()));
        let mut manager = ToolManager::new(Box::new(RecordTool {
            calls: calls.clone(),
        }));
        let mut doc = Document::default();
        let mut hist = History::default();

        let input = ToolInput::Point(Vec2::new(42.0, -7.0));
        assert!(manager.on_command_input(input, &mut doc, &mut hist));

        let recorded = calls.borrow();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0], input);
    }

    /// LCV-048 AC#7 — `SelectTool::on_text_input` is a no-op (default impl).
    #[test]
    fn select_tool_on_text_input_is_noop() {
        SelectTool::default().on_text_input('x');
        SelectTool::default().on_text_input(' ');
        SelectTool::default().on_text_input('\n');
    }

    /// LCV-048 AC#8 — `ToolManager::on_text_input` with the default SelectTool
    /// does not panic.
    #[test]
    fn tool_manager_on_text_input_noop_for_select() {
        let mut mgr = ToolManager::default();
        mgr.on_text_input('a');
        mgr.on_text_input('Z');
        mgr.on_text_input('!');
    }

    /// LCV-053 AC#10 — `ToolManager::anchor()` delegates to the active tool.
    ///
    /// With `LineTool` active: idle → `None`; after first click → `Some(p1)`.
    #[test]
    fn tool_manager_anchor_delegates_to_active_tool() {
        use crate::tools::LineTool;
        let mut manager = ToolManager::new(Box::new(LineTool::new()));
        assert_eq!(manager.anchor(), None, "idle LineTool has no anchor");

        let mut app = App::default();
        manager.handle_pointer_down(Vec2::new(1.0, 2.0), &mut app);
        assert_eq!(
            manager.anchor(),
            Some(Vec2::new(1.0, 2.0)),
            "anchor should be the first click point"
        );
    }
}
