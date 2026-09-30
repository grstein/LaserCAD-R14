//! LCV-158 AC9 — the agent's `rotate_entity` commits the same
//! `TransformEntities` as ROTATE, in degrees, as one undo step.

use core::f64::consts::FRAC_PI_2;

use lasercad::agent::{AgentAction, AgentOutcome, parse_tool_call};
use lasercad::app::{App, apply};
use lasercad::document::{AddLayer, Command, Entity, TransformEntities};
use lasercad::geometry::{Line, Transform, Vec2};
use serde_json::json;

fn action(args: serde_json::Value) -> AgentAction {
    parse_tool_call("rotate_entity", &args).unwrap_or_else(|e| panic!("{e}"))
}

/// A line (10,0)→(20,0) on a non-current `Engrave` layer, selected.
fn app_with_line() -> (App, Line) {
    let mut app = App::default();
    AddLayer::new("Engrave", [0, 0, 255], true).do_(&mut app.document);
    let engrave = app.document.layer_by_name("Engrave").unwrap().id;
    let line = Line::new(Vec2::new(10.0, 0.0), Vec2::new(20.0, 0.0));
    app.document.push_entity(Entity::Line(line), engrave);
    app.document.selection.add(0);
    (app, line)
}

/// AC9 — `rotate_entity` by 90 degrees about the origin gives the same
/// entity as `TransformEntities` in radians, keeps the layer and the
/// selection, narrates the turn, and is one undo step.
#[test]
fn rotate_entity_applies_the_rotate_command() {
    let (mut app, line) = app_with_line();
    let layer = app.document.entity_layer(0);
    let before = app.history.revision();

    let outcome = apply(
        &mut app,
        &action(json!({"index":0,"x":0.0,"y":0.0,"degrees":90.0})),
    );

    assert!(matches!(outcome, AgentOutcome::Ok(_)), "{outcome:?}");
    assert!(
        outcome.text().starts_with("Rotated entity 0 (line"),
        "{}",
        outcome.text()
    );
    assert!(outcome.text().contains("by 90.000"), "{}", outcome.text());
    let mut expected = lasercad::document::Document::default();
    expected.push_current(Entity::Line(line));
    TransformEntities::new(
        vec![0],
        Transform::Rotate {
            base: Vec2::new(0.0, 0.0),
            angle: FRAC_PI_2,
        },
    )
    .do_(&mut expected);
    assert_eq!(app.document.entities, expected.entities);
    assert_eq!(app.document.entity_layer(0), layer);
    assert!(app.document.selection.is_selected(0));
    assert_eq!(app.history.revision(), before + 1);
    assert!(app.history.undo(&mut app.document));
    assert_eq!(app.document.entities, vec![Entity::Line(line)]);
}

/// AC9 — out of range is refused; AC8 — a whole turn commits nothing.
#[test]
fn rotate_entity_refuses_out_of_range_and_skips_whole_turns() {
    let (mut app, line) = app_with_line();
    let before = app.history.revision();

    let far = apply(
        &mut app,
        &action(json!({"index":3,"x":0,"y":0,"degrees":45})),
    );
    assert!(matches!(far, AgentOutcome::Refused(_)), "{far:?}");
    let whole = apply(
        &mut app,
        &action(json!({"index":0,"x":5,"y":5,"degrees":-360})),
    );
    assert!(matches!(whole, AgentOutcome::Ok(_)), "{whole:?}");

    assert_eq!(app.history.revision(), before);
    assert_eq!(app.document.entities, vec![Entity::Line(line)]);
}
