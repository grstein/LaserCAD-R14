//! LCV-195 — answer the loop's after-batch feedback ask from the live
//! document: its size and CHECK summary, once per change, when the operator
//! turned `Feedback after changes` on.
//!
//! Runs on the UI thread, like every `Act` (ADR 0007 §D1): the worker never
//! sees the document, only the sentence.

use crate::agent::AgentOutcome;
use crate::app::App;
use crate::document::{Document, check_drawing};

/// `AgentAction::Feedback`: [`summary`] when the setting is on, read live,
/// and the turn applied something since its last feedback; else `Ok("")`,
/// which adds nothing to the tool result. Not a step and not fenced.
pub(crate) fn feedback(app: &mut App) -> AgentOutcome {
    let turn = &mut app.agent.turn;
    let moved = turn.tally.applied != turn.fed_at;
    turn.fed_at = turn.tally.applied;
    if !(moved && app.settings.agent_feedback_after_changes) {
        return AgentOutcome::Ok(String::new());
    }
    AgentOutcome::Ok(summary(&app.document))
}

/// `Drawing now: <n> entities, X <x0>..<x1> mm, Y <y0>..<y1> mm. <check>`,
/// or `Drawing now: 0 entities. <check>` for an empty drawing, where
/// `<check>` is the CHECK report's summary lines joined with `; `.
pub(crate) fn summary(doc: &Document) -> String {
    let n = doc.entity_count();
    let noun = if n == 1 { "entity" } else { "entities" };
    let size = match doc.bounds() {
        Some((lo, hi)) => format!(
            ", X {:.3}..{:.3} mm, Y {:.3}..{:.3} mm",
            lo.x, hi.x, lo.y, hi.y
        ),
        None => String::new(),
    };
    let lines = check_drawing(doc).lines();
    let check: Vec<&str> = lines
        .iter()
        .map(String::as_str)
        .filter(|l| l.starts_with("CHECK:"))
        .collect();
    format!("Drawing now: {n} {noun}{size}. {}", check.join("; "))
}
