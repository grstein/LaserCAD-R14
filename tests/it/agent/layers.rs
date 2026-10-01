//! LCV-156 AC 14 — the agent sees each entity's layer, creates on an existing
//! layer by name, and an unknown name is refused with nothing committed
//! (ADR 0012 §6).

use lasercad::agent::{AgentAction, AgentOutcome, DrawingItem, ToolCallError, parse_tool_call};
use lasercad::app::{App, apply};
use lasercad::document::{AddLayer, Command, LayerId};
use serde_json::json;

fn app_with_mark() -> (App, LayerId) {
    let mut app = App::default();
    let mut add = AddLayer::new("Mark", [0, 0, 255], true);
    add.do_(&mut app.document);
    let mark = add.id().expect("allocated");
    (app, mark)
}

fn action(name: &str, args: serde_json::Value) -> AgentAction {
    parse_tool_call(name, &args).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// AC 14 — each creation tool accepts an optional `layer` string; absent
/// means `None`, a non-string or a name outside 1..=64 characters is a
/// shape error naming the field.
#[test]
fn creation_tools_take_an_optional_layer_string() {
    let line = json!({"x1":0,"y1":0,"x2":1,"y2":1,"layer":"Mark"});
    let circle = json!({"cx":0,"cy":0,"r":1,"layer":"Mark"});
    let arc = json!({"cx":0,"cy":0,"r":1,"start_deg":0,"end_deg":90,"ccw":true,"layer":"Mark"});
    let drawing = json!({"version":1,"layer":"Mark",
        "entities":[{"type":"circle","cx":0,"cy":0,"r":1}]});
    for (name, args) in [
        ("create_line", line),
        ("create_circle", circle),
        ("create_arc", arc),
        ("create_drawing", drawing),
    ] {
        assert_eq!(action(name, args.clone()).layer(), Some("Mark"), "{name}");
        let mut bare = args.clone();
        bare.as_object_mut().unwrap().remove("layer");
        assert_eq!(action(name, bare).layer(), None, "{name}");
        // LCV-192 AC 1 — the refusal names tool, field, reason and form.
        let form = "expected the name of an existing layer, 1 to 64 characters";
        for (bad, reason) in [
            (json!(""), "has 0 characters"),
            (json!("x".repeat(65)), "has 65 characters"),
            (json!(7), "not a string"),
        ] {
            let mut wrong = args.clone();
            wrong["layer"] = bad.clone();
            let err = parse_tool_call(name, &wrong).expect_err("refused");
            assert_eq!(err.to_string(), format!("{name} layer: {reason}; {form}"));
        }
    }
    let long = json!({"cx":0,"cy":0,"r":1,"layer":"é".repeat(64)});
    assert!(
        parse_tool_call("create_circle", &long).is_ok(),
        "64 chars, not bytes"
    );
    let bad = json!({"version":1,"layer":3,"entities":[{"type":"circle","cx":0,"cy":0,"r":1}]});
    assert!(matches!(
        parse_tool_call("create_drawing", &bad),
        Err(ToolCallError::Arg { ref path, .. }) if path == "layer"
    ));
}

/// AC 14 — a named layer is resolved by key (case-folded) and the entity
/// lands on it; no name means the current layer.
#[test]
fn a_named_layer_receives_the_new_entities() {
    let (mut app, mark) = app_with_mark();
    let args = json!({"x1":0,"y1":0,"x2":5,"y2":5,"layer":"mark"});
    let out = apply(&mut app, &action("create_line", args));
    assert!(matches!(out, AgentOutcome::Ok(_)), "{out:?}");
    assert_eq!(app.document.entity_layer(0), Some(mark));

    let args = json!({"cx":1,"cy":1,"r":1});
    apply(&mut app, &action("create_circle", args));
    assert_eq!(
        app.document.entity_layer(1),
        Some(LayerId(0)),
        "current layer"
    );

    let batch = AgentAction::CreateDrawing {
        items: vec![
            DrawingItem::Circle {
                cx: 0.0,
                cy: 0.0,
                r: 1.0,
            },
            DrawingItem::Circle {
                cx: 3.0,
                cy: 0.0,
                r: 1.0,
            },
        ],
        layer: Some("MARK".into()),
    };
    apply(&mut app, &batch);
    assert_eq!(app.document.entity_layer(2), Some(mark));
    assert_eq!(app.document.entity_layer(3), Some(mark));
}

/// AC 14 — an unknown layer name is refused, the refusal names the existing
/// layers, and nothing is committed.
#[test]
fn an_unknown_layer_is_refused_naming_the_layers() {
    let (mut app, _) = app_with_mark();
    let r0 = app.history.revision();
    let args = json!({"cx":0,"cy":0,"r":1,"layer":"Engrave"});
    let batch = json!({"version":1,"layer":"Engrave",
        "entities":[{"type":"circle","cx":0,"cy":0,"r":1}]});
    for (name, args) in [("create_circle", args), ("create_drawing", batch)] {
        let out = apply(&mut app, &action(name, args));
        let AgentOutcome::Refused(text) = out else {
            panic!("{name}: {out:?}")
        };
        // LCV-192 AC 2 — the shape, with the existing names as the form.
        assert_eq!(
            text,
            format!(r#"{name} layer: unknown layer "Engrave"; expected one of "Cut", "Mark""#)
        );
    }
    assert_eq!(app.history.revision(), r0);
    assert_eq!(app.document.entity_count(), 0);
}

/// AC 14 — `query_entities` lists the layers (current marked) and each
/// entity's layer.
#[test]
fn query_entities_reports_each_entity_layer() {
    let (mut app, _) = app_with_mark();
    let empty = apply(&mut app, &AgentAction::QueryEntities);
    assert!(
        empty.text().contains("Layers: Cut (current), Mark"),
        "{}",
        empty.text()
    );

    apply(
        &mut app,
        &action("create_circle", json!({"cx":0,"cy":0,"r":1})),
    );
    let line = json!({"x1":0,"y1":0,"x2":5,"y2":5,"layer":"Mark"});
    apply(&mut app, &action("create_line", line));
    let listing = apply(&mut app, &AgentAction::QueryEntities);
    let text = listing.text();
    assert!(text.contains("Layers: Cut (current), Mark"), "{text}");
    let rows: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with(char::is_numeric))
        .collect();
    assert_eq!(rows.len(), 2, "{text}");
    assert!(
        rows[0].starts_with("0 e1: circle") && rows[0].ends_with("layer Cut"),
        "{text}"
    );
    assert!(
        rows[1].starts_with("1 e2: line") && rows[1].ends_with("layer Mark"),
        "{text}"
    );
}

/// LCV-170 AC 9 — a creation call naming a layer with a control character is
/// refused (at parse or apply) and adds no entity, even though the name's
/// file key matches `Cut`. The refusal text is not pinned (LCV-192 rewords it).
#[test]
fn a_control_character_layer_name_is_refused() {
    let (mut app, _) = app_with_mark();
    let r0 = app.history.revision();
    let args = json!({"x1":0,"y1":0,"x2":5,"y2":5,"layer":"Cut\u{7}"});
    if let Ok(call) = parse_tool_call("create_line", &args) {
        let out = apply(&mut app, &call);
        assert!(matches!(out, AgentOutcome::Refused(_)), "{out:?}");
    }
    assert_eq!(app.history.revision(), r0);
    assert_eq!(app.document.entity_count(), 0);
}

/// LCV-191 AC 1 — `set_layer {indices, layer}` parses into one set action
/// carrying the layer name, indices in the order given.
#[test]
fn set_layer_parses_into_a_layer_set() {
    use lasercad::agent::SetOp;
    assert_eq!(
        action("set_layer", json!({"indices":[3,0,1],"layer":"Mark"})),
        AgentAction::Set {
            indices: vec![3, 0, 1],
            op: SetOp::Layer {
                layer: "Mark".into()
            },
        }
    );
    assert_eq!(
        action("set_layer", json!({"indices":[0],"layer":"m"})).tool_name(),
        "set_layer"
    );
}

/// LCV-191 AC 1, AC 4 — a missing or malformed `layer`, a missing
/// `indices`, any `index`, and each bad list are refused naming the field.
#[test]
fn set_layer_refuses_bad_arguments_naming_the_field() {
    let name = "expected the name of an existing layer, 1 to 64 characters";
    let list = "expected a list of 1 to 1000 distinct entity indices";
    let index = "expected a non-negative integer (an index from query_entities)";
    let too_many: Vec<usize> = (0..1001).collect();
    let cases = [
        (json!({"indices":[0]}), format!("layer: missing; {name}")),
        (
            json!({"indices":[0],"layer":7}),
            format!("layer: not a string; {name}"),
        ),
        (
            json!({"indices":[0],"layer":""}),
            format!("layer: has 0 characters; {name}"),
        ),
        (
            json!({"indices":[0],"layer":"x".repeat(65)}),
            format!("layer: has 65 characters; {name}"),
        ),
        (json!({"layer":"Mark"}), format!("indices: missing; {list}")),
        (
            json!({"indices":null,"layer":"Mark"}),
            format!("indices: missing; {list}"),
        ),
        (
            json!({"index":0,"layer":"Mark"}),
            "index: not accepted; expected indices instead".to_owned(),
        ),
        (
            json!({"index":0,"indices":[0],"layer":"Mark"}),
            "index: not accepted; expected indices instead".to_owned(),
        ),
        (
            json!({"indices":[],"layer":"Mark"}),
            format!("indices: empty list; {list}"),
        ),
        (
            json!({"indices":too_many,"layer":"Mark"}),
            format!("indices: has 1001 entries; {list}"),
        ),
        (
            json!({"indices":3,"layer":"Mark"}),
            format!("indices: not a list; {list}"),
        ),
        (
            json!({"indices":[0,4,2,4],"layer":"Mark"}),
            "indices[3]: duplicate of indices[1]; expected distinct indices".to_owned(),
        ),
        (
            json!({"indices":[0,-1],"layer":"Mark"}),
            format!("indices[1]: -1 is not an index; {index}"),
        ),
        (
            json!({"indices":[1.5],"layer":"Mark"}),
            format!("indices[0]: 1.5 is not an index; {index}"),
        ),
        (
            json!({"indices":[0,"2"],"layer":"Mark"}),
            format!("indices[1]: not a number; {index}"),
        ),
    ];
    for (args, want) in cases {
        let err = parse_tool_call("set_layer", &args).expect_err("refused");
        assert_eq!(err.to_string(), format!("set_layer {want}"), "{args}");
    }
}

/// Three circles on the current layer (Cut), plus the Mark layer.
fn app_with_three_circles() -> (App, LayerId) {
    let (mut app, mark) = app_with_mark();
    for cx in [0, 10, 20] {
        apply(
            &mut app,
            &action("create_circle", json!({"cx":cx,"cy":0,"r":1})),
        );
    }
    (app, mark)
}

fn layers_of(app: &App) -> Vec<Option<LayerId>> {
    (0..app.document.entity_count())
        .map(|i| app.document.entity_layer(i))
        .collect()
}

/// LCV-191 AC 1 — every listed entity moves, indices in any order, as one
/// command; the current layer is untouched.
#[test]
fn set_layer_moves_every_listed_entity_as_one_command() {
    let (mut app, mark) = app_with_three_circles();
    let cut = LayerId(0);
    let r0 = app.history.revision();
    let out = apply(
        &mut app,
        &action("set_layer", json!({"indices":[2,0],"layer":"mark"})),
    );
    assert_eq!(
        out,
        AgentOutcome::Ok(
            r#"Moved 2 entities to layer "Mark". The drawing now has 3 entities."#.into()
        )
    );
    assert_eq!(layers_of(&app), vec![Some(mark), Some(cut), Some(mark)]);
    assert_eq!(app.history.revision(), r0 + 1);
    assert_eq!(app.document.current_layer(), cut);
    assert!(app.history.undo(&mut app.document));
    assert_eq!(layers_of(&app), vec![Some(cut); 3]);
}

/// LCV-191 AC 2 — entities already on the layer are left and counted; when
/// all are, the call succeeds and commits nothing.
#[test]
fn set_layer_leaves_entities_already_there() {
    let (mut app, mark) = app_with_three_circles();
    let cut = LayerId(0);
    apply(
        &mut app,
        &action("set_layer", json!({"indices":[1],"layer":"Mark"})),
    );
    let r1 = app.history.revision();
    let out = apply(
        &mut app,
        &action("set_layer", json!({"indices":[1,2],"layer":"Mark"})),
    );
    assert_eq!(
        out.text(),
        r#"Moved 1 entity to layer "Mark" (1 already there). The drawing now has 3 entities."#
    );
    assert_eq!(app.history.revision(), r1 + 1);
    assert_eq!(layers_of(&app), vec![Some(cut), Some(mark), Some(mark)]);

    let r2 = app.history.revision();
    let out = apply(
        &mut app,
        &action("set_layer", json!({"indices":[2,1],"layer":"MARK"})),
    );
    assert_eq!(
        out,
        AgentOutcome::Ok(r#"2 entities already on layer "Mark"; nothing committed."#.into())
    );
    assert_eq!(app.history.revision(), r2, "no empty undo step");
    assert_eq!(layers_of(&app), vec![Some(cut), Some(mark), Some(mark)]);
}

/// LCV-191 AC 3, AC 4 — an unknown layer or an out-of-range index is
/// refused and nothing changes.
#[test]
fn set_layer_refuses_an_unknown_layer_or_index() {
    let (mut app, _) = app_with_three_circles();
    let r0 = app.history.revision();
    let out = apply(
        &mut app,
        &action("set_layer", json!({"indices":[0],"layer":"Engrave"})),
    );
    assert_eq!(
        out,
        AgentOutcome::Refused(
            r#"set_layer layer: unknown layer "Engrave"; expected one of "Cut", "Mark""#.into()
        )
    );
    let out = apply(
        &mut app,
        &action("set_layer", json!({"indices":[0,3],"layer":"Mark"})),
    );
    let AgentOutcome::Refused(text) = out else {
        panic!("{out:?}")
    };
    assert!(
        text.starts_with("set_layer indices[1]: 3 is out of range"),
        "{text}"
    );
    assert_eq!(app.history.revision(), r0);
    assert_eq!(layers_of(&app), vec![Some(LayerId(0)); 3]);
}
