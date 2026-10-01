//! The end of every agent turn (ADR 0007 §D11): the seal, the undo note, the
//! metrics note (LCV-193), memory, the busy flag and the channel. Split out of
//! `agent_poll.rs` for the LOC cap; [`end_turn`] is still its only writer of
//! `agent.busy = false` and `agent.rx = None`.

use crate::agent::{ChatMessage, TurnEnd};
use crate::app::{App, agent_memory};

/// Every exit path, in one place: push an optional chat row, do the turn-end
/// work, add the metrics note, record the turn in memory, clear the busy
/// flag, drop the channel. ADR 0007 §D11 — a reviewer checks this list.
///
/// The order of the rows is the tail of AC 23's total ordering: the terminal
/// row says how the turn ended, the undo note what it left behind, and the
/// metrics note (LCV-193), always last and always written, what it cost.
/// Memory is recorded after the seal, so a `Done` mark sees the sealed history
/// (LCV-153, ADR 0007 §D16); it keeps batches, never these rows.
pub(super) fn end_turn(
    app: &mut App,
    row: Option<(&str, String)>,
    end: TurnEnd,
    batches: Vec<ChatMessage>,
) {
    if let Some((role, text)) = row {
        app.agent.chat.push((role.to_owned(), text));
    }
    finish_turn(app);
    let metrics = app.agent.turn.tally.note();
    app.agent.chat.push(("note".to_owned(), metrics));
    agent_memory::record(app, end, batches);
    app.agent.busy = false;
    app.agent.rx = None;
}

/// The work every turn end does, whichever exit got here (LCV-123 AC 22).
///
/// Seals the turn's history group — exactly once per turn, whatever it holds
/// (ADR 0007 §D12) — then tells the operator what the turn left behind. A
/// turn that applied nothing gets no undo note: it would be noise.
///
/// The note is derived from **`end_group`'s report**, never from the fence:
/// only when this seal found the turn's group still open is the whole turn
/// one `Ctrl+Z` away, and the count is what it sealed — fewer than `applied`
/// after a rollback, and no note when nothing survived (LCV-198). A group
/// sealed earlier by a foreign event, or dropped with a replaced document,
/// reports `None` here and gets the neutral sentence.
///
/// The tally is read, not taken: the metrics note reads it next, and it stays
/// readable until `arm_turn` replaces it whole (LCV-193).
pub(super) fn finish_turn(app: &mut App) {
    let applied = app.agent.turn.tally.applied;
    let sealed = app.history.end_group();
    if applied == 0 {
        return;
    }
    let note = match sealed {
        Some(0) => return,
        Some(survivors) => undo_note(survivors, true),
        None => undo_note(usize::try_from(applied).unwrap_or(usize::MAX), false),
    };
    app.agent.chat.push(("note".to_owned(), note));
}

/// What the operator is told about the turn they just watched (LCV-142 AC 12).
///
/// `whole` is true only when the turn's own seal took every applied action;
/// otherwise the drawing changed outside the turn and no undo claim is made.
pub(super) fn undo_note(applied: usize, whole: bool) -> String {
    match (applied, whole) {
        (1, true) => "Applied 1 action — Ctrl+Z undoes it.".to_owned(),
        (n, true) => format!("Applied {n} actions — Ctrl+Z undoes the whole turn."),
        (1, false) => "Applied 1 action before the drawing changed outside this turn.".to_owned(),
        (n, false) => format!("Applied {n} actions before the drawing changed outside this turn."),
    }
}
