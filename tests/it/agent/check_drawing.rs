//! LCV-190 AC 9 — the agent's `check_drawing` tool answers the check report as
//! one read-only step: the result is the dock's lines, the revision does not
//! move, and the turn's fence stays armed for the mutations that follow.
//!
//! The turn is armed with `arm_turn` and fed hand-pushed `Act`s drained by
//! `poll_agent_rx` — no thread, no socket, no endpoint.
//!
//! ADR 0002 §A2: `App::default()` only. ADR 0005: no dialog is armed and no
//! test sends `Ctrl+O` / `Ctrl+S`. ADR 0006: no per-user path is injected.

use lasercad::agent::{AgentAction, AgentEvent, AgentOutcome};
use lasercad::app::{App, arm_turn, poll_agent_rx};
use lasercad::document::{CreateLine, check_drawing};
use lasercad::geometry::{Line, Vec2};
use std::sync::mpsc::{Sender, channel};

/// Push `action`, drain once, and return its answer.
fn answer(app: &mut App, tx: &Sender<AgentEvent>, action: AgentAction) -> AgentOutcome {
    let (reply, answer) = channel::<AgentOutcome>();
    tx.send(AgentEvent::Act { action, reply })
        .expect("the armed Receiver must still be on App");
    poll_agent_rx(app);
    answer.try_recv().expect("the Act was answered")
}

/// An open line on the bed, with a second line 0.2 mm past its end: two open
/// ends and one gap, so the report is not the clean one-liner.
fn open_drawing(app: &mut App) {
    for (a, b) in [((10.0, 10.0), (50.0, 10.0)), ((50.2, 10.0), (90.0, 10.0))] {
        app.commit(Box::new(CreateLine::new(Line::new(
            Vec2::new(a.0, a.1),
            Vec2::new(b.0, b.1),
        ))));
    }
}

/// AC 9 — the answer is the report's lines joined by newlines, it is one
/// step, and it moves neither the revision nor the history.
#[test]
fn check_drawing_answers_the_report_as_one_read_only_step() {
    let mut app = App::default();
    open_drawing(&mut app);
    let want = check_drawing(&app.document).lines().join("\n");
    assert!(want.contains("gap:"), "control: the drawing has findings");
    let tx = arm_turn(&mut app, "check it");
    let revision = app.history.revision();
    let undo = app.history.len();

    let got = answer(&mut app, &tx, AgentAction::CheckDrawing);

    assert_eq!(got, AgentOutcome::Ok(want));
    assert_eq!(app.agent.turn.tally.steps, 1, "one step");
    assert_eq!(app.agent.turn.tally.applied, 0, "nothing applied");
    assert_eq!(app.history.revision(), revision, "the revision stays");
    assert_eq!(app.history.len(), undo, "nothing committed");
}

/// AC 9 — a clean drawing answers the clean line.
#[test]
fn check_drawing_on_a_clean_drawing_answers_no_problems() {
    let mut app = App::default();
    let tx = arm_turn(&mut app, "check it");
    assert_eq!(
        answer(&mut app, &tx, AgentAction::CheckDrawing),
        AgentOutcome::Ok("CHECK: no problems found.".to_owned())
    );
}

/// AC 9 — the check does not trip the fence: a mutation after it in the same
/// turn applies.
#[test]
fn a_mutation_after_the_check_is_not_fenced() {
    let mut app = App::default();
    open_drawing(&mut app);
    let tx = arm_turn(&mut app, "check then draw");
    answer(&mut app, &tx, AgentAction::CheckDrawing);
    let count = app.document.entities.len();

    let line = AgentAction::CreateLine {
        layer: None,
        x1: 0.0,
        y1: 0.0,
        x2: 10.0,
        y2: 0.0,
    };
    let got = answer(&mut app, &tx, line);

    assert!(matches!(got, AgentOutcome::Ok(_)), "{got:?}");
    assert_eq!(app.document.entities.len(), count + 1);
    assert_eq!(app.agent.turn.tally.applied, 1);
}
