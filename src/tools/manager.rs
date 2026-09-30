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
use crate::tools::feedback::FeedbackGate;
use crate::tools::pointer_event::{PointerButton, PointerEvent};
use crate::tools::{Mark, Tool};
use std::borrow::Cow;

/// Owner of the active tool, routes pointer and key events.
///
/// Holds exactly one active tool as `Box<dyn Tool>`. Delegates all pointer
/// and keyboard events to the active tool.
pub struct ToolManager {
    /// The currently active tool. Never `None` — the app always has a tool.
    active: Box<dyn Tool>,
    /// The live zoom in mm per screen point, as last set by
    /// [`Self::set_pick_scale`]; `1.0` (the default camera) until then.
    mm_per_pt: f64,
    /// Esc mute of the hover and danger feedback (LCV-163 AC 9).
    gate: FeedbackGate,
}

impl std::fmt::Debug for ToolManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolManager")
            .field("active", &self.active.name())
            .finish()
    }
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
        Self {
            active: initial,
            mm_per_pt: 1.0,
            gate: FeedbackGate::default(),
        }
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
    ///
    /// The new tool gets the stored pick scale before its first event
    /// (LCV-162), so a pick right after a switch uses the live zoom.
    pub fn set_tool(&mut self, mut tool: Box<dyn Tool>) {
        self.active.cancel();
        tool.set_pick_scale(self.mm_per_pt);
        self.active = tool;
    }

    /// Store the live zoom in mm per screen point and forward it to the
    /// active tool (LCV-162). Called every frame by the viewport before any
    /// pointer event.
    pub fn set_pick_scale(&mut self, mm_per_pt: f64) {
        self.mm_per_pt = mm_per_pt;
        self.active.set_pick_scale(mm_per_pt);
    }

    /// True while the active tool waits for an entity pick (LCV-162 AC 5):
    /// the canvas paints the pickbox and resolves no running snap.
    pub fn wants_entity_pick(&self) -> bool {
        self.active.wants_entity_pick()
    }

    /// The entity pick aperture in mm at the stored zoom:
    /// [`PICK_APERTURE_PT`](crate::tools::PICK_APERTURE_PT) × mm per point.
    pub fn pick_aperture_mm(&self) -> f64 {
        crate::tools::PICK_APERTURE_PT * self.mm_per_pt
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
                self.gate.on_move(*world_pos);
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

    /// Route a keyboard event to the active tool. Escape also mutes the
    /// hover and danger feedback until the pointer moves (LCV-163 AC 9).
    pub fn handle_key(&mut self, key: egui::Key, app: &mut App) {
        if key == egui::Key::Escape {
            self.gate.on_escape();
        }
        self.active.on_key(key, app);
    }

    /// True while the active tool wants the command line as a free-text
    /// field rather than the parser (ADR 0003 §D, LCV-112). Delegates to
    /// [`Tool::wants_raw_input`].
    pub fn wants_raw_input(&self) -> bool {
        self.active.wants_raw_input()
    }

    /// Forward one submitted, unparsed command-line string to the active
    /// tool (ADR 0003 §D, LCV-112). Called only while
    /// [`Self::wants_raw_input`] is `true`. Delegates to
    /// [`Tool::on_raw_input`] and returns its verdict verbatim.
    pub fn on_raw_input(&mut self, raw: &str, doc: &mut Document, history: &mut History) -> bool {
        self.active.on_raw_input(raw, doc, history)
    }

    /// Get the preview geometry from the active tool. Returns an empty vector
    /// if the tool has no in-progress preview.
    pub fn preview(&self) -> Vec<Entity> {
        self.active.preview()
    }

    /// The active tool's styled canvas feedback at `cursor` (ADR 0013), with
    /// `cursor` passed as `None` while Esc has muted it (LCV-163 AC 9).
    pub fn feedback(&self, doc: &Document, cursor: Option<Vec2>) -> Vec<Mark> {
        self.active.feedback(doc, self.gate.cursor(cursor))
    }

    /// Context-sensitive prompt text for the command-line widget (LCV-068).
    ///
    /// Delegates to `active.status_text()`. When `SelectTool` is active this
    /// returns R14's idle prompt `"Command:"`; the drawing tools return a
    /// per-phase prompt (LCV-111 AC 17).
    pub fn active_status_text(&self) -> Cow<'_, str> {
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

    /// Forward `take_message` to the active tool: its single-shot result
    /// line, if any (LCV-159).
    pub fn take_message(&mut self) -> Option<String> {
        self.active.take_message()
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
    use crate::tools::SelectTool;
    use crate::tools::pointer_event::{PointerButton, PointerEvent};
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
            fn status_text(&self) -> Cow<'_, str> {
                "LINE: Click start point".into()
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

    /// LCV-165 AC 2 — the manager hands an owned prompt through unchanged.
    #[test]
    fn tool_manager_hands_an_owned_prompt_through() {
        let mut manager = ToolManager::default();
        manager.set_tool(Box::new(crate::tools::TextTool::default()));
        let mut doc = Document::default();
        let mut history = History::default();
        let press = PointerEvent::Press {
            world_pos: Vec2::new(0.0, 0.0),
            button: PointerButton::Primary,
            shift: false,
        };
        manager.on_pointer_event(&press, &mut doc, &mut history);
        manager.on_raw_input("HELLO", &mut doc, &mut history);
        let prompt = manager.active_status_text();
        assert_eq!(prompt, "TEXT  Specify height <5>:");
        assert!(matches!(prompt, Cow::Owned(_)), "{prompt:?}");
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

    /// LCV-112 AC 1 — `SelectTool` keeps the `wants_raw_input` default and
    /// `ToolManager` forwards it unchanged.
    #[test]
    fn select_tool_does_not_want_raw_input() {
        assert!(!SelectTool::default().wants_raw_input());
        assert!(!ToolManager::default().wants_raw_input());
    }

    /// LCV-112 AC 1 — `ToolManager::on_raw_input` with the default
    /// `SelectTool` refuses (the `false` default) and mutates nothing.
    #[test]
    fn tool_manager_on_raw_input_noop_for_select() {
        let mut mgr = ToolManager::default();
        let mut doc = Document::default();
        let mut hist = History::default();
        assert!(!mgr.on_raw_input("anything", &mut doc, &mut hist));
        assert_eq!(doc.entity_count(), 0);
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

    /// A tool that records the last pick scale it was handed.
    struct ScaleProbe {
        scale: Rc<RefCell<Option<f64>>>,
        picks: bool,
    }
    impl Tool for ScaleProbe {
        fn name(&self) -> &'static str {
            "Probe"
        }
        fn on_pointer_down(&mut self, _: Vec2, _: bool, _: &mut Document, _: &mut History) {}
        fn on_pointer_move(&mut self, _: Vec2, _: &mut Document) {}
        fn on_pointer_up(&mut self, _: Vec2, _: bool, _: &mut Document, _: &mut History) {}
        fn on_key(&mut self, _: egui::Key, _: &mut App) {}
        fn preview(&self) -> Vec<Entity> {
            vec![]
        }
        fn cancel(&mut self) {}
        fn wants_entity_pick(&self) -> bool {
            self.picks
        }
        fn set_pick_scale(&mut self, mm_per_pt: f64) {
            *self.scale.borrow_mut() = Some(mm_per_pt);
        }
    }

    fn probe(picks: bool) -> (ScaleProbe, Rc<RefCell<Option<f64>>>) {
        let scale = Rc::new(RefCell::new(None));
        let tool = ScaleProbe {
            scale: scale.clone(),
            picks,
        };
        (tool, scale)
    }

    /// LCV-162 AC 7 — `set_pick_scale` reaches the active tool, and the
    /// aperture in mm is `PICK_APERTURE_PT` times the scale (5 mm at the
    /// default 1 mm/pt).
    #[test]
    fn set_pick_scale_forwards_and_scales_the_aperture() {
        let (tool, scale) = probe(true);
        let mut manager = ToolManager::new(Box::new(tool));
        assert_eq!(manager.pick_aperture_mm(), 5.0);
        manager.set_pick_scale(20.0);
        assert_eq!(*scale.borrow(), Some(20.0));
        assert_eq!(manager.pick_aperture_mm(), 100.0);
        manager.set_pick_scale(0.05);
        assert_eq!(
            manager.pick_aperture_mm(),
            crate::tools::PICK_APERTURE_PT * 0.05
        );
    }

    /// LCV-162 AC 7–9 — a tool switched in gets the stored scale at once,
    /// without waiting for the next frame.
    #[test]
    fn set_tool_forwards_the_stored_scale() {
        let mut manager = ToolManager::default();
        manager.set_pick_scale(0.05);
        let (tool, scale) = probe(false);
        manager.set_tool(Box::new(tool));
        assert_eq!(*scale.borrow(), Some(0.05));
    }

    /// LCV-162 AC 5/AC 6 — `wants_entity_pick` delegates; the trait default
    /// is `false` (a point pick).
    #[test]
    fn wants_entity_pick_delegates_with_a_false_default() {
        let (tool, _) = probe(true);
        assert!(ToolManager::new(Box::new(tool)).wants_entity_pick());
        let (tool, _) = probe(false);
        assert!(!ToolManager::new(Box::new(tool)).wants_entity_pick());
        let line = ToolManager::new(Box::new(crate::tools::LineTool::default()));
        assert!(!line.wants_entity_pick());
        assert_eq!(crate::tools::DRAG_THRESHOLD_PT, 2.0);
    }

    /// LCV-163 AC 9 — after Escape the tool sees no cursor at the same
    /// point; a move to another point hands it the cursor again.
    #[test]
    fn escape_mutes_feedback_until_the_pointer_moves() {
        struct Probe;
        impl Tool for Probe {
            fn name(&self) -> &'static str {
                "Probe"
            }
            fn on_pointer_down(&mut self, _: Vec2, _: bool, _: &mut Document, _: &mut History) {}
            fn on_pointer_move(&mut self, _: Vec2, _: &mut Document) {}
            fn on_pointer_up(&mut self, _: Vec2, _: bool, _: &mut Document, _: &mut History) {}
            fn on_key(&mut self, _: egui::Key, _: &mut App) {}
            fn preview(&self) -> Vec<Entity> {
                vec![]
            }
            fn cancel(&mut self) {}
            fn feedback(&self, _: &Document, cursor: Option<Vec2>) -> Vec<Mark> {
                cursor.map(|_| Mark::Hover(0)).into_iter().collect()
            }
        }
        let mut manager = ToolManager::new(Box::new(Probe));
        let (mut app, mut doc, mut hist) =
            (App::default(), Document::default(), History::default());
        let (a, b) = (Vec2::new(1.0, 1.0), Vec2::new(2.0, 1.0));
        let mv = |world_pos| PointerEvent::Move { world_pos };
        manager.on_pointer_event(&mv(a), &mut doc, &mut hist);
        assert_eq!(manager.feedback(&doc, Some(a)), vec![Mark::Hover(0)]);
        manager.handle_key(egui::Key::Escape, &mut app);
        assert!(manager.feedback(&doc, Some(a)).is_empty(), "muted");
        manager.on_pointer_event(&mv(a), &mut doc, &mut hist);
        assert!(manager.feedback(&doc, Some(a)).is_empty(), "same point");
        manager.on_pointer_event(&mv(b), &mut doc, &mut hist);
        assert_eq!(manager.feedback(&doc, Some(b)), vec![Mark::Hover(0)]);
        manager.handle_key(egui::Key::Enter, &mut app);
        assert_eq!(
            manager.feedback(&doc, Some(b)),
            vec![Mark::Hover(0)],
            "only Esc mutes"
        );
    }
}
