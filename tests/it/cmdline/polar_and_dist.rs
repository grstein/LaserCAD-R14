//! LCV-159 — polar input and DIST driven from the command line through the
//! real frame.

use crate::harness;

use harness::{frame, submit_command};
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
        "LINE Specify first point:"
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
