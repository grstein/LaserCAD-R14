//! LCV-165 AC 4, AC 5 — Enter on an empty line while Select is at rest
//! repeats the last CAD command word from the recall ring.
//!
//! The empty Enter is a real key tap through the frame: after a submit the
//! field has lost focus, so the tap reaches the keyboard gate, exactly as an
//! operator's second Enter does.

use crate::harness;

use harness::{frame, submit_command, tap};
use lasercad::app::App;
use lasercad::tools::SelectTool;

fn boot() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    let mut app = App::default();
    frame(&ctx, &mut app, vec![]);
    (ctx, app)
}

fn enter(ctx: &egui::Context, app: &mut App) {
    tap(ctx, app, egui::Key::Enter, egui::Modifiers::NONE);
}

fn prompt(app: &App) -> String {
    app.tool_manager.active_status_text().into_owned()
}

/// AC 4 — `l`, Esc, back at Select (as the rail does), empty ⏎ → LINE.
#[test]
fn ac4_empty_enter_at_select_repeats_line() {
    let (ctx, mut app) = boot();
    submit_command(&ctx, &mut app, "l");
    tap(&ctx, &mut app, egui::Key::Escape, egui::Modifiers::NONE);
    app.tool_manager.set_tool(Box::new(SelectTool::default()));
    assert_eq!(prompt(&app), "Command:");
    enter(&ctx, &mut app);
    assert_eq!(prompt(&app), "LINE  Specify first point:");
}

/// AC 4 — DIST hands back to Select on its own; the next empty ⏎ starts
/// DIST again, and the result line is cleared as a typed word clears it.
#[test]
fn ac4_empty_enter_after_dist_repeats_dist() {
    let (ctx, mut app) = boot();
    for line in ["dist", "0,0", "3,4"] {
        submit_command(&ctx, &mut app, line);
    }
    assert_eq!(prompt(&app), "Command:");
    enter(&ctx, &mut app);
    assert_eq!(prompt(&app), "DIST  Specify first point:");
    assert!(app.command_feedback.is_empty(), "{}", app.command_feedback);
}

/// AC 4 — the empty ⏎ through the focused field's submit path repeats too.
#[test]
fn ac4_submitted_empty_line_repeats() {
    let (_ctx, mut app) = boot();
    lasercad::app::submit(&mut app, "c");
    app.tool_manager.set_tool(Box::new(SelectTool::default()));
    lasercad::app::submit(&mut app, "");
    assert_eq!(prompt(&app), "CIRCLE  Specify center point:");
}

/// AC 4 — mid-command, the empty ⏎ finishes LINE and does not repeat.
#[test]
fn ac4_mid_command_enter_finishes_without_repeat() {
    let (ctx, mut app) = boot();
    for line in ["l", "0,0", "10,0"] {
        submit_command(&ctx, &mut app, line);
    }
    assert_eq!(prompt(&app), "LINE  Specify next point <Enter to finish>:");
    enter(&ctx, &mut app);
    assert_eq!(prompt(&app), "LINE  Specify first point:");
    assert_eq!(app.document.entity_count(), 1);
    assert_eq!(app.tool_manager.anchor(), None, "finished, not restarted");
}

/// AC 4 — a repeat pushes nothing: the ring and Up recall are unchanged.
#[test]
fn ac4_repeat_leaves_the_ring_unchanged() {
    let (ctx, mut app) = boot();
    submit_command(&ctx, &mut app, "c");
    app.tool_manager.set_tool(Box::new(SelectTool::default()));
    let before = app.command_history.len();
    enter(&ctx, &mut app);
    assert_eq!(prompt(&app), "CIRCLE  Specify center point:");
    assert_eq!(app.command_history.len(), before);
    assert_eq!(app.command_history.older().as_deref(), Some("c"));
}

/// AC 5 — an agent prompt is skipped: `c`, `:x` (AI unavailable), empty ⏎
/// → CIRCLE, and no agent turn starts.
#[test]
fn ac5_agent_prompt_is_skipped() {
    let (ctx, mut app) = boot();
    submit_command(&ctx, &mut app, "c");
    app.tool_manager.set_tool(Box::new(SelectTool::default()));
    submit_command(&ctx, &mut app, ":x");
    enter(&ctx, &mut app);
    assert_eq!(prompt(&app), "CIRCLE  Specify center point:");
    assert!(!app.agent.busy);
}

/// AC 5 — with no tool word in the ring the empty ⏎ does nothing.
#[test]
fn ac5_no_tool_word_does_nothing() {
    let (ctx, mut app) = boot();
    submit_command(&ctx, &mut app, ":draw");
    submit_command(&ctx, &mut app, "grid");
    let grid = app.grid_enabled;
    enter(&ctx, &mut app);
    assert_eq!(prompt(&app), "Command:");
    assert_eq!(app.grid_enabled, grid, "a toggle is not repeated");
    assert_eq!(app.document.entity_count(), 0);
}
