//! LCV-102 — autosave dirty tracking regression (ADR 0002 §B).
//!
//! The defect this demand fixes: `dirty_since` was written in exactly one
//! place, `App::commit`, but every tool except `TextTool` calls
//! `history.commit` directly with no `&mut App` (LCV-041). A unit test that
//! only exercises `History` in isolation would not catch that — it would
//! prove the revision counter works without proving `App::sync_dirty` is
//! actually reached by a real, tool-driven edit. This test drives the real
//! frame body through `egui::Context::run` (`tests/harness/mod.rs`,
//! landed by LCV-103) with `LineTool` — a tool that never calls
//! `App::commit` — and checks `app.dirty_since` after each frame.

use crate::harness;

use std::time::{Duration, Instant};

use harness::frame;
use lasercad::app::{App, Severity};
use lasercad::document::Entity;
use lasercad::geometry::{Line, Vec2};
use lasercad::tools::LineTool;

/// A complete primary-button click at `pos`: move, press, release, all in
/// one frame. Mirrors `tests/it/app/keyboard_routing.rs`'s local helper — the shared harness
/// deliberately exposes no pointer helpers (ADR 0002 §A3).
fn click_events(pos: egui::Pos2) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

/// Boot an app with `LineTool` active and run the warm-up frame that
/// registers the `CentralPanel` for hit-testing (ADR 0002 §A4 rule 3),
/// returning the context, the app, and the viewport rect the frame laid out.
fn boot_with_line_tool() -> (egui::Context, App, egui::Rect) {
    let ctx = egui::Context::default();
    let mut app = App::default();
    app.tool_manager.set_tool(Box::new(LineTool::default()));
    let mut viewport = egui::Rect::NOTHING;
    let _ = ctx.run(harness::raw_input(vec![]), |c| {
        app.update_ui(c);
        viewport = c.available_rect();
    });
    (ctx, app, viewport)
}

/// Click once, with the mandatory warm-up `PointerMoved` frame ahead of the
/// frame carrying the button (ADR 0002 §A4 rule 3).
fn click(ctx: &egui::Context, app: &mut App, pos: egui::Pos2) {
    frame(ctx, app, vec![egui::Event::PointerMoved(pos)]); // warm-up
    frame(ctx, app, click_events(pos));
}

/// AC 9 (end-to-end) — a `LineTool`-driven edit, which commits via
/// `history.commit` and never touches `App::commit`, must dirty the document
/// by the end of the same frame. This is the regression `history`-only unit
/// tests cannot express: it proves `sync_dirty()` is actually wired into the
/// real frame body, not just correct in isolation.
#[test]
fn tool_driven_edit_marks_document_dirty() {
    let (ctx, mut app, viewport) = boot_with_line_tool();
    assert!(
        app.dirty_since.is_none(),
        "a freshly booted app must start clean"
    );

    // First click only sets LineTool's anchor point (p1) — no command is
    // committed yet, so the document must still read clean.
    let p1 = viewport.center();
    click(&ctx, &mut app, p1);
    assert!(!app.history.can_undo(), "first click must not commit yet");
    assert!(
        app.dirty_since.is_none(),
        "no commit happened yet, so the document must still be clean"
    );

    // Second click, far enough from p1 to not be discarded as degenerate,
    // commits `CreateLine` via `history.commit` directly (LineTool holds no
    // `&mut App` — LCV-041). `App::commit` is never called on this path.
    let p2 = egui::pos2(p1.x + 120.0, p1.y + 80.0);
    click(&ctx, &mut app, p2);

    assert!(
        app.history.can_undo(),
        "the second click must have committed CreateLine through History"
    );
    assert_eq!(app.document.entity_count(), 1);
    assert!(
        app.dirty_since.is_some(),
        "a tool-driven history.commit must dirty the document by the end of \
         the same frame (the defect this demand fixes: before the fix, \
         dirty_since stayed None forever for every tool but TextTool)"
    );
}

/// AC 10 (end-to-end) — undo through the real frame body re-dirties a
/// just-saved document.
#[test]
fn undo_via_frame_body_marks_document_dirty() {
    let (ctx, mut app, viewport) = boot_with_line_tool();
    let p1 = viewport.center();
    click(&ctx, &mut app, p1);
    let p2 = egui::pos2(p1.x + 120.0, p1.y + 80.0);
    click(&ctx, &mut app, p2);
    assert!(app.dirty_since.is_some());

    // Simulate a successful autosave / save having just run.
    app.mark_clean();
    assert!(app.dirty_since.is_none());

    assert!(app.history.undo(&mut app.document));
    // One more frame so `sync_dirty()` observes the bumped revision.
    frame(&ctx, &mut app, vec![]);

    assert!(
        app.dirty_since.is_some(),
        "undo must re-dirty the document on the next frame"
    );
}

/// LCV-168 AC 6 — an autosave flush, through the real frame body, with an
/// entity outside the bed shows neither the `Saved` line nor the out-of-bed
/// warning: the dock keeps its previous message.
#[test]
fn autosave_flush_announces_nothing() {
    let dir = std::env::temp_dir().join("lcv168_autosave_silent");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let autosave = dir.join("autosave.json");
    let ctx = egui::Context::default();
    let mut app = App {
        autosave_path: Some(autosave.clone()),
        ..App::default()
    };
    frame(&ctx, &mut app, vec![]); // warm-up
    let cut = app.document.current_layer();
    let off_bed = Line::new(Vec2::new(-5.0, 10.0), Vec2::new(50.0, 10.0));
    app.document.push_entity(Entity::Line(off_bed), cut);
    app.dirty_since = Instant::now().checked_sub(Duration::from_secs(5));
    app.say(Severity::Error, "sentinel");

    frame(&ctx, &mut app, vec![]);

    assert!(
        autosave.is_file(),
        "positive control: the autosave was written"
    );
    assert!(app.last_autosave_at.is_some());
    assert_eq!(app.command_feedback, "sentinel");
    assert_eq!(app.command_feedback_severity, Severity::Error);
}
