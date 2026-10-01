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
use crate::cmdline::ToolInput;
use crate::document::{Document, Entity, History};
use crate::geometry::Vec2;
use crate::tools::Mark;
use std::borrow::Cow;

/// Entity pick aperture in screen points (LCV-162, DESIGN.md §5): an entity
/// within this distance of the cursor can be picked; the pickbox side is
/// twice this. Tools turn it into mm with the live zoom.
pub const PICK_APERTURE_PT: f64 = 5.0;

/// How far, in screen points, a Select press must move before it becomes a
/// box drag (LCV-162, DESIGN.md §5).
pub const DRAG_THRESHOLD_PT: f64 = 2.0;

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

    /// True while this tool wants the command line as a free-text field
    /// rather than the parser (ADR 0003 §D, LCV-112).
    ///
    /// While `true`, `src/app/cmdline.rs::submit` skips [`crate::cmdline::parse`]
    /// entirely and forwards the submitted text verbatim to
    /// [`Self::on_raw_input`], and `src/ui/command_line.rs` keeps the field
    /// focused every frame — which is also why no keyboard-gate exception is
    /// needed for raw mode: a focused field already makes
    /// `ctx.wants_keyboard_input()` suppress the bare tool-activation keys.
    ///
    /// The default is `false`. [`TextTool`](super::TextTool) is the only
    /// implementor.
    fn wants_raw_input(&self) -> bool {
        false
    }

    /// The command line's submitted text, unparsed (ADR 0003 §D, LCV-112).
    ///
    /// Called only while [`Self::wants_raw_input`] is `true`. Returns `true`
    /// when the input was consumed and a phase advanced (possibly
    /// committing); the default `false` means "not for me", which no tool
    /// reaches in practice since the app never calls this while
    /// `wants_raw_input()` is `false`.
    fn on_raw_input(&mut self, _raw: &str, _doc: &mut Document, _history: &mut History) -> bool {
        false
    }

    /// Context-sensitive status bar text for the current tool state.
    ///
    /// Defaults to `self.name()`. Override to return richer prompts that
    /// reflect the tool's internal state, in the grammar
    /// `VERB  Specify <thing> [Opt/Opt] <default>:` (DESIGN.md §7). A `Cow`
    /// so a prompt can carry a runtime value, e.g. a formatted default
    /// (LCV-165 AC 2); `src/ui/command_line.rs` is the only consumer.
    fn status_text(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.name())
    }

    /// The anchor point for ortho / snap constraints: the last committed
    /// endpoint that constrains the next cursor position.
    ///
    /// Returns `Some(p)` when the tool is awaiting a second point (i.e. has a
    /// fixed first point); returns `None` in idle state or for tools that have
    /// no meaningful anchor (select, move, etc.). Defaults to `None`.
    ///
    /// Object-safe: no generic parameters, takes only `&self`, returns
    /// `Option<Vec2>`. Introduced by demand LCV-053.
    fn anchor(&self) -> Option<crate::geometry::Vec2> {
        None
    }

    /// True while this tool waits for an entity pick (Select idle, TRIM,
    /// EXTEND): the canvas then paints the pickbox and resolves no running
    /// snap (LCV-162 AC 5, AC 11). Defaults to `false`: a point pick.
    fn wants_entity_pick(&self) -> bool {
        false
    }

    /// The live zoom in mm per screen point, forwarded by
    /// [`ToolManager::set_pick_scale`](super::ToolManager::set_pick_scale)
    /// (LCV-162). An entity-picking tool multiplies [`PICK_APERTURE_PT`] and
    /// [`DRAG_THRESHOLD_PT`] by it. The default ignores it.
    fn set_pick_scale(&mut self, _mm_per_pt: f64) {}

    /// Preview geometry for the current tool state. Returns an empty vector
    /// if the tool has no in-progress preview. The returned entities are
    /// painted with a translucent amber stroke by `draw_preview` (LCV-037).
    fn preview(&self) -> Vec<Entity>;

    /// Styled canvas feedback for this frame (ADR 0013), a pure query at
    /// paint time. `cursor` is the world point sent as this frame's `Move`,
    /// or `None` off the canvas or after Esc; `Hover`/`Danger` marks are
    /// returned only for `Some`. The default wraps [`Self::preview`] as
    /// [`Mark::Preview`], so a tool that does not override it paints as before.
    fn feedback(&self, _doc: &Document, _cursor: Option<Vec2>) -> Vec<Mark> {
        self.preview().into_iter().map(Mark::Preview).collect()
    }

    /// Reset the tool to idle state. Called on Escape press or tool switch.
    /// Clears any in-progress state and preview geometry.
    fn cancel(&mut self);

    /// Handle one resolved command-line input (LCV-111, ADR 0003 §B1).
    ///
    /// Called from `crate::app::submit` after the raw text has been parsed
    /// **and resolved**: `@dx,dy` is already added to [`Self::anchor`] and a
    /// bare distance is already projected along the cursor direction. A tool
    /// therefore never parses a string and never reads `App`.
    ///
    /// Returns `true` when the input was consumed and a phase advanced
    /// (possibly committing); `false` means "not for me" — on `false` the app
    /// mutates nothing, sets a feedback message and leaves this tool's phase
    /// untouched.
    ///
    /// The default is `false`, so `SelectTool`, `TrimTool`, `ExtendTool` and
    /// `DeleteTool` are untouched and a typed coordinate can never silently
    /// select or delete something.
    ///
    /// The method must remain object-safe — no generic parameters, no `Self`
    /// bounds.
    fn on_command_input(
        &mut self,
        _input: ToolInput,
        _doc: &mut Document,
        _history: &mut History,
    ) -> bool {
        false
    }

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

    /// Optionally hand the command line one result line, e.g. DIST's
    /// `Distance = …` report (LCV-159, ADR 0003 amendment (5)).
    ///
    /// Single-shot like [`Self::take_successor`], and drained just before it
    /// by `src/app/viewport.rs::poll_successor`, so the pointer path and the
    /// typed path share one body and a hand-over cannot swallow the text. The
    /// default always returns `None`.
    fn take_message(&mut self) -> Option<String> {
        None
    }

    /// True when the tool is at rest, waiting for a command: an empty Enter
    /// then repeats the last command word (LCV-165 AC 4). Only SELECT at
    /// rest overrides this; the default is `false`.
    fn at_rest(&self) -> bool {
        false
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

    /// LCV-068 AC#4 / LCV-111 AC 2 — `SelectTool::on_command_input` keeps the
    /// `false` default: it consumes nothing and mutates nothing.
    #[test]
    fn select_tool_on_command_input_is_noop() {
        let mut tool = SelectTool::default();
        let mut doc = Document::default();
        let mut hist = History::default();
        // Must not panic and must not mutate the document.
        let consumed =
            tool.on_command_input(ToolInput::Point(Vec2::new(50.0, 0.0)), &mut doc, &mut hist);
        assert!(!consumed, "the default impl must refuse every input");
        assert_eq!(doc.entity_count(), 0);
    }

    /// LCV-111 AC 3 — the tools that keep the default (`SelectTool`,
    /// `TrimTool`, `ExtendTool`, `DeleteTool`) all refuse both input shapes
    /// and leave the document alone.
    #[test]
    fn select_trim_extend_delete_reject_command_input() {
        use crate::tools::{DeleteTool, ExtendTool, TrimTool};

        let inputs = [
            ToolInput::Point(Vec2::new(50.0, 25.0)),
            ToolInput::Distance {
                value_mm: 50.0,
                along: Some(Vec2::new(50.0, 0.0)),
            },
        ];
        let mut tools: Vec<Box<dyn Tool>> = vec![
            Box::new(SelectTool::default()),
            Box::new(TrimTool::default()),
            Box::new(ExtendTool::default()),
            Box::new(DeleteTool),
        ];
        for tool in &mut tools {
            for input in inputs {
                let mut doc = Document::default();
                let mut hist = History::default();
                assert!(
                    !tool.on_command_input(input, &mut doc, &mut hist),
                    "{} must refuse {input:?}",
                    tool.name()
                );
                assert_eq!(doc.entity_count(), 0);
                assert_eq!(hist.len(), 0);
            }
        }
    }

    /// LCV-053 AC#11 — `Tool::anchor()` default returns `None`; `SelectTool`
    /// has no override, so it inherits the default.
    #[test]
    fn tool_default_anchor_is_none() {
        assert_eq!(SelectTool::default().anchor(), None);
    }

    /// LCV-159 AC7 — `Tool::take_message()` defaults to `None`.
    #[test]
    fn tool_default_message_is_none() {
        assert_eq!(SelectTool::default().take_message(), None);
    }
}
