//! LCV-159 — polar input and DIST driven from the command line through the
//! real frame.

use crate::harness;

use harness::{frame, raw_input, submit_command};
use lasercad::app::App;
use lasercad::document::Entity;
use lasercad::geometry::{Line, Vec2};

const EPS: f64 = 1e-9;

fn boot() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    let mut app = App::default();
    frame(&ctx, &mut app, vec![]);
    (ctx, app)
}

fn lines(app: &App) -> Vec<Line> {
    app.document
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Line(l) => Some(*l),
            _ => None,
        })
        .collect()
}

fn assert_near(actual: Vec2, expected: Vec2) {
    assert!(
        (actual - expected).length() <= EPS,
        "expected {expected:?}, got {actual:?}"
    );
}

/// AC1 — `l` ⏎ `0,0` ⏎ `@50<30` ⏎ draws a 50 mm segment at 30° from the anchor.
#[test]
fn relative_polar_draws_the_expected_line() {
    let (ctx, mut app) = boot();
    submit_command(&ctx, &mut app, "l");
    submit_command(&ctx, &mut app, "0,0");
    submit_command(&ctx, &mut app, "@50<30");

    let drawn = lines(&app);
    assert_eq!(drawn.len(), 1, "got {drawn:?}");
    let a = 30f64.to_radians();
    assert_near(drawn[0].p1, Vec2::new(0.0, 0.0));
    assert_near(drawn[0].p2, Vec2::new(50.0 * a.cos(), 50.0 * a.sin()));
    assert!(app.command_feedback.is_empty(), "{}", app.command_feedback);
}

/// AC4 — `@d<a` with no anchor is refused with the `@dx,dy` message and
/// changes nothing.
#[test]
fn relative_polar_without_anchor_is_refused() {
    let (ctx, mut app) = boot();
    submit_command(&ctx, &mut app, "l");
    submit_command(&ctx, &mut app, "@10<45");

    assert_eq!(app.command_feedback, "No base point for relative input.");
    assert_eq!(app.document.entity_count(), 0);
    assert_eq!(app.history.len(), 0);
    assert_eq!(
        app.tool_manager.active_status_text(),
        "LINE  Specify first point:"
    );
}

/// AC2 — `100<0` is the absolute point (100, 0), whatever the anchor is.
#[test]
fn absolute_polar_is_measured_from_the_origin() {
    let (ctx, mut app) = boot();
    submit_command(&ctx, &mut app, "l");
    submit_command(&ctx, &mut app, "20,20");
    submit_command(&ctx, &mut app, "100<0");

    let drawn = lines(&app);
    assert_eq!(drawn.len(), 1, "got {drawn:?}");
    assert_near(drawn[0].p1, Vec2::new(20.0, 20.0));
    assert_near(drawn[0].p2, Vec2::new(100.0, 0.0));
}

/// AC6, AC7 — `di` ⏎ `0,0` ⏎ `@30,40` ⏎ prompts twice, prints the report,
/// returns to SELECT and leaves the drawing and the undo history alone.
#[test]
fn typed_dist_reports_and_returns_to_select() {
    let (ctx, mut app) = boot();
    submit_command(&ctx, &mut app, "l");
    submit_command(&ctx, &mut app, "0,0");
    submit_command(&ctx, &mut app, "5,0");
    let (entities, depth) = (app.document.entity_count(), app.history.len());

    submit_command(&ctx, &mut app, "di");
    assert_eq!(app.tool_manager.active_tool_name(), "DIST");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "DIST  Specify first point:"
    );
    submit_command(&ctx, &mut app, "0,0");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "DIST  Specify second point:"
    );
    submit_command(&ctx, &mut app, "@30,40");

    assert_eq!(
        app.command_feedback,
        "Distance = 50.000, Angle = 53.130°, Delta X = 30.000, Delta Y = 40.000"
    );
    assert_eq!(app.tool_manager.active_tool_name(), "Select");
    assert_eq!(app.document.entity_count(), entities);
    assert_eq!(app.history.len(), depth, "the undo depth is unchanged");
}

/// A complete primary-button click at `pos`, after a warm-up move frame
/// (ADR 0003 §F3 trap 3).
fn click(ctx: &egui::Context, app: &mut App, pos: egui::Pos2) {
    frame(ctx, app, vec![egui::Event::PointerMoved(pos)]);
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(ctx, app, vec![button(true), button(false)]);
}

/// AC6, AC7 — `dist` ⏎ then two clicks on the canvas: the pointer path
/// reports the same way and hands back to SELECT.
#[test]
fn clicked_dist_reports_and_returns_to_select() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    let mut viewport = egui::Rect::NOTHING;
    let _ = ctx.run(raw_input(vec![]), |c| {
        app.update_ui(c);
        viewport = c.available_rect();
    });
    app.snap_enabled = false;
    app.grid_enabled = false;

    submit_command(&ctx, &mut app, "dist");
    click(&ctx, &mut app, viewport.center());
    let first = app.last_cursor_world.expect("the cursor is on the canvas");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "DIST  Specify second point:"
    );
    click(&ctx, &mut app, viewport.center() + egui::vec2(80.0, 0.0));
    let second = app.last_cursor_world.expect("the cursor is on the canvas");

    let dx = second.x - first.x;
    assert!(dx > 0.0 && second.y == first.y, "{first:?} → {second:?}");
    assert_eq!(
        app.command_feedback,
        format!("Distance = {dx:.3}, Angle = 0.000°, Delta X = {dx:.3}, Delta Y = 0.000")
    );
    assert_eq!(app.tool_manager.active_tool_name(), "Select");
    assert_eq!(app.document.entity_count(), 0);
    assert_eq!(app.history.len(), 0);
}
