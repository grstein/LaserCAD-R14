//! LCV-080 / LCV-122 — the once-per-frame drain of the agent channel.
//!
//! Extracted from `App::update` so the polling logic is unit-testable
//! headlessly, with no egui context.
//!
//! ## Why a disconnected channel is a real event (ADR 0007 §D11)
//!
//! `agent_busy` is not just the spinner's flag. `App::update_ui` reads it to
//! decide whether to `ctx.request_repaint()`, so while it is `true` the app
//! never idles — and under ADR 0007 §D2 it is also what keeps frames turning so
//! an in-flight rendezvous can advance. A turn that ends without saying so
//! therefore leaves the app repainting every frame for the rest of the session,
//! silently reopening LCV-120 with a symptom that surfaces nowhere near the
//! agent. So `agent_busy` is cleared on **every** exit — `Done`, `Failed` and a
//! dropped sender alike — and a worker thread that panics or returns closes the
//! channel as a matter of course, because the `Sender` is moved into it.
//!
//! The **fourth** exit is a worker that vanishes between sending an `Act` and
//! reading its answer. The reply channel is then dead while the event channel
//! may still look alive, so nothing else would ever bring `agent_busy` down.
//! It reports the same thing as a dropped sender does — the worker is gone —
//! so it writes the same row, and ADR 0007 §D11 requires that: two spellings
//! of one fact must never show the operator different things.
//!
//! What holds all four together is not the list but the invariant under it:
//! [`end_turn`] is the only place in the program that writes
//! `agent_busy = false` or clears `agent_rx` after startup, and every path out
//! of [`poll_agent_rx`] that does not put the receiver back is a tail call to
//! it. A fifth exit must obey that, not this paragraph.

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
/// `Act` is the only non-terminal event: it is applied to the live document and
/// answered down its own reply channel, and the drain continues. `Done`,
/// `Failed` and a disconnected channel all end the turn.
pub fn poll_agent_rx(app: &mut App) {
    // Taken, not borrowed: applying an `Act` needs `&mut App`, which the
    // receiver's borrow would forbid. It goes back below unless the turn ended.
    let Some(rx) = app.agent_rx.take() else {
        return;
    };
    loop {
        match rx.try_recv() {
            Ok(AgentEvent::Act { action, reply }) => {
                let outcome = agent_apply::apply(app, &action);
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

/// Every exit path, in one place: push an optional chat row, clear the busy
/// flag, drop the channel. ADR 0007 §D11 — a reviewer checks this list.
fn end_turn(app: &mut App, row: Option<(&str, String)>) {
    if let Some((role, text)) = row {
        app.agent_chat.push((role.to_owned(), text));
    }
    app.agent_busy = false;
    app.agent_rx = None;
}
