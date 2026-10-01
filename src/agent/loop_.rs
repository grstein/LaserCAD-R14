//! LCV-079 / LCV-121 — Multi-turn agent loop.
//!
//! Control flow only: send the conversation, honour the step budget, feed tool
//! results back. The wire shapes come from [`crate::agent::wire`] and the HTTP
//! call from [`crate::agent::transport`]; nothing is declared twice.
//!
//! Since LCV-122 the loop has no idea what a drawing is: `dispatch_fn` takes a
//! tool name and a JSON argument string and hands back the outcome whose
//! sentence the model reads. Whoever supplies that closure — `crate::app::run_agent_turn` — is the
//! only party that knows where the geometry goes (ADR 0007 §D1).
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use crate::agent::bridge::AgentOutcome;
use crate::agent::wire::{AssistantMessage, ChatMessage};

mod batch;
mod images;
use batch::{Steps, run_batch};
use images::send_images;

// ── Step budget ──────────────────────────────────────────────────────────────

/// Tool-call dispatches (steps) allowed in one turn when the operator has not
/// chosen otherwise (ADR 0007 §D13, LCV-142).
pub const AGENT_STEP_BUDGET_DEFAULT: u32 = 256;

/// Smallest budget that still lets the agent do anything at all.
pub const AGENT_STEP_BUDGET_MIN: u32 = 1;

/// Largest budget. Not a structural limit — the flat history group (§D12)
/// holds any turn as one entry — but a runaway turn stays bounded in minutes
/// and in money.
pub const AGENT_STEP_BUDGET_MAX: u32 = 4096;

/// Hold a stored budget inside `AGENT_STEP_BUDGET_MIN..=AGENT_STEP_BUDGET_MAX`.
///
/// The value comes from a JSON file the operator can hand-edit, so every reader
/// goes through here rather than trusting the field: `0` becomes the minimum
/// and anything oversized becomes the maximum. Clamping deliberately lives with
/// the loop that enforces the budget, not in `io::settings` — that module must
/// not import `crate::agent` (ADR 0007 §D7).
pub fn clamp_step_budget(value: u32) -> u32 {
    value.clamp(AGENT_STEP_BUDGET_MIN, AGENT_STEP_BUDGET_MAX)
}

/// The tool result every call left in a batch gets once the fence has stopped
/// the turn (ADR 0007 §D14): not dispatched, but still answered, so every
/// `tool_call_id` the model sent stays paired.
pub(crate) const FENCE_STOP_PLACEHOLDER: &str =
    "not run: the turn stopped after the drawing changed outside it";

/// What an image part becomes once it has ridden its one request (ADR 0011
/// item 9), whether that send succeeded or failed.
pub(crate) const IMAGE_ELIDED: &str =
    "canvas image elided after one use; call capture_canvas to look again";

/// What an image part becomes when the UI did not authorise its upload (ADR
/// 0011 item 10): the request still goes out, text-only.
pub(crate) const IMAGE_WITHHELD: &str =
    "canvas image withheld: capture permission changed during the turn";

/// One request from the loop to whoever owns the drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dispatch<'a> {
    /// Run one tool call — a step.
    Tool {
        /// The tool name as the model sent it.
        name: &'a str,
        /// The raw JSON argument string.
        args: &'a str,
    },
    /// May the next request carry its canvas images? Not a step (ADR 0011
    /// item 10); `AgentOutcome::Ok` is yes, anything else is no.
    AuthorizeUpload,
    /// A transcript note: what happened to one canvas image once its request
    /// returned (LCV-187). Not a step; its answer is not read.
    Note(&'a str),
    /// The model answered one request (LCV-193): `captures` is the number of
    /// image parts that request carried with its upload authorised. Not a
    /// step; its answer is not read.
    Replied {
        /// Authorised image parts the answered request carried.
        captures: u32,
    },
}

// ── Error ────────────────────────────────────────────────────────────────────

/// Errors that [`crate::app::run_agent_turn`] can return.
#[derive(Debug)]
pub enum AgentError {
    /// HTTP or serialisation failure from the transport layer.
    Transport(String),
    /// A tool call could not be dispatched.
    ToolDispatch(String),
    /// The loop guard fired. Carries the budget that was in force, so the
    /// message names the real number rather than a constant.
    IterationLimitExceeded(u32),
    /// The endpoint answered with neither content nor tool calls.
    NoContent,
    /// The fence stopped the turn and the model's one last reply asked for
    /// more tool calls anyway (ADR 0007 §D14). None of them was dispatched.
    FenceStopped,
    /// The UI thread stopped answering: the app is closing, the turn was
    /// abandoned, or the frame loop is gone (ADR 0007 §D2). The worker returns
    /// on this **without** sending a terminal event — there is nobody left to
    /// read one, and inventing a row for a turn the app already forgot about
    /// would be the only thing that could show up in a *later* turn's
    /// transcript.
    Cancelled,
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(m) => write!(f, "transport error: {m}"),
            Self::ToolDispatch(m) => write!(f, "tool dispatch error: {m}"),
            Self::IterationLimitExceeded(budget) => {
                write!(f, "step budget exceeded ({budget} tool calls per turn)")
            }
            Self::NoContent => write!(f, "API response contained neither content nor tool calls"),
            Self::Cancelled => write!(f, "the turn was cancelled"),
            Self::FenceStopped => {
                write!(f, "the turn stopped after the drawing changed outside it")
            }
        }
    }
}

impl std::error::Error for AgentError {}

// ── Inner loop (testable seam) ───────────────────────────────────────────────

/// Inner loop with injectable `send_fn` (HTTP) and `dispatch_fn` (tool
/// execution) closures — unit tests substitute stubs without a live endpoint.
///
/// `step_budget` is the number of individual tool-call dispatches this turn may
/// make; the guard fires **before** dispatching any call of a batch that would
/// cross it, so a turn never half-applies a batch it cannot finish. The
/// comparison is done in `usize` — narrower arithmetic could wrap on a large
/// batch and wave it through. An overrunning batch is answered "not run" call
/// by call and the model gets one more reply; a second overrun in a row ends
/// the turn [`AgentError::IterationLimitExceeded`] (ADR 0007 §D13, LCV-189). A
/// batch that runs ends its last result with the steps left.
///
/// `dispatch_fn` receives [`Dispatch::Tool`] and returns the
/// outcome whose text becomes the `tool`-role result. It is the caller's
/// business whether that came from a real mutation, a refusal or a stub; this
/// loop only sequences it — with one exception it reads rather than decides.
/// On [`AgentOutcome::Fenced`] (ADR 0007 §D14) nothing more is dispatched: the
/// rest of the batch gets [`FENCE_STOP_PLACEHOLDER`], exactly one more
/// completion is sent, and its text ends the turn `Ok` while tool calls end it
/// [`AgentError::FenceStopped`].
///
/// An [`AgentOutcome::Observed`] (LCV-145) is a step like any other; its PNG
/// rides after **all** of the batch's tool results in one `user` message,
/// and every send goes through [`send_images`], which notes each image's
/// fate by its call id (LCV-187).
pub(crate) fn agent_loop<F, D>(
    send_fn: &mut F,
    dispatch_fn: &mut D,
    messages: &mut Vec<ChatMessage>,
    step_budget: u32,
) -> Result<String, AgentError>
where
    F: FnMut(&[ChatMessage]) -> Result<AssistantMessage, AgentError>,
    D: FnMut(Dispatch<'_>) -> Result<AgentOutcome, AgentError>,
{
    let budget = usize::try_from(step_budget).unwrap_or(usize::MAX);
    let mut steps = Steps {
        dispatched: 0,
        budget,
        limit: step_budget,
    };
    let mut overran = false;
    // The call ids whose images ride the next send (LCV-187).
    let mut shown: Vec<String> = Vec::new();
    loop {
        let message = send_images(send_fn, dispatch_fn, messages, &mut shown)?;
        match (message.tool_calls, message.content) {
            (Some(calls), content) if !calls.is_empty() => {
                let over = steps.dispatched + calls.len() > budget;
                if over && overran {
                    return Err(AgentError::IterationLimitExceeded(step_budget));
                }
                messages.push(
                    ChatMessage::assistant_with_tool_calls(content, calls.clone())
                        .with_reasoning(message.reasoning_content),
                );
                overran = over;
                if over {
                    let left = budget - steps.dispatched;
                    let text = format!(
                        "not run: this reply has {} tool calls but {left} steps are left",
                        calls.len()
                    );
                    for call in &calls {
                        messages.push(ChatMessage::tool_result(call.id.clone(), text.clone()));
                    }
                    continue;
                }
                let fenced = run_batch(dispatch_fn, messages, &calls, &mut shown, &mut steps)?;
                if fenced {
                    return last_word(send_images(send_fn, dispatch_fn, messages, &mut shown)?);
                }
            }
            (_, Some(text)) => return Ok(text),
            _ => return Err(AgentError::NoContent),
        }
    }
}

/// The one completion a fence-stopped turn is allowed (ADR 0007 §D14): text is
/// the model's report of what it got done; anything else is not dispatched.
fn last_word(message: AssistantMessage) -> Result<String, AgentError> {
    match (message.tool_calls, message.content) {
        (Some(calls), _) if !calls.is_empty() => Err(AgentError::FenceStopped),
        (_, Some(text)) => Ok(text),
        _ => Err(AgentError::NoContent),
    }
}

#[cfg(test)]
mod tests;
