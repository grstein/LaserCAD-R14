//! LCV-123 — the agent turn on the operator's real drawing.
//!
//! Every test here drives a real `App` through real frames. All but the last
//! one does it **without a thread and without a socket**: `arm_turn` hands back
//! the `Sender` the worker would hold, so a test can push exactly the events it
//! wants, in the order it wants, and read the answers off its own reply
//! `Receiver`. That is what makes assertions about applying, fencing,
//! coalescing and ending a turn deterministic — there is no race to lose and no
//! sleep to tune (ADR 0008's no-fail-fast gate is about *other* tests flaking;
//! these cannot).
//!
//! The one exception is `a_whole_turn_lands_on_the_bed`, which runs a real
//! worker thread against a mockito socket. It earns its place: it is the only
//! thing in the suite that proves the CHANGELOG gap — *"the agent can read and
//! narrate the drawing but does not yet modify it end-to-end from chat"* — is
//! closed.
//!
//! ADR 0002 §A2: `App::default()` only, never `App::new()`. §A4: no test sends
//! `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S`, and every `App` here leaves
//! `settings_path` and `autosave_path` at `None`, so nothing is ever written to
//! a real per-user path (ADR 0006, ADR 0007 §D10).

use crate::harness;

use harness::{frame, tap};
use lasercad::agent::{AgentAction, AgentEvent, AgentOutcome};
use lasercad::app::{arm_turn, start_turn, App, AGENT_FENCE_REFUSAL, AGENT_LOST_MESSAGE};
use lasercad::document::{CreateCircle, SelectionCommand};
use lasercad::geometry::{Circle, Vec2};
use std::sync::mpsc::{channel, Receiver, Sender};

/// A recognisable key that must never appear anywhere the operator can read.
/// Built with `concat!` so a grep for the whole string finds no copy of it.
const DUMMY_KEY: &str = concat!("sk-test-", "DO-NOT-LEAK");

// ── Plumbing ────────────────────────────────────────────────────────────────

/// A headless egui context and an `App` with no injected paths.
fn ctx_and_app() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let app = App::default();
    assert!(
        app.settings_path.is_none(),
        "ADR 0006: no real settings path"
    );
    assert!(
        app.autosave_path.is_none(),
        "ADR 0006: no real autosave path"
    );
    (ctx, app)
}

/// One frame with no input events.
fn idle(ctx: &egui::Context, app: &mut App) {
    frame(ctx, app, Vec::new());
}

/// Push one `Act` and hand back the reply `Receiver` the test reads its answer
/// from. This is the worker's half of ADR 0007 §D3, written by hand.
fn push_act(tx: &Sender<AgentEvent>, action: AgentAction) -> Receiver<AgentOutcome> {
    let (reply, answer) = channel::<AgentOutcome>();
    tx.send(AgentEvent::Act { action, reply })
        .expect("the armed Receiver must still be on App");
    answer
}

fn line(x2: f64) -> AgentAction {
    AgentAction::CreateLine {
        layer: None,
        x1: 0.0,
        y1: 0.0,
        x2,
        y2: 0.0,
    }
}

/// The roles in `agent.chat`, in order — AC 23's total ordering, made visible.
fn roles(app: &App) -> Vec<&str> {
    app.agent.chat.iter().map(|(r, _)| r.as_str()).collect()
}

fn row(app: &App, index: usize) -> (&str, &str) {
    let (role, text) = &app.agent.chat[index];
    (role.as_str(), text.as_str())
}

/// `Ctrl` **as egui reports it on Linux**: `ctrl` and `command` both set.
///
/// `egui::Modifiers::CTRL` leaves `command` at `false`, and
/// `dispatch_shortcuts` gates Ctrl+Z / Ctrl+Y on `modifiers.command_only()` —
/// so a tap built from the bare constant is silently swallowed and every undo
/// assertion downstream reads as a broken coalesce. `the_ctrl_taps_use_the_shape_the_dispatcher_tests_for` pins that.
fn ctrl() -> egui::Modifiers {
    egui::Modifiers {
        ctrl: true,
        command: true,
        ..egui::Modifiers::NONE
    }
}

/// The guard for the trap [`ctrl`] documents: a modifier set the dispatcher
/// does not recognise makes every `tap` in this file a no-op that fails
/// somewhere else entirely.
#[test]
fn the_ctrl_taps_use_the_shape_the_dispatcher_tests_for() {
    assert!(
        ctrl().command_only(),
        "dispatch_shortcuts gates Ctrl+Z on command_only()"
    );
    assert!(
        !egui::Modifiers::CTRL.command_only(),
        "if this ever becomes true the helper above is redundant, not wrong"
    );
}

// ── AC 8: an action really lands on the operator's drawing ──────────────────

/// AC 8 — one `Act`, one frame, one entity on the real bed.
///
/// The coordinates are read back off `app.document`, not off the outcome
/// string, and the outcome is read back off the test's own reply receiver, so
/// nothing here can be satisfied by a fabricated success (mutation (f)).
#[test]
fn one_act_creates_the_exact_entity_on_the_live_document() {
    let (ctx, mut app) = ctx_and_app();
    let before_entities = app.document.entities.len();
    let before_revision = app.history.revision();

    let tx = arm_turn(&mut app, "draw a 20 mm line");
    let answer = push_act(
        &tx,
        AgentAction::CreateLine {
            layer: None,
            x1: 1.5,
            y1: 2.5,
            x2: 21.5,
            y2: 2.5,
        },
    );

    idle(&ctx, &mut app);

    assert_eq!(app.document.entities.len(), before_entities + 1);
    let lasercad::document::Entity::Line(drawn) = app.document.entities[0] else {
        panic!(
            "the agent must have drawn a line, got {:?}",
            app.document.entities[0]
        );
    };
    assert_eq!(drawn.p1, Vec2::new(1.5, 2.5), "exact millimetres");
    assert_eq!(drawn.p2, Vec2::new(21.5, 2.5), "exact millimetres");
    assert_eq!(app.history.revision(), before_revision + 1);
    assert!(
        app.history.can_undo(),
        "the agent's edit is the user's to undo"
    );

    let outcome = answer
        .try_recv()
        .expect("the worker must have been answered");
    assert!(!outcome.is_refused(), "{outcome:?}");
    assert!(
        outcome.text().contains("The drawing now has 1 entities"),
        "the answer carries the post-mutation count: {}",
        outcome.text()
    );
}

// ── AC 6, AC 7: the drain and the four exits ────────────────────────────────

/// AC 6 — one frame drains **everything** that is waiting, in order.
///
/// Two `Act`s and the terminal event are queued before a single frame runs.
/// Mutation (a) — a `try_recv` that handles one message per frame — leaves the
/// second line undrawn and the turn running, and fails here by name.
#[test]
fn one_frame_drains_every_queued_act_and_then_the_terminal_event() {
    let (ctx, mut app) = ctx_and_app();
    let tx = arm_turn(&mut app, "draw two lines");
    let first = push_act(&tx, line(10.0));
    let second = push_act(&tx, line(20.0));
    tx.send(AgentEvent::done("Drew two lines.")).unwrap();

    idle(&ctx, &mut app);

    assert_eq!(app.document.entities.len(), 2, "both actions, one frame");
    assert!(first.try_recv().is_ok(), "the first Act was answered");
    assert!(second.try_recv().is_ok(), "the second Act was answered");
    assert!(!app.agent.busy, "and the turn ended in the same frame");
    assert!(app.agent.rx.is_none());
    assert_eq!(roles(&app), ["user", "tool", "tool", "assistant", "note"]);
}

/// AC 7 — a frame that carried only `Act`s leaves the turn running. Mutation
/// (b) — clearing `agent.rx` after an `Act` — strands every later tool call
/// and fails here.
#[test]
fn a_frame_of_only_acts_leaves_the_turn_armed() {
    let (ctx, mut app) = ctx_and_app();
    let tx = arm_turn(&mut app, "draw one line");
    let _answer = push_act(&tx, line(10.0));

    idle(&ctx, &mut app);

    assert!(app.agent.busy, "an Act is not a verdict");
    assert!(app.agent.rx.is_some(), "the turn must survive the frame");
    assert_eq!(app.document.entities.len(), 1);

    // And the next frame still reaches the worker's next tool call.
    let _second = push_act(&tx, line(30.0));
    idle(&ctx, &mut app);
    assert_eq!(app.document.entities.len(), 2);
}

/// AC 7 exit (3) — the worker's `Sender` dropped without a verdict. The turn
/// must end anyway: `agent.busy` stuck true is a permanent repaint loop
/// (LCV-120, reopened). Mutation (c) — deleting the `Disconnected` arm — hangs
/// the turn and fails here.
#[test]
fn a_dropped_event_sender_ends_the_turn_with_the_lost_row() {
    let (ctx, mut app) = ctx_and_app();
    let tx = arm_turn(&mut app, "draw something");
    drop(tx);

    idle(&ctx, &mut app);

    assert!(
        !app.agent.busy,
        "a lost worker must not leave the app spinning"
    );
    assert!(app.agent.rx.is_none());
    assert_eq!(roles(&app), ["user", "error"]);
    assert_eq!(row(&app, 1), ("error", AGENT_LOST_MESSAGE));
}

/// AC 7 exit (4) — the worker vanished between asking and reading the answer.
///
/// The event `Sender` is deliberately kept alive, so no `Disconnected` can
/// arrive and the dead reply channel is the **only** thing that says anything
/// is wrong. Mutation (h) — falling through to the next `try_recv` instead of
/// returning — leaves `agent.busy` set and fails here.
///
/// The row is compared against the constant *and* against the row exit (3)
/// wrote, never against a copy of the sentence: two spellings of "the worker is
/// gone" must not show the operator two different things (ADR 0007 §D11).
/// Mutation (i) — ending this exit silently — fails that comparison.
#[test]
fn a_dead_reply_channel_ends_the_turn_with_the_same_row() {
    // Exit (3), for the row to be compared against.
    let (ctx, mut lost) = ctx_and_app();
    let tx = arm_turn(&mut lost, "draw something");
    drop(tx);
    idle(&ctx, &mut lost);
    let expected = lost.agent.chat[1].clone();

    // Exit (4).
    let (ctx, mut app) = ctx_and_app();
    let tx = arm_turn(&mut app, "draw something");
    let answer = push_act(&tx, line(10.0));
    drop(answer);

    idle(&ctx, &mut app);

    assert!(
        !app.agent.busy,
        "a worker that stopped listening must not spin"
    );
    assert!(app.agent.rx.is_none());
    assert_eq!(
        app.document.entities.len(),
        1,
        "the action was applied before the answer was lost, and it stays applied"
    );
    assert!(app.history.can_undo(), "and it stays undoable");
    assert_eq!(roles(&app), ["user", "tool", "error", "note"]);
    assert_eq!(
        app.agent.chat[2], expected,
        "exit (4) reports the same fact as exit (3), so it writes the same row"
    );
    assert_eq!(row(&app, 2), ("error", AGENT_LOST_MESSAGE));

    // The event Sender really was still alive — nothing else could have
    // ended this turn.
    assert!(
        tx.send(AgentEvent::done("late")).is_err(),
        "the channel is closed only because the turn ended, not before it"
    );
}

/// AC 7 exit (2) — a worker error is the operator's to read, with role
/// `error`, and it ends the turn like any other exit.
#[test]
fn a_failed_turn_reports_the_workers_error() {
    let (ctx, mut app) = ctx_and_app();
    let tx = arm_turn(&mut app, "draw something");
    tx.send(AgentEvent::failed("HTTP 401")).unwrap();

    idle(&ctx, &mut app);

    assert_eq!(roles(&app), ["user", "error"]);
    assert_eq!(row(&app, 1), ("error", "HTTP 401"));
    assert!(!app.agent.busy);
    assert!(app.agent.rx.is_none());
}

// ── AC 9: the fence ─────────────────────────────────────────────────────────

/// AC 9 — a foreign commit mid-turn aborts every later action, and the fence
/// stays tripped. Run twice: once with a `CreateCircle`, once with a
/// `SelectionCommand`, which bumps the revision just the same and must abort
/// just the same (ADR 0007 §Risks — do not "fix" this by exempting selection).
///
/// Mutation (d) — a fence check that always returns `Ok` — fails here.
#[test]
fn a_foreign_commit_mid_turn_refuses_every_later_action() {
    for foreign in ["circle", "selection"] {
        let (ctx, mut app) = ctx_and_app();
        let tx = arm_turn(&mut app, "draw three lines");

        let first = push_act(&tx, line(10.0));
        idle(&ctx, &mut app);
        assert!(!first.try_recv().unwrap().is_refused(), "{foreign}");
        assert_eq!(app.document.entities.len(), 1, "{foreign}");

        // The operator does something of their own, mid-turn.
        match foreign {
            "circle" => app.commit(Box::new(CreateCircle::new(Circle::new(
                Vec2::new(50.0, 50.0),
                5.0,
            )))),
            _ => app.commit(Box::new(SelectionCommand::new(vec![0]))),
        }
        let after_foreign = app.document.entities.len();

        let second = push_act(&tx, line(20.0));
        let third = push_act(&tx, line(30.0));
        idle(&ctx, &mut app);

        assert_eq!(
            second.try_recv().unwrap(),
            AgentOutcome::Fenced(AGENT_FENCE_REFUSAL.to_owned()),
            "{foreign}: the fence must refuse in the ADR 0007 §D4 wording, as \
             `Fenced` (§D14)"
        );
        assert_eq!(
            third.try_recv().unwrap(),
            AgentOutcome::Fenced(AGENT_FENCE_REFUSAL.to_owned()),
            "{foreign}: the trip is sticky — every later action is refused too"
        );
        assert_eq!(
            app.document.entities.len(),
            after_foreign,
            "{foreign}: nothing the agent asked for after the trip may land"
        );

        // The turn still ends normally when the terminal event arrives, and
        // the first action stays applied and undoable.
        tx.send(AgentEvent::done("I stopped.")).unwrap();
        idle(&ctx, &mut app);
        assert!(!app.agent.busy, "{foreign}");
        assert!(app.history.can_undo(), "{foreign}");
        assert!(
            app.history.undo(&mut app.document),
            "{foreign}: the first action is still the operator's to take back"
        );
    }
}

/// AC 23 (b) — a refused action's row is role `refused`, content the refusal
/// verbatim, and the applied action's `tool` row comes first. Mutation (k) —
/// writing `tool` for a refusal — fails here.
#[test]
fn a_fence_refusal_is_transcribed_as_refused_after_the_applied_row() {
    let (ctx, mut app) = ctx_and_app();
    let tx = arm_turn(&mut app, "draw two lines");
    let applied = push_act(&tx, line(10.0));
    idle(&ctx, &mut app);
    app.commit(Box::new(CreateCircle::new(Circle::new(
        Vec2::new(50.0, 50.0),
        5.0,
    ))));
    let refused = push_act(&tx, line(20.0));
    tx.send(AgentEvent::done("I stopped.")).unwrap();
    idle(&ctx, &mut app);

    assert_eq!(
        roles(&app),
        ["user", "tool", "refused", "assistant", "note"]
    );
    assert_eq!(row(&app, 1).1, applied.try_recv().unwrap().text());
    assert_eq!(row(&app, 2).1, refused.try_recv().unwrap().text());
    assert_eq!(row(&app, 2).1, AGENT_FENCE_REFUSAL);
}

// ── AC 10, AC 11, AC 22: one turn, one undo entry ───────────────────────────

/// AC 10 / AC 11 — four actions seal into one undo entry, `revision()` does
/// not move across the seal, one `Ctrl+Z` reverses the whole turn and one
/// `Ctrl+Y` reapplies it in order. Since LCV-142 (ADR 0007 §D12) the turn's
/// work sits in an open group that already counts as one entry mid-turn.
///
/// `revision()` not moving is deliberate (ADR 0007 §Risks): the autosave
/// debounce watches it, and an autosave that fired mid-turn already wrote the
/// right bytes.
#[test]
fn a_four_action_turn_is_one_undo_entry() {
    let (ctx, mut app) = ctx_and_app();
    let stack_before = app.history.len();
    let tx = arm_turn(&mut app, "draw a 20 mm square at the origin");
    // The receivers are **kept**: dropping one is exit (4) (ADR 0007 §D11), not
    // a worker patiently waiting for its answer.
    let _answers: Vec<_> = [10.0, 20.0, 30.0, 40.0]
        .map(|x| push_act(&tx, line(x)))
        .into_iter()
        .collect();
    idle(&ctx, &mut app);
    assert_eq!(
        app.history.len(),
        stack_before + 1,
        "LCV-142: the open group counts as one entry"
    );
    let revision_before_fold = app.history.revision();

    tx.send(AgentEvent::done("Drew it.")).unwrap();
    idle(&ctx, &mut app);

    assert_eq!(
        app.history.len(),
        stack_before + 1,
        "AC 10: one turn, one undo entry"
    );
    assert_eq!(
        app.history.revision(),
        revision_before_fold,
        "AC 10: the seal reshapes the undo stack, it does not commit"
    );
    assert_eq!(
        roles(&app),
        ["user", "tool", "tool", "tool", "tool", "assistant", "note"]
    );
    assert_eq!(
        row(&app, 6),
        ("note", "Applied 4 actions — Ctrl+Z undoes the whole turn.")
    );

    // One Ctrl+Z, and the bed is empty again.
    tap(&ctx, &mut app, egui::Key::Z, ctrl());
    assert!(
        app.document.entities.is_empty(),
        "one undo reverses the turn"
    );
    // One Ctrl+Y, and all four are back, in order.
    tap(&ctx, &mut app, egui::Key::Y, ctrl());
    assert_eq!(app.document.entities.len(), 4);
    let xs: Vec<f64> = app
        .document
        .entities
        .iter()
        .map(|e| match e {
            lasercad::document::Entity::Line(l) => l.p2.x,
            other => panic!("expected lines, got {other:?}"),
        })
        .collect();
    assert_eq!(xs, [10.0, 20.0, 30.0, 40.0], "redo reapplies them in order");
}

/// LCV-142 AC 7 / AC 12 — a foreign commit seals the turn's group, so the turn
/// is still one entry, directly beneath the foreign one, and the note makes no
/// undo claim (ADR 0007 §D12 retired the "separate undo steps" shape).
#[test]
fn a_fence_aborted_turn_is_one_entry_beneath_the_foreign_one() {
    let (ctx, mut app) = ctx_and_app();
    let stack_before = app.history.len();
    let tx = arm_turn(&mut app, "draw two lines");
    let _answers = [push_act(&tx, line(10.0)), push_act(&tx, line(20.0))];
    idle(&ctx, &mut app);

    // A foreign commit lands after both actions, so the count is 2 and the
    // fence is tripped without any action having been refused.
    app.commit(Box::new(CreateCircle::new(Circle::new(
        Vec2::new(50.0, 50.0),
        5.0,
    ))));
    assert_eq!(app.history.len(), stack_before + 2, "turn, then circle");

    tx.send(AgentEvent::done("Drew them.")).unwrap();
    idle(&ctx, &mut app);

    assert_eq!(app.history.len(), stack_before + 2);
    assert_eq!(
        app.agent.chat.last().map(|(r, t)| (r.as_str(), t.as_str())),
        Some((
            "note",
            "Applied 2 actions before the drawing changed outside this turn."
        ))
    );
    tap(&ctx, &mut app, egui::Key::Z, ctrl());
    assert_eq!(app.document.entities.len(), 2, "the circle went first");
    tap(&ctx, &mut app, egui::Key::Z, ctrl());
    assert!(app.document.entities.is_empty(), "then the whole turn");
}

/// AC 11 — a one-action turn gets its own sentence, and a turn that applied
/// nothing gets no note row at all.
#[test]
fn one_action_and_zero_action_turns_say_their_own_thing() {
    let (ctx, mut app) = ctx_and_app();
    let tx = arm_turn(&mut app, "draw one line");
    let _answer = push_act(&tx, line(10.0));
    tx.send(AgentEvent::done("Drew it.")).unwrap();
    idle(&ctx, &mut app);
    assert_eq!(roles(&app), ["user", "tool", "assistant", "note"]);
    assert_eq!(
        row(&app, 3),
        ("note", "Applied 1 action — Ctrl+Z undoes it.")
    );

    // A turn made only of queries applies nothing (AC 16).
    let (ctx, mut app) = ctx_and_app();
    let stack_before = app.history.len();
    let revision_before = app.history.revision();
    let tx = arm_turn(&mut app, "what is on the bed?");
    let _answers = [
        push_act(&tx, AgentAction::QueryEntities),
        push_act(&tx, AgentAction::QuerySelection),
    ];
    tx.send(AgentEvent::done("Nothing yet.")).unwrap();
    idle(&ctx, &mut app);

    assert_eq!(roles(&app), ["user", "tool", "tool", "assistant"]);
    assert!(
        !roles(&app).contains(&"note"),
        "AC 11: zero applied actions means no note row"
    );
    assert_eq!(
        app.history.len(),
        stack_before,
        "AC 16: a query commits nothing"
    );
    assert_eq!(app.history.revision(), revision_before);
}

/// AC 22 — the turn-end work is the same on the two *lost* exits, not only on
/// `Done`. Three applied actions, ended by a dropped sender and by a dead reply
/// channel: both seal to one undo entry, both write the note row, and in both
/// the note comes **after** the error row. Mutation (j) — skipping the seal
/// on these exits — fails here.
#[test]
fn a_lost_turn_still_coalesces_and_still_says_so() {
    for exit in ["disconnected", "dead-reply"] {
        let (ctx, mut app) = ctx_and_app();
        let stack_before = app.history.len();
        let tx = arm_turn(&mut app, "draw three lines");

        match exit {
            "disconnected" => {
                let _answers = [10.0, 20.0, 30.0].map(|x| push_act(&tx, line(x)));
                idle(&ctx, &mut app);
                drop(tx);
                idle(&ctx, &mut app);
            }
            _ => {
                let _answers = [10.0, 20.0].map(|x| push_act(&tx, line(x)));
                idle(&ctx, &mut app);
                // The third action is applied, and then its answer is lost.
                let answer = push_act(&tx, line(30.0));
                drop(answer);
                idle(&ctx, &mut app);
            }
        }

        assert_eq!(app.document.entities.len(), 3, "{exit}");
        assert_eq!(
            app.history.len(),
            stack_before + 1,
            "{exit}: AC 22 — a lost turn gets the same undo shape as a clean one"
        );
        let last_two: Vec<&str> = roles(&app).into_iter().rev().take(2).rev().collect();
        assert_eq!(last_two, ["error", "note"], "{exit}: AC 22 row order");
        assert_eq!(
            app.agent.chat.last().map(|(_, t)| t.as_str()),
            Some("Applied 3 actions — Ctrl+Z undoes the whole turn."),
            "{exit}: including the action whose answer was lost"
        );

        tap(&ctx, &mut app, egui::Key::Z, ctrl());
        assert!(
            app.document.entities.is_empty(),
            "{exit}: one Ctrl+Z empties what the turn drew"
        );
    }
}

/// AC 22 / LCV-142 AC 12 — a fence-aborted turn ended by a dropped sender is
/// still one entry beneath the foreign one, and gets the neutral sentence.
#[test]
fn a_lost_fence_aborted_turn_is_still_one_entry() {
    let (ctx, mut app) = ctx_and_app();
    let stack_before = app.history.len();
    let tx = arm_turn(&mut app, "draw three lines");
    let _answers = [10.0, 20.0, 30.0].map(|x| push_act(&tx, line(x)));
    idle(&ctx, &mut app);
    app.commit(Box::new(CreateCircle::new(Circle::new(
        Vec2::new(50.0, 50.0),
        5.0,
    ))));
    drop(tx);
    idle(&ctx, &mut app);

    assert_eq!(app.history.len(), stack_before + 2, "turn, then circle");
    assert_eq!(
        app.agent.chat.last().map(|(r, t)| (r.as_str(), t.as_str())),
        Some((
            "note",
            "Applied 3 actions before the drawing changed outside this turn."
        ))
    );
}

// ── AC 23: the transcript ───────────────────────────────────────────────────

/// AC 23 (a) — the whole ordering of a mixed turn, and the content of every
/// `tool` row read back off the test's own reply receivers.
///
/// Mutation (l) — appending a row only for mutating actions — drops the query's
/// row and fails on the role sequence.
#[test]
fn the_transcript_holds_one_verbatim_row_per_action_in_order() {
    let (ctx, mut app) = ctx_and_app();
    let tx = arm_turn(&mut app, "draw two lines and tell me what is there");
    let first = push_act(&tx, line(10.0));
    let second = push_act(&tx, line(20.0));
    let query = push_act(&tx, AgentAction::QueryEntities);
    tx.send(AgentEvent::done("Two lines.")).unwrap();

    idle(&ctx, &mut app);

    assert_eq!(
        roles(&app),
        ["user", "tool", "tool", "tool", "assistant", "note"],
        "AC 23: user, one row per action in apply order, terminal, note"
    );
    assert_eq!(
        row(&app, 0),
        ("user", "draw two lines and tell me what is there")
    );
    assert_eq!(row(&app, 1).1, first.try_recv().unwrap().text());
    assert_eq!(row(&app, 2).1, second.try_recv().unwrap().text());
    let listing = query.try_recv().unwrap();
    assert_eq!(row(&app, 3).1, listing.text());
    assert!(
        listing.text().starts_with("The drawing has 2 entities."),
        "the query answered from the live document: {}",
        listing.text()
    );
    assert_eq!(row(&app, 4), ("assistant", "Two lines."));
    assert_eq!(
        row(&app, 5),
        ("note", "Applied 2 actions — Ctrl+Z undoes the whole turn."),
        "AC 23: the query is not an applied action, so the count is 2, not 3"
    );
}

// ── AC 12: the operator is never blocked ────────────────────────────────────

/// AC 12 — a user command committed while an `Act` is outstanding succeeds, and
/// no modal is up. Drawing during a turn is allowed; it ends the turn, which is
/// AC 9's business, not a block.
#[test]
fn the_operator_can_still_commit_while_an_act_is_outstanding() {
    let (ctx, mut app) = ctx_and_app();
    let tx = arm_turn(&mut app, "draw a line");
    let _outstanding = push_act(&tx, line(10.0));

    // The Act is queued and not yet drained: the worker is waiting.
    assert!(app.agent.busy);
    app.commit(Box::new(CreateCircle::new(Circle::new(
        Vec2::new(50.0, 50.0),
        5.0,
    ))));
    assert_eq!(app.document.entities.len(), 1, "the user's commit landed");

    idle(&ctx, &mut app);

    assert!(
        app.guard.pending_action.is_none(),
        "no modal parked the operator"
    );
    assert!(app.error_message.is_none(), "no error dialog");
    assert!(app.bed_dialog.is_none(), "no dialog opened by the turn");
}

// ── AC 21: the key never leaves the transport ───────────────────────────────

/// AC 21 — a full turn against a real endpoint, with a recognisable key: the
/// key appears in no transcript row and no command-line feedback.
#[test]
fn the_api_key_never_reaches_anything_the_operator_reads() {
    let mut server = mockito::Server::new();
    let _mock = server
        .mock("POST", "/chat/completions")
        .with_status(401)
        .with_body("unauthorized")
        .create();

    let (ctx, mut app) = ctx_and_app();
    app.settings.agent_endpoint = server.url();
    app.settings.agent_api_key = DUMMY_KEY.to_owned();
    start_turn(&mut app, "draw a line");

    run_until_idle(&ctx, &mut app, "the 401 turn must end");

    assert!(
        app.agent.chat.iter().any(|(role, _)| role == "error"),
        "the turn really did fail, so there was something to leak: {:?}",
        app.agent.chat
    );
    for (role, content) in &app.agent.chat {
        assert!(
            !content.contains(DUMMY_KEY),
            "the key leaked into a `{role}` row: {content}"
        );
    }
    assert!(
        !app.command_feedback.contains(DUMMY_KEY),
        "the key leaked into the command line: {}",
        app.command_feedback
    );
}

// ── The whole demand, over a real socket ────────────────────────────────────

/// Run frames until the turn ends, or fail with `message`.
///
/// Bounded at 400 frames × 5 ms ≈ 2 s. The sleep is between frames, not inside
/// a turn: the worker is doing real network I/O and the UI thread has nothing
/// to do until it answers. This is the only place in the suite that waits on
/// anything.
fn run_until_idle(ctx: &egui::Context, app: &mut App, message: &str) {
    for _ in 0..400 {
        idle(ctx, app);
        if !app.agent.busy {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!(
        "{message}: still busy after 400 frames (~2 s), chat was {:?}",
        app.agent.chat
    );
}

/// The whole demand, end to end: a real thread, a real socket, and two lines on
/// the operator's real bed.
///
/// This is the test that closes `CHANGELOG.md`'s *"the agent can read and
/// narrate the drawing but does not yet modify it end-to-end from chat"*.
///
/// It does **not** prove `src/app/mod.rs`'s `if self.agent.busy {
/// ctx.request_repaint(); }` is load-bearing, and no headless test can: this
/// loop calls `update_ui` on its own schedule, so it keeps running frames
/// whether or not anything asked for a repaint. Deleting the guard leaves this
/// test green. What catches that is a source scan —
/// `every_repaint_request_in_src_is_conditional` in `src/app/viewport/tests.rs` for
/// the call site, and `the_agent_repaint_is_guarded_on_the_busy_flag` below
/// for the condition it is guarded on.
#[test]
fn a_whole_turn_lands_on_the_bed() {
    let mut server = mockito::Server::new();
    let _first = server
        .mock("POST", "/chat/completions")
        .with_status(200)
        .with_body(
            r#"{"choices":[{"message":{"role":"assistant","content":null,"tool_calls":[
                {"id":"a","type":"function","function":{"name":"create_line",
                 "arguments":"{\"x1\":0,\"y1\":0,\"x2\":20,\"y2\":0}"}},
                {"id":"b","type":"function","function":{"name":"create_line",
                 "arguments":"{\"x1\":20,\"y1\":0,\"x2\":20,\"y2\":20}"}}
            ]}}]}"#,
        )
        .expect(1)
        .create();
    let _second = server
        .mock("POST", "/chat/completions")
        .with_status(200)
        .with_body(r#"{"choices":[{"message":{"role":"assistant","content":"Drew two lines."}}]}"#)
        .create();

    let (ctx, mut app) = ctx_and_app();
    app.settings.agent_endpoint = server.url();
    app.settings.agent_api_key = DUMMY_KEY.to_owned();
    app.settings.agent_model = "test/model".to_owned();
    let stack_before = app.history.len();

    start_turn(&mut app, "draw two 20 mm lines");
    run_until_idle(&ctx, &mut app, "the end-to-end turn must finish");

    assert_eq!(app.document.entities.len(), 2, "{:?}", app.agent.chat);
    let points: Vec<(Vec2, Vec2)> = app
        .document
        .entities
        .iter()
        .map(|e| match e {
            lasercad::document::Entity::Line(l) => (l.p1, l.p2),
            other => panic!("expected lines, got {other:?}"),
        })
        .collect();
    assert_eq!(
        points,
        [
            (Vec2::new(0.0, 0.0), Vec2::new(20.0, 0.0)),
            (Vec2::new(20.0, 0.0), Vec2::new(20.0, 20.0)),
        ],
        "the exact millimetres the model asked for"
    );
    assert_eq!(
        app.history.len(),
        stack_before + 1,
        "one turn, one undo entry"
    );
    assert_eq!(
        app.agent
            .chat
            .iter()
            .rev()
            .find(|(role, _)| role == "assistant")
            .map(|(_, text)| text.as_str()),
        Some("Drew two lines."),
    );

    tap(&ctx, &mut app, egui::Key::Z, ctrl());
    assert!(
        app.document.entities.is_empty(),
        "one undo empties what the turn drew"
    );
}

// ── AC 19: the repaint that keeps a turn moving ────────────────────────────

/// Everything in `src` up to the first bare `#[cfg(test)]` at column 0.
///
/// Scanning the whole file would let a needle match a *test* that merely
/// mentions the thing the implementation is supposed to contain — the scan
/// would then be asserting about itself.
fn implementation(src: &str) -> &str {
    let marker = concat!("#[cfg(", "test)]\n");
    match src.find(marker) {
        Some(at) if at == 0 || src.as_bytes()[at - 1] == b'\n' => &src[..at],
        _ => src,
    }
}

/// The nearest preceding statement to the line holding `needle`, comments and
/// blank lines skipped — i.e. whatever opens the block that line sits in.
fn guard_above(src: &str, needle: &str) -> String {
    let lines: Vec<&str> = src.lines().collect();
    let at = lines
        .iter()
        .position(|l| l.contains(needle) && !l.trim_start().starts_with("//"))
        .unwrap_or_else(|| panic!("`{needle}` must appear outside a comment"));
    lines[..at]
        .iter()
        .rev()
        .map(|l| l.trim())
        .find(|l| !l.is_empty() && !l.starts_with("//"))
        .unwrap_or("")
        .to_owned()
}

/// AC 19 — the agent's repaint is guarded on `agent.busy`, and on nothing else.
///
/// `every_repaint_request_in_src_is_conditional` (LCV-120 AC 8) pins that
/// `src/app/mod.rs` holds one repaint call and that it sits inside an `if`.
/// That much survives `if true {`, which is an unconditional per-frame repaint
/// wearing a guard's clothes — the exact regression LCV-120 closed. This pins
/// the condition itself.
///
/// Both halves run through the same [`guard_above`] over a witness first, so a
/// needle that could not find a real guard fails here rather than passing over
/// the real file for free.
#[test]
fn the_agent_repaint_is_guarded_on_the_busy_flag() {
    let needle = concat!("request_", "repaint");
    let flag = concat!("agent", ".busy");

    let witness = "if self.agent.busy {\n    ctx.request_repaint();\n}\n";
    assert!(
        guard_above(witness, needle).contains(flag),
        "control: the needles must be able to find a real guarded repaint"
    );
    let unguarded = "if true {\n    ctx.request_repaint();\n}\n";
    assert!(
        !guard_above(unguarded, needle).contains(flag),
        "control: a guard that is not the busy flag must not pass"
    );

    let src = implementation(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/app/mod.rs"
    )));
    let guard = guard_above(src, needle);
    assert!(
        guard.contains(flag),
        "AC 19: the repaint in src/app/mod.rs must be guarded on `{flag}`, found {guard:?}"
    );
}
