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

/// Both canvas opt-ins, live.
fn allow(app: &mut App, allow: bool, supports: bool) {
    app.settings.agent_allow_canvas_capture = allow;
    app.settings.agent_model_supports_vision = supports;
}

const SUMMARY: &str =
    "Drawing now: 1 entity, X 10.000..50.000 mm, Y 10.000..30.000 mm. CHECK: 2 open ends";

/// AC 3, AC 5 — with both opt-ins on, the feedback observes: the summary,
/// then the drawing frame's capture text, and a PNG. Still not a step.
#[test]
fn with_both_opt_ins_feedback_observes_the_drawing() {
    let (mut app, tx) = fed_turn(true);
    allow(&mut app, true, true);
    answer(&mut app, &tx, line(10.0, 10.0, 50.0, 30.0));

    let got = answer(&mut app, &tx, AgentAction::Feedback);

    let AgentOutcome::Observed { text, png } = got else {
        panic!("expected Observed, got {got:?}");
    };
    let (summary, capture) = text.split_once('\n').expect("summary, then capture");
    assert_eq!(summary, SUMMARY);
    assert!(capture.starts_with("Canvas 1024×"), "{capture}");
    assert!(png.starts_with(b"\x89PNG"), "a PNG");
    assert_eq!(app.agent.turn.tally.steps, 1, "the line alone");
    assert_eq!(app.agent.turn.tally.applied, 1);
}

/// AC 3 — with either opt-in off, the feedback is the text alone.
#[test]
fn with_one_opt_in_off_feedback_is_text_only() {
    for (on_allow, on_supports) in [(true, false), (false, true)] {
        let (mut app, tx) = fed_turn(true);
        allow(&mut app, on_allow, on_supports);
        answer(&mut app, &tx, line(10.0, 10.0, 50.0, 30.0));
        assert_eq!(answer(&mut app, &tx, AgentAction::Feedback), ok(SUMMARY));
    }
}

/// AC 3 — an empty drawing gets no image.
#[test]
fn an_empty_drawing_gets_no_image() {
    let (mut app, tx) = fed_turn(true);
    allow(&mut app, true, true);
    answer(&mut app, &tx, line(10.0, 10.0, 50.0, 30.0));
    answer(&mut app, &tx, AgentAction::Delete { index: 0 });
    assert_eq!(
        answer(&mut app, &tx, AgentAction::Feedback),
        ok("Drawing now: 0 entities. CHECK: no problems found.")
    );
    assert_eq!(app.agent.turn.tally.steps, 2);
}
