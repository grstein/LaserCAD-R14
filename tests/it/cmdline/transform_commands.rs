//! LCV-158 — ROTATE driven from the command line and the canvas through the
//! real frame.

use crate::harness;

use harness::{frame, raw_input, submit_command, tap};
use lasercad::app::App;
use lasercad::document::{AddLayer, Command, Entity};
use lasercad::geometry::{Line, Transform, Vec2};

const EPS: f64 = 1e-9;

fn line_at(app: &App, i: usize) -> Line {
    match app.document.entities[i] {
        Entity::Line(l) => l,
        ref e => panic!("expected a Line at {i}, got {e:?}"),
    }
}

fn assert_near(actual: Vec2, expected: Vec2) {
    assert!(
        (actual - expected).length() <= EPS,
        "expected {expected:?}, got {actual:?}"
    );
}

/// AC1, AC4, AC7 — `ro` ⏎ `0,0` ⏎ `90` ⏎ turns a line on a non-current layer
/// a quarter turn CCW as one undo step, keeps its layer and the selection,
/// and hands back to SELECT; Ctrl+Z restores it bit-exact.
#[test]
fn ro_typed_ninety_degrees_rotates_ccw() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    frame(&ctx, &mut app, vec![]);
    AddLayer::new("Engrave", [0, 0, 255], true).do_(&mut app.document);
    let engrave = app.document.layer_by_name("Engrave").unwrap().id;
    let source = Line::new(Vec2::new(10.0, 0.0), Vec2::new(20.0, 5.0));
    app.document.push_entity(Entity::Line(source), engrave);
    app.document.selection.add(0);

    submit_command(&ctx, &mut app, "ro");
    assert_eq!(app.tool_manager.active_tool_name(), "ROTATE");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "ROTATE Specify base point:"
    );
    submit_command(&ctx, &mut app, "0,0");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "ROTATE Specify rotation angle:"
    );
    submit_command(&ctx, &mut app, "90");

    let l = line_at(&app, 0);
    assert_near(l.p1, Vec2::new(0.0, 10.0));
    assert_near(l.p2, Vec2::new(-5.0, 20.0));
    assert_eq!(app.document.entity_layer(0), Some(engrave));
    assert!(app.document.selection.is_selected(0));
    assert_eq!(app.history.len(), 1, "one undo step");
    assert_eq!(app.tool_manager.active_tool_name(), "Select");

    tap(&ctx, &mut app, egui::Key::Z, egui::Modifiers::COMMAND);
    assert_eq!(line_at(&app, 0), source, "undo is bit-exact");
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

/// AC5 — `rotate` ⏎ then two canvas clicks: the second click, straight
/// above the base on screen, rotates by the angle base → click (+90°).
#[test]
fn rotate_by_picked_angle() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    let mut viewport = egui::Rect::NOTHING;
    let _ = ctx.run(raw_input(vec![]), |c| {
        app.update_ui(c);
        viewport = c.available_rect();
    });
    app.snap_enabled = false;
    app.grid_enabled = false;
    let source = Line::new(Vec2::new(10.0, 0.0), Vec2::new(20.0, 5.0));
    app.document.push_current(Entity::Line(source));
    app.document.selection.add(0);

    submit_command(&ctx, &mut app, "rotate");
    click(&ctx, &mut app, viewport.center());
    let base = app.last_cursor_world.expect("the cursor is on the canvas");
    click(&ctx, &mut app, viewport.center() - egui::vec2(0.0, 80.0));
    let pick = app.last_cursor_world.expect("the cursor is on the canvas");

    let d = pick - base;
    assert!(d.x == 0.0 && d.y > 0.0, "{base:?} → {pick:?}");
    let t = Transform::Rotate {
        base,
        angle: d.y.atan2(d.x),
    };
    assert_eq!(app.document.entities[0], Entity::Line(t.line(source)));
    assert_eq!(app.history.len(), 1);
    assert_eq!(app.tool_manager.active_tool_name(), "Select");
}
