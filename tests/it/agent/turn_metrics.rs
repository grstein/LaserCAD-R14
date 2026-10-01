//! LCV-193 — the turn's counts, kept UI-side in `app.agent.turn.tally` from the
//! `Act`s the frame loop answers, and the metrics note every turn ends with.
//!
//! The tests push `Act`s by hand onto a turn armed with `arm_turn` and drain
//! them with `poll_agent_rx` — no thread, no socket, no endpoint.
//!
//! ADR 0002 §A2: `App::default()` only. ADR 0005: no dialog is armed and no
//! test sends `Ctrl+O` / `Ctrl+S`. ADR 0006: no per-user path is injected.

use lasercad::agent::{AgentAction, AgentEvent, AgentOutcome, RefusedCalls, TurnMetrics};
use lasercad::app::{App, arm_turn, poll_agent_rx};
use lasercad::document::CreateLine;
use lasercad::geometry::{Line, Vec2};
use std::sync::mpsc::{Receiver, Sender, channel};

fn push_act(tx: &Sender<AgentEvent>, action: AgentAction) -> Receiver<AgentOutcome> {
    let (reply, answer) = channel::<AgentOutcome>();
    tx.send(AgentEvent::Act { action, reply })
        .expect("the armed Receiver must still be on App");
    answer
}

/// Push `action`, drain once, and return its answer.
fn answer(app: &mut App, tx: &Sender<AgentEvent>, action: AgentAction) -> AgentOutcome {
    let answer = push_act(tx, action);
    poll_agent_rx(app);
    answer.try_recv().expect("the Act was answered")
}

fn human_line(app: &mut App) {
    app.commit(Box::new(CreateLine::new(Line::new(
        Vec2::new(0.0, 50.0),
        Vec2::new(5.0, 50.0),
    ))));
}

fn line() -> AgentAction {
    AgentAction::CreateLine {
        layer: None,
        x1: 0.0,
        y1: 0.0,
        x2: 10.0,
        y2: 0.0,
    }
}

/// The reason the worker sends for a call repeating a refused one (LCV-192),
/// built by the worker's own `RefusedCalls`, not copied here.
fn repeat_reason(first: &str) -> String {
    let mut calls = RefusedCalls::default();
    let refusal = AgentOutcome::Refused(first.to_owned());
    calls.record("delete_entity", "{}", &refusal);
    calls.check("delete_entity", "{}").expect("a repeat")
}

/// AC 1 / AC 3 — `Replied` is answered `Ok`, writes no row and is not a
/// step; it counts one reply and its captures, and is answered the same
/// after the fence tripped, when a step `Act` is `Fenced`.
#[test]
fn a_reply_is_answered_ok_with_no_row_and_is_not_a_step() {
    let mut app = App::default();
    let tx = arm_turn(&mut app, "look");
    let rows = app.agent.chat.len();

    let ok = AgentOutcome::Ok(String::new());
    assert_eq!(
        answer(&mut app, &tx, AgentAction::Replied { captures: 2 }),
        ok
    );
    assert_eq!(app.agent.chat.len(), rows, "no transcript row");
    let want = TurnMetrics {
        captures: 2,
        replies: 1,
        ..TurnMetrics::default()
    };
    assert_eq!(app.agent.turn.tally, want);

    human_line(&mut app);
    assert!(matches!(
        answer(&mut app, &tx, line()),
        AgentOutcome::Fenced(_)
    ));
    assert!(app.agent.turn.fence.is_tripped());
    let rows = app.agent.chat.len();
    assert_eq!(
        answer(&mut app, &tx, AgentAction::Replied { captures: 0 }),
        ok
    );
    assert_eq!(app.agent.chat.len(), rows, "no row after the trip either");
    let want = TurnMetrics {
        steps: 1,
        refused: 1,
        captures: 2,
        replies: 2,
        ..TurnMetrics::default()
    };
    assert_eq!(app.agent.turn.tally, want);
    assert!(app.agent.busy, "a reply is no verdict");
}

/// AC 2 / AC 3 — step `Act`s: an applied line, a query, a refused delete, a
/// malformed call and an LCV-192 repeat count five steps, one applied, three
/// refused of which one repeated.
#[test]
fn step_acts_tally_applied_refused_and_repeated() {
    let mut app = App::default();
    let tx = arm_turn(&mut app, "draw and delete");

    assert!(matches!(answer(&mut app, &tx, line()), AgentOutcome::Ok(_)));
    assert!(matches!(
        answer(&mut app, &tx, AgentAction::QueryEntities),
        AgentOutcome::Ok(_)
    ));
    let first = match answer(&mut app, &tx, AgentAction::Delete { index: 7 }) {
        AgentOutcome::Refused(text) => text,
        other => panic!("expected a refusal, got {other:?}"),
    };
    let unknown = AgentAction::Malformed {
        tool: "draw_unicorn".into(),
        reason: "unknown tool: `draw_unicorn`".into(),
    };
    assert!(matches!(
        answer(&mut app, &tx, unknown),
        AgentOutcome::Refused(_)
    ));
    let repeat = AgentAction::Malformed {
        tool: "delete_entity".into(),
        reason: repeat_reason(&first),
    };
    assert!(matches!(
        answer(&mut app, &tx, repeat),
        AgentOutcome::Refused(_)
    ));

    let want = TurnMetrics {
        steps: 5,
        applied: 1,
        refused: 3,
        repeated: 1,
        ..TurnMetrics::default()
    };
    assert_eq!(app.agent.turn.tally, want);
}
