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

// ── AC 1 / AC 4: every turn ends with the metrics note ───────────────────

/// How a scripted turn ends.
#[derive(Debug, Clone, Copy)]
enum Exit {
    Done,
    Failed,
    Lost,
    Cancel,
    FenceStopped,
}

/// Arm a turn, answer one reply and one applied line (plus, for a
/// fence-stopped turn, a foreign commit and a fenced step), end it by `exit`
/// and return the transcript rows after the user row.
fn ended(exit: Exit) -> Vec<(String, String)> {
    let mut app = App::default();
    let tx = arm_turn(&mut app, "draw a line");
    let user = app.agent.chat.len();
    answer(&mut app, &tx, AgentAction::Replied { captures: 0 });
    answer(&mut app, &tx, line());
    match exit {
        Exit::Done => tx.send(AgentEvent::done("done")).unwrap(),
        Exit::Failed => tx.send(AgentEvent::failed("transport error: 503")).unwrap(),
        Exit::Lost => drop(tx),
        Exit::Cancel => lasercad::app::cancel_turn(&mut app),
        Exit::FenceStopped => {
            human_line(&mut app);
            answer(&mut app, &tx, line());
            let stop = "the turn stopped after the drawing changed outside it";
            tx.send(AgentEvent::failed(stop)).unwrap();
        }
    }
    poll_agent_rx(&mut app);
    assert!(!app.agent.busy, "{exit:?}: the turn ended");
    app.agent.chat[user..].to_vec()
}

/// AC 1 — Done, Failed, lost, cancelled and fence-stopped turns each end
/// with the metrics note as their last row, right after the undo note.
#[test]
fn every_exit_ends_with_the_metrics_note_after_the_undo_note() {
    let clean = "Applied 1 action — Ctrl+Z undoes it.";
    let foreign = "Applied 1 action before the drawing changed outside this turn.";
    for exit in [
        Exit::Done,
        Exit::Failed,
        Exit::Lost,
        Exit::Cancel,
        Exit::FenceStopped,
    ] {
        let rows = ended(exit);
        let (undo, metrics) = match exit {
            Exit::FenceStopped => (
                foreign,
                "Turn: 2 steps, 1 actions applied, 1 refused (0 repeated), \
                 0 captures sent, 1 model replies.",
            ),
            _ => (
                clean,
                "Turn: 1 steps, 1 actions applied, 0 refused (0 repeated), \
                 0 captures sent, 1 model replies.",
            ),
        };
        let n = rows.len();
        assert!(n >= 2, "{exit:?}: {rows:?}");
        let tail = [rows[n - 2].clone(), rows[n - 1].clone()];
        let want = [
            ("note".to_owned(), undo.to_owned()),
            ("note".to_owned(), metrics.to_owned()),
        ];
        assert_eq!(tail, want, "{exit:?}: {rows:?}");
    }
}

/// AC 4 — a turn that applied nothing still gets the metrics note, with
/// zero counts, and no undo note.
#[test]
fn a_turn_that_applied_nothing_gets_the_zero_count_note_only() {
    let mut app = App::default();
    let tx = arm_turn(&mut app, "hello");
    let user = app.agent.chat.len();
    tx.send(AgentEvent::done("hi")).unwrap();
    poll_agent_rx(&mut app);
    let rows = app.agent.chat[user..].to_vec();
    let zero = "Turn: 0 steps, 0 actions applied, 0 refused (0 repeated), \
                0 captures sent, 0 model replies.";
    assert_eq!(
        rows,
        [
            ("assistant".to_owned(), "hi".to_owned()),
            ("note".to_owned(), zero.to_owned()),
        ]
    );
}
