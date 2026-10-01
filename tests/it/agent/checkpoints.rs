//! LCV-198 — agent turn checkpoints: `checkpoint {name}` records a mark in
//! the turn's open group, `rollback {name}` rewinds to it (ADR 0007 §D12).
//!
//! The tests push `Act`s by hand onto a turn armed with `arm_turn` and drain
//! them with `poll_agent_rx` — no thread, no socket, no endpoint.
//!
//! ADR 0002 §A2: `App::default()` only. ADR 0005: no dialog is armed and no
//! test sends `Ctrl+O` / `Ctrl+S`. ADR 0006: no per-user path is injected.

use lasercad::agent::{AgentAction, AgentEvent, AgentOutcome, SetOp};
use lasercad::app::{App, arm_turn, poll_agent_rx};
use lasercad::document::{CreateLine, Entity, EntityId, LayerId};
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

fn ok(app: &mut App, tx: &Sender<AgentEvent>, action: AgentAction) -> String {
    match answer(app, tx, action) {
        AgentOutcome::Ok(text) => text,
        other => panic!("expected Ok, got {other:?}"),
    }
}

fn checkpoint(name: &str) -> AgentAction {
    AgentAction::Checkpoint { name: name.into() }
}

fn rollback(name: &str) -> AgentAction {
    AgentAction::Rollback { name: name.into() }
}

fn agent_line(y: f64) -> AgentAction {
    AgentAction::CreateLine {
        x1: 0.0,
        y1: y,
        x2: 10.0,
        y2: y,
        layer: None,
    }
}

fn by_id(id: EntityId, op: SetOp) -> AgentAction {
    AgentAction::ById {
        ids: vec![id.0],
        op,
    }
}

/// Three human lines, committed before any turn: e1, e2, e3.
fn fixture() -> App {
    let mut app = App::default();
    for y in [10.0, 20.0, 30.0] {
        let line = Line::new(Vec2::new(0.0, y), Vec2::new(5.0, y));
        app.commit(Box::new(CreateLine::new(line)));
    }
    app
}

/// Every entity with its layer and id, in order.
fn snapshot(app: &App) -> Vec<(Entity, Option<LayerId>, Option<EntityId>)> {
    let doc = &app.document;
    (0..doc.entity_count())
        .map(|i| (doc.entities[i], doc.entity_layer(i), doc.entity_id(i)))
        .collect()
}

fn end_turn(app: &mut App, tx: &Sender<AgentEvent>) {
    tx.send(AgentEvent::done("done")).expect("armed");
    poll_agent_rx(app);
    assert!(!app.agent.busy, "the turn ended");
}

/// AC 1 — `checkpoint` answers with its mark as one step, and moves neither
/// the drawing nor the revision.
#[test]
fn a_checkpoint_is_one_step_that_changes_nothing() {
    let mut app = fixture();
    let tx = arm_turn(&mut app, "try");
    ok(&mut app, &tx, agent_line(0.0));
    let (before, rev) = (snapshot(&app), app.history.revision());
    let steps = app.agent.turn.tally.steps;

    let got = ok(&mut app, &tx, checkpoint("a"));

    assert_eq!(got, "Checkpoint a set at 1 changes.");
    assert_eq!((snapshot(&app), app.history.revision()), (before, rev));
    assert_eq!(app.agent.turn.tally.steps, steps + 1);
}

/// AC 2 — a rollback reverts the create → move → delete after the
/// checkpoint, newest first, and says how much it undid.
#[test]
fn a_rollback_reverts_every_later_change_in_reverse_order() {
    let mut app = fixture();
    let tx = arm_turn(&mut app, "try");
    ok(&mut app, &tx, agent_line(0.0));
    ok(&mut app, &tx, checkpoint("a"));
    let at_a = snapshot(&app);
    ok(&mut app, &tx, agent_line(1.0));
    let new = app.document.entity_id(4).expect("the new line has an id");
    ok(&mut app, &tx, by_id(new, SetOp::Move { dx: 3.0, dy: 3.0 }));
    ok(&mut app, &tx, by_id(new, SetOp::Delete));

    let got = ok(&mut app, &tx, rollback("a"));

    assert_eq!(got, "Rolled back to a: 3 changes undone, 4 entities.");
    assert_eq!(snapshot(&app), at_a);
}

/// AC 3 — an entity moved and then deleted after the checkpoint comes back
/// at its old index and geometry, with its old id.
#[test]
fn a_rollback_restores_e2_with_its_old_id() {
    let mut app = fixture();
    let e2 = app.document.entity_id(1).expect("e2");
    let tx = arm_turn(&mut app, "try");
    ok(&mut app, &tx, checkpoint("a"));
    let at_a = snapshot(&app);
    ok(&mut app, &tx, by_id(e2, SetOp::Move { dx: 7.0, dy: 0.0 }));
    ok(&mut app, &tx, by_id(e2, SetOp::Delete));
    assert_eq!(app.document.index_of(e2), None, "control: e2 is gone");

    ok(&mut app, &tx, rollback("a"));

    assert_eq!(app.document.index_of(e2), Some(1));
    assert_eq!(snapshot(&app), at_a, "old geometry, layer and id");
}

/// AC 4 — `start` exists in every turn and reverts the whole turn; it
/// cannot be set by `checkpoint`.
#[test]
fn rolling_back_to_start_reverts_the_whole_turn() {
    let mut app = fixture();
    let at_start = snapshot(&app);
    let tx = arm_turn(&mut app, "try");
    let e1 = app.document.entity_id(0).expect("e1");
    ok(&mut app, &tx, agent_line(0.0));
    ok(&mut app, &tx, by_id(e1, SetOp::Delete));

    let refused = answer(&mut app, &tx, checkpoint("start"));
    assert!(refused.is_refused(), "{refused:?}");
    let got = ok(&mut app, &tx, rollback("start"));

    assert_eq!(got, "Rolled back to start: 2 changes undone, 3 entities.");
    assert_eq!(snapshot(&app), at_start);
}

/// AC 5 — a rollback keeps its target and forgets the checkpoints set after
/// it; setting an existing name moves it to the current position.
#[test]
fn a_rollback_keeps_its_target_and_a_name_can_move() {
    let mut app = fixture();
    let tx = arm_turn(&mut app, "try");
    ok(&mut app, &tx, checkpoint("a"));
    ok(&mut app, &tx, agent_line(0.0));
    ok(&mut app, &tx, checkpoint("b"));
    ok(&mut app, &tx, agent_line(1.0));
    ok(&mut app, &tx, rollback("a"));

    let gone = answer(&mut app, &tx, rollback("b"));
    assert!(gone.is_refused(), "b was set after a: {gone:?}");
    assert!(
        gone.text().ends_with("Known checkpoints: start, a."),
        "{gone:?}"
    );
    let kept = ok(&mut app, &tx, rollback("a"));
    assert_eq!(kept, "Rolled back to a: 0 changes undone, 3 entities.");

    ok(&mut app, &tx, agent_line(2.0));
    let moved = ok(&mut app, &tx, checkpoint("a"));
    assert_eq!(moved, "Checkpoint a set at 1 changes.");
    ok(&mut app, &tx, agent_line(3.0));
    let got = ok(&mut app, &tx, rollback("a"));
    assert_eq!(got, "Rolled back to a: 1 changes undone, 4 entities.");
}

/// AC 6 — the turn seals at most one undo entry holding only the survivors,
/// and none when nothing survived.
#[test]
fn the_turn_leaves_one_undo_entry_of_survivors_or_none() {
    let mut app = fixture();
    let before = (app.history.len(), snapshot(&app));
    let tx = arm_turn(&mut app, "try");
    ok(&mut app, &tx, agent_line(0.0));
    ok(&mut app, &tx, checkpoint("a"));
    ok(&mut app, &tx, agent_line(1.0));
    ok(&mut app, &tx, agent_line(2.0));
    ok(&mut app, &tx, rollback("a"));
    end_turn(&mut app, &tx);
    assert_eq!(app.history.len(), before.0 + 1, "one entry for the turn");
    let note = (
        "note".to_owned(),
        "Applied 1 action — Ctrl+Z undoes it.".to_owned(),
    );
    assert!(app.agent.chat.contains(&note), "{:?}", app.agent.chat);
    assert!(app.history.undo(&mut app.document));
    assert_eq!((app.history.len(), snapshot(&app)), before);

    let tx = arm_turn(&mut app, "again");
    ok(&mut app, &tx, agent_line(0.0));
    ok(&mut app, &tx, rollback("start"));
    end_turn(&mut app, &tx);
    assert_eq!((app.history.len(), snapshot(&app)), before, "no entry");
}

/// AC 7 — a malformed or unknown name is refused, changes nothing and
/// lists the known checkpoints.
#[test]
fn a_bad_or_unknown_name_is_refused_with_the_known_list() {
    let mut app = fixture();
    let tx = arm_turn(&mut app, "try");
    ok(&mut app, &tx, checkpoint("a"));
    ok(&mut app, &tx, agent_line(0.0));
    let (before, rev) = (snapshot(&app), app.history.revision());

    let long = "x".repeat(33);
    for action in [
        checkpoint(""),
        checkpoint("a b"),
        rollback(&long),
        rollback("zz"),
    ] {
        let got = answer(&mut app, &tx, action);
        assert!(got.is_refused(), "{got:?}");
        assert!(
            got.text().ends_with("Known checkpoints: start, a."),
            "{got:?}"
        );
    }
    assert_eq!((snapshot(&app), app.history.revision()), (before, rev));
    assert_eq!(app.history.group_len(), 1, "the group is untouched");
}

/// AC 8 — after an operator Undo both tools answer `Fenced`, and the
/// rollback rewinds nothing.
#[test]
fn an_operator_undo_fences_checkpoint_and_rollback() {
    let mut app = fixture();
    let tx = arm_turn(&mut app, "try");
    ok(&mut app, &tx, agent_line(0.0));
    ok(&mut app, &tx, checkpoint("a"));
    assert!(app.history.undo(&mut app.document), "the operator's Ctrl+Z");
    let after_undo = snapshot(&app);

    for action in [rollback("start"), checkpoint("b")] {
        let got = answer(&mut app, &tx, action);
        assert!(matches!(got, AgentOutcome::Fenced(_)), "{got:?}");
    }
    assert_eq!(snapshot(&app), after_undo);
}

/// AC 9 — a new turn knows no checkpoint from an earlier one but `start`.
#[test]
fn a_new_turn_knows_only_start() {
    let mut app = fixture();
    let tx = arm_turn(&mut app, "first");
    ok(&mut app, &tx, checkpoint("a"));
    end_turn(&mut app, &tx);

    let tx = arm_turn(&mut app, "second");
    let got = answer(&mut app, &tx, rollback("a"));
    assert!(got.is_refused(), "{got:?}");
    assert!(got.text().ends_with("Known checkpoints: start."), "{got:?}");
    let start = ok(&mut app, &tx, rollback("start"));
    assert_eq!(start, "Rolled back to start: 0 changes undone, 3 entities.");
}
