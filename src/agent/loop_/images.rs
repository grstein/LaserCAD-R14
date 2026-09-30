//! Every send of the agent loop and what happened to its canvas images (ADR
//! 0011 items 9–10, LCV-187). Split out of `loop_.rs` for the LOC cap.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use super::{AgentError, Dispatch, IMAGE_ELIDED, IMAGE_WITHHELD};
use crate::agent::bridge::AgentOutcome;
use crate::agent::wire::{AssistantMessage, ChatMessage, replace_images};

/// Send `messages`. A request carrying an image first asks
/// [`Dispatch::AuthorizeUpload`], once: no — or anything but `Ok` — withholds
/// every image and sends text-only; a failed ask (cancel) returns before
/// anything is sent. After the send returns, success or error, every image is
/// elided, so none outlives its one request, and each call id in `shown` gets
/// one [`Dispatch::Note`] saying whether its image was sent, withheld or not
/// delivered — before an error returns. `shown` is left empty.
pub(super) fn send_images<F, D>(
    send_fn: &mut F,
    dispatch_fn: &mut D,
    messages: &mut [ChatMessage],
    shown: &mut Vec<String>,
) -> Result<AssistantMessage, AgentError>
where
    F: FnMut(&[ChatMessage]) -> Result<AssistantMessage, AgentError>,
    D: FnMut(Dispatch<'_>) -> Result<AgentOutcome, AgentError>,
{
    let mut withheld = false;
    if messages.iter().any(ChatMessage::has_image) {
        let verdict = dispatch_fn(Dispatch::AuthorizeUpload)?;
        withheld = !matches!(verdict, AgentOutcome::Ok(_));
        if withheld {
            replace_images(messages, IMAGE_WITHHELD);
        }
    }
    let reply = send_fn(messages);
    replace_images(messages, IMAGE_ELIDED);
    let fate = match (withheld, &reply) {
        (true, _) => "withheld (permission changed)",
        (false, Ok(_)) => "sent",
        (false, Err(_)) => "not delivered (request failed)",
    };
    for id in shown.drain(..) {
        dispatch_fn(Dispatch::Note(&format!(
            "Canvas image for call {id} {fate}."
        )))?;
    }
    reply
}
