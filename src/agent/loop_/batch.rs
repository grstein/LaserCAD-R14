//! One tool-call batch of the agent loop that fits the step budget: dispatch
//! every call, answer each with its `tool` result, end the last one with the
//! steps left, and gather the batch's canvas images. Split out of `loop_.rs`
//! for the LOC cap (LCV-195).
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use super::{AgentError, Dispatch, FENCE_STOP_PLACEHOLDER};
use crate::agent::bridge::AgentOutcome;
use crate::agent::wire::{ChatMessage, ContentPart, ToolCall};

/// The step count of one turn: what was dispatched so far and the budget.
pub(super) struct Steps {
    /// Tool calls dispatched so far this turn.
    pub(super) dispatched: usize,
    /// The budget as a `usize`, for the overflow-free comparison.
    pub(super) budget: usize,
    /// The budget as the operator set it, for the steps-left line.
    pub(super) limit: u32,
}

/// Run `calls`, which the caller has already checked fit `steps`. Pushes one
/// `tool` result per call, then one `user` message carrying the batch's
/// images, if any; each image's call id goes into `shown`. Once a call is
/// answered [`AgentOutcome::Fenced`] the rest get [`FENCE_STOP_PLACEHOLDER`]
/// undispatched. A batch that ran to its end unfenced asks
/// [`Dispatch::Feedback`] once, after its last call, and appends a non-empty
/// answer to the last result before the steps-left line (LCV-195). Returns
/// whether the batch was fenced.
pub(super) fn run_batch<D>(
    dispatch_fn: &mut D,
    messages: &mut Vec<ChatMessage>,
    calls: &[ToolCall],
    shown: &mut Vec<String>,
    steps: &mut Steps,
) -> Result<bool, AgentError>
where
    D: FnMut(Dispatch<'_>) -> Result<AgentOutcome, AgentError>,
{
    let (mut fenced, mut images) = (false, Vec::new());
    for (i, call) in calls.iter().enumerate() {
        let mut result = if fenced {
            FENCE_STOP_PLACEHOLDER.to_owned()
        } else {
            let (name, args) = (&call.function.name, &call.function.arguments);
            let outcome = dispatch_fn(Dispatch::Tool { name, args })?;
            steps.dispatched += 1;
            fenced = outcome.is_fenced();
            match outcome {
                AgentOutcome::Observed { text, png } => {
                    let label = format!("canvas image for tool call {}", call.id);
                    images.extend([ContentPart::text(label), ContentPart::png(&png)]);
                    shown.push(call.id.clone());
                    text
                }
                other => other.into_text(),
            }
        };
        if i + 1 == calls.len() && !fenced {
            let text = dispatch_fn(Dispatch::Feedback)?.into_text();
            if !text.is_empty() {
                result.push('\n');
                result.push_str(&text);
            }
            let left = steps.budget - steps.dispatched;
            result.push_str(&steps_left_line(left, steps.limit));
        }
        messages.push(ChatMessage::tool_result(call.id.clone(), result));
    }
    if !images.is_empty() {
        messages.push(ChatMessage::user_parts(images));
    }
    Ok(fenced)
}

/// The line that ends the last tool result of a batch that ran to its end
/// (LCV-189): what the model may still spend this turn. A fence-stopped
/// batch gets none — its turn has no steps left to plan with.
fn steps_left_line(left: usize, budget: u32) -> String {
    format!("\nSteps left this turn: {left} of {budget}.")
}
