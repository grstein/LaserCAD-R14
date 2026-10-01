//! LCV-198 — the turn's checkpoints: named marks in its open history group
//! (ADR 0007 §D12, Amended (17)). `checkpoint` records `group_len()` under a
//! name; `rollback` rewinds the group to a recorded mark. `start` (mark 0)
//! is built in. The list lives in `TurnState`, which `arm_turn` rebuilds, so
//! no checkpoint outlives its turn (AC 9).

use crate::agent::AgentOutcome;
use crate::agent::tools::{expected_form, refusal};
use crate::app::App;

/// The built-in checkpoint: the start of the turn (AC 4).
pub(crate) const START: &str = "start";

/// The refusal when no turn's group is open, e.g. `apply` called directly.
pub(crate) const NO_TURN: &str = "No agent turn is open.";

/// The longest checkpoint name, in characters (AC 7).
const MAX_NAME: usize = 32;

/// The turn's named group marks, in the order they were last set; `start`
/// is implicit. Marks never decrease along the list: a rollback drops every
/// checkpoint set after its target, and the group only grows otherwise.
#[derive(Debug, Default)]
pub struct Checkpoints {
    marks: Vec<(String, usize)>,
}

impl Checkpoints {
    /// Record `name` at `mark`; an existing name moves here, set last (AC 5).
    fn set(&mut self, name: &str, mark: usize) {
        self.marks.retain(|(n, _)| n != name);
        self.marks.push((name.to_owned(), mark));
    }

    /// The mark `name` was set at; `start` is 0.
    fn target(&self, name: &str) -> Option<usize> {
        if name == START {
            return Some(0);
        }
        self.marks.iter().find(|(n, _)| n == name).map(|&(_, m)| m)
    }

    /// Keep `name` and forget every checkpoint set after it (AC 5); `start`
    /// forgets them all.
    fn truncate_after(&mut self, name: &str) {
        let keep = self
            .marks
            .iter()
            .position(|(n, _)| n == name)
            .map_or(0, |i| i + 1);
        self.marks.truncate(keep);
    }

    /// `start, a, b`: every name a rollback accepts, oldest first.
    fn known(&self) -> String {
        let names = self.marks.iter().map(|(n, _)| n.as_str());
        std::iter::once(START)
            .chain(names)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// 1 to 32 characters of `A-Z a-z 0-9 _ -` (AC 7).
pub(crate) fn valid_name(name: &str) -> bool {
    (1..=MAX_NAME).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// `checkpoint {name}`: record the turn's position as one step that moves
/// neither the drawing nor the revision (AC 1, AC 5).
pub(crate) fn checkpoint(app: &mut App, name: &str) -> AgentOutcome {
    if let Some(refused) = refuse("checkpoint", app, name) {
        return refused;
    }
    if name == START {
        let reason = "\"start\" is built in and cannot be set";
        return known_refusal("checkpoint", app, reason, "any other checkpoint name");
    }
    let mark = app.history.group_len();
    app.agent.turn.checkpoints.set(name, mark);
    AgentOutcome::Ok(format!("Checkpoint {name} set at {mark} changes."))
}

/// `rollback {name}`: revert, newest first, every change the turn made after
/// the checkpoint, keep it and forget the later ones (AC 2–AC 5).
pub(crate) fn rollback(app: &mut App, name: &str) -> AgentOutcome {
    if let Some(refused) = refuse("rollback", app, name) {
        return refused;
    }
    let Some(mark) = app.agent.turn.checkpoints.target(name) else {
        let reason = format!("no checkpoint named \"{name}\"");
        return known_refusal("rollback", app, &reason, "the name of a known checkpoint");
    };
    let undone = app.history.rewind_group(mark, &mut app.document);
    app.agent.turn.checkpoints.truncate_after(name);
    let count = app.document.entity_count();
    AgentOutcome::Ok(format!(
        "Rolled back to {name}: {undone} changes undone, {count} entities."
    ))
}

/// The refusals both tools share: no open turn, or a malformed name (whose
/// text is never echoed, LCV-192 AC 5).
fn refuse(tool: &str, app: &App, name: &str) -> Option<AgentOutcome> {
    if !app.history.group_open() {
        return Some(AgentOutcome::Refused(NO_TURN.to_owned()));
    }
    (!valid_name(name)).then(|| known_refusal(tool, app, "invalid", expected_form("name")))
}

/// An LCV-192 refusal of `name`, followed by the known checkpoints (AC 7).
fn known_refusal(tool: &str, app: &App, reason: &str, expected: &str) -> AgentOutcome {
    let known = app.agent.turn.checkpoints.known();
    let text = refusal(tool, "name", reason, expected);
    AgentOutcome::Refused(format!("{text}. Known checkpoints: {known}."))
}

#[cfg(test)]
mod tests;
