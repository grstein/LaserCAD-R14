//! The verify reminder of the agent loop (LCV-197): before a turn that
//! changed the drawing ends on a text reply, ask the UI once whether the
//! model still owes a verification, and if so send it one fixed reminder.
//! Split out of `loop_.rs` for the LOC cap.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use super::{AgentError, Dispatch};
use crate::agent::bridge::AgentOutcome;
use crate::agent::wire::ChatMessage;

/// The one user message a granted reminder sends (LCV-197 AC 3). Code, not
/// prompt, so a prompt override cannot remove it; it grants nothing.
pub(crate) const VERIFY_REMINDER: &str = "Before you finish: verify the drawing against the request with measure, check_drawing or capture_canvas, fix what fails, then report each check as pass or fail.";

/// Decide what the text-only reply `text` does: `Some(text)` ends the turn
/// with it, `None` continues the turn.
///
/// [`Dispatch::VerifyDue`] is asked only while the turn has not been
/// reminded and `step_left` holds. On `Ok` the text (with its `reasoning`)
/// and [`VERIFY_REMINDER`] are pushed and `reminded` is set; any other
/// answer ends the turn with the text. A failed ask propagates.
pub(super) fn verify_or_end<D>(
    dispatch_fn: &mut D,
    messages: &mut Vec<ChatMessage>,
    text: String,
    reasoning: Option<String>,
    reminded: &mut bool,
    step_left: bool,
) -> Result<Option<String>, AgentError>
where
    D: FnMut(Dispatch<'_>) -> Result<AgentOutcome, AgentError>,
{
    if *reminded || !step_left {
        return Ok(Some(text));
    }
    if !matches!(dispatch_fn(Dispatch::VerifyDue)?, AgentOutcome::Ok(_)) {
        return Ok(Some(text));
    }
    *reminded = true;
    messages.push(ChatMessage::assistant(text).with_reasoning(reasoning));
    messages.push(ChatMessage::user(VERIFY_REMINDER));
    Ok(None)
}
