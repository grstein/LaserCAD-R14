//! LCV-142 AC 6 / 7 / 8 / 10 / 12 — one agent turn is one flat history group
//! (ADR 0007 §D12), the fence has two witnesses and stops the turn (§D14), and
//! a malformed call is an ordinary `Act` (§D15).
//!
//! **No test in this file reaches an endpoint.** Every turn is armed with
//! `arm_turn` and fed by hand-pushed `AgentEvent`s, the worker's half of ADR
//! 0007 §D3 written out in the test.
//!
//! ADR 0002 §A2: `App::default()` only. ADR 0005: no dialog is armed and no
//! test sends `Ctrl+O` / `Ctrl+S`; the one file this suite opens is loaded
//! through `action_open_path` from a tempdir (ADR 0006), with `settings_path`
//! and `autosave_path` left at `None`.

use crate::harness;

use harness::{frame, tap};
use lasercad::agent::{AgentAction, AgentEvent, AgentOutcome};
use lasercad::app::{arm_turn, cancel_turn, App, AGENT_FENCE_REFUSAL};
use lasercad::document::{CreateCircle, CreateLine, DeleteEntities, SelectionCommand};
use lasercad::geometry::{Circle, Line, Vec2};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};

// ── Plumbing ────────────────────────────────────────────────────────────────

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

fn agent_line(x2: f64) -> AgentAction {
    AgentAction::CreateLine {
        x1: 0.0,
        y1: 0.0,
        x2,
        y2: 0.0,
    }
}

fn human_line(app: &mut App, y: f64) {
    app.commit(Box::new(CreateLine::new(Line::new(
        Vec2::new(0.0, y),
        Vec2::new(5.0, y),
    ))));
}

/// Linux Ctrl as egui reports it: `ctrl` and `command` both set (see
/// `tests/it/agent/turn.rs`, which pins why the bare constant is a no-op).
fn ctrl() -> egui::Modifiers {
    egui::Modifiers {
        ctrl: true,
        command: true,
        ..egui::Modifiers::NONE
    }
}

fn notes(app: &App) -> Vec<&str> {
    app.agent
        .chat
        .iter()
        .filter(|(role, _)| role == "note")
        .map(|(_, text)| text.as_str())
        .collect()
}

fn tempdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lcv142_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A minimal, valid, test-owned SVG fixture: one line on a 200×200 mm bed.
const VALID_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200" viewBox="0 0 200 200">
<line x1="10" y1="10" x2="150" y2="10" stroke="#ff0000" stroke-width="0.1"/>
</svg>"##;

const FENCED: fn() -> AgentOutcome = || AgentOutcome::Fenced(AGENT_FENCE_REFUSAL.to_owned());

// ── AC 6: more than 200 steps undo as one ───────────────────────────────────

/// AC 6 — 200 human entries, then a 300-mutation turn: `len() == 200`, one
/// Ctrl+Z removes all 300, one Ctrl+Y re-applies them, and of the 200 human
/// entries exactly the oldest was evicted.
#[test]
fn ac6_a_300_step_turn_over_200_human_entries_undoes_as_one() {
    let (ctx, mut app) = ctx_and_app();
    for i in 0..200 {
        human_line(&mut app, f64::from(i));
    }
    assert_eq!(app.history.len(), 200);

    let tx = arm_turn(&mut app, "draw 300 lines");
    let answers: Vec<_> = (1..=300)
        .map(|i| push_act(&tx, agent_line(f64::from(i))))
        .collect();
    idle(&ctx, &mut app);
    assert!(answers.iter().all(|a| !a.try_recv().unwrap().is_refused()));
    tx.send(AgentEvent::done("done")).unwrap();
    idle(&ctx, &mut app);
    assert!(!app.agent.busy);

    assert_eq!(app.document.entity_count(), 500);
    assert_eq!(app.history.len(), 200, "199 human entries + the turn");

    tap(&ctx, &mut app, egui::Key::Z, ctrl());
    assert_eq!(
        app.document.entity_count(),
        200,
        "one Ctrl+Z removes all 300"
    );
    tap(&ctx, &mut app, egui::Key::Y, ctrl());
    assert_eq!(
        app.document.entity_count(),
        500,
        "one Ctrl+Y re-applies all"
    );
    tap(&ctx, &mut app, egui::Key::Z, ctrl());
    assert_eq!(app.document.entity_count(), 200);

    for _ in 0..199 {
        assert!(app.history.undo(&mut app.document));
    }
    assert!(
        !app.history.undo(&mut app.document),
        "exactly one human entry was evicted"
    );
    assert_eq!(app.document.entity_count(), 1);
    let Some(lasercad::document::Entity::Line(oldest)) = app.document.entities.first() else {
        panic!("the survivor must be a line: {:?}", app.document.entities);
    };
    assert_eq!(oldest.p1.y, 0.0, "the evicted entry is the oldest one");
}

// ── AC 7: sealing and chronology ────────────────────────────────────────────

/// AC 7 — a foreign create, delete and selection each seal the group before
/// touching the stack and trip the fence. The turn's work is one entry
/// directly beneath the foreign one: the first undo takes back only the
/// foreign event, the second takes back both agent lines at once.
#[test]
fn ac7_a_foreign_commit_seals_the_turn_beneath_it() {
    for foreign in ["create", "delete", "selection"] {
        let (ctx, mut app) = ctx_and_app();
        human_line(&mut app, 100.0);
        let base = app.history.len();
        let tx = arm_turn(&mut app, "two lines");
        let _ok = [10.0, 20.0].map(|x| push_act(&tx, agent_line(x)));
        idle(&ctx, &mut app);
        assert_eq!(app.document.entity_count(), 3, "{foreign}");

        match foreign {
            "create" => app.commit(Box::new(CreateCircle::new(Circle::new(
                Vec2::new(50.0, 50.0),
                5.0,
            )))),
            "delete" => app.commit(Box::new(DeleteEntities::new(vec![0]))),
            _ => app.commit(Box::new(SelectionCommand::new(vec![0]))),
        }
        assert!(!app.history.group_open(), "{foreign}: sealed");
        let after_foreign = app.document.entity_count();

        let late = [
            push_act(&tx, agent_line(30.0)),
            push_act(&tx, agent_line(40.0)),
        ];
        idle(&ctx, &mut app);
        for answer in &late {
            assert_eq!(answer.try_recv().unwrap(), FENCED(), "{foreign}: sticky");
        }
        assert_eq!(app.document.entity_count(), after_foreign, "{foreign}");
        tx.send(AgentEvent::done("stopped")).unwrap();
        idle(&ctx, &mut app);

        assert_eq!(app.history.len(), base + 2, "{foreign}: turn + foreign");
        assert!(app.history.undo(&mut app.document));
        assert_eq!(
            app.document.entity_count(),
            3,
            "{foreign}: foreign undone only"
        );
        assert!(app.history.undo(&mut app.document));
        assert_eq!(
            app.document.entity_count(),
            1,
            "{foreign}: both agent lines go together, the human line stays"
        );
    }
}

/// AC 7 — Undo mid-turn seals the group and then undoes it whole; Redo mid-turn
/// seals it too. Either way the fence trips and nothing later lands.
#[test]
fn ac7_undo_and_redo_mid_turn_seal_the_group_first() {
    for foreign in ["undo", "redo"] {
        let (ctx, mut app) = ctx_and_app();
        human_line(&mut app, 100.0);
        let base = app.history.len();
        let tx = arm_turn(&mut app, "two lines");
        let _ok = [10.0, 20.0].map(|x| push_act(&tx, agent_line(x)));
        idle(&ctx, &mut app);

        let key = if foreign == "undo" {
            egui::Key::Z
        } else {
            egui::Key::Y
        };
        tap(&ctx, &mut app, key, ctrl());
        assert!(!app.history.group_open(), "{foreign}: sealed");
        let expected = if foreign == "undo" { 1 } else { 3 };
        assert_eq!(app.document.entity_count(), expected, "{foreign}");

        let late = push_act(&tx, agent_line(30.0));
        idle(&ctx, &mut app);
        assert_eq!(late.try_recv().unwrap(), FENCED(), "{foreign}");
        assert_eq!(app.document.entity_count(), expected, "{foreign}");
        tx.send(AgentEvent::done("stopped")).unwrap();
        idle(&ctx, &mut app);

        if foreign == "undo" {
            assert!(app.history.redo(&mut app.document), "the turn is one redo");
            assert_eq!(app.document.entity_count(), 3);
        } else {
            assert_eq!(app.history.len(), base + 1);
            assert!(app.history.undo(&mut app.document));
            assert_eq!(app.document.entity_count(), 1, "one undo, both lines");
        }
    }
}

/// AC 7 — document replacement drops the group, so the next `Act` is `Fenced`
/// even on a turn armed at revision 0 on a freshly opened file, where the
/// revision alone would still read `0 == 0`.
#[test]
fn ac7_a_turn_armed_at_revision_zero_is_fenced_after_a_reopen() {
    for replacement in ["open", "new"] {
        let dir = tempdir(&format!("reopen_{replacement}"));
        let svg = dir.join("fresh.svg");
        std::fs::write(&svg, VALID_SVG).unwrap();
        let (ctx, mut app) = ctx_and_app();
        app.action_open_path(svg.clone());
        assert_eq!(app.error_message, None);
        assert_eq!(app.history.revision(), 0, "a freshly opened file");
        assert_eq!(app.document.entity_count(), 1);

        let tx = arm_turn(&mut app, "delete it");
        match replacement {
            "open" => app.action_open_path(svg),
            _ => app.action_new(),
        }
        assert_eq!(app.history.revision(), 0, "{replacement}: still 0 == 0");
        let count = app.document.entity_count();

        let answer = push_act(&tx, AgentAction::Delete { index: 0 });
        idle(&ctx, &mut app);
        assert_eq!(answer.try_recv().unwrap(), FENCED(), "{replacement}");
        assert_eq!(app.document.entity_count(), count, "{replacement}");
        tx.send(AgentEvent::done("stopped")).unwrap();
        idle(&ctx, &mut app);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

// ── AC 8: one finalization per exit ─────────────────────────────────────────

/// AC 8 — after 250 applied `Act`s, each of the five exits clears `busy`,
/// drops `rx`, writes exactly one note, and leaves one undo entry covering all
/// 250. Cancel is not rollback.
#[test]
fn ac8_every_exit_after_250_acts_finalizes_once() {
    for exit in ["done", "failed", "disconnected", "dead-reply", "cancel"] {
        let (ctx, mut app) = ctx_and_app();
        let base = app.history.len();
        let tx = arm_turn(&mut app, "draw 250 lines");
        let mut answers: Vec<_> = (1..=249)
            .map(|i| push_act(&tx, agent_line(f64::from(i))))
            .collect();
        let last = push_act(&tx, agent_line(250.0));
        if exit == "dead-reply" {
            drop(last);
        } else {
            answers.push(last);
        }
        match exit {
            "done" => tx.send(AgentEvent::done("ok")).unwrap(),
            "failed" => tx.send(AgentEvent::failed("boom")).unwrap(),
            "disconnected" => drop(tx),
            "dead-reply" => {}
            _ => {
                idle(&ctx, &mut app);
                cancel_turn(&mut app);
            }
        }
        idle(&ctx, &mut app);

        assert!(!app.agent.busy, "{exit}");
        assert!(app.agent.rx.is_none(), "{exit}");
        assert_eq!(app.document.entity_count(), 250, "{exit}: applied stays");
        // Cancel writes its own `note` row first (LCV-129); the undo note is
        // the one AC 8 counts, and it is written exactly once, last.
        let undo_notes: Vec<_> = notes(&app)
            .into_iter()
            .filter(|n| n.starts_with("Applied"))
            .collect();
        assert_eq!(
            undo_notes,
            ["Applied 250 actions — Ctrl+Z undoes the whole turn."],
            "{exit}: exactly one undo note"
        );
        assert_eq!(
            app.agent.chat.last().map(|(_, t)| t.as_str()),
            Some(undo_notes[0]),
            "{exit}"
        );
        assert_eq!(app.history.len(), base + 1, "{exit}");
        assert!(app.history.undo(&mut app.document));
        assert_eq!(app.document.entity_count(), 0, "{exit}: one undo, all 250");
    }
}

// ── AC 10: a malformed call is an ordinary `Act` ────────────────────────────

/// AC 10 — a `Malformed` `Act` answers `Refused(reason)`, leaves revision and
/// entities alone, adds one `refused` row, counts one step, and the turn goes
/// on; after a trip it is `Fenced` like anything else.
#[test]
fn ac10_a_malformed_act_is_refused_counted_and_then_fenced_after_a_trip() {
    let (ctx, mut app) = ctx_and_app();
    let tx = arm_turn(&mut app, "try something");
    let reason = "tool `create_line` missing required argument `x2`";
    let malformed = || AgentAction::Malformed {
        tool: "create_line".into(),
        reason: reason.into(),
    };
    let revision = app.history.revision();

    let answer = push_act(&tx, malformed());
    idle(&ctx, &mut app);
    assert_eq!(
        answer.try_recv().unwrap(),
        AgentOutcome::Refused(reason.into())
    );
    assert_eq!(app.history.revision(), revision);
    assert_eq!(app.document.entity_count(), 0);
    assert_eq!(
        app.agent.chat.last(),
        Some(&("refused".to_owned(), reason.to_owned()))
    );
    assert_eq!(app.agent.turn.steps, 1);
    assert!(app.agent.busy, "the turn continues");

    human_line(&mut app, 0.0);
    let answer = push_act(&tx, malformed());
    idle(&ctx, &mut app);
    assert_eq!(answer.try_recv().unwrap(), FENCED());
    assert_eq!(app.agent.turn.steps, 2);
}

// ── AC 12: the end-of-turn note comes from `end_group` ──────────────────────

/// AC 12 — a clean three-action turn gets the "whole turn" note; a turn sealed
/// by a foreign commit and a turn whose document was replaced get the neutral
/// one. The replacement case never trips the fence before the turn ends, so a
/// note derived from the fence would claim an undo that no longer exists.
#[test]
fn ac12_the_note_follows_the_seal_not_the_fence() {
    for case in ["clean", "foreign", "replaced"] {
        let (ctx, mut app) = ctx_and_app();
        let tx = arm_turn(&mut app, "three lines");
        let _ok = [10.0, 20.0, 30.0].map(|x| push_act(&tx, agent_line(x)));
        idle(&ctx, &mut app);
        match case {
            "foreign" => human_line(&mut app, 50.0),
            "replaced" => app.action_new(),
            _ => {}
        }
        tx.send(AgentEvent::done("done")).unwrap();
        idle(&ctx, &mut app);
        let expected = if case == "clean" {
            "Applied 3 actions — Ctrl+Z undoes the whole turn."
        } else {
            "Applied 3 actions before the drawing changed outside this turn."
        };
        assert_eq!(notes(&app), [expected], "{case}");
        if case == "replaced" {
            assert!(!app.agent.turn.fence.is_tripped(), "not from the fence");
        }
    }
}
