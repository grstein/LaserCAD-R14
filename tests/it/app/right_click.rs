//! LCV-165 AC 6 — a right press on the canvas is Enter on an empty line:
//! it finishes, accepts or repeats, and never picks or places a point.
//! Middle-drag pan is unchanged (reverses LCV-041 AC 4).

use crate::harness;

use harness::{frame, raw_input, submit_command};
use lasercad::app::App;
use lasercad::document::{CreateLine, Entity};
use lasercad::geometry::{Line, Vec2};
use lasercad::tools::SelectTool;

/// A booted app with snap and grid off, and the canvas rect.
fn boot() -> (egui::Context, App, egui::Rect) {
    let ctx = egui::Context::default();
    let mut app = App::default();
    let mut canvas = egui::Rect::NOTHING;
    let _ = ctx.run(raw_input(vec![]), |c| {
        app.update_ui(c);
        canvas = c.available_rect();
    });
    app.snap_enabled = false;
    app.grid_enabled = false;
    (ctx, app, canvas)
}

/// One full press-and-release of `button` at `pos`, after a hover frame.
fn press(ctx: &egui::Context, app: &mut App, pos: egui::Pos2, button: egui::PointerButton) {
    frame(ctx, app, vec![egui::Event::PointerMoved(pos)]);
    let event = |pressed| egui::Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(ctx, app, vec![event(true)]);
    frame(ctx, app, vec![event(false)]);
}

fn right_click(ctx: &egui::Context, app: &mut App, pos: egui::Pos2) {
    press(ctx, app, pos, egui::PointerButton::Secondary);
}

fn prompt(app: &App) -> String {
    app.tool_manager.active_status_text().into_owned()
}

/// AC 6 — LINE waiting for its next point: the right press finishes it
/// (back to the first-point prompt) and adds no entity.
#[test]
fn ac6_right_click_finishes_line_without_a_point() {
    let (ctx, mut app, canvas) = boot();
    submit_command(&ctx, &mut app, "l");
    submit_command(&ctx, &mut app, "0,0");
    assert_eq!(prompt(&app), "LINE  Specify next point <Enter to finish>:");
    right_click(&ctx, &mut app, canvas.center());
    assert_eq!(prompt(&app), "LINE  Specify first point:");
    assert_eq!(app.document.entity_count(), 0, "no point was placed");
    assert_eq!(app.history.len(), 0);
}

/// AC 6 — at `Command:` the right press repeats the last command word.
#[test]
fn ac6_right_click_at_command_repeats_circle() {
    let (ctx, mut app, canvas) = boot();
    submit_command(&ctx, &mut app, "c");
    tap_escape(&ctx, &mut app);
    app.tool_manager.set_tool(Box::new(SelectTool::default()));
    assert_eq!(prompt(&app), "Command:");
    right_click(&ctx, &mut app, canvas.center());
    assert_eq!(prompt(&app), "CIRCLE  Specify center point:");
}

fn tap_escape(ctx: &egui::Context, app: &mut App) {
    harness::tap(ctx, app, egui::Key::Escape, egui::Modifiers::NONE);
}

/// AC 6 — a right press on an entity does not pick it; a left press on the
/// same spot does (positive control).
#[test]
fn ac6_right_click_does_not_pick() {
    let (ctx, mut app, canvas) = boot();
    let at = app
        .camera
        .screen_to_world(canvas.center() - canvas.min.to_vec2());
    let line = Line::new(at - Vec2::new(20.0, 0.0), at + Vec2::new(20.0, 0.0));
    app.commit(Box::new(CreateLine::new(line)));
    assert!(matches!(app.document.entities[0], Entity::Line(_)));

    right_click(&ctx, &mut app, canvas.center());
    assert!(app.document.selection.is_empty(), "the right press picked");
    assert_eq!(prompt(&app), "Command:");

    press(
        &ctx,
        &mut app,
        canvas.center(),
        egui::PointerButton::Primary,
    );
    assert!(
        !app.document.selection.is_empty(),
        "control: a left press picks"
    );
}

/// AC 6 — middle-drag still pans the camera.
#[test]
fn ac6_middle_drag_still_pans() {
    let (ctx, mut app, canvas) = boot();
    let before = app.camera.center_world;
    let from = canvas.center();
    let to = from + egui::vec2(60.0, 0.0);
    let event = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Middle,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(&ctx, &mut app, vec![egui::Event::PointerMoved(from)]);
    frame(&ctx, &mut app, vec![event(from, true)]);
    for step in 1..=6 {
        let pos = from + egui::vec2(10.0 * step as f32, 0.0);
        frame(&ctx, &mut app, vec![egui::Event::PointerMoved(pos)]);
    }
    frame(&ctx, &mut app, vec![event(to, false)]);
    assert_ne!(app.camera.center_world, before, "middle-drag pans");
    assert_eq!(app.document.entity_count(), 0);
}
