//! LCV-188 — the agent addresses entities by stable ids (ADR 0014): listed
//! by `query_entities` (AC 1), accepted as `id`/`ids` by every tool that
//! takes `index`/`indices` (AC 4, AC 5), returned by creations (AC 6).

use lasercad::agent::{AgentAction, AgentOutcome, parse_tool_call};
use lasercad::app::{App, apply};
use lasercad::document::{AddLayer, Command, Entity, EntityId, LayerId};
use lasercad::geometry::{Arc, Circle, Line, Vec2};
use serde_json::{Value, json};

fn line(x: f64) -> Entity {
    Entity::Line(Line::new(Vec2::new(x, 0.0), Vec2::new(x, 10.0)))
}

fn ok(outcome: AgentOutcome) -> String {
    match outcome {
        AgentOutcome::Ok(text) => text,
        other => panic!("not ok: {other:?}"),
    }
}

/// AC 1 — every listed entity shows `e<N>` beside its index, and the ids
/// survive a delete while the indices shift.
#[test]
fn query_entities_lists_the_id_beside_the_index() {
    let mut app = App::default();
    app.document.push_current(line(0.0));
    app.document
        .push_current(Entity::Circle(Circle::new(Vec2::new(5.0, 5.0), 2.0)));
    app.document.push_current(line(3.0));
    app.document.remove_entity(0);
    let listing = ok(apply(&mut app, &AgentAction::QueryEntities));
    let rows: Vec<&str> = listing.lines().skip(2).collect();
    assert_eq!(
        rows,
        [
            "0 e2: circle center (5.000, 5.000) mm, r = 2.000 mm layer Cut",
            "1 e3: line (3.000, 0.000) → (3.000, 10.000) mm layer Cut",
        ]
    );
}

/// Four entities on `Cut` (a line, a circle, an arc, a line) after `e1` was
/// deleted, so entity `i` is `e(i + 2)`, plus a `Mark` layer.
fn fixture() -> App {
    let mut app = App::default();
    AddLayer::new("Mark", [0, 0, 255], true).do_(&mut app.document);
    app.document.push_current(line(-5.0));
    app.document.push_current(line(0.0));
    app.document
        .push_current(Entity::Circle(Circle::new(Vec2::new(5.0, 5.0), 2.0)));
    let arc = Arc::new(Vec2::new(9.0, 9.0), 3.0, 0.0, 1.0, true);
    app.document.push_current(Entity::Arc(arc));
    app.document.push_current(line(3.0));
    app.document.remove_entity(0);
    app
}

/// Every entity with its layer and id, in order.
fn snapshot(app: &App) -> Vec<(Entity, Option<LayerId>, Option<EntityId>)> {
    let doc = &app.document;
    (0..doc.entity_count())
        .map(|i| (doc.entities[i], doc.entity_layer(i), doc.entity_id(i)))
        .collect()
}

fn call(app: &mut App, tool: &str, args: Value) -> AgentOutcome {
    let action = parse_tool_call(tool, &args).unwrap_or_else(|e| panic!("{tool}: {e}"));
    apply(app, &action)
}

/// `args` with `key` set to `value`.
fn with(args: &Value, key: &str, value: Value) -> Value {
    let mut out = args.clone();
    out[key] = value;
    out
}

/// AC 4 — for every tool that takes `indices`, `ids` naming the same
/// entities (in another order) gives the same document and outcome; a one-
/// entry `id` matches a one-entry `indices`.
#[test]
fn ids_edit_exactly_like_the_matching_indices() {
    let cases = [
        ("delete_entity", json!({})),
        ("move_entity", json!({"dx":1.5,"dy":-2.0})),
        ("copy_entity", json!({"dx":20.0,"dy":0.0})),
        ("rotate_entity", json!({"x":1.0,"y":2.0,"degrees":30.0})),
        (
            "mirror_entity",
            json!({"x1":0,"y1":0,"x2":0,"y2":10,"erase_source":true}),
        ),
        (
            "mirror_entity",
            json!({"x1":0,"y1":0,"x2":0,"y2":10,"erase_source":false}),
        ),
        ("scale_entity", json!({"x":0.0,"y":0.0,"factor":2.0})),
        ("set_layer", json!({"layer":"Mark"})),
    ];
    for (tool, args) in cases {
        for (indices, ids, id) in [
            (json!([3, 0, 2]), json!(["e5", "e2", "e4"]), None),
            (json!([1]), json!(["e3"]), Some("e3")),
        ] {
            let mut by_index = fixture();
            let expected = call(&mut by_index, tool, with(&args, "indices", indices));
            assert!(
                matches!(expected, AgentOutcome::Ok(_)),
                "{tool}: {expected:?}"
            );
            let mut forms = vec![with(&args, "ids", ids)];
            forms.extend(id.map(|id| with(&args, "id", json!(id))));
            for form in forms {
                let mut by_id = fixture();
                let outcome = call(&mut by_id, tool, form.clone());
                assert_eq!(outcome, expected, "{tool} {form}");
                assert_eq!(snapshot(&by_id), snapshot(&by_index), "{tool} {form}");
            }
        }
    }
}

/// AC 4 — an id still names its entity after an earlier delete in the same
/// turn shifted the indices.
#[test]
fn ids_survive_an_earlier_delete() {
    let mut app = fixture();
    ok(call(&mut app, "delete_entity", json!({"index":0})));
    let arc_at = app.document.index_of(EntityId(4)).expect("e4 is live");
    assert_eq!(arc_at, 1, "the arc shifted down");
    ok(call(
        &mut app,
        "move_entity",
        json!({"id":"e4","dx":1.0,"dy":0.0}),
    ));
    let Entity::Arc(arc) = app.document.entities[arc_at] else {
        panic!("e4 is the arc");
    };
    assert_eq!(arc.center, Vec2::new(10.0, 9.0));
    assert_eq!(app.document.entity_id(arc_at), Some(EntityId(4)));
}

/// AC 5 — one unknown id (never issued, or deleted) refuses the whole call,
/// names its entry, and changes nothing: same document, same revision.
#[test]
fn an_unknown_id_refuses_the_whole_call() {
    for (args, path, unknown) in [
        (json!({"ids":["e2","e1"],"dx":1,"dy":1}), "ids[1]", "e1"),
        (json!({"ids":["e99"],"dx":1,"dy":1}), "ids[0]", "e99"),
        (json!({"id":"e6","dx":1,"dy":1}), "ids[0]", "e6"),
    ] {
        let mut app = fixture();
        let (before, revision) = (snapshot(&app), app.history.revision());
        let outcome = call(&mut app, "move_entity", args.clone());
        let AgentOutcome::Refused(text) = outcome else {
            panic!("not refused: {outcome:?}");
        };
        assert!(text.starts_with(&format!("move_entity {path}: ")), "{text}");
        assert!(text.contains(unknown), "{text}");
        assert_eq!(snapshot(&app), before, "{args}");
        assert_eq!(app.history.revision(), revision, "{args}");
    }
}

/// AC 6 — every call that appended entities ends with their new ids; a call
/// that appended nothing has no suffix. The fixture's next id is `e6`.
#[test]
fn appending_calls_end_with_the_new_ids() {
    let drawing = json!({"version":1,"entities":[
        {"type":"circle","cx":0,"cy":0,"r":1},
        {"type":"line","x1":0,"y1":0,"x2":1,"y2":1},
        {"type":"circle","cx":5,"cy":5,"r":1}]});
    let mirror =
        |erase: bool| json!({"ids":["e2","e4"],"x1":0,"y1":0,"x2":0,"y2":10,"erase_source":erase});
    let arc = json!({"cx":0,"cy":0,"r":1,"start_deg":0,"end_deg":90,"ccw":true});
    for (tool, args, suffix) in [
        (
            "create_line",
            json!({"x1":0,"y1":0,"x2":1,"y2":1}),
            " New id: e6.",
        ),
        (
            "create_circle",
            json!({"cx":0,"cy":0,"r":1}),
            " New id: e6.",
        ),
        ("create_arc", arc, " New id: e6."),
        ("create_drawing", drawing, " New ids: e6..=e8."),
        (
            "copy_entity",
            json!({"ids":["e3","e2"],"dx":1,"dy":0}),
            " New ids: e6..=e7.",
        ),
        (
            "copy_entity",
            json!({"index":0,"dx":1,"dy":0}),
            " New id: e6.",
        ),
        ("mirror_entity", mirror(false), " New ids: e6..=e7."),
    ] {
        let mut app = fixture();
        let text = ok(call(&mut app, tool, args.clone()));
        assert!(text.ends_with(suffix), "{tool} {args}: {text}");
        assert_eq!(text.matches(" New id").count(), 1, "{text}");
    }
    for (tool, args) in [
        ("delete_entity", json!({"ids":["e2"]})),
        ("move_entity", json!({"id":"e2","dx":1,"dy":0})),
        ("rotate_entity", json!({"id":"e2","x":0,"y":0,"degrees":30})),
        ("scale_entity", json!({"id":"e2","x":0,"y":0,"factor":2})),
        ("mirror_entity", mirror(true)),
        ("set_layer", json!({"ids":["e2"],"layer":"Mark"})),
    ] {
        let mut app = fixture();
        let text = ok(call(&mut app, tool, args.clone()));
        assert!(!text.contains("New id"), "{tool} {args}: {text}");
    }
}
