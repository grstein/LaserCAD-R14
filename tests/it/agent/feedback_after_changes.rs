//! LCV-195 — `Feedback after changes`: after a reply that changed the
//! drawing, the UI thread answers the loop's feedback ask from the live
//! document with its size and CHECK summary, outside the fence and the tally.
//!
//! The turn is armed with `arm_turn` and fed hand-pushed `Act`s drained by
//! `poll_agent_rx` — no thread, no socket, no endpoint.
//!
//! ADR 0002 §A2: `App::default()` only. ADR 0005: no dialog is armed and no
//! test sends `Ctrl+O` / `Ctrl+S`. ADR 0006: no per-user path is injected.

use lasercad::agent::{AgentAction, AgentEvent, AgentOutcome};
use lasercad::app::{App, arm_turn, poll_agent_rx};
use std::sync::mpsc::{Sender, channel};

/// Push `action`, drain once, and return its answer.
fn answer(app: &mut App, tx: &Sender<AgentEvent>, action: AgentAction) -> AgentOutcome {
    let (reply, answer) = channel::<AgentOutcome>();
    tx.send(AgentEvent::Act { action, reply })
        .expect("the armed Receiver must still be on App");
    poll_agent_rx(app);
    answer.try_recv().expect("the Act was answered")
}

fn line(x1: f64, y1: f64, x2: f64, y2: f64) -> AgentAction {
    AgentAction::CreateLine {
        x1,
        y1,
        x2,
        y2,
        layer: None,
    }
}

/// An app with the setting on and a turn armed.
fn fed_turn(on: bool) -> (App, Sender<AgentEvent>) {
    let mut app = App::default();
    app.settings.agent_feedback_after_changes = on;
    let tx = arm_turn(&mut app, "draw");
    (app, tx)
}

fn ok(text: &str) -> AgentOutcome {
    AgentOutcome::Ok(text.to_owned())
}

/// AC 1, AC 4, AC 5 — after a line, the exact sentence; a second ask with
/// nothing applied since adds nothing; neither ask is a step.
#[test]
fn after_a_line_feedback_is_its_size_and_check_once() {
    let (mut app, tx) = fed_turn(true);
    answer(&mut app, &tx, line(10.0, 10.0, 50.0, 30.0));

    assert_eq!(
        answer(&mut app, &tx, AgentAction::Feedback),
        ok("Drawing now: 1 entity, X 10.000..50.000 mm, Y 10.000..30.000 mm. CHECK: 2 open ends")
    );
    assert_eq!(answer(&mut app, &tx, AgentAction::Feedback), ok(""));
    answer(&mut app, &tx, AgentAction::QueryEntities);
    assert_eq!(
        answer(&mut app, &tx, AgentAction::Feedback),
        ok(""),
        "a read-only reply mutated nothing"
    );
    assert_eq!(app.agent.turn.tally.steps, 2, "the tool calls alone");
    assert_eq!(app.agent.turn.tally.applied, 1);
}

/// AC 6 — with the setting off, the ask adds nothing.
#[test]
fn with_the_setting_off_feedback_adds_nothing() {
    let (mut app, tx) = fed_turn(false);
    answer(&mut app, &tx, line(10.0, 10.0, 50.0, 30.0));
    assert_eq!(answer(&mut app, &tx, AgentAction::Feedback), ok(""));
    assert_eq!(app.agent.turn.tally.steps, 1);
}

/// AC 2 — a reply that empties the drawing gets the short sentence.
#[test]
fn an_emptied_drawing_reads_zero_entities() {
    let (mut app, tx) = fed_turn(true);
    answer(&mut app, &tx, line(10.0, 10.0, 50.0, 30.0));
    answer(&mut app, &tx, AgentAction::Feedback);
    answer(&mut app, &tx, AgentAction::Delete { index: 0 });
    assert_eq!(
        answer(&mut app, &tx, AgentAction::Feedback),
        ok("Drawing now: 0 entities. CHECK: no problems found.")
    );
}

/// AC 1 — two check kinds: their summary lines joined with `; `, and no
/// finding line.
#[test]
fn two_check_kinds_are_joined_with_a_semicolon() {
    let (mut app, tx) = fed_turn(true);
    answer(&mut app, &tx, line(10.0, 10.0, 50.0, 10.0));
    answer(&mut app, &tx, line(50.2, 10.0, 90.0, 10.0));
    assert_eq!(
        answer(&mut app, &tx, AgentAction::Feedback),
        ok(concat!(
            "Drawing now: 2 entities, X 10.000..90.000 mm, Y 10.000..10.000 mm. ",
            "CHECK: 2 open ends; CHECK: 1 gap"
        ))
    );
    assert_eq!(app.agent.turn.tally.steps, 2);
}
