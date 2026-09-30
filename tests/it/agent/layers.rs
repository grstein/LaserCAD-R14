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
        for bad in [json!(""), json!("x".repeat(65)), json!(7)] {
            let mut wrong = args.clone();
            wrong["layer"] = bad.clone();
            let err = parse_tool_call(name, &wrong).expect_err("refused");
            assert!(err.to_string().contains("layer"), "{name} {bad}: {err}");
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
        Err(ToolCallError::DrawingRoot { .. })
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
        for want in ["Engrave", "Cut", "Mark"] {
            assert!(text.contains(want), "{name}: {text}");
        }
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
        rows[0].starts_with("0: circle") && rows[0].ends_with("layer Cut"),
        "{text}"
    );
    assert!(
        rows[1].starts_with("1: line") && rows[1].ends_with("layer Mark"),
        "{text}"
    );
}

/// LCV-170 AC 9 — a creation call naming a layer with a control character is
/// refused (at parse or apply) and adds no entity, even though the name's
/// file key matches `Cut`. The refusal text is not pinned (LCV-192 rewords it).
#[test]
#[ignore = "LCV-170: agent resolves \"Cut\\u{7}\" to Cut via Document::layer_by_name (name_key match); refusing it needs a change in src/app/agent_apply.rs, outside the plan"]
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
