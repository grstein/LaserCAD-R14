//! LCV-121 — the OpenAI-compatible chat + tool-call wire types.
//!
//! This is the **single declaration site** for the JSON shapes the agent
//! exchanges with an OpenAI-compatible `/chat/completions` endpoint.
//! [`transport`](super::transport) serialises requests and deserialises
//! responses with them, and [`loop_`](super::loop_) builds the conversation out
//! of them; neither keeps a private copy (ADR 0007 §D8).
//!
//! Kernel-pure: `serde` and `base64` are the only things this file imports. It describes bytes
//! on a socket, so it knows nothing about HTTP clients, the drawing, or the UI
//! toolkit, and a scan in the test module below keeps it that way.
//!
//! Every `Option` field is skipped when it is `None`, and that is load-bearing:
//! some OpenAI-compatible endpoints reject `"tool_calls": null` on a user turn,
//! so a plain user message has to reach the wire as
//! `{"role":"user","content":"…"}` and nothing else.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};

/// The only tool-call `type` the API defines today.
///
/// Written on every request-side tool call, and supplied as the serde default
/// for a response that leaves the field out.
fn function_kind() -> String {
    "function".to_string()
}

/// The name and JSON-encoded arguments of one model-requested function call.
///
/// `arguments` is the **raw string the model produced**, never a parsed value:
/// it is echoed back verbatim in the assistant turn, and interpreting it is the
/// tool registry's job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallFunction {
    /// Tool name; matches one entry of `tools::tool_definitions()`.
    pub name: String,
    /// A JSON object encoded as a string, exactly as the model emitted it.
    pub arguments: String,
}

/// One tool call requested by the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Opaque id minted by the model, echoed back as
    /// [`ChatMessage::tool_call_id`]. Never invented here.
    pub id: String,
    /// Always `"function"`. Required on the request side; defaulted when a
    /// response omits it.
    #[serde(rename = "type", default = "function_kind")]
    pub kind: String,
    /// The call itself.
    pub function: ToolCallFunction,
}

impl ToolCall {
    /// A `"function"` call with `id`, tool `name` and raw `arguments`.
    pub fn function(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: function_kind(),
            function: ToolCallFunction {
                name: name.into(),
                arguments: arguments.into(),
            },
        }
    }
}

/// A single turn of the conversation, in the shape the endpoint expects.
///
/// Build one through the four constructors rather than by hand: they are what
/// guarantee a turn carries exactly the fields its role allows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    /// `"system"`, `"user"`, `"assistant"` or `"tool"`.
    pub role: String,
    /// The text of the turn — or, on a `user` turn that carries canvas
    /// images, its typed parts. `None` on an assistant turn that is nothing
    /// but tool calls.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<Content>,
    /// Tool calls requested by an assistant turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// The [`ToolCall::id`] a `"tool"` turn answers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// A thinking model's reasoning for a tool-call assistant turn, sent back
    /// verbatim (LCV-154). Declared last so every other key keeps its place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
}

impl ChatMessage {
    /// A `"system"` turn.
    pub fn system(content: impl Into<String>) -> Self {
        Self::text("system", content)
    }

    /// A `"user"` turn.
    pub fn user(content: impl Into<String>) -> Self {
        Self::text("user", content)
    }

    /// A plain-text `"assistant"` turn: a final reply, as memory replays it
    /// (LCV-153).
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::text("assistant", content)
    }

    /// An `"assistant"` turn carrying tool-call requests, with the optional
    /// text the model sent alongside them.
    pub fn assistant_with_tool_calls(content: Option<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.map(Content::Text),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
            reasoning_content: None,
        }
    }

    /// This turn with `reasoning_content` set; `None` leaves no key (LCV-154).
    #[must_use]
    pub fn with_reasoning(self, reasoning_content: Option<String>) -> Self {
        Self {
            reasoning_content,
            ..self
        }
    }

    /// A `"tool"` turn answering one tool call.
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(Content::Text(content.into())),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
            reasoning_content: None,
        }
    }

    /// A `"user"` turn made of typed parts (LCV-145: canvas images).
    pub fn user_parts(parts: Vec<ContentPart>) -> Self {
        Self {
            role: "user".to_string(),
            content: Some(Content::Parts(parts)),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }
    }

    /// The turn's text, when it is plain text; `None` for parts or no content.
    pub fn text_content(&self) -> Option<&str> {
        match &self.content {
            Some(Content::Text(text)) => Some(text),
            _ => None,
        }
    }

    /// How many parts of this turn are images.
    pub fn image_count(&self) -> usize {
        match &self.content {
            Some(Content::Parts(parts)) => parts
                .iter()
                .filter(|part| matches!(part, ContentPart::ImageUrl { .. }))
                .count(),
            _ => 0,
        }
    }

    /// A plain text turn in `role`.
    fn text(role: &str, content: impl Into<String>) -> Self {
        Self {
            role: role.to_string(),
            content: Some(Content::Text(content.into())),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }
    }
}

/// The `content` of a [`ChatMessage`]: a bare string — which serialises
/// exactly as `content` did before LCV-145 — or a list of typed parts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    /// Plain text; every turn except a canvas-image `user` turn.
    Text(String),
    /// Typed parts, OpenAI chat-completions form.
    Parts(Vec<ContentPart>),
}

/// One typed part of a multimodal `user` turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    /// `{"type":"text","text":…}`.
    Text {
        /// The text.
        text: String,
    },
    /// `{"type":"image_url","image_url":{"url":…}}`.
    ImageUrl {
        /// The image location; here always a `data:` URL.
        image_url: ImageUrl,
    },
}

/// The `image_url` object of an [`ContentPart::ImageUrl`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageUrl {
    /// `data:image/png;base64,…`.
    pub url: String,
}

impl ContentPart {
    /// A text part.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// An image part carrying `png` as a base64 `data:` URL. This file is the
    /// only place `base64` is used (LCV-145 AC 7).
    pub fn png(png: &[u8]) -> Self {
        let url = format!("data:image/png;base64,{}", STANDARD.encode(png));
        Self::ImageUrl {
            image_url: ImageUrl { url },
        }
    }
}

/// Replace every image part in `messages` with a text part `placeholder`;
/// returns how many were replaced. Used once an image has ridden its one
/// request, or when its upload was not authorised (ADR 0011 items 9–10).
pub fn replace_images(messages: &mut [ChatMessage], placeholder: &str) -> usize {
    let mut replaced = 0;
    for message in messages {
        if let Some(Content::Parts(parts)) = &mut message.content {
            for part in parts.iter_mut() {
                if matches!(part, ContentPart::ImageUrl { .. }) {
                    *part = ContentPart::text(placeholder);
                    replaced += 1;
                }
            }
        }
    }
    replaced
}

/// The assistant half of one [`Choice`] — what the endpoint answered.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantMessage {
    /// The reply text. `None` when the model only asked for tools.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// The tool calls the model wants dispatched, in the order it sent them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// A thinking model's reasoning, when it sent a string (LCV-154).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
}

/// One completion choice. Only `choices[0]` is ever read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    /// The assistant message of this choice.
    pub message: AssistantMessage,
}

/// The body of one chat-completion response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Completion choices; an empty list is a protocol failure, not a reply.
    pub choices: Vec<Choice>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The implementation section of this file: everything before the bare
    /// `#[cfg(test)]` at column 0. Scanning the whole file would match this
    /// test module's own needles.
    fn implementation() -> &'static str {
        let src = include_str!("wire.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("wire.rs must have a bare #[cfg(test)] marker to bound the scan");
        &src[..at]
    }

    /// AC 1 — a plain user turn serialises with no `null` keys at all.
    #[test]
    fn user_turn_serialises_to_role_and_content_only() {
        let json = serde_json::to_string(&ChatMessage::user("hi")).unwrap();
        assert_eq!(json, r#"{"role":"user","content":"hi"}"#);
    }

    /// AC 1 — a system turn is equally bare.
    #[test]
    fn system_turn_serialises_to_role_and_content_only() {
        let json = serde_json::to_string(&ChatMessage::system("be brief")).unwrap();
        assert_eq!(json, r#"{"role":"system","content":"be brief"}"#);
    }

    /// AC 1 — a tool result carries `role`, `content` and `tool_call_id`, and
    /// nothing else.
    #[test]
    fn tool_result_serialises_role_content_and_id_only() {
        let json = serde_json::to_string(&ChatMessage::tool_result("call_7", "ok")).unwrap();
        assert_eq!(
            json,
            r#"{"role":"tool","content":"ok","tool_call_id":"call_7"}"#
        );
    }

    /// AC 1 — an assistant turn with tool calls round-trips unchanged, `type`
    /// included, and carries no `tool_call_id` key.
    #[test]
    fn assistant_with_tool_calls_round_trips() {
        let original = ChatMessage::assistant_with_tool_calls(
            None,
            vec![ToolCall::function(
                "call_1",
                "create_line",
                r#"{"x1":0,"y1":0,"x2":20,"y2":0}"#,
            )],
        );
        let json = serde_json::to_string(&original).unwrap();
        assert!(
            json.contains(r#""type":"function""#),
            "the request side needs the type field, got {json}"
        );
        assert!(!json.contains("tool_call_id"), "got {json}");
        assert!(!json.contains("content"), "got {json}");

        let back: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(back, original);
    }

    /// AC 1 — the raw `arguments` string survives a round trip byte for byte,
    /// including the whitespace the model chose.
    #[test]
    fn raw_arguments_survive_a_round_trip() {
        let raw = r#"{ "cx": 1.5, "cy": -2.0, "r": 3.25 }"#;
        let call = ToolCall::function("call_9", "create_circle", raw);
        let back: ToolCall = serde_json::from_str(&serde_json::to_string(&call).unwrap()).unwrap();
        assert_eq!(back.function.arguments, raw);
        assert_eq!(back.kind, "function");
    }

    /// AC 1 — a response that omits `type` still deserialises, and a message
    /// with `content: null` keeps its tool calls.
    #[test]
    fn response_without_type_field_deserialises() {
        let body = r#"{"choices":[{"message":{"content":null,"tool_calls":[
            {"id":"c1","function":{"name":"create_line","arguments":"{}"}}]}}]}"#;
        let parsed: ChatResponse = serde_json::from_str(body).unwrap();
        let msg = &parsed.choices[0].message;
        assert!(msg.content.is_none());
        let calls = msg.tool_calls.as_ref().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].kind, "function");
        assert_eq!(calls[0].function.name, "create_line");
    }

    /// LCV-145 AC 9 — today's text-only requests, pinned as literal bytes
    /// before `content` became `Option<Content>`: the type change must not
    /// move one byte of any of them.
    #[test]
    fn text_only_messages_serialise_byte_identically() {
        let calls = vec![ToolCall::function("call_1", "create_line", r#"{"x1":0}"#)];
        for (message, pinned) in [
            (
                ChatMessage::system("be brief"),
                r#"{"role":"system","content":"be brief"}"#,
            ),
            (
                ChatMessage::user("draw a line"),
                r#"{"role":"user","content":"draw a line"}"#,
            ),
            (
                ChatMessage::assistant_with_tool_calls(None, calls.clone()),
                r#"{"role":"assistant","tool_calls":[{"id":"call_1","type":"function","function":{"name":"create_line","arguments":"{\"x1\":0}"}}]}"#,
            ),
            (
                ChatMessage::assistant_with_tool_calls(Some("Let me look.".into()), calls),
                r#"{"role":"assistant","content":"Let me look.","tool_calls":[{"id":"call_1","type":"function","function":{"name":"create_line","arguments":"{\"x1\":0}"}}]}"#,
            ),
            (
                ChatMessage::tool_result("call_1", "Line created."),
                r#"{"role":"tool","content":"Line created.","tool_call_id":"call_1"}"#,
            ),
        ] {
            assert_eq!(serde_json::to_string(&message).unwrap(), pinned);
        }
    }

    /// LCV-154 AC 1/5 — `reasoning_content` parses when it is a string and is
    /// `None` when missing or `null`; `None` adds no key, `Some` goes last.
    #[test]
    fn reasoning_content_parses_and_serialises_last_or_not_at_all() {
        let parse = |body: &str| -> Option<String> {
            serde_json::from_str::<AssistantMessage>(body)
                .unwrap()
                .reasoning_content
        };
        assert_eq!(
            parse(r#"{"content":"a","reasoning_content":"R"}"#),
            Some("R".to_owned())
        );
        assert_eq!(parse(r#"{"content":"a"}"#), None);
        assert_eq!(parse(r#"{"content":"a","reasoning_content":null}"#), None);
        let calls = vec![ToolCall::function("call_1", "create_line", r#"{"x1":0}"#)];
        let pinned = r#"{"role":"assistant","tool_calls":[{"id":"call_1","type":"function","function":{"name":"create_line","arguments":"{\"x1\":0}"}}]}"#;
        let bare = ChatMessage::assistant_with_tool_calls(None, calls.clone()).with_reasoning(None);
        assert_eq!(serde_json::to_string(&bare).unwrap(), pinned);
        let with =
            ChatMessage::assistant_with_tool_calls(None, calls).with_reasoning(Some("R".into()));
        let json = serde_json::to_string(&with).unwrap();
        assert_eq!(
            json,
            format!(
                "{},\"reasoning_content\":\"R\"}}",
                &pinned[..pinned.len() - 1]
            )
        );
        assert_eq!(serde_json::from_str::<ChatMessage>(&json).unwrap(), with);
    }

    /// LCV-145 AC 9 — a parts message has the OpenAI chat-completions shape:
    /// typed `text` and `image_url` parts, the image a base64 PNG data URL.
    #[test]
    fn parts_message_serialises_to_the_openai_shape() {
        let message = ChatMessage::user_parts(vec![
            ContentPart::text("canvas image for tool call call_2"),
            ContentPart::png(&[0x89, b'P', b'N', b'G']),
        ]);
        assert_eq!(
            serde_json::to_string(&message).unwrap(),
            concat!(
                r#"{"role":"user","content":["#,
                r#"{"type":"text","text":"canvas image for tool call call_2"},"#,
                r#"{"type":"image_url","image_url":{"url":"data:image/png;base64,iVBORw=="}}"#,
                r#"]}"#
            )
        );
        assert_eq!(message.image_count(), 1);
        assert_eq!(message.text_content(), None);
        let back: ChatMessage =
            serde_json::from_str(&serde_json::to_string(&message).unwrap()).unwrap();
        assert_eq!(back, message);
    }

    /// LCV-145 AC 10/11 — `replace_images` swaps every image part, in every
    /// message, for a text placeholder, and leaves everything else alone.
    #[test]
    fn replace_images_swaps_only_image_parts() {
        let mut messages = vec![
            ChatMessage::user("u"),
            ChatMessage::user_parts(vec![
                ContentPart::text("a"),
                ContentPart::png(b"1"),
                ContentPart::text("b"),
                ContentPart::png(b"2"),
            ]),
        ];
        let before_text = messages[0].clone();
        assert_eq!(replace_images(&mut messages, "gone"), 2);
        assert_eq!(messages[0], before_text);
        assert!(messages.iter().all(|m| m.image_count() == 0));
        assert_eq!(
            messages[1],
            ChatMessage::user_parts(vec![
                ContentPart::text("a"),
                ContentPart::text("gone"),
                ContentPart::text("b"),
                ContentPart::text("gone"),
            ])
        );
        assert_eq!(replace_images(&mut messages, "gone"), 0, "idempotent");
        assert_eq!(messages[0].text_content(), Some("u"));
        assert_eq!(ChatMessage::user("u").image_count(), 0);
        assert_eq!(
            ChatMessage::user_parts(vec![ContentPart::text("t")]).image_count(),
            0
        );
        let two = [
            ContentPart::png(&[1]),
            ContentPart::text("t"),
            ContentPart::png(&[2]),
        ];
        assert_eq!(ChatMessage::user_parts(two.to_vec()).image_count(), 2);
    }

    /// AC 3 — the wire types stay kernel-pure. Bounded to the implementation
    /// section, needles built with `concat!` so the scan cannot match the
    /// literals in this test, and each absence is backed by a positive control
    /// over the same slice: if the scan were looking at the wrong bytes, the
    /// control would fail first.
    #[test]
    fn wire_is_kernel_pure() {
        let implementation = implementation();
        for control in [
            concat!("Tool", "CallFunction"),
            concat!("Assistant", "Message"),
            concat!("ser", "de"),
        ] {
            assert!(
                implementation.contains(control),
                "positive control: the scanned slice must contain {control}"
            );
        }
        for forbidden in [
            concat!("req", "west"),
            concat!("eg", "ui"),
            concat!("ef", "rame"),
            concat!("rf", "d"),
            concat!("crate::", "document"),
        ] {
            assert!(
                !implementation.contains(forbidden),
                "wire.rs must not mention {forbidden} outside its test module"
            );
        }
    }
}
