//! TextTool — AutoCAD R14 TEXT command. Click to set anchor, type to build
//! the string, Enter to commit as `Entity::Line` strokes.
//! Uses `layout_text` (LCV-055). Never mutates the document directly.
//!
//! MUST NOT import `eframe` or `rfd`. Introduced by demand LCV-048.

use crate::app::App;
use crate::document::{commands::CreateEntities, Document, Entity, History};
use crate::geometry::Vec2;
use crate::text::layout_text;
use crate::tools::Tool;

const DEFAULT_TEXT_HEIGHT_MM: f64 = 5.0;
const DEFAULT_SPACING_FACTOR: f64 = 1.0;

#[derive(Debug, Clone)]
enum TextToolState {
    Idle,
    WaitingInput { anchor: Vec2, text: String },
}

/// Click-to-place Hershey stroke text tool (LCV-048).
///
/// Click → anchor; type → buffer grows; Enter → commit [`CreateEntities`];
/// Escape / `cancel()` → `Idle` without committing.
#[derive(Debug)]
pub struct TextTool {
    state: TextToolState,
    height_mm: f64,
    spacing_factor: f64,
}

impl Default for TextTool {
    fn default() -> Self {
        Self {
            state: TextToolState::Idle,
            height_mm: DEFAULT_TEXT_HEIGHT_MM,
            spacing_factor: DEFAULT_SPACING_FACTOR,
        }
    }
}

impl Tool for TextTool {
    fn name(&self) -> &'static str {
        "TEXT"
    }

    fn status_text(&self) -> &'static str {
        match &self.state {
            TextToolState::Idle => "TEXT: Click to set insertion point",
            TextToolState::WaitingInput { .. } => {
                "TEXT: Type text, Enter to confirm, Escape to cancel"
            }
        }
    }

    /// First click transitions `Idle → WaitingInput`. Subsequent clicks while
    /// already waiting are no-ops (anchor does not move, nothing committed).
    fn on_pointer_down(
        &mut self,
        pos: Vec2,
        _shift: bool,
        _doc: &mut Document,
        _history: &mut History,
    ) {
        if let TextToolState::Idle = self.state {
            self.state = TextToolState::WaitingInput {
                anchor: pos,
                text: String::new(),
            };
        }
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

    fn on_text_input(&mut self, ch: char) {
        if let TextToolState::WaitingInput { ref mut text, .. } = self.state {
            text.push(ch);
        }
    }

    fn on_key(&mut self, key: egui::Key, app: &mut App) {
        match key {
            egui::Key::Backspace => {
                if let TextToolState::WaitingInput { ref mut text, .. } = self.state {
                    text.pop();
                }
            }
            egui::Key::Enter => {
                let old = std::mem::replace(&mut self.state, TextToolState::Idle);
                if let TextToolState::WaitingInput { anchor, text } = old {
                    if !text.is_empty() {
                        let lines = layout_text(&text, anchor, self.height_mm, self.spacing_factor);
                        app.commit(Box::new(CreateEntities::new(lines)));
                    }
                }
            }
            egui::Key::Escape => self.cancel(),
            _ => {}
        }
    }

    fn preview(&self) -> Vec<Entity> {
        match &self.state {
            TextToolState::Idle => vec![],
            TextToolState::WaitingInput { anchor, text } if !text.is_empty() => {
                layout_text(text, *anchor, self.height_mm, self.spacing_factor)
            }
            _ => vec![],
        }
    }

    fn cancel(&mut self) {
        self.state = TextToolState::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::document::{Document, History};
    use crate::geometry::{Vec2, EPSILON};

    fn idle() -> TextTool {
        TextTool::default()
    }
    fn make() -> (TextTool, Document, History) {
        (idle(), Document::default(), History::default())
    }

    // AC#9, #10, #12, #13, #30 — name, idle, first-click anchor, object-safe
    #[test]
    fn text_tool_default_is_idle_preview_empty() {
        let t = idle();
        assert_eq!(t.name(), "TEXT");
        assert!(t.preview().is_empty());
        let _: Box<dyn Tool> = Box::new(idle()); // AC#30
                                                 // AC#13: first click anchors; buffer empty → preview still empty
        let (mut t2, mut d, mut h) = make();
        t2.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut d, &mut h);
        assert!(t2.preview().is_empty());
    }

    // AC#11 — status text both states
    #[test]
    fn status_text_both_states() {
        assert_eq!(idle().status_text(), "TEXT: Click to set insertion point");
        let (mut t, mut d, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut d, &mut h);
        assert_eq!(
            t.status_text(),
            "TEXT: Type text, Enter to confirm, Escape to cancel"
        );
    }

    // AC#14 — typing 'H' gives non-empty Entity::Line preview above baseline
    #[test]
    fn typing_h_produces_line_preview_above_baseline() {
        let (mut t, mut d, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut d, &mut h);
        t.on_text_input('H');
        let pv = t.preview();
        assert!(!pv.is_empty());
        for e in &pv {
            assert!(matches!(e, Entity::Line(_)));
            if let Entity::Line(l) = e {
                assert!(l.p1.y >= -EPSILON);
                assert!(l.p2.y >= -EPSILON);
            }
        }
    }

    // AC#15 — more chars → monotonically more preview lines
    #[test]
    fn more_chars_more_preview_lines() {
        let (mut t, mut d, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut d, &mut h);
        t.on_text_input('A');
        let n1 = t.preview().len();
        t.on_text_input('B');
        let n2 = t.preview().len();
        t.on_text_input('C');
        let n3 = t.preview().len();
        assert!(n2 >= n1 && n3 >= n2 && n3 > n1);
    }

    // AC#16, #17, #18 — backspace in all states
    #[test]
    fn backspace_all_states() {
        let mut app = App::default();
        // #16: removes char → empty preview
        let (mut t, mut d, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut d, &mut h);
        t.on_text_input('X');
        assert!(!t.preview().is_empty());
        t.on_key(egui::Key::Backspace, &mut app);
        assert!(t.preview().is_empty());
        // #17: empty buffer — no panic
        t.on_key(egui::Key::Backspace, &mut app);
        // #18: Idle — no panic
        idle().on_key(egui::Key::Backspace, &mut App::default());
    }

    // AC#19 a-d, AC#22 — Enter "H" commits; one history entry; undo clears; Idle
    #[test]
    fn enter_with_h_commits_and_returns_idle() {
        let mut app = App::default();
        let (mut t, mut d, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut d, &mut h);
        t.on_text_input('H');
        t.on_key(egui::Key::Enter, &mut app);
        assert!(app.document.entity_count() > 0);
        for e in &app.document.entities {
            assert!(matches!(e, Entity::Line(_)));
        }
        assert!(app.history.can_undo());
        assert!(t.preview().is_empty());
        // AC#22: exactly one history entry; undo restores empty doc
        assert_eq!(app.history.len(), 1);
        app.history.undo(&mut app.document);
        assert_eq!(app.document.entity_count(), 0);
    }

    // AC#20, #21 — Enter empty buffer: no commit; Enter in Idle: no panic
    #[test]
    fn enter_edge_cases_no_panic() {
        let mut app = App::default();
        let (mut t, mut d, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut d, &mut h);
        t.on_key(egui::Key::Enter, &mut app);
        assert_eq!(app.document.entity_count(), 0);
        assert!(t.preview().is_empty());
        idle().on_key(egui::Key::Enter, &mut App::default()); // #21
    }

    // AC#23, #24, #25, #26 — Escape discards; cancel resets; both idempotent
    #[test]
    fn escape_and_cancel() {
        let mut app = App::default();
        let (mut t, mut d, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut d, &mut h);
        t.on_text_input('H');
        t.on_key(egui::Key::Escape, &mut app);
        assert_eq!(app.document.entity_count(), 0);
        assert!(t.preview().is_empty());
        // #24: Idle escape — no panic
        idle().on_key(egui::Key::Escape, &mut App::default());
        // AC#25, #26: cancel() resets and is idempotent
        let (mut t2, mut d2, mut h2) = make();
        t2.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut d2, &mut h2);
        t2.on_text_input('H');
        t2.cancel();
        assert!(t2.preview().is_empty());
        t2.cancel(); // idempotent on Idle
    }

    // AC#27 — second click in WaitingInput is a no-op
    #[test]
    fn second_click_is_noop() {
        let (mut t, mut d, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut d, &mut h);
        t.on_text_input('A');
        t.on_pointer_down(Vec2::new(99.0, 99.0), false, &mut d, &mut h);
        for e in t.preview() {
            if let Entity::Line(l) = e {
                assert!(l.p1.x < 50.0 && l.p2.x < 50.0, "anchor must not have moved");
            }
        }
        assert_eq!(d.entity_count(), 0);
    }

    // AC#28, #29 — pointer move and up are no-ops
    #[test]
    fn pointer_move_and_up_noop() {
        let (mut t, mut d, mut h) = make();
        t.on_pointer_move(Vec2::new(1.0, 2.0), &mut d);
        assert!(t.preview().is_empty());
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut d, &mut h);
        t.on_pointer_move(Vec2::new(100.0, 200.0), &mut d);
        assert!(t.preview().is_empty()); // buffer still empty
        t.on_text_input('A');
        t.on_pointer_up(Vec2::new(10.0, 10.0), false, &mut d, &mut h);
        assert!(!t.preview().is_empty()); // buffer unchanged by up
    }
}
