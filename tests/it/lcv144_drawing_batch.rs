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
use lasercad::agent::{parse_tool_call, AgentAction, AgentEvent, AgentOutcome};
use lasercad::app::{arm_turn, cancel_turn, App, AGENT_FENCE_REFUSAL};
use lasercad::document::{CreateLine, Entity, SelectionCommand};
use lasercad::geometry::{Line, Vec2};
use lasercad::io::svg::{export_svg, import_svg, Preset};
use serde_json::{json, Value};
use std::sync::mpsc::{channel, Receiver, Sender};

/// The existing SVG round-trip tolerance, mm
/// (`tests/it/lcv100_svg_orientation.rs::EXPORT_QUANTISATION_TOL`).
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

    let reason = "create_drawing entities[17].r: -3 is not a positive finite number";
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
/// non-default bed and preset appends in order and touches nothing else.
#[test]
fn ac5_a_mixed_batch_appends_in_order_and_touches_nothing_else() {
    let (ctx, mut app) = ctx_and_app();
    human_line(&mut app, 0.0);
    human_line(&mut app, 1.0);
    app.commit(Box::new(SelectionCommand::new(vec![1])));
    app.document.bed_mm = [300.0, 180.0];
    app.export_preset = Preset::Mark;
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
    assert_eq!(app.export_preset, Preset::Mark);
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
    tx.send(AgentEvent::Done("done".into())).unwrap();
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
    tx.send(AgentEvent::Done("done".into())).unwrap();
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

    let svg = export_svg(&app.document, Preset::Cut);
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
