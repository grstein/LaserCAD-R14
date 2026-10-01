//! LCV-158 AC9 — the agent's `rotate_entity` commits the same
//! `TransformEntities` as ROTATE, in degrees, as one undo step; LCV-181 AC10
//! — `mirror_entity` commits the same `TransformEntities` as MIRROR; LCV-182
//! AC8 — `scale_entity` commits the same `TransformEntities` as SCALE;
//! LCV-186 — the six edit tools also take a set of `indices`.

use core::f64::consts::FRAC_PI_2;

use lasercad::agent::{AgentAction, AgentOutcome, parse_tool_call, tool_definitions};
use lasercad::app::{App, apply};
use lasercad::document::{
    AddLayer, Command, CopyEntities, DeleteEntities, Entity, MoveEntities, TransformEntities,
};
use lasercad::geometry::{Arc, Circle, Line, Transform, Vec2};
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
/// refusal names the offending entry. LCV-192 AC 1 — pinned exactly in the
/// `<tool> <path>: <reason>; expected <form>` shape.
#[test]
fn a_bad_indices_list_is_refused_naming_the_entry() {
    let too_many: Vec<usize> = (0..1001).collect();
    let list = "expected a list of 1 to 1000 distinct entity indices";
    let index = "expected a non-negative integer (an index from query_entities)";
    let cases = [
        (
            json!({"indices": []}),
            format!("indices: empty list; {list}"),
        ),
        (
            json!({"indices": too_many}),
            format!("indices: has 1001 entries; {list}"),
        ),
        (
            json!({"indices": [0, 4, 2, 4]}),
            "indices[3]: duplicate of indices[1]; expected distinct indices".to_owned(),
        ),
        (
            json!({"indices": [0, -1]}),
            format!("indices[1]: -1 is not an index; {index}"),
        ),
        (
            json!({"indices": [0, 2, 1.5]}),
            format!("indices[2]: 1.5 is not an index; {index}"),
        ),
        (
            json!({"indices": [0, "2"]}),
            format!("indices[1]: not a number; {index}"),
        ),
        (
            json!({"indices": 3}),
            format!("indices: not a list; {list}"),
        ),
        (
            json!({"index": 0, "indices": [1]}),
            "index: given together with indices; expected either index or indices, not both"
                .to_owned(),
        ),
    ];
    for (tool, args) in set_tools() {
        for (extra, want) in &cases {
            let call = with(&args, extra.clone());
            let text = match parse_tool_call(tool, &call) {
                Err(e) => e.to_string(),
                Ok(a) => panic!("{tool} {extra}: accepted as {a:?}"),
            };
            assert_eq!(text, format!("{tool} {want}"), "{tool} {extra}");
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

/// Line 0 and arc 2 on the current layer, circle 1 and line 3 on `Engrave`.
fn set_app() -> App {
    let mut app = App::default();
    AddLayer::new("Engrave", [0, 0, 255], true).do_(&mut app.document);
    let engrave = app.document.layer_by_name("Engrave").unwrap().id;
    let doc = &mut app.document;
    doc.push_current(Entity::Line(Line::new(
        Vec2::new(0.0, 0.0),
        Vec2::new(10.0, 0.0),
    )));
    doc.push_entity(
        Entity::Circle(Circle::new(Vec2::new(20.0, 20.0), 5.0)),
        engrave,
    );
    doc.push_current(Entity::Arc(Arc::new(
        Vec2::new(30.0, 0.0),
        3.0,
        0.0,
        FRAC_PI_2,
        true,
    )));
    doc.push_entity(
        Entity::Line(Line::new(Vec2::new(0.0, 10.0), Vec2::new(10.0, 10.0))),
        engrave,
    );
    app
}

/// Parse and apply one call; the outcome and the revisions it took.
fn run(app: &mut App, tool: &str, args: serde_json::Value) -> (AgentOutcome, u64) {
    let before = app.history.revision();
    let action = parse_tool_call(tool, &args).unwrap_or_else(|e| panic!("{e}"));
    let outcome = apply(app, &action);
    (outcome, app.history.revision() - before)
}

/// `set_app()`'s document after `command`.
fn expected(mut command: impl Command) -> Vec<Entity> {
    let mut app = set_app();
    command.do_(&mut app.document);
    app.document.entities
}

/// LCV-186 AC1, AC2 — a set move, rotate, scale and in-place mirror given
/// in any order commit one command that uses the one base point or axis for
/// every listed entity, leave the others alone, and undo in one step.
#[test]
fn a_set_transform_is_one_command_about_one_base() {
    let indices = json!([3, 0, 1]);
    let cases: [(&str, serde_json::Value, Vec<Entity>, &str); 4] = [
        (
            "move_entity",
            json!({"indices":indices,"dx":1,"dy":2}),
            expected(MoveEntities::new(vec![0, 1, 3], Vec2::new(1.0, 2.0))),
            "Moved 3 entities by (1.000, 2.000) mm.",
        ),
        (
            "rotate_entity",
            json!({"indices":indices,"x":5,"y":5,"degrees":90}),
            expected(TransformEntities::new(
                vec![0, 1, 3],
                Transform::Rotate {
                    base: Vec2::new(5.0, 5.0),
                    angle: FRAC_PI_2,
                },
            )),
            "Rotated 3 entities by 90.000° about (5.000, 5.000) mm.",
        ),
        (
            "scale_entity",
            json!({"indices":indices,"x":1,"y":2,"factor":2}),
            expected(TransformEntities::new(
                vec![0, 1, 3],
                Transform::Scale {
                    base: Vec2::new(1.0, 2.0),
                    factor: 2.0,
                },
            )),
            "Scaled 3 entities by 2.000 about (1.000, 2.000) mm.",
        ),
        (
            "mirror_entity",
            json!({"indices":indices,"x1":0,"y1":0,"x2":0,"y2":5,"erase_source":true}),
            expected(TransformEntities::new(vec![0, 1, 3], across_y())),
            "Mirrored 3 entities across the line (0.000, 0.000)–(0.000, 5.000) mm.",
        ),
    ];
    for (tool, args, entities, sentence) in cases {
        let mut app = set_app();
        let (outcome, steps) = run(&mut app, tool, args);
        assert!(
            matches!(outcome, AgentOutcome::Ok(_)),
            "{tool}: {outcome:?}"
        );
        assert_eq!(
            outcome.text(),
            format!("{sentence} The drawing now has 4 entities."),
            "{tool}"
        );
        assert_eq!(steps, 1, "{tool}: one command");
        assert_eq!(app.document.entities, entities, "{tool}");
        assert!(app.history.undo(&mut app.document), "{tool}");
        assert_eq!(app.document.entities, set_app().document.entities, "{tool}");
    }
}

/// LCV-186 AC3 — a set copy and a keep-source set mirror, listed in
/// descending order, append the new entities in ascending source order, each
/// on its source's layer, and name the new indices.
#[test]
fn set_copies_append_in_ascending_source_order_on_source_layers() {
    let cases = [
        (
            "copy_entity",
            json!({"indices":[3, 1, 0],"dx":1,"dy":2}),
            expected(CopyEntities::new(vec![0, 1, 3], Vec2::new(1.0, 2.0))),
            "Copied 3 entities by (1.000, 2.000) mm as entities 4..6.",
        ),
        (
            "mirror_entity",
            json!({"indices":[3, 1, 0],"x1":0,"y1":0,"x2":0,"y2":5,"erase_source":false}),
            expected(TransformEntities::new(vec![0, 1, 3], across_y()).with_keep_source(true)),
            "Mirrored 3 entities across the line (0.000, 0.000)–(0.000, 5.000) mm as entities 4..6.",
        ),
    ];
    for (tool, args, entities, sentence) in cases {
        let mut app = set_app();
        let (outcome, steps) = run(&mut app, tool, args);
        assert_eq!(
            outcome.text(),
            format!("{sentence} The drawing now has 7 entities."),
            "{tool}"
        );
        assert_eq!(steps, 1, "{tool}");
        assert_eq!(app.document.entities, entities, "{tool}");
        for (copy, source) in [(4, 0), (5, 1), (6, 3)] {
            assert_eq!(
                app.document.entity_layer(copy),
                app.document.entity_layer(source),
                "{tool}: entity {copy}"
            );
        }
        assert_ne!(
            app.document.entity_layer(4),
            app.document.entity_layer(5),
            "control"
        );
    }
    let mut app = set_app();
    let (one, _) = run(
        &mut app,
        "copy_entity",
        json!({"indices":[2],"dx":0,"dy":1}),
    );
    assert_eq!(
        one.text(),
        "Copied 1 entity by (0.000, 1.000) mm as entity 4. The drawing now has 5 entities."
    );
}

/// LCV-186 AC4 — a set delete removes every listed entity in one command;
/// the result states the new count and that later indices shifted, or that
/// none did when the tail went.
#[test]
fn a_set_delete_states_the_count_and_the_shift() {
    let mut app = set_app();
    let (outcome, steps) = run(&mut app, "delete_entity", json!({"indices":[2, 0]}));
    assert_eq!(steps, 1);
    assert_eq!(
        app.document.entities,
        expected(DeleteEntities::new(vec![0, 2]))
    );
    assert_eq!(
        outcome.text(),
        "Deleted 2 entities (indices 0, 2). Every later index moved down by the \
         number of deleted entities before it. The drawing now has 2 entities."
    );
    assert!(app.history.undo(&mut app.document));
    assert_eq!(app.document.entities, set_app().document.entities);

    let mut app = set_app();
    let (tail, _) = run(&mut app, "delete_entity", json!({"indices":[3, 2]}));
    assert_eq!(
        tail.text(),
        "Deleted 2 entities (indices 2, 3). No indices shifted. The drawing now has 2 entities."
    );
    let mut app = set_app();
    let (last, _) = run(&mut app, "delete_entity", json!({"indices":[3]}));
    assert_eq!(
        last.text(),
        "Deleted 1 entity (index 3). No indices shifted. The drawing now has 3 entities."
    );
    let mut app = set_app();
    let (one, _) = run(&mut app, "delete_entity", json!({"indices":[1]}));
    assert!(
        one.text()
            .starts_with("Deleted 1 entity (index 1). Every later"),
        "{}",
        one.text()
    );
}

/// LCV-186 AC5, AC6 — an out-of-range entry, a mirror line that is a point
/// and a factor that is not positive each change no entity at all; a
/// whole-turn rotation or factor 1 commits nothing.
#[test]
fn a_set_that_cannot_apply_to_every_entity_changes_nothing() {
    let untouched = set_app().document.entities;
    let mut app = set_app();
    let refused = [
        (
            "move_entity",
            json!({"indices":[0, 4, 1],"dx":1,"dy":1}),
            "indices[1] = 4 is out of range (the drawing has 4 entities); nothing was changed",
        ),
        (
            "delete_entity",
            json!({"indices":[9]}),
            "indices[0] = 9 is out of range (the drawing has 4 entities); nothing was changed",
        ),
        (
            "mirror_entity",
            json!({"indices":[0, 1],"x1":2,"y1":2,"x2":2,"y2":2,"erase_source":false}),
            "the mirror line needs two distinct points, got (2.000, 2.000) mm twice",
        ),
    ];
    for (tool, args, text) in refused {
        let (outcome, steps) = run(&mut app, tool, args);
        assert!(
            matches!(outcome, AgentOutcome::Refused(_)),
            "{tool}: {outcome:?}"
        );
        assert_eq!(outcome.text(), text, "{tool}");
        assert_eq!(steps, 0, "{tool}");
    }
    let bad = parse_tool_call(
        "scale_entity",
        &json!({"indices":[0, 1],"x":0,"y":0,"factor":-2}),
    );
    assert!(bad.unwrap_err().to_string().contains("factor:"));
    let noop = [
        (
            "rotate_entity",
            json!({"indices":[0, 1],"x":3,"y":3,"degrees":360}),
            "A whole-turn rotation leaves 2 entities unchanged; nothing committed.",
        ),
        (
            "scale_entity",
            json!({"indices":[0, 1],"x":3,"y":3,"factor":1}),
            "A factor of 1 leaves 2 entities unchanged; nothing committed.",
        ),
    ];
    for (tool, args, text) in noop {
        let (outcome, steps) = run(&mut app, tool, args);
        assert!(
            matches!(outcome, AgentOutcome::Ok(_)),
            "{tool}: {outcome:?}"
        );
        assert_eq!(outcome.text(), text, "{tool}");
        assert_eq!(steps, 0, "{tool}");
    }
    assert_eq!(app.document.entities, untouched);
}

/// LCV-186 AC8 — every edit tool called with `index` gives today's result
/// text and document, and commits one command.
#[test]
fn a_single_index_call_behaves_as_before() {
    let cases: [(&str, serde_json::Value, Vec<Entity>, &str); 7] = [
        (
            "delete_entity",
            json!({"index":1}),
            expected(DeleteEntities::new(vec![1])),
            "Deleted entity 1 (circle, center (20.000, 20.000) mm, r = 5.000 mm). Indices 2..3 are now 1..2. The drawing now has 3 entities.",
        ),
        (
            "move_entity",
            json!({"index":1,"dx":1,"dy":2}),
            expected(MoveEntities::new(vec![1], Vec2::new(1.0, 2.0))),
            "Moved entity 1 (circle, center (20.000, 20.000) mm, r = 5.000 mm) by (1.000, 2.000) mm. The drawing now has 4 entities.",
        ),
        (
            "copy_entity",
            json!({"index":1,"dx":1,"dy":2}),
            expected(CopyEntities::new(vec![1], Vec2::new(1.0, 2.0))),
            "Copied entity 1 (circle, center (20.000, 20.000) mm, r = 5.000 mm) by (1.000, 2.000) mm as entity 4. The drawing now has 5 entities.",
        ),
        (
            "rotate_entity",
            json!({"index":2,"x":5,"y":5,"degrees":90}),
            expected(TransformEntities::new(
                vec![2],
                Transform::Rotate {
                    base: Vec2::new(5.0, 5.0),
                    angle: FRAC_PI_2,
                },
            )),
            "Rotated entity 2 (arc, center (30.000, 0.000) mm, r = 3.000 mm, 0.0°→90.0° ccw) by 90.000° about (5.000, 5.000) mm. The drawing now has 4 entities.",
        ),
        (
            "mirror_entity",
            json!({"index":0,"x1":0,"y1":0,"x2":0,"y2":5,"erase_source":true}),
            expected(TransformEntities::new(vec![0], across_y())),
            "Mirrored entity 0 (line, (0.000, 0.000) → (10.000, 0.000) mm) across the line (0.000, 0.000)–(0.000, 5.000) mm. The drawing now has 4 entities.",
        ),
        (
            "mirror_entity",
            json!({"index":3,"x1":0,"y1":0,"x2":0,"y2":5,"erase_source":false}),
            expected(TransformEntities::new(vec![3], across_y()).with_keep_source(true)),
            "Mirrored entity 3 (line, (0.000, 10.000) → (10.000, 10.000) mm) across the line (0.000, 0.000)–(0.000, 5.000) mm as entity 4. The drawing now has 5 entities.",
        ),
        (
            "scale_entity",
            json!({"index":1,"x":1,"y":2,"factor":2}),
            expected(TransformEntities::new(
                vec![1],
                Transform::Scale {
                    base: Vec2::new(1.0, 2.0),
                    factor: 2.0,
                },
            )),
            "Scaled entity 1 (circle, center (20.000, 20.000) mm, r = 5.000 mm) by 2.000 about (1.000, 2.000) mm. The drawing now has 4 entities.",
        ),
    ];
    for (tool, args, entities, text) in cases {
        let mut app = set_app();
        let (outcome, steps) = run(&mut app, tool, args);
        assert_eq!(outcome.text(), text, "{tool}");
        assert_eq!(steps, 1, "{tool}");
        assert_eq!(app.document.entities, entities, "{tool}");
    }
}
