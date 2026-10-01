//! LCV-158 — ROTATE, LCV-181 — MIRROR and LCV-182 — SCALE driven from the
//! command line and the canvas through the real frame.

use crate::harness;

use harness::{frame, raw_input, submit_command, tap};
use lasercad::app::{App, Severity};
use lasercad::document::{AddLayer, Command, Entity};
use lasercad::geometry::{Line, Transform, Vec2};

const EPS: f64 = 1e-9;

/// SCALE's refusal of a non-positive factor (LCV-165 AC7).
const SCALE_REFUSAL: &str = "Scale factor must be greater than 0.";

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
        "ROTATE  Specify base point:"
    );
    submit_command(&ctx, &mut app, "0,0");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "ROTATE  Specify rotation angle:"
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
    let _ = ctx.run_ui(raw_input(vec![]), |ui| {
        let c = &ui.ctx().clone();
        app.update_ui(ui);
        viewport = crate::harness::canvas_rect(c);
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

/// A line on the non-current `Engrave` layer, selected, mirrored across the
/// Y axis by `word` ⏎ `0,0` ⏎ `0,10` ⏎; returns the app at the Yes/No prompt.
fn mirror_to_confirm(ctx: &egui::Context, word: &str) -> (App, Line, Transform) {
    let mut app = App::default();
    frame(ctx, &mut app, vec![]);
    AddLayer::new("Engrave", [0, 0, 255], true).do_(&mut app.document);
    let engrave = app.document.layer_by_name("Engrave").unwrap().id;
    let source = Line::new(Vec2::new(10.0, 0.0), Vec2::new(20.0, 5.0));
    app.document.push_entity(Entity::Line(source), engrave);
    app.document.selection.add(0);

    submit_command(ctx, &mut app, word);
    assert_eq!(app.tool_manager.active_tool_name(), "MIRROR");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "MIRROR  Specify first point of mirror line:"
    );
    submit_command(ctx, &mut app, "0,0");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "MIRROR  Specify second point of mirror line:"
    );
    submit_command(ctx, &mut app, "0,10");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "MIRROR  Erase source objects? [Yes/No] <N>:"
    );
    let t = Transform::Mirror {
        a: Vec2::new(0.0, 0.0),
        b: Vec2::new(0.0, 10.0),
    };
    (app, source, t)
}

/// AC5, AC6, AC9 — `mi` ⏎ two points ⏎ blank ⏎ (and `n` ⏎) adds the mirrored
/// line on the source's layer, keeps the source and the selection, as one
/// undo step, and hands back to SELECT; Ctrl+Z removes the copy.
#[test]
fn mi_blank_or_n_keeps_the_source() {
    for answer in ["", "n"] {
        let ctx = egui::Context::default();
        let (mut app, source, t) = mirror_to_confirm(&ctx, "mi");
        let engrave = app.document.entity_layer(0);
        submit_command(&ctx, &mut app, answer);

        assert_eq!(app.document.entity_count(), 2, "{answer:?}");
        assert_eq!(line_at(&app, 0), source);
        let m = line_at(&app, 1);
        assert_near(m.p1, Vec2::new(-10.0, 0.0));
        assert_near(m.p2, Vec2::new(-20.0, 5.0));
        assert_eq!(app.document.entities[1], Entity::Line(t.line(source)));
        assert_eq!(app.document.entity_layer(1), engrave);
        assert!(app.document.selection.is_selected(0));
        assert!(!app.document.selection.is_selected(1));
        assert_eq!(app.history.len(), 1, "one undo step");
        assert_eq!(app.tool_manager.active_tool_name(), "Select");

        tap(&ctx, &mut app, egui::Key::Z, egui::Modifiers::COMMAND);
        assert_eq!(app.document.entity_count(), 1);
        assert_eq!(line_at(&app, 0), source);
    }
}

/// AC7, AC9 — `mirror` ⏎ two points ⏎ `y` ⏎ replaces the source in place, on
/// its layer, as one undo step; Ctrl+Z restores it bit-exact.
#[test]
fn mirror_yes_replaces_the_source() {
    let ctx = egui::Context::default();
    let (mut app, source, t) = mirror_to_confirm(&ctx, "mirror");
    let engrave = app.document.entity_layer(0);
    submit_command(&ctx, &mut app, "y");

    assert_eq!(app.document.entity_count(), 1);
    assert_eq!(app.document.entities[0], Entity::Line(t.line(source)));
    assert_eq!(app.document.entity_layer(0), engrave);
    assert!(app.document.selection.is_selected(0));
    assert_eq!(app.history.len(), 1, "one undo step");
    assert_eq!(app.tool_manager.active_tool_name(), "Select");

    tap(&ctx, &mut app, egui::Key::Z, egui::Modifiers::COMMAND);
    assert_eq!(line_at(&app, 0), source, "undo is bit-exact");
}

/// Escape at the Yes/No prompt commits nothing; the tool stays MIRROR, back
/// at the first-point prompt.
#[test]
fn escape_at_the_yes_no_prompt_cancels() {
    let ctx = egui::Context::default();
    let (mut app, source, _) = mirror_to_confirm(&ctx, "mi");
    tap(&ctx, &mut app, egui::Key::Escape, egui::Modifiers::NONE);

    assert_eq!(app.document.entity_count(), 1);
    assert_eq!(line_at(&app, 0), source);
    assert_eq!(app.history.len(), 0);
    assert_eq!(app.tool_manager.active_tool_name(), "MIRROR");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "MIRROR  Specify first point of mirror line:"
    );
}

/// LCV-182 AC1, AC4, AC7 — `sc` ⏎ `0,0` ⏎ `2` ⏎ doubles a line on a
/// non-current layer about the origin as one undo step, keeps its layer and
/// the selection, and hands back to SELECT; Ctrl+Z restores it bit-exact.
#[test]
fn sc_typed_factor_two_doubles() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    frame(&ctx, &mut app, vec![]);
    AddLayer::new("Engrave", [0, 0, 255], true).do_(&mut app.document);
    let engrave = app.document.layer_by_name("Engrave").unwrap().id;
    let source = Line::new(Vec2::new(10.0, 0.0), Vec2::new(20.0, 5.0));
    app.document.push_entity(Entity::Line(source), engrave);
    app.document.selection.add(0);

    submit_command(&ctx, &mut app, "sc");
    assert_eq!(app.tool_manager.active_tool_name(), "SCALE");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "SCALE  Specify base point:"
    );
    submit_command(&ctx, &mut app, "0,0");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "SCALE  Specify scale factor:"
    );
    submit_command(&ctx, &mut app, "2");

    let l = line_at(&app, 0);
    assert_near(l.p1, Vec2::new(20.0, 0.0));
    assert_near(l.p2, Vec2::new(40.0, 10.0));
    assert_eq!(app.document.entity_layer(0), Some(engrave));
    assert!(app.document.selection.is_selected(0));
    assert_eq!(app.history.len(), 1, "one undo step");
    assert_eq!(app.tool_manager.active_tool_name(), "Select");

    tap(&ctx, &mut app, egui::Key::Z, egui::Modifiers::COMMAND);
    assert_eq!(line_at(&app, 0), source, "undo is bit-exact");
}

/// LCV-182 AC5 — with the cursor on the canvas, `scale` ⏎ `0,0` ⏎ then `-1`
/// or `0` ⏎ shows the refusal line, changes nothing, and keeps prompting for
/// the factor.
#[test]
fn scale_refuses_a_non_positive_factor() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    let mut viewport = egui::Rect::NOTHING;
    let _ = ctx.run_ui(raw_input(vec![]), |ui| {
        let c = &ui.ctx().clone();
        app.update_ui(ui);
        viewport = crate::harness::canvas_rect(c);
    });
    let source = Line::new(Vec2::new(10.0, 0.0), Vec2::new(20.0, 5.0));
    app.document.push_current(Entity::Line(source));
    app.document.selection.add(0);
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(viewport.center())],
    );

    submit_command(&ctx, &mut app, "scale");
    submit_command(&ctx, &mut app, "0,0");
    for factor in ["-1", "0"] {
        app.command_feedback.clear();
        submit_command(&ctx, &mut app, factor);
        assert_eq!(app.command_feedback, SCALE_REFUSAL, "{factor:?}");
        assert_eq!(
            app.tool_manager.active_status_text(),
            "SCALE  Specify scale factor:"
        );
    }
    assert_eq!(line_at(&app, 0), source);
    assert_eq!(app.history.len(), 0);
}

/// LCV-165 AC7 — with the pointer never on the canvas, `-1` ⏎ and `0` ⏎ at
/// the factor prompt show SCALE's own refusal as a Warning, never the
/// no-direction line, and SCALE keeps prompting.
#[test]
fn scale_refusal_without_a_cursor_is_the_tools_own() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    let source = Line::new(Vec2::new(10.0, 0.0), Vec2::new(20.0, 5.0));
    app.document.push_current(Entity::Line(source));
    app.document.selection.add(0);

    submit_command(&ctx, &mut app, "sc");
    submit_command(&ctx, &mut app, "0,0");
    for factor in ["-1", "0"] {
        app.say(Severity::Info, "");
        submit_command(&ctx, &mut app, factor);
        assert_eq!(app.command_feedback, SCALE_REFUSAL, "{factor:?}");
        assert_eq!(app.command_feedback_severity, Severity::Warning);
        assert!(!app.command_feedback.starts_with("No direction"));
        assert_eq!(
            app.tool_manager.active_status_text(),
            "SCALE  Specify scale factor:"
        );
    }
    assert_eq!(line_at(&app, 0), source);
    assert_eq!(app.history.len(), 0);
}
