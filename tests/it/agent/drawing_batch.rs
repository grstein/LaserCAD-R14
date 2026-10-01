//! LCV-144 — `create_drawing`: one validated JSON batch is one dispatch, one
//! command, one revision (ADR 0010).
//!
//! Every haystack of a source scan is a `src/` file cut at its first column-0
//! `#[cfg(test)]`, so a needle quoted by a test module cannot satisfy or defeat
//! the scan.

use std::path::Path;

fn implementation(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let end = src.find("\n#[cfg(test)]").unwrap_or(src.len());
    src[..end].to_owned()
}

/// AC 11 — the eight narration helpers are defined in `agent_narrate.rs` and
/// not in `agent_apply.rs` (ADR 0010 §9). The positive control is `apply`,
/// which must stay in `agent_apply.rs`, so a wrong path or an empty read fails.
#[test]
fn ac11_the_narration_helpers_live_in_agent_narrate() {
    let apply = implementation("src/app/agent_apply.rs");
    let narrate = implementation("src/app/agent_narrate.rs");
    assert!(
        apply.contains("pub fn apply("),
        "positive control: agent_apply.rs keeps `apply`"
    );
    for helper in [
        "pt",
        "sweep",
        "kind",
        "geometry",
        "describe",
        "bed_line",
        "list_entities",
        "list_selection",
    ] {
        let needle = format!("fn {helper}(");
        assert!(
            narrate.contains(&needle),
            "agent_narrate.rs must define `{needle}`"
        );
        assert!(
            !apply.contains(&needle),
            "agent_apply.rs must not define `{needle}`"
        );
    }
}

// ── Integration: hand-pushed `Act`s against an armed turn ───────────────────
//
// No test below reaches an endpoint. Every turn is armed with `arm_turn` and
// fed by hand-pushed `AgentEvent`s (ADR 0007 §D3). ADR 0002 §A2:
// `App::default()` only; ADR 0005: no dialog, no `Ctrl+O` / `Ctrl+S`.

use crate::harness::{frame, tap};
use lasercad::agent::{AgentAction, AgentEvent, AgentOutcome, parse_tool_call};
use lasercad::app::{AGENT_FENCE_REFUSAL, App, arm_turn, cancel_turn};
use lasercad::document::{CreateLine, Entity, SelectionCommand};
use lasercad::geometry::{Line, Vec2};
use lasercad::io::svg::{export_svg, import_svg};
use serde_json::{Value, json};
use std::sync::mpsc::{Receiver, Sender, channel};

/// The existing SVG round-trip tolerance, mm
/// (`tests/it/io_svg/orientation.rs::EXPORT_QUANTISATION_TOL`).
const ROUNDTRIP_TOL: f64 = 1e-3;

fn ctx_and_app() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let app = App::default();
    assert!(app.settings_path.is_none() && app.autosave_path.is_none());
    (ctx, app)
}

fn idle(ctx: &egui::Context, app: &mut App) {
    frame(ctx, app, Vec::new());
}

fn push_act(tx: &Sender<AgentEvent>, action: AgentAction) -> Receiver<AgentOutcome> {
    let (reply, answer) = channel::<AgentOutcome>();
    tx.send(AgentEvent::Act { action, reply })
        .expect("the armed Receiver must still be on App");
    answer
}

/// What the worker's dispatch sends for these arguments: the parsed action,
/// or `Malformed` with the error text (ADR 0007 §D15).
fn dispatch(args: Value) -> AgentAction {
    parse_tool_call("create_drawing", &args).unwrap_or_else(|e| AgentAction::Malformed {
        tool: "create_drawing".into(),
        reason: e.to_string(),
    })
}

fn batch(entities: Value) -> AgentAction {
    dispatch(json!({"version": 1, "entities": entities}))
}

fn mixed() -> AgentAction {
    batch(json!([
        {"type": "line", "x1": 10, "y1": 10, "x2": 40, "y2": 10},
        {"type": "circle", "cx": 25, "cy": 25, "r": 5},
        {"type": "arc", "cx": 40, "cy": 25, "r": 10,
         "start_deg": 0, "end_deg": 90, "ccw": true}
    ]))
}

fn human_line(app: &mut App, y: f64) {
    app.commit(Box::new(CreateLine::new(Line::new(
        Vec2::new(0.0, y),
        Vec2::new(5.0, y),
    ))));
}

fn ctrl() -> egui::Modifiers {
    egui::Modifiers {
        ctrl: true,
        command: true,
        ..egui::Modifiers::NONE
    }
}

fn rows(app: &App, role: &str) -> usize {
    app.agent.chat.iter().filter(|(r, _)| r == role).count()
}

/// AC 3 — an invalid batch is one `refused` row, one step, nothing applied,
/// revision unchanged; the reason names the path.
#[test]
fn ac3_an_invalid_batch_is_one_refused_step_and_changes_nothing() {
    let (ctx, mut app) = ctx_and_app();
    human_line(&mut app, 0.0);
    let revision = app.history.revision();
    let tx = arm_turn(&mut app, "draw");
    let mut entities: Vec<Value> = (0..20)
        .map(|i| json!({"type": "circle", "cx": i, "cy": 0, "r": 1}))
        .collect();
    entities[17]["r"] = json!(-3);
    let answer = push_act(&tx, batch(Value::Array(entities)));
    idle(&ctx, &mut app);

    let reason = "create_drawing entities[17].r: -3 is out of range";
    assert_eq!(
        answer.try_recv().unwrap(),
        AgentOutcome::Refused(reason.into())
    );
    assert_eq!(rows(&app, "refused"), 1);
    assert_eq!(rows(&app, "tool"), 0);
    assert_eq!(app.agent.turn.steps, 1);
    assert_eq!(app.history.revision(), revision);
    assert_eq!(app.document.entity_count(), 1);
}

/// AC 3 — no bed-bounds rule: an entity wholly off the bed is accepted.
#[test]
fn ac3_an_entity_far_off_the_bed_is_accepted() {
    let (ctx, mut app) = ctx_and_app();
    let tx = arm_turn(&mut app, "draw");
    let answer = push_act(
        &tx,
        batch(json!([{"type": "circle", "cx": -10000, "cy": -10000, "r": 1}])),
    );
    idle(&ctx, &mut app);
    assert!(!answer.try_recv().unwrap().is_refused());
    let Some(Entity::Circle(c)) = app.document.entities.first() else {
        panic!("{:?}", app.document.entities)
    };
    assert_eq!((c.center.x, c.center.y), (-10000.0, -10000.0));
}

/// AC 5 — a mixed batch on a 2-entity drawing with a selection and a
/// non-default bed appends in order and touches nothing else.
#[test]
fn ac5_a_mixed_batch_appends_in_order_and_touches_nothing_else() {
    let (ctx, mut app) = ctx_and_app();
    human_line(&mut app, 0.0);
    human_line(&mut app, 1.0);
    app.commit(Box::new(SelectionCommand::new(vec![1])));
    app.document.bed_mm = [300.0, 180.0];
    let before = app.document.entities.clone();

    let tx = arm_turn(&mut app, "draw");
    let answer = push_act(&tx, mixed());
    idle(&ctx, &mut app);
    assert!(!answer.try_recv().unwrap().is_refused());

    assert_eq!(app.document.entity_count(), 5);
    assert_eq!(app.document.entities[..2], before[..]);
    let Entity::Line(l) = app.document.entities[2] else {
        panic!("entity 2 is the line")
    };
    assert_eq!((l.p1, l.p2), (Vec2::new(10.0, 10.0), Vec2::new(40.0, 10.0)));
    assert!(matches!(app.document.entities[3], Entity::Circle(c) if c.r == 5.0));
    assert!(matches!(app.document.entities[4], Entity::Arc(a) if a.ccw && a.r == 10.0));
    assert_eq!(app.document.selection.iter().collect::<Vec<_>>(), [1]);
    assert_eq!(app.document.bed_mm, [300.0, 180.0]);
    assert_eq!(app.document.layers().len(), 1);
}

/// AC 6 — a fenced batch and a cancelled one with its `Act` still queued
/// append nothing and leave the revision alone.
#[test]
fn ac6_a_fenced_or_cancelled_batch_appends_nothing() {
    for case in ["fenced", "cancelled"] {
        let (ctx, mut app) = ctx_and_app();
        let tx = arm_turn(&mut app, "draw");
        if case == "fenced" {
            human_line(&mut app, 0.0);
        }
        let (count, revision) = (app.document.entity_count(), app.history.revision());
        let answer = push_act(&tx, mixed());
        if case == "cancelled" {
            cancel_turn(&mut app);
        }
        idle(&ctx, &mut app);
        match case {
            "fenced" => assert_eq!(
                answer.try_recv().unwrap(),
                AgentOutcome::Fenced(AGENT_FENCE_REFUSAL.to_owned())
            ),
            _ => assert!(answer.try_recv().is_err(), "the reply is dropped"),
        }
        assert_eq!(app.document.entity_count(), count, "{case}");
        assert_eq!(app.history.revision(), revision, "{case}");
    }
}

/// AC 7 — a 1000-item batch is one revision and one step; a scalar + batch +
/// scalar turn undoes with one `Ctrl+Z`.
#[test]
fn ac7_a_thousand_items_are_one_revision_one_step_and_one_undo() {
    let (ctx, mut app) = ctx_and_app();
    let revision = app.history.revision();
    let tx = arm_turn(&mut app, "draw");
    let line = AgentAction::CreateLine {
        layer: None,
        x1: 0.0,
        y1: 0.0,
        x2: 1.0,
        y2: 1.0,
    };
    let first = push_act(&tx, line.clone());
    idle(&ctx, &mut app);
    assert!(!first.try_recv().unwrap().is_refused());
    let (steps, revision_mid) = (app.agent.turn.steps, app.history.revision());
    assert_eq!(revision_mid, revision + 1);

    let entities: Vec<Value> = (0..1000)
        .map(|i| json!({"type": "circle", "cx": i, "cy": 0, "r": 1}))
        .collect();
    let answer = push_act(&tx, batch(Value::Array(entities)));
    idle(&ctx, &mut app);
    assert_eq!(
        answer.try_recv().unwrap(),
        AgentOutcome::Ok(format!(
            "Created 1000 entities (indices 1..=1000). The drawing now has 1001 entities. \
             Revision {}.",
            revision_mid + 1
        ))
    );
    assert_eq!(app.history.revision(), revision_mid + 1);
    assert_eq!(app.agent.turn.steps, steps + 1);

    let last = push_act(&tx, line);
    tx.send(AgentEvent::done("done")).unwrap();
    idle(&ctx, &mut app);
    assert!(!last.try_recv().unwrap().is_refused());
    assert_eq!(app.document.entity_count(), 1002);
    tap(&ctx, &mut app, egui::Key::Z, ctrl());
    assert_eq!(app.document.entity_count(), 0, "one Ctrl+Z, whole turn");
}

/// AC 9 — undo then redo restores identical geometry, and the result
/// round-trips through the LaserGRBL SVG export.
#[test]
fn ac9_undo_redo_and_svg_round_trip() {
    let (ctx, mut app) = ctx_and_app();
    app.document.bed_mm = [300.0, 180.0];
    let tx = arm_turn(&mut app, "draw");
    let answer = push_act(&tx, mixed());
    tx.send(AgentEvent::done("done")).unwrap();
    idle(&ctx, &mut app);
    assert!(!answer.try_recv().unwrap().is_refused());
    let drawn = app.document.entities.clone();
    assert_eq!(drawn.len(), 3);

    tap(&ctx, &mut app, egui::Key::Z, ctrl());
    assert_eq!(app.document.entity_count(), 0);
    tap(&ctx, &mut app, egui::Key::Y, ctrl());
    assert_eq!(
        app.document.entities, drawn,
        "redo restores identical geometry"
    );

    let svg = export_svg(&app.document);
    assert!(svg.contains(r#"viewBox="0 0 300 180""#), "{svg}");
    assert!(
        svg.contains(r#"fill="none""#) && svg.contains(" A 10.0000 10.0000 0 0 0 "),
        "{svg}"
    );
    let imported = import_svg(&svg).expect("the exporter's own output imports");
    assert_eq!(imported.bed_mm, [300.0, 180.0]);
    assert_eq!(imported.entities.len(), 3);
    let close = |a: Vec2, b: Vec2| (a - b).length() < ROUNDTRIP_TOL;
    for (got, want) in imported.entities.iter().zip(&drawn) {
        match (got, want) {
            (Entity::Line(g), Entity::Line(w)) => assert!(close(g.p1, w.p1) && close(g.p2, w.p2)),
            (Entity::Circle(g), Entity::Circle(w)) => {
                assert!(close(g.center, w.center) && (g.r - w.r).abs() < ROUNDTRIP_TOL)
            }
            (Entity::Arc(g), Entity::Arc(w)) => {
                assert!(close(g.center, w.center) && (g.r - w.r).abs() < ROUNDTRIP_TOL);
                assert!(close(g.start_point(), w.start_point()));
                assert!(close(g.end_point(), w.end_point()));
                assert_eq!(g.ccw, w.ccw);
            }
            _ => panic!("kind changed: {got:?} vs {want:?}"),
        }
    }
}

// ── LCV-185: the published schema matches the validator ─────────────────────

/// The reason `dispatch` gives for these entities, or a panic if accepted.
fn refusal(entities: Value) -> String {
    match batch(entities) {
        AgentAction::Malformed { reason, .. } => reason,
        other => panic!("accepted: {other:?}"),
    }
}

/// Draw `entities` in a fresh armed turn; the created entities.
fn drawn(entities: Value) -> Vec<Entity> {
    let (ctx, mut app) = ctx_and_app();
    let tx = arm_turn(&mut app, "draw");
    let answer = push_act(&tx, batch(entities));
    idle(&ctx, &mut app);
    let outcome = answer.try_recv().unwrap();
    assert!(!outcome.is_refused(), "{outcome:?}");
    app.document.entities.clone()
}

/// A valid item of each type with its own keys only.
fn own_items() -> [Value; 3] {
    [
        json!({"type": "line", "x1": 1, "y1": 2, "x2": 3, "y2": 4}),
        json!({"type": "circle", "cx": 5, "cy": 6, "r": 7}),
        json!({"type": "arc", "cx": 8, "cy": 9, "r": 10,
               "start_deg": 0, "end_deg": 90, "ccw": true}),
    ]
}

/// Every item property the published `create_drawing` schema lists.
fn published_keys() -> Vec<String> {
    let schema = lasercad::agent::drawing::schema();
    let props = schema["properties"]["entities"]["items"]["properties"]
        .as_object()
        .expect("item properties object");
    props.keys().cloned().collect()
}

/// LCV-185 AC 1 — a foreign key set to `null` is ignored; the item is
/// validated and drawn by its own type.
#[test]
fn lcv185_ac1_a_null_foreign_key_is_ignored() {
    let cases = [
        ("line", "r"),
        ("circle", "start_deg"),
        ("arc", "x1"),
        ("line", "ccw"),
    ];
    for (kind, foreign) in cases {
        let mut item = own_items().into_iter().find(|i| i["type"] == kind).unwrap();
        item[foreign] = Value::Null;
        let entities = drawn(json!([item]));
        assert_eq!(entities.len(), 1, "{kind} with {foreign}: null");
        let same_kind = matches!(
            (kind, &entities[0]),
            ("line", Entity::Line(_)) | ("circle", Entity::Circle(_)) | ("arc", Entity::Arc(_))
        );
        assert!(same_kind, "{kind}: {:?}", entities[0]);
    }
}

/// LCV-185 AC 2 — a foreign key with any non-null value refuses the batch,
/// naming the key, the item's type and the keys that type takes.
#[test]
fn lcv185_ac2_a_non_null_foreign_key_is_refused_with_the_type_keys() {
    let cases = [
        ("line", "r", json!(5), "a line takes x1, y1, x2, y2"),
        ("circle", "start_deg", json!(0), "a circle takes cx, cy, r"),
        (
            "arc",
            "x1",
            json!(1),
            "an arc takes cx, cy, r, start_deg, end_deg, ccw",
        ),
        ("line", "cx", json!(0), "a line takes x1, y1, x2, y2"),
        ("circle", "ccw", json!(false), "a circle takes cx, cy, r"),
    ];
    for (kind, foreign, value, takes) in cases {
        let mut item = own_items().into_iter().find(|i| i["type"] == kind).unwrap();
        item[foreign] = value.clone();
        let mut items = vec![own_items()[1].clone(), own_items()[0].clone()];
        items.push(item);
        assert_eq!(
            refusal(Value::Array(items)),
            format!(
                "create_drawing entities[2].{foreign}: not {} {kind} key; {takes}",
                &takes[..takes.find(' ').unwrap()]
            ),
            "{kind} with {foreign}: {value}"
        );
    }
    // The empty string on a foreign key is not null either.
    let mut circle = own_items()[1].clone();
    circle["x2"] = json!("");
    assert_eq!(
        refusal(json!([circle])),
        "create_drawing entities[0].x2: not a circle key; a circle takes cx, cy, r"
    );
}

/// LCV-185 AC 3 — a key no type publishes keeps the `unknown key` refusal,
/// even when its value is `null`.
#[test]
fn lcv185_ac3_an_unpublished_key_is_still_unknown() {
    for value in [json!(1), Value::Null] {
        let mut line = own_items()[0].clone();
        line["radius"] = value.clone();
        assert_eq!(
            refusal(json!([line])),
            "create_drawing entities[0].radius: unknown key",
            "radius: {value}"
        );
    }
}

/// LCV-185 AC 4 — for each type, an item setting every published property
/// (its own keys valid, every foreign key `null`) is accepted and draws that
/// one entity.
#[test]
fn lcv185_ac4_an_item_with_every_published_property_draws_one_entity() {
    let published = published_keys();
    assert!(published.len() >= 11, "control: {published:?}");
    for own in own_items() {
        let mut item = own.clone();
        for key in &published {
            if item.get(key).is_none() {
                item[key.as_str()] = Value::Null;
            }
        }
        assert_eq!(item.as_object().unwrap().len(), published.len());
        let entities = drawn(json!([item]));
        assert_eq!(entities.len(), 1, "{own}");
        match (own["type"].as_str().unwrap(), &entities[0]) {
            ("line", Entity::Line(l)) => {
                assert_eq!((l.p1, l.p2), (Vec2::new(1.0, 2.0), Vec2::new(3.0, 4.0)))
            }
            ("circle", Entity::Circle(c)) => {
                assert_eq!((c.center, c.r), (Vec2::new(5.0, 6.0), 7.0))
            }
            ("arc", Entity::Arc(a)) => {
                assert_eq!((a.center, a.r, a.ccw), (Vec2::new(8.0, 9.0), 10.0, true))
            }
            (kind, got) => panic!("{kind} drew {got:?}"),
        }
    }
}

/// LCV-185 AC 5 — every published item property belongs to at least one
/// type, and every type key is published.
#[test]
fn lcv185_ac5_the_published_properties_are_the_union_of_the_type_keys() {
    use lasercad::agent::drawing::ENTITY_KEYS;
    let mut union: Vec<String> = vec!["type".to_owned()];
    for (_, keys) in ENTITY_KEYS {
        union.extend(keys.iter().map(|k| (*k).to_owned()));
    }
    union.sort();
    union.dedup();
    let mut published = published_keys();
    published.sort();
    assert_eq!(published, union);
    let schema = lasercad::agent::drawing::schema();
    let types = &schema["properties"]["entities"]["items"]["properties"]["type"]["enum"];
    let names: Vec<&str> = ENTITY_KEYS.iter().map(|(t, _)| *t).collect();
    assert_eq!(types, &json!(names));
}

/// LCV-185 AC 4, 5 — each published property has the JSON type the validator
/// wants: `type` a string, `ccw` a boolean, every other key a number.
#[test]
fn lcv185_the_published_property_types_match_the_validator() {
    let schema = lasercad::agent::drawing::schema();
    let props = schema["properties"]["entities"]["items"]["properties"]
        .as_object()
        .expect("item properties object");
    for (key, prop) in props {
        let want = match key.as_str() {
            "type" => "string",
            "ccw" => "boolean",
            _ => "number",
        };
        assert_eq!(prop["type"], json!(want), "{key}");
    }
}

/// Every object key anywhere in `value`, depth first.
fn all_keys(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                out.push(k.clone());
                all_keys(v, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|v| all_keys(v, out)),
        _ => {}
    }
}

/// LCV-185 AC 6 — the published schema carries no union or closed-object
/// keyword anywhere (ADR 0010 §2).
#[test]
fn lcv185_ac6_the_schema_has_no_union_keywords() {
    let mut keys = Vec::new();
    all_keys(&lasercad::agent::drawing::schema(), &mut keys);
    assert!(keys.iter().any(|k| k == "enum"), "control: {keys:?}");
    for banned in ["oneOf", "anyOf", "allOf", "const", "additionalProperties"] {
        assert!(!keys.iter().any(|k| k == banned), "{banned} in the schema");
    }
}
