//! Tool trait: state-machine interface for drawing and modify tools.
//!
//! Tools never mutate [`crate::document::Document`] directly; they construct
//! a `Box<dyn Command>` and call [`crate::app::App::commit`] (LCV-040).
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

use crate::document::Entity;
use crate::geometry::Vec2;

/// Forward-declare App to avoid circular dependency.
/// Tools take `&mut App` to access camera, document, and commit.
use crate::app::App;

/// Tool trait: state-machine interface for drawing and modify tools.
///
/// Every tool implements this trait. `ToolManager` owns the active tool
/// as `Box<dyn Tool>` and routes pointer and keyboard events to it.
///
/// Tools NEVER mutate the document directly — they construct a
/// `Box<dyn Command>` and call `app.commit(cmd)`.
pub trait Tool {
    /// Tool name for status-bar display (e.g., `"Select"`, `"Line"`, `"Circle"`).
    fn name(&self) -> &'static str;

    /// Left-button press at `pos` (world mm). Called when the pointer is
    /// pressed inside the viewport.
    fn on_pointer_down(&mut self, pos: Vec2, app: &mut App);

    /// Cursor movement to `pos` (world mm). Called whenever the cursor moves
    /// over the viewport, whether the button is down or not.
    fn on_pointer_move(&mut self, pos: Vec2, app: &mut App);

    /// Left-button release at `pos` (world mm). Called when the pointer is
    /// released while hovering the viewport.
    fn on_pointer_up(&mut self, pos: Vec2, app: &mut App);

    /// Keyboard input (e.g., Escape to cancel). `on_key(Key::Escape, app)`
    /// typically calls `self.cancel()` internally.
    fn on_key(&mut self, key: egui::Key, app: &mut App);

    /// Preview geometry for the current tool state. Returns an empty vector
    /// if the tool has no in-progress preview. The returned entities are
    /// painted with a translucent amber stroke by `draw_preview` (LCV-037).
    fn preview(&self) -> Vec<Entity>;

    /// Reset the tool to idle state. Called on Escape press or tool switch.
    /// Clears any in-progress state and preview geometry.
    fn cancel(&mut self);
}
