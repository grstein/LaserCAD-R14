//! LCV-121 — the OpenAI-compatible chat + tool-call wire types.
//!
//! This is the **single declaration site** for the JSON shapes the agent
//! exchanges with an OpenAI-compatible `/chat/completions` endpoint.
//! [`transport`](super::transport) serialises requests and deserialises
//! responses with them, and [`loop_`](super::loop_) builds the conversation out
//! of them; neither keeps a private copy (ADR 0007 §D8).
//!
//! Kernel-pure: `serde` is the only thing this file imports. It describes bytes
//! on a socket, so it knows nothing about HTTP clients, the drawing, or the UI
//! toolkit, and a scan in the test module below keeps it that way.
//!
//! Every `Option` field is skipped when it is `None`, and that is load-bearing:
//! some OpenAI-compatible endpoints reject `"tool_calls": null` on a user turn,
//! so a plain user message has to reach the wire as
//! `{"role":"user","content":"…"}` and nothing else.

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
    /// The text of the turn. `None` on an assistant turn that is nothing but
    /// tool calls.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Tool calls requested by an assistant turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// The [`ToolCall::id`] a `"tool"` turn answers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
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

    /// An `"assistant"` turn carrying tool-call requests, with the optional
    /// text the model sent alongside them.
    pub fn assistant_with_tool_calls(content: Option<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".to_string(),
            content,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    /// A `"tool"` turn answering one tool call.
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }

    /// A plain text turn in `role`.
    fn text(role: &str, content: impl Into<String>) -> Self {
        Self {
            role: role.to_string(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }
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
