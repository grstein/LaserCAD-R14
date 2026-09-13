//! LCV-080 / LCV-122 — the once-per-frame drain of the agent channel.
//!
//! Extracted from `App::update` so the polling logic is unit-testable
//! headlessly, with no egui context.
//!
//! ## Why a turn must always announce its end (ADR 0007 §D11)
//!
//! `agent_busy` is not just the spinner's flag. `App::update_ui` reads it to
//! decide whether to `ctx.request_repaint()`, so while it is `true` the app
//! never idles — and under ADR 0007 §D2 it is also what keeps frames turning so
//! an in-flight rendezvous can advance. A turn that ends without saying so
//! therefore leaves the app repainting every frame for the rest of the session,
//! silently reopening LCV-120 with a symptom that surfaces nowhere near the
//! agent.
//!
//! What guarantees that cannot happen is a closure property, not a list of
//! arms — a count rots the moment someone adds one. [`end_turn`] is the only
//! place in the program that writes `agent_busy = false` or clears `agent_rx`
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

use crate::agent::bridge::{AgentAction, AgentOutcome};
use crate::agent::AgentEvent;
use crate::app::{agent_apply, App};
use std::sync::mpsc::TryRecvError;

/// Text shown in the chat when the worker thread ended without a verdict.
pub const AGENT_LOST_MESSAGE: &str = "Agent turn ended without a reply.";

/// Drain `app.agent_rx` and update `agent_chat`, `agent_busy` and `agent_rx`.
///
/// Called once per frame at the top of `App::update_ui`, before any panel is
/// drawn. A no-op when `agent_rx` is `None`.
///
/// `Act` applies to the live document and the drain continues unless nobody is
/// left to answer; every other arm ends the turn, all through [`end_turn`].
pub fn poll_agent_rx(app: &mut App) {
    // Taken, not borrowed: applying an `Act` needs `&mut App`, which the
    // receiver's borrow would forbid. It goes back below unless the turn ended.
    let Some(rx) = app.agent_rx.take() else {
        return;
    };
    loop {
        match rx.try_recv() {
            Ok(AgentEvent::Act { action, reply }) => {
                let outcome = apply_fenced(app, &action);
                if reply.send(outcome).is_err() {
                    // The fourth turn exit: the worker panicked or returned
                    // between sending this `Act` and reading its answer. It
                    // reports the same fact as `Disconnected` below, one
                    // instant earlier, so it writes the same row (ADR 0007
                    // §D11). It returns rather than falling through, because
                    // falling through assumes both channels die in the same
                    // instant; where they do not, `try_recv` says `Empty`, the
                    // receiver goes back, and `agent_busy` latches for good.
                    end_turn(app, Some(("error", AGENT_LOST_MESSAGE.to_string())));
                    return;
                }
            }
            Ok(AgentEvent::Done(text)) => {
                end_turn(app, Some(("assistant", text)));
                return;
            }
            Ok(AgentEvent::Failed(error)) => {
                end_turn(app, Some(("error", error)));
                return;
            }
            Err(TryRecvError::Empty) => {
                // The turn is still running; keep the channel for next frame.
                app.agent_rx = Some(rx);
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
/// Three things happen here that `agent_apply` deliberately does not know
/// about, because they are properties of the *turn* rather than of the action:
///
/// - the fence check, which refuses anything at all once the revision moved
///   outside this turn — including a query, whose answer would be read against
///   a drawing the model has not seen (AC 9);
/// - the advance, which re-anchors the expectation on the revision this apply
///   produced;
/// - the applied-action count that AC 10's coalesce and AC 11's note read.
///
/// Both of the last two are gated on the revision **actually moving**, which is
/// what keeps a query out of the count and off the fence (AC 16): a query is an
/// action, it gets a transcript row like any other, and it is not an *applied*
/// action.
fn apply_fenced(app: &mut App, action: &AgentAction) -> AgentOutcome {
    if let Err(refusal) = app.agent_fence.check(app.history.revision()) {
        let outcome = AgentOutcome::Refused(refusal);
        agent_apply::transcribe(app, &outcome);
        return outcome;
    }
    let before = app.history.revision();
    let outcome = agent_apply::apply(app, action);
    let after = app.history.revision();
    if after != before {
        app.agent_fence.advance(after);
        app.agent_applied += 1;
    }
    outcome
}

/// The work every turn end does, whichever of the four exits got here (AC 22).
///
/// Folds this turn's commits into one undo entry when the fence says they are
/// still contiguous at the top of the stack, then tells the operator which of
/// the two undo shapes they got. A turn that applied nothing says nothing:
/// there is no undo shape to describe and a note row would be noise.
///
/// The counter is **taken**, not read: `end_turn` is the only caller, but a
/// counter that survived its turn would silently fold the next one's entries.
fn finish_turn(app: &mut App) {
    let applied = std::mem::take(&mut app.agent_applied);
    if applied == 0 {
        return;
    }
    let coalesced = app
        .agent_fence
        .may_coalesce(app.history.revision(), applied);
    if coalesced {
        let label = std::mem::take(&mut app.agent_turn_label);
        app.history.coalesce_last(applied, &label);
    }
    app.agent_chat
        .push(("note".to_owned(), undo_note(applied, coalesced)));
}

/// What the operator is told about undoing the turn they just watched (AC 11).
///
/// One action is one undo entry whether or not anything else happened, so it
/// gets its own sentence rather than a coalesced/not-coalesced pair.
fn undo_note(applied: usize, coalesced: bool) -> String {
    match (applied, coalesced) {
        (1, _) => "Applied 1 action — Ctrl+Z undoes it.".to_owned(),
        (n, true) => format!("Applied {n} actions — Ctrl+Z undoes the whole turn."),
        (n, false) => format!(
            "Applied {n} actions — the drawing changed mid-turn, so they stay \
             {n} separate undo steps."
        ),
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
        app.agent_chat.push((role.to_owned(), text));
    }
    finish_turn(app);
    app.agent_busy = false;
    app.agent_rx = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::arm_turn;

    /// AC 11 — the four sentences, exactly as the demand decided them. Nothing
    /// in `src/` owned this copy before, so it is pinned character for
    /// character rather than by `contains`.
    #[test]
    fn the_undo_note_says_which_of_the_two_shapes_the_turn_left() {
        assert_eq!(
            undo_note(4, true),
            "Applied 4 actions — Ctrl+Z undoes the whole turn."
        );
        assert_eq!(
            undo_note(2, false),
            "Applied 2 actions — the drawing changed mid-turn, so they stay 2 \
             separate undo steps."
        );
        assert_eq!(undo_note(1, false), "Applied 1 action — Ctrl+Z undoes it.");
        assert_eq!(
            undo_note(1, true),
            "Applied 1 action — Ctrl+Z undoes it.",
            "one action is one undo entry either way — it never reads `1 actions`"
        );
    }

    /// AC 11 — the count in the not-coalesced sentence is the real one, in
    /// both of its two places. A hardcoded `2` passes the case above.
    #[test]
    fn the_not_coalesced_note_counts_the_actions_it_really_left() {
        assert_eq!(
            undo_note(3, false),
            "Applied 3 actions — the drawing changed mid-turn, so they stay 3 \
             separate undo steps."
        );
        assert!(undo_note(7, true).contains('7'));
    }

    /// AC 11 — a turn that applied nothing says nothing: no note row, and no
    /// coalesce attempted. A query-only turn is the common case.
    #[test]
    fn a_turn_that_applied_nothing_writes_no_note() {
        let mut app = App::default();
        let _tx = arm_turn(&mut app, "what is on the bed?");
        let rows = app.agent_chat.len();
        let stack = app.history.len();

        finish_turn(&mut app);

        assert_eq!(app.agent_chat.len(), rows, "no note row for zero actions");
        assert_eq!(app.history.len(), stack);
    }

    /// The applied-action counter must not survive its turn: a second turn
    /// that inherited it would fold entries that are not its own.
    #[test]
    fn finishing_a_turn_clears_the_applied_counter() {
        let mut app = App::default();
        let _tx = arm_turn(&mut app, "one");
        app.agent_applied = 3;
        finish_turn(&mut app);
        assert_eq!(app.agent_applied, 0);
        finish_turn(&mut app);
        assert_eq!(
            app.agent_chat.iter().filter(|(r, _)| r == "note").count(),
            1,
            "a second finish must not invent a second note"
        );
    }
}
