//! Tool trait: state-machine interface for drawing and modify tools.
//!
//! Tools MUST mutate the document via `history.commit(cmd, doc)` — never by
//! direct field access. The `on_pointer_down` / `on_pointer_up` methods
//! receive `&mut Document` and `&mut History` so they can commit commands
//! without needing the full `App` struct (LCV-041). `on_key` still takes
//! `&mut App` because it may need access to camera or other app state; that
//! will narrow in a later demand if warranted.
//!
//! The trait is **object-safe** — [`ToolManager`](super::ToolManager) stores
//! `Box<dyn Tool>`. No generic methods, no associated types.
//!
//! The `preview()` method returns ephemeral preview geometry that the render
//! pipeline paints with a translucent amber stroke (LCV-037). The returned
//! `Vec<Entity>` wires into `App::preview_entities` each frame.
//!
//! `cancel()` resets the tool to idle state. It is called on Escape press or
//! tool switch; `on_key(Key::Escape, app)` typically calls `self.cancel()`
//! internally. These are related but distinct: `on_key` is event-driven,
//! `cancel` is state-driven.
//!
//! MUST NOT import `eframe` or `rfd`. Introduced by demand LCV-040.

use crate::app::App;
use crate::document::{Document, Entity, History};
use crate::geometry::Vec2;

/// Tool trait: state-machine interface for drawing and modify tools.
///
/// Every tool implements this trait. `ToolManager` owns the active tool
/// as `Box<dyn Tool>` and routes pointer and keyboard events to it.
///
/// Pointer methods receive `&mut Document` and `&mut History` (where
/// applicable) so tools can commit commands without holding a full `App`
/// reference. Use `history.commit(cmd, doc)` — never mutate `doc.entities`
/// directly.
pub trait Tool {
    /// Tool name for status-bar display (e.g., `"Select"`, `"Line"`, `"Circle"`).
    fn name(&self) -> &'static str;

    /// Primary-button press at `pos` (world mm). Called when the left button
    /// is pressed while the cursor is inside the viewport.
    ///
    /// `shift` mirrors the Shift-key modifier state at the time of the press
    /// (read from `egui::Modifiers`). Tools that toggle or extend selections
    /// inspect `shift`; others ignore it (name it `_shift`).
    fn on_pointer_down(
        &mut self,
        pos: Vec2,
        shift: bool,
        doc: &mut Document,
        history: &mut History,
    );

    /// Cursor movement to `pos` (world mm). Called every frame the cursor
    /// hovers the viewport, regardless of button state.
    fn on_pointer_move(&mut self, pos: Vec2, doc: &mut Document);

    /// Primary-button release at `pos` (world mm). Called when the left
    /// button is released while the cursor is inside the viewport.
    ///
    /// `shift` mirrors the Shift-key modifier state at the time of the release.
    /// Tools that finalise a shifted operation (e.g., toggle-add to selection)
    /// inspect `shift`; others ignore it (name it `_shift`).
    fn on_pointer_up(&mut self, pos: Vec2, shift: bool, doc: &mut Document, history: &mut History);

    /// Keyboard input (e.g., Escape to cancel). `on_key(Key::Escape, app)`
    /// typically calls `self.cancel()` internally.
    fn on_key(&mut self, key: egui::Key, app: &mut App);

    /// Character input from the keyboard (e.g. letter/digit typed by the
    /// operator while a text tool is active). The default implementation is a
    /// no-op; override in tools that accept typed input (LCV-048).
    fn on_text_input(&mut self, _ch: char) {}

    /// Context-sensitive status bar text for the current tool state.
    ///
    /// Defaults to `self.name()`. Override to return richer prompts that
    /// reflect the tool's internal state (e.g. `"LINE: Click to set end point"`).
    fn status_text(&self) -> &'static str {
        self.name()
    }

    /// Preview geometry for the current tool state. Returns an empty vector
    /// if the tool has no in-progress preview. The returned entities are
    /// painted with a translucent amber stroke by `draw_preview` (LCV-037).
    fn preview(&self) -> Vec<Entity>;

    /// Reset the tool to idle state. Called on Escape press or tool switch.
    /// Clears any in-progress state and preview geometry.
    fn cancel(&mut self);

    /// Handle text input submitted from the command-line widget (LCV-068).
    ///
    /// Called when the user presses Enter in the command-line strip while this
    /// tool is active. `input` contains the trimmed text that was in the field.
    /// The default implementation is a no-op; tools that accept numeric or
    /// coordinate input override this method.
    ///
    /// The method must remain object-safe — no generic parameters, no `Self`
    /// bounds.
    fn on_command_input(&mut self, _input: &str, _doc: &mut Document, _history: &mut History) {}

    /// Optionally hand control to a successor tool after a pointer event.
    ///
    /// Returns `Some(tool)` exactly once when the active tool wants to
    /// transfer control (e.g. [`MoveTool`](super::MoveTool) returns a fresh
    /// [`SelectTool`](super::SelectTool) after a successful commit). The flag
    /// is **single-shot**: subsequent calls return `None` until the next
    /// qualifying event.
    ///
    /// The default implementation always returns `None`. Existing tools
    /// ([`LineTool`](super::LineTool), [`SelectTool`](super::SelectTool), …)
    /// do not need to override this method.
    ///
    /// The trait remains object-safe — no generics, no `Self` bounds.
    ///
    /// Introduced by demand LCV-049.
    fn take_successor(&mut self) -> Option<Box<dyn Tool>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::SelectTool;

    /// LCV-068 AC#4 — `Tool` remains object-safe after adding `on_command_input`.
    #[test]
    fn tool_with_command_input_is_object_safe() {
        // If this compiles, the trait is object-safe.
        let _: Box<dyn Tool> = Box::new(SelectTool::default());
    }

    /// LCV-068 AC#4 — `SelectTool::on_command_input` is a no-op (default impl).
    #[test]
    fn select_tool_on_command_input_is_noop() {
        let mut tool = SelectTool::default();
        let mut doc = Document::default();
        let mut hist = History::default();
        // Must not panic and must not mutate the document.
        tool.on_command_input("50", &mut doc, &mut hist);
        assert_eq!(doc.entity_count(), 0);
    }
}
