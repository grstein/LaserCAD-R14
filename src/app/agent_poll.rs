//! LCV-080 / LCV-122 / LCV-123 — the once-per-frame drain of the agent
//! channel, and everything one turn owes the document on its way out.
//!
//! Extracted from `App::update` so the polling logic is unit-testable
//! headlessly, with no egui context. Since LCV-123 the drain also *applies*:
//! ADR 0007 §D8 gives this file the drain, the dispatch, the answer and the
//! turn end, which is why the fence check, the group seal and the undo note
//! live here rather than in `agent_turn.rs`.
//!
//! ## Why a turn must always announce its end (ADR 0007 §D11)
//!
//! `agent.busy` is not just the spinner's flag. `App::update_ui` reads it to
//! decide whether to `ctx.request_repaint()`, so while it is `true` the app
//! never idles — and under ADR 0007 §D2 it is also what keeps frames turning so
//! an in-flight rendezvous can advance. A turn that ends without saying so
//! therefore leaves the app repainting every frame for the rest of the session,
//! silently reopening LCV-120 with a symptom that surfaces nowhere near the
//! agent.
//!
//! What guarantees that cannot happen is a closure property, not a list of
//! arms — a count rots the moment someone adds one. [`end_turn`] is the only
//! place in the program that writes `agent.busy = false` or clears `agent.rx`
//! after startup, and every path out of [`poll_agent_rx`] that does not put
//! the receiver back is a tail call to it. Any exit added later must obey
//! that, not this paragraph.
//!
//! Two of those arms report the same fact by different routes: a
//! `TryRecvError::Disconnected`, and a `reply.send` that finds nobody waiting
//! for an answer. Both mean the worker is gone — its event `Sender` is moved
//! into the thread closure and held nowhere else, so a thread that panics or
//! returns closes that channel as a matter of course. §D11 requires they show
//! the operator the same row: two spellings of one fact must never look like
//! two different facts.
//!
//! [`cancel_turn`] is the fifth exit §D11 allowed for (LCV-129), and it obeys
//! the property rather than widening it: it is a new *caller* of [`end_turn`],
//! not a second writer of `agent.busy`. It is the only exit the operator
//! chooses, and the only one that does not come off the channel — which is
//! exactly why it has to end the turn the same way. Dropping the receiver is
//! also what tells the worker to stop: both halves of its rendezvous fail the
//! moment the channel dies (ADR 0007 §D2), so no thread is killed and none is
//! asked to check a flag.

use crate::agent::{AgentAction, AgentEvent, AgentOutcome};
use crate::app::{agent_apply, agent_capture, App};
use std::sync::mpsc::TryRecvError;

/// Text shown in the chat when the worker thread ended without a verdict.
pub const AGENT_LOST_MESSAGE: &str = "Agent turn ended without a reply.";

/// Text shown in the chat when the operator cancelled the turn (LCV-129 AC 7).
///
/// It says the one thing a cancel leaves ambiguous: a turn stopped halfway
/// still *drew* half of something, and that half is real, on the bed, and
/// undoable. The row that follows it says in how many steps.
pub const AGENT_CANCELLED_MESSAGE: &str =
    "Turn cancelled. The agent stopped; anything it already applied stays applied and stays undoable.";

/// Drain `app.agent.rx` and update `agent.chat`, `agent.busy` and `agent.rx`.
///
/// Called once per frame at the top of `App::update_ui`, before any panel is
/// drawn. A no-op when `agent.rx` is `None`.
///
/// `Act` applies to the live document and the drain continues unless nobody is
/// left to answer; every other arm ends the turn, all through [`end_turn`].
pub fn poll_agent_rx(app: &mut App) {
    // Taken, not borrowed: applying an `Act` needs `&mut App`, which the
    // receiver's borrow would forbid. It goes back below unless the turn ended.
    let Some(rx) = app.agent.rx.take() else {
        return;
    };
    loop {
        match rx.try_recv() {
            Ok(AgentEvent::Act { action, reply }) => {
                // LCV-145: the pre-upload check is a rendezvous, not a step —
                // no count, no fence, no `tool` row (ADR 0011 item 10).
                let outcome = match &action {
                    AgentAction::AuthorizeUpload { endpoint, model } => {
                        agent_capture::authorize(app, endpoint, model)
                    }
                    _ => apply_fenced(app, &action),
                };
                if reply.send(outcome).is_err() {
                    // The fourth turn exit: the worker panicked or returned
                    // between sending this `Act` and reading its answer. It
                    // reports the same fact as `Disconnected` below, one
                    // instant earlier, so it writes the same row (ADR 0007
                    // §D11). It returns rather than falling through, because
                    // falling through assumes both channels die in the same
                    // instant; where they do not, `try_recv` says `Empty`, the
                    // receiver goes back, and `agent.busy` latches for good.
                    end_turn(app, Some(("error", AGENT_LOST_MESSAGE.to_string())));
                    return;
                }
            }
            Ok(AgentEvent::Done(text, _batches)) => {
                end_turn(app, Some(("assistant", text)));
                return;
            }
            Ok(AgentEvent::Failed(error, _batches)) => {
                end_turn(app, Some(("error", error)));
                return;
            }
            Err(TryRecvError::Empty) => {
                // The turn is still running; keep the channel for next frame.
                app.agent.rx = Some(rx);
                return;
            }
            Err(TryRecvError::Disconnected) => {
                end_turn(app, Some(("error", AGENT_LOST_MESSAGE.to_string())));
                return;
            }
        }
    }
}

/// Apply one action behind the turn's fence (ADR 0007 §D4), and count it.
///
/// Four things happen here that `agent_apply` deliberately does not know
/// about, because they are properties of the *turn* rather than of the action:
///
/// - the step count the panel's progress row reads — every `Act`, ungated
///   (LCV-142 AC 3/4);
/// - the fence check, which answers `Fenced` to anything at all once the
///   revision moved outside this turn or its group was sealed — including a
///   query, whose answer would be read against a drawing the model has not
///   seen (AC 9, ADR 0007 §D14);
/// - the advance, which re-anchors the expectation on the revision this apply
///   produced;
/// - the applied-action count the end-of-turn note reads (LCV-142 AC 12).
///
/// Both of the last two are gated on the revision **actually moving**, which is
/// what keeps a query out of the count and off the fence (AC 16): a query is an
/// action, it gets a transcript row like any other, and it is not an *applied*
/// action.
fn apply_fenced(app: &mut App, action: &AgentAction) -> AgentOutcome {
    // Every `Act` is a step — fenced, malformed and queries included (§D13).
    app.agent.turn.steps = app.agent.turn.steps.saturating_add(1);
    let (revision, group_open) = (app.history.revision(), app.history.group_open());
    if let Err(refusal) = app.agent.turn.fence.check(revision, group_open) {
        // §D14: the worker reads `Fenced` as "stop dispatching".
        let outcome = AgentOutcome::Fenced(refusal);
        agent_apply::transcribe(app, &outcome);
        return outcome;
    }
    let before = app.history.revision();
    let outcome = agent_apply::apply(app, action);
    let after = app.history.revision();
    if after != before {
        app.agent.turn.fence.advance(after);
        app.agent.turn.applied += 1;
    }
    outcome
}

/// The work every turn end does, whichever exit got here (LCV-123 AC 22).
///
/// Seals the turn's history group — exactly once per turn, whatever it holds
/// (ADR 0007 §D12) — then tells the operator what the turn left behind. A
/// turn that applied nothing says nothing: a note row would be noise.
///
/// The note is derived from **`end_group`'s report**, never from the fence:
/// only when this seal took all `applied` commands is the whole turn one
/// `Ctrl+Z` away. A group sealed earlier by a foreign event, or dropped with
/// a replaced document, reports `None` here and gets the neutral sentence.
///
/// The counter is **taken**, not read, so a second finish writes no note.
fn finish_turn(app: &mut App) {
    let applied = std::mem::take(&mut app.agent.turn.applied);
    let sealed = app.history.end_group();
    if applied == 0 {
        return;
    }
    let whole = sealed == Some(applied);
    app.agent
        .chat
        .push(("note".to_owned(), undo_note(applied, whole)));
}

/// What the operator is told about the turn they just watched (LCV-142 AC 12).
///
/// `whole` is true only when the turn's own seal took every applied action;
/// otherwise the drawing changed outside the turn and no undo claim is made.
fn undo_note(applied: usize, whole: bool) -> String {
    match (applied, whole) {
        (1, true) => "Applied 1 action — Ctrl+Z undoes it.".to_owned(),
        (n, true) => format!("Applied {n} actions — Ctrl+Z undoes the whole turn."),
        (1, false) => "Applied 1 action before the drawing changed outside this turn.".to_owned(),
        (n, false) => format!("Applied {n} actions before the drawing changed outside this turn."),
    }
}

/// Every exit path, in one place: push an optional chat row, do the turn-end
/// work, clear the busy flag, drop the channel. ADR 0007 §D11 — a reviewer
/// checks this list.
///
/// The order of the first two is the tail of AC 23's total ordering: the
/// terminal row says how the turn ended, the note row says what it left behind.
fn end_turn(app: &mut App, row: Option<(&str, String)>) {
    if let Some((role, text)) = row {
        app.agent.chat.push((role.to_owned(), text));
    }
    finish_turn(app);
    app.agent.busy = false;
    app.agent.rx = None;
}

/// End the in-flight turn because the operator asked to (LCV-129 AC 6).
///
/// The fifth exit of ADR 0007 §D11, and the only one that is not an event off
/// the channel. It obeys the closure property by construction: it writes
/// neither `agent.busy` nor `agent.rx`, pushes no row itself, and its whole
/// body is a tail call to [`end_turn`] — so a cancelled turn seals into one
/// undo entry, says what it left behind and clears the repaint gate exactly as
/// a `Done` or a `Failed` does. A cancel that assigned the flag here would
/// "work" in every manual test and leave three separate undo entries behind a
/// note claiming one.
///
/// A no-op when no turn is in flight: without the guard, the panel's button
/// would write a cancellation row for a turn that is not running.
///
/// The worker is not killed — Rust has no safe way to — and is not asked to
/// check anything. [`end_turn`] drops the receiver, which fails both halves of
/// its next rendezvous (ADR 0007 §D2, §D3): a `tx.send` has nobody to send to,
/// and an `Act` still queued unread is dropped along with the reply `Sender`
/// riding inside it, so a worker already blocked on the answer wakes with an
/// `Err`. Either way it returns `AgentError::Cancelled` and says nothing,
/// because its channel is dead and a later turn owns a different one.
pub fn cancel_turn(app: &mut App) {
    if !app.agent.busy {
        return;
    }
    end_turn(app, Some(("note", AGENT_CANCELLED_MESSAGE.to_owned())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::arm_turn;

    /// LCV-142 AC 12 — the four sentences, exactly as the demand decided
    /// them, pinned character for character.
    #[test]
    fn the_undo_note_says_whether_the_turn_is_one_undo_away() {
        assert_eq!(undo_note(1, true), "Applied 1 action — Ctrl+Z undoes it.");
        assert_eq!(
            undo_note(4, true),
            "Applied 4 actions — Ctrl+Z undoes the whole turn."
        );
        assert_eq!(
            undo_note(1, false),
            "Applied 1 action before the drawing changed outside this turn."
        );
        assert_eq!(
            undo_note(2, false),
            "Applied 2 actions before the drawing changed outside this turn."
        );
        assert!(undo_note(7, true).contains('7'));
        assert!(undo_note(9, false).contains('9'));
    }

    /// LCV-142 AC 12 — the old sentence is gone: nothing claims the turn left
    /// separate undo steps, and the neutral sentences make no undo claim.
    #[test]
    fn no_note_mentions_separate_undo_steps_or_undo_when_not_whole() {
        for n in [1usize, 2, 5] {
            assert!(!undo_note(n, true).contains("separate"));
            assert!(!undo_note(n, false).contains("Ctrl+Z"));
        }
    }

    /// AC 11 — a turn that applied nothing says nothing: no note row, and the
    /// empty group seals into nothing. A query-only turn is the common case.
    #[test]
    fn a_turn_that_applied_nothing_writes_no_note() {
        let mut app = App::default();
        let _tx = arm_turn(&mut app, "what is on the bed?");
        let rows = app.agent.chat.len();
        let stack = app.history.len();

        finish_turn(&mut app);

        assert_eq!(app.agent.chat.len(), rows, "no note row for zero actions");
        assert_eq!(app.history.len(), stack);
        assert!(!app.history.group_open(), "the group was still sealed");
    }

    /// ADR 0007 §D11 — [`end_turn`] is the one place that clears `agent.rx`,
    /// and it clears it whichever exit got here.
    ///
    /// `poll_agent_rx` **takes** the receiver at the top of the drain, so on
    /// every path through this file the field is already `None` by the time
    /// `end_turn` runs and the assignment looks like dead code. It is not the
    /// assignment that is load-bearing, it is the *property*: a future exit
    /// that puts the receiver back before ending the turn — the shape the
    /// `Empty` arm already has — would otherwise latch `agent.busy` on a dead
    /// channel for the rest of the session. Calling `end_turn` directly with
    /// the receiver in place is the only way to state that, so this test does.
    #[test]
    fn end_turn_clears_the_channel_even_when_it_is_still_there() {
        let (_tx, rx) = std::sync::mpsc::channel::<AgentEvent>();
        let mut app = App {
            agent: crate::app::AgentState {
                rx: Some(rx),
                busy: true,
                ..Default::default()
            },
            ..Default::default()
        };

        end_turn(&mut app, None);

        assert!(
            app.agent.rx.is_none(),
            "the channel is dropped on the way out"
        );
        assert!(!app.agent.busy, "and the flag goes with it");
    }

    /// LCV-129 AC 6 — a cancel with no turn to cancel changes nothing.
    ///
    /// Without the guard the panel could not be at fault — the button is inside
    /// the busy block — but `cancel_turn` is `pub`, and a public entry point
    /// that writes a cancellation row for a turn that never ran would put a
    /// note in the transcript describing work nobody did.
    #[test]
    fn ac6_cancelling_an_idle_app_does_nothing_at_all() {
        let mut app = App::default();
        app.agent.chat.push(("user".to_owned(), "hello".to_owned()));
        let before = app.agent.chat.clone();

        cancel_turn(&mut app);

        assert_eq!(app.agent.chat, before, "no row is written for no turn");
        assert!(!app.agent.busy);
        assert!(app.agent.rx.is_none());
    }

    /// LCV-129 AC 7 — the cancel row, pinned character for character, in the
    /// role LCV-125 AC 1 already closed the vocabulary around.
    ///
    /// The sentence is compared against a literal assembled with `concat!`, so
    /// no scan of this file can ever match it, and the `note` role is asserted
    /// by name: a seventh role would be silently dropped by `panel.rs`'s six
    /// match arms and the operator would be told nothing.
    #[test]
    fn ac7_cancelling_a_live_turn_writes_the_note_row() {
        let mut app = App::default();
        let _tx = arm_turn(&mut app, "draw something slow");
        assert!(app.agent.busy, "arm_turn must leave a turn in flight");

        cancel_turn(&mut app);

        let (role, text) = app.agent.chat.last().expect("a row must have been written");
        assert_eq!(role, "note");
        assert_eq!(
            text,
            concat!(
                "Turn cancelled. The agent stopped; anything it already ",
                "applied stays applied and stays undoable."
            )
        );
        assert_eq!(text, AGENT_CANCELLED_MESSAGE);
    }

    /// The applied-action counter must not survive its turn: a second turn
    /// that inherited it would write a note about work that is not its own.
    #[test]
    fn finishing_a_turn_clears_the_applied_counter() {
        let mut app = App::default();
        let _tx = arm_turn(&mut app, "one");
        app.agent.turn.applied = 3;
        finish_turn(&mut app);
        assert_eq!(app.agent.turn.applied, 0);
        finish_turn(&mut app);
        assert_eq!(
            app.agent.chat.iter().filter(|(r, _)| r == "note").count(),
            1,
            "a second finish must not invent a second note"
        );
    }
}
