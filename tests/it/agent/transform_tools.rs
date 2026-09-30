//! LCV-158 AC9 — the agent's `rotate_entity` commits the same
//! `TransformEntities` as ROTATE, in degrees, as one undo step; LCV-181 AC10
//! — `mirror_entity` commits the same `TransformEntities` as MIRROR; LCV-182
//! AC8 — `scale_entity` commits the same `TransformEntities` as SCALE;
//! LCV-186 — the six edit tools also take a set of `indices`.

use core::f64::consts::FRAC_PI_2;

use lasercad::agent::{AgentAction, AgentOutcome, parse_tool_call, tool_definitions};
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

fn mirror_action(args: serde_json::Value) -> AgentAction {
    parse_tool_call("mirror_entity", &args).unwrap_or_else(|e| panic!("{e}"))
}

/// Across the Y axis, as `mirror_entity` is called below.
fn across_y() -> Transform {
    Transform::Mirror {
        a: Vec2::new(0.0, 0.0),
        b: Vec2::new(0.0, 5.0),
    }
}

/// AC10 — `erase_source: false` appends the same image MIRROR's keep-source
/// command appends, on the source's layer, keeps the source and the
/// selection, names the new index, and is one undo step.
#[test]
fn mirror_entity_keep_source_appends_the_image() {
    let (mut app, line) = app_with_line();
    let layer = app.document.entity_layer(0);
    let before = app.history.revision();

    let outcome = apply(
        &mut app,
        &mirror_action(json!({"index":0,"x1":0,"y1":0,"x2":0,"y2":5,"erase_source":false})),
    );

    assert!(matches!(outcome, AgentOutcome::Ok(_)), "{outcome:?}");
    assert!(
        outcome.text().starts_with("Mirrored entity 0 (line"),
        "{}",
        outcome.text()
    );
    assert!(
        outcome.text().contains("as entity 1."),
        "{}",
        outcome.text()
    );
    let mut expected = lasercad::document::Document::default();
    expected.push_current(Entity::Line(line));
    TransformEntities::new(vec![0], across_y())
        .with_keep_source(true)
        .do_(&mut expected);
    assert_eq!(app.document.entities, expected.entities);
    assert_eq!(app.document.entity_layer(0), layer);
    assert_eq!(app.document.entity_layer(1), layer);
    assert!(app.document.selection.is_selected(0));
    assert_eq!(app.history.revision(), before + 1);
    assert!(app.history.undo(&mut app.document));
    assert_eq!(app.document.entities, vec![Entity::Line(line)]);
}

/// AC10 — `erase_source: true` replaces the entity in place with its image.
#[test]
fn mirror_entity_erase_source_replaces_the_entity() {
    let (mut app, line) = app_with_line();
    let layer = app.document.entity_layer(0);

    let outcome = apply(
        &mut app,
        &mirror_action(json!({"index":0,"x1":0,"y1":0,"x2":0,"y2":5,"erase_source":true})),
    );

    assert!(matches!(outcome, AgentOutcome::Ok(_)), "{outcome:?}");
    assert_eq!(
        app.document.entities,
        vec![Entity::Line(across_y().line(line))]
    );
    assert_eq!(app.document.entity_layer(0), layer);
    assert!(app.history.undo(&mut app.document));
    assert_eq!(app.document.entities, vec![Entity::Line(line)]);
}

/// AC10 — out of range and coincident line points are refused and commit
/// nothing.
#[test]
fn mirror_entity_refuses_out_of_range_and_coincident_points() {
    let (mut app, line) = app_with_line();
    let before = app.history.revision();

    let far = apply(
        &mut app,
        &mirror_action(json!({"index":3,"x1":0,"y1":0,"x2":0,"y2":5,"erase_source":false})),
    );
    assert!(matches!(far, AgentOutcome::Refused(_)), "{far:?}");
    let point = apply(
        &mut app,
        &mirror_action(json!({"index":0,"x1":2,"y1":2,"x2":2,"y2":2,"erase_source":true})),
    );
    assert!(matches!(point, AgentOutcome::Refused(_)), "{point:?}");

    assert_eq!(app.history.revision(), before);
    assert_eq!(app.document.entities, vec![Entity::Line(line)]);
}

fn scale_action(args: serde_json::Value) -> AgentAction {
    parse_tool_call("scale_entity", &args).unwrap_or_else(|e| panic!("{e}"))
}

/// LCV-182 AC8 — `scale_entity` by 2 about the origin gives the same entity
/// as SCALE's `TransformEntities`, keeps the layer and the selection,
/// narrates the scale, and is one undo step.
#[test]
fn scale_entity_applies_the_scale_command() {
    let (mut app, line) = app_with_line();
    let layer = app.document.entity_layer(0);
    let before = app.history.revision();

    let outcome = apply(
        &mut app,
        &scale_action(json!({"index":0,"x":0.0,"y":0.0,"factor":2.0})),
    );

    assert!(matches!(outcome, AgentOutcome::Ok(_)), "{outcome:?}");
    assert!(
        outcome.text().starts_with("Scaled entity 0 (line"),
        "{}",
        outcome.text()
    );
    assert!(outcome.text().contains("by 2.000"), "{}", outcome.text());
    let t = Transform::Scale {
        base: Vec2::new(0.0, 0.0),
        factor: 2.0,
    };
    assert_eq!(app.document.entities, vec![Entity::Line(t.line(line))]);
    assert_eq!(app.document.entity_layer(0), layer);
    assert!(app.document.selection.is_selected(0));
    assert_eq!(app.history.revision(), before + 1);
    assert!(app.history.undo(&mut app.document));
    assert_eq!(app.document.entities, vec![Entity::Line(line)]);
}

/// LCV-182 AC8 — out of range is refused; AC6 — a factor of 1 commits
/// nothing.
#[test]
fn scale_entity_refuses_out_of_range_and_skips_factor_one() {
    let (mut app, line) = app_with_line();
    let before = app.history.revision();

    let far = apply(
        &mut app,
        &scale_action(json!({"index":3,"x":0,"y":0,"factor":2})),
    );
    assert!(matches!(far, AgentOutcome::Refused(_)), "{far:?}");
    let one = apply(
        &mut app,
        &scale_action(json!({"index":0,"x":5,"y":5,"factor":1})),
    );
    assert!(matches!(one, AgentOutcome::Ok(_)), "{one:?}");

    assert_eq!(app.history.revision(), before);
    assert_eq!(app.document.entities, vec![Entity::Line(line)]);
}

/// The six set-capable tools, each with valid arguments for its operation.
fn set_tools() -> [(&'static str, serde_json::Value); 6] {
    [
        ("delete_entity", json!({})),
        ("move_entity", json!({"dx":1,"dy":2})),
        ("copy_entity", json!({"dx":1,"dy":2})),
        ("rotate_entity", json!({"x":0,"y":0,"degrees":90})),
        (
            "mirror_entity",
            json!({"x1":0,"y1":0,"x2":0,"y2":5,"erase_source":false}),
        ),
        ("scale_entity", json!({"x":0,"y":0,"factor":2})),
    ]
}

/// `base` with the keys of `extra` added.
fn with(base: &serde_json::Value, extra: serde_json::Value) -> serde_json::Value {
    let mut out = base.clone();
    for (k, v) in extra.as_object().unwrap() {
        out[k] = v.clone();
    }
    out
}

/// LCV-186 AC5 — an empty, oversized, duplicated, negative, fractional or
/// non-numeric `indices` list, a list that is not a list, and `index` given
/// together with `indices` are each refused by every set tool, and the
/// refusal names the offending entry.
#[test]
fn a_bad_indices_list_is_refused_naming_the_entry() {
    let too_many: Vec<usize> = (0..1001).collect();
    let cases = [
        (json!({"indices": []}), vec!["`indices`", "empty"]),
        (
            json!({"indices": too_many}),
            vec!["`indices`", "1001", "1000"],
        ),
        (
            json!({"indices": [0, 4, 2, 4]}),
            vec!["indices[3]", "duplicate of indices[1]"],
        ),
        (
            json!({"indices": [0, -1]}),
            vec!["indices[1]", "-1", "non-negative integer"],
        ),
        (
            json!({"indices": [0, 2, 1.5]}),
            vec!["indices[2]", "1.5", "non-negative integer"],
        ),
        (
            json!({"indices": [0, "2"]}),
            vec!["indices[1]", "non-negative integer"],
        ),
        (json!({"indices": 3}), vec!["`indices`", "list"]),
        (
            json!({"index": 0, "indices": [1]}),
            vec!["`index`", "`indices`", "not both"],
        ),
    ];
    for (tool, args) in set_tools() {
        for (extra, needles) in &cases {
            let call = with(&args, extra.clone());
            let text = match parse_tool_call(tool, &call) {
                Err(e) => e.to_string(),
                Ok(a) => panic!("{tool} {extra}: accepted as {a:?}"),
            };
            assert!(text.contains(tool), "{text}");
            for needle in needles {
                assert!(
                    text.contains(needle),
                    "{tool} {extra}: `{needle}` missing: {text}"
                );
            }
        }
    }
}

/// LCV-186 AC1 — each of the six published schemas offers an optional
/// `indices` list of 1..=1000 integers beside `index`, and requires neither.
#[test]
fn the_six_schemas_offer_an_optional_indices_list() {
    let defs = tool_definitions(false);
    for (tool, _) in set_tools() {
        let def = defs
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["function"]["name"] == tool)
            .unwrap_or_else(|| panic!("{tool} is published"));
        let params = &def["function"]["parameters"];
        assert_eq!(
            params["properties"]["index"],
            json!({"type":"integer"}),
            "{tool}"
        );
        assert_eq!(
            params["properties"]["indices"],
            json!({"type":"array","items":{"type":"integer"},"minItems":1,"maxItems":1000}),
            "{tool}"
        );
        let required = params["required"].as_array().unwrap();
        assert!(!required.contains(&json!("index")), "{tool}: {required:?}");
        assert!(
            !required.contains(&json!("indices")),
            "{tool}: {required:?}"
        );
        let text = def["function"]["description"].as_str().unwrap();
        assert!(text.contains("indices"), "{tool}: {text}");
    }
}
