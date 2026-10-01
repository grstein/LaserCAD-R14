//! Every send of the agent loop and what happened to its canvas images (ADR
//! 0011 items 9–10, LCV-187). Split out of `loop_.rs` for the LOC cap.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use super::{AgentError, Dispatch, IMAGE_ELIDED, IMAGE_WITHHELD};
use crate::agent::attachment::ATTACHED_IMAGE_ELIDED;
use crate::agent::bridge::AgentOutcome;
use crate::agent::wire::{AssistantMessage, ChatMessage, replace_images};

/// Send `messages`. `messages[from..]` is what followed the turn's user
/// message; an image there is a canvas capture. A request carrying a capture
/// first asks [`Dispatch::AuthorizeUpload`], once: no — or anything but `Ok`
/// — withholds every capture and sends them as text; a failed ask (cancel)
/// returns before anything is sent. An image in `messages[..from]` is the
/// operator's attached image (LCV-199): the attach was its upload consent, so
/// it neither asks nor counts as a capture. After the send returns, success
/// or error, every image is elided — the attached one to
/// [`ATTACHED_IMAGE_ELIDED`], as memory keeps it — so none outlives its one
/// request, and each call id in `shown` gets one [`Dispatch::Note`] saying
/// whether its image was sent, withheld or not delivered — before an error
/// returns. `shown` is left empty. A send that returned a reply then gets one
/// [`Dispatch::Replied`] with the captures it carried, 0 when withheld
/// (LCV-193); a failed send gets none.
pub(super) fn send_images<F, D>(
    send_fn: &mut F,
    dispatch_fn: &mut D,
    messages: &mut [ChatMessage],
    from: usize,
    shown: &mut Vec<String>,
) -> Result<AssistantMessage, AgentError>
where
    F: FnMut(&[ChatMessage]) -> Result<AssistantMessage, AgentError>,
    D: FnMut(Dispatch<'_>) -> Result<AgentOutcome, AgentError>,
{
    let at = from.min(messages.len());
    let images: usize = messages[at..].iter().map(ChatMessage::image_count).sum();
    let mut withheld = false;
    if images > 0 {
        let verdict = dispatch_fn(Dispatch::AuthorizeUpload)?;
        withheld = !matches!(verdict, AgentOutcome::Ok(_));
        if withheld {
            replace_images(&mut messages[at..], IMAGE_WITHHELD);
        }
    }
    let reply = send_fn(messages);
    replace_images(&mut messages[..at], ATTACHED_IMAGE_ELIDED);
    replace_images(&mut messages[at..], IMAGE_ELIDED);
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
    if reply.is_ok() {
        let captures = if withheld {
            0
        } else {
            u32::try_from(images).unwrap_or(u32::MAX)
        };
        dispatch_fn(Dispatch::Replied { captures })?;
    }
    reply
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::wire::ContentPart;

    fn attached() -> ContentPart {
        ContentPart::image("image/jpeg", &[0xFF, 0xD8, 0xFF])
    }

    /// The turn's user message with an attached image, then — when
    /// `capture` — a canvas capture that followed it.
    fn conversation(capture: bool) -> Vec<ChatMessage> {
        let mut messages = vec![ChatMessage::user_parts(vec![
            ContentPart::text("draw"),
            attached(),
        ])];
        if capture {
            messages.push(ChatMessage::user_parts(vec![
                ContentPart::text("canvas"),
                ContentPart::png(&[1]),
            ]));
        }
        messages
    }

    /// One `send_images` with the user message at index 0; the upload ask
    /// is answered `yes`. Returns every dispatch and the request as sent.
    fn send(messages: &mut [ChatMessage], yes: bool) -> (Vec<String>, Vec<ChatMessage>) {
        let (mut events, mut sent) = (Vec::new(), Vec::new());
        let reply = send_images(
            &mut |m: &[ChatMessage]| {
                sent = m.to_vec();
                Ok(AssistantMessage {
                    content: Some("ok".into()),
                    tool_calls: None,
                    reasoning_content: None,
                })
            },
            &mut |d: Dispatch<'_>| {
                events.push(format!("{d:?}"));
                Ok(if yes || d != Dispatch::AuthorizeUpload {
                    AgentOutcome::Ok(String::new())
                } else {
                    AgentOutcome::Refused(String::new())
                })
            },
            messages,
            1,
            &mut Vec::new(),
        );
        assert!(reply.is_ok());
        (events, sent)
    }

    /// LCV-199 AC 2 / AC 6 — the attached image asks nothing, is not a
    /// capture, rides its one request and is then `image elided`.
    #[test]
    fn an_attached_image_asks_nothing_and_is_elided_after_one_send() {
        let mut messages = conversation(false);
        let (events, sent) = send(&mut messages, false);
        assert_eq!(events, ["Replied { captures: 0 }"]);
        assert_eq!(sent, conversation(false));
        assert_eq!(
            messages[0],
            ChatMessage::user_parts(vec![
                ContentPart::text("draw"),
                ContentPart::text(ATTACHED_IMAGE_ELIDED),
            ])
        );
    }

    /// LCV-199 — a canvas capture after the user message still asks, and
    /// only it counts; once sent each image gets its own placeholder.
    #[test]
    fn a_later_capture_still_asks_and_counts_alone() {
        let mut messages = conversation(true);
        let (events, sent) = send(&mut messages, true);
        assert_eq!(events, ["AuthorizeUpload", "Replied { captures: 1 }"]);
        assert_eq!(sent, conversation(true));
        assert_eq!(
            messages.iter().map(ChatMessage::image_count).sum::<usize>(),
            0
        );
        let elided = serde_json::to_string(&messages).unwrap_or_default();
        assert!(elided.contains(ATTACHED_IMAGE_ELIDED) && elided.contains(IMAGE_ELIDED));
    }

    /// LCV-199 — a refused upload withholds the capture only: the attached
    /// image still goes.
    #[test]
    fn a_refused_upload_withholds_the_capture_not_the_attachment() {
        let mut messages = conversation(true);
        let (events, sent) = send(&mut messages, false);
        assert_eq!(events, ["AuthorizeUpload", "Replied { captures: 0 }"]);
        assert_eq!(sent[0], conversation(false)[0]);
        assert_eq!(sent[1].image_count(), 0);
        assert!(
            serde_json::to_string(&sent[1])
                .unwrap_or_default()
                .contains(IMAGE_WITHHELD)
        );
    }
}
