//! LCV-077 / LCV-121 — Agent HTTP transport.
//!
//! This module is the **sole HTTP boundary** of the crate: it is the only file
//! that may import `reqwest` (ADR 0007 §D8). Every other agent module stays
//! HTTP-unaware and calls into here.
//!
//! Public items:
//! - [`TransportError`] — typed error returned by the transport function.
//! - [`chat_completion`] — one blocking POST to an OpenAI-compatible
//!   `/chat/completions` endpoint; carries the tool schemas up and the
//!   assistant message — text, tool calls, or both — back down.
//!
//! The wire shapes live in [`crate::agent::wire`]; nothing here declares its
//! own copy. The API key is written into the `Authorization` header and
//! nowhere else: no variant of [`TransportError`] carries it, and the request
//! is never logged.

use crate::agent::wire::{AssistantMessage, ChatMessage, ChatResponse};

/// Errors that can occur when calling the OpenAI-compatible chat endpoint.
///
/// Response bodies are truncated to the first 1 024 characters; the request —
/// which holds the API key — never appears in any of these.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// Network-level or connection error (DNS, TLS, timeout, etc.).
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// HTTP 401: the key is missing, wrong, or refused by this endpoint.
    #[error("Authentication failed (HTTP 401): the API key is missing, invalid, or not accepted by this endpoint. Check Help > Agent settings.")]
    Unauthorized,

    /// HTTP 429: the endpoint is asking the caller to slow down.
    #[error("Rate limited (HTTP 429): the endpoint asked you to slow down. Wait a moment and try again.")]
    RateLimited,

    /// HTTP 5xx: the endpoint itself failed.
    #[error("The endpoint failed (HTTP {status}): {body}")]
    ServerError {
        /// The 5xx status code as received.
        status: u16,
        /// Response body, truncated to 1 024 characters.
        body: String,
    },

    /// Any other non-2xx status.
    #[error("HTTP {status}: {body}")]
    Http {
        /// The status code as received.
        status: u16,
        /// Response body, truncated to 1 024 characters.
        body: String,
    },

    /// Response body could not be parsed as the expected JSON shape.
    #[error("JSON parse error: {0}")]
    Parse(#[from] serde_json::Error),

    /// `choices` is empty, or `choices[0].message` carries neither `content`
    /// nor a non-empty `tool_calls` array.
    #[error("response missing choices[0].message content and tool calls")]
    MissingContent,
}

/// The body of one `/chat/completions` request.
#[derive(serde::Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<&'a serde_json::Value>,
}

/// The `tools` value to put on the wire, or `None` when there is nothing to
/// offer.
///
/// `Value::Null`, an empty array, and anything that is not an array at all are
/// left **off** the body rather than sent as `"tools": null` — several
/// OpenAI-compatible endpoints reject that outright.
fn tools_for_request(tools: &serde_json::Value) -> Option<&serde_json::Value> {
    match tools.as_array() {
        Some(list) if !list.is_empty() => Some(tools),
        _ => None,
    }
}

/// Map a non-2xx status onto the error the operator should read.
fn status_error(status: u16, body: String) -> TransportError {
    match status {
        401 => TransportError::Unauthorized,
        429 => TransportError::RateLimited,
        500..=599 => TransportError::ServerError { status, body },
        _ => TransportError::Http { status, body },
    }
}

/// POST one conversation to an OpenAI-compatible `/chat/completions` endpoint
/// and return the assistant message it answered with.
///
/// `messages` goes on the wire verbatim — an assistant turn keeps its
/// `tool_calls` and a `"tool"` turn keeps its `tool_call_id`, so the model sees
/// a conversation with no holes in it. `tools` is the schema array from
/// `tools::tool_definitions()`; pass [`serde_json::Value::Null`] to offer none.
///
/// A reply with `"content": null` and a populated `tool_calls` array is a
/// **success**: [`TransportError::MissingContent`] is returned only when there
/// is neither text nor a tool call to act on.
///
/// `endpoint` may or may not have a trailing slash; it is normalised
/// automatically. A new [`reqwest::blocking::Client`] is created per call
/// (connection pooling is deliberately out of scope).
pub fn chat_completion(
    endpoint: &str,
    api_key: &str,
    model: &str,
    messages: &[ChatMessage],
    tools: &serde_json::Value,
) -> Result<AssistantMessage, TransportError> {
    let url = format!("{}/chat/completions", endpoint.trim_end_matches('/'));
    let body = ChatRequest {
        model,
        messages,
        tools: tools_for_request(tools),
    };

    let client = reqwest::blocking::Client::new();
    let response = client.post(&url).bearer_auth(api_key).json(&body).send()?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().unwrap_or_default();
        let truncated: String = text.chars().take(1024).collect();
        return Err(status_error(status.as_u16(), truncated));
    }

    let text = response.text()?;
    let parsed: ChatResponse = serde_json::from_str(&text)?;

    let message = parsed
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message)
        .ok_or(TransportError::MissingContent)?;

    let has_calls = message
        .tool_calls
        .as_ref()
        .is_some_and(|calls| !calls.is_empty());
    if message.content.is_none() && !has_calls {
        return Err(TransportError::MissingContent);
    }
    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::wire::ToolCall;
    use serde_json::{json, Value};
    use std::sync::{Arc, Mutex};

    /// A recognisable stand-in for the operator's key. Built with `concat!` so
    /// the absence assertions in the status tests cannot match this literal by
    /// scanning their own source.
    const DUMMY_KEY: &str = concat!("sk-test-", "DO-NOT-LEAK");

    /// Captures every request body mockito receives, in arrival order.
    ///
    /// `matcher()` always answers `true`, so it changes nothing about which
    /// mock is selected; it is a tap on the wire, not a filter.
    #[derive(Clone, Default)]
    struct Bodies(Arc<Mutex<Vec<String>>>);

    impl Bodies {
        fn matcher(&self) -> impl Fn(&mockito::Request) -> bool + Send + Sync + 'static {
            let sink = self.0.clone();
            move |request| {
                let body = request
                    .utf8_lossy_body()
                    .map(|b| b.into_owned())
                    .unwrap_or_default();
                sink.lock().expect("recorder mutex").push(body);
                true
            }
        }

        /// The `n`-th captured body, parsed as JSON.
        fn json(&self, n: usize) -> Value {
            let captured = self.0.lock().expect("recorder mutex");
            let raw = captured.get(n).unwrap_or_else(|| {
                panic!(
                    "expected at least {} request(s), saw {}",
                    n + 1,
                    captured.len()
                )
            });
            serde_json::from_str(raw).expect("the request body must be JSON")
        }

        fn len(&self) -> usize {
            self.0.lock().expect("recorder mutex").len()
        }
    }

    fn ok_text(text: &str) -> String {
        json!({"choices": [{"message": {"role": "assistant", "content": text}}]}).to_string()
    }

    // ── AC 4: the request carries the tools ──────────────────────────────────

    /// AC 4 — a non-empty tools array reaches the body with every entry, and
    /// the request is authenticated.
    ///
    /// The `Authorization` header is asserted **here**, on the one mock that
    /// already inspects a real request, rather than in a test of its own: until
    /// LCV-123 nothing pinned it, and deleting `.bearer_auth(…)` from
    /// `chat_completion` left the whole suite green. `match_header` makes the
    /// mock refuse to answer an unauthenticated request, so the mutant fails on
    /// `result.is_ok()` below. The expected value is assembled with `concat!`
    /// so no grep for the dummy key finds a whole one in this file.
    #[test]
    fn tools_array_is_sent_in_the_request_body() {
        let mut server = mockito::Server::new();
        let bodies = Bodies::default();
        let _mock = server
            .mock("POST", "/chat/completions")
            .match_header(
                "authorization",
                concat!("Bearer ", "sk-test-", "DO-NOT-LEAK"),
            )
            .match_request(bodies.matcher())
            .with_status(200)
            .with_body(ok_text("ok"))
            .create();

        let tools = crate::agent::tools::tool_definitions();
        assert_eq!(tools.as_array().map(|a| a.len()), Some(7), "fixture check");

        let result = chat_completion(
            &server.url(),
            DUMMY_KEY,
            "m",
            &[ChatMessage::user("x")],
            &tools,
        );
        assert!(result.is_ok(), "got {result:?}");

        let body = bodies.json(0);
        assert_eq!(
            body["tools"].as_array().map(|a| a.len()),
            Some(7),
            "AC 4: all seven tool schemas must reach the wire, body was {body}"
        );
        assert_eq!(body["tools"][0]["function"]["name"], "create_line");
    }

    /// AC 4 — `Null` and `[]` leave the field off the body entirely; a
    /// `"tools": null` key is rejected by some endpoints.
    #[test]
    fn empty_tools_omits_the_field() {
        for tools in [Value::Null, json!([])] {
            let mut server = mockito::Server::new();
            let bodies = Bodies::default();
            let _mock = server
                .mock("POST", "/chat/completions")
                .match_request(bodies.matcher())
                .with_status(200)
                .with_body(ok_text("ok"))
                .create();

            let result = chat_completion(
                &server.url(),
                DUMMY_KEY,
                "m",
                &[ChatMessage::user("x")],
                &tools,
            );
            assert!(result.is_ok(), "got {result:?}");

            let body = bodies.json(0);
            assert!(
                body.get("tools").is_none(),
                "AC 4: {tools} must not put a tools key on the wire, body was {body}"
            );
            assert_eq!(body["model"], "m", "the rest of the body is still there");
        }
    }

    // ── AC 5: tool calls come back ───────────────────────────────────────────

    /// AC 5 — a reply with `content: null` and one tool call is a success, and
    /// the name plus the raw argument string survive verbatim.
    #[test]
    fn single_tool_call_is_returned_without_content() {
        let raw_args = r#"{"x1":0,"y1":0,"x2":20.5,"y2":0}"#;
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(
                json!({"choices": [{"message": {
                    "role": "assistant",
                    "content": Value::Null,
                    "tool_calls": [{
                        "id": "call_abc",
                        "type": "function",
                        "function": {"name": "create_line", "arguments": raw_args}
                    }]
                }}]})
                .to_string(),
            )
            .create();

        let message = chat_completion(
            &server.url(),
            DUMMY_KEY,
            "m",
            &[ChatMessage::user("draw")],
            &Value::Null,
        )
        .expect("tool calls without content are a success, not MissingContent");

        assert!(message.content.is_none());
        let calls = message.tool_calls.expect("tool_calls must survive");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_abc");
        assert_eq!(calls[0].function.name, "create_line");
        assert_eq!(calls[0].function.arguments, raw_args);
    }

    /// AC 5 — an empty `choices` array is `MissingContent`.
    #[test]
    fn empty_choices_is_missing_content() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(r#"{"choices":[]}"#)
            .create();

        let result = chat_completion(
            &server.url(),
            DUMMY_KEY,
            "m",
            &[ChatMessage::user("x")],
            &Value::Null,
        );
        assert!(matches!(result, Err(TransportError::MissingContent)));
    }

    /// AC 5 — neither content nor a usable tool call is `MissingContent`;
    /// an empty `tool_calls` array counts as none.
    #[test]
    fn neither_content_nor_tool_calls_is_missing_content() {
        for body in [
            r#"{"choices":[{"message":{"role":"assistant","content":null}}]}"#,
            r#"{"choices":[{"message":{"role":"assistant","content":null,"tool_calls":[]}}]}"#,
        ] {
            let mut server = mockito::Server::new();
            let _mock = server
                .mock("POST", "/chat/completions")
                .with_status(200)
                .with_body(body)
                .create();

            let result = chat_completion(
                &server.url(),
                DUMMY_KEY,
                "m",
                &[ChatMessage::user("x")],
                &Value::Null,
            );
            assert!(
                matches!(result, Err(TransportError::MissingContent)),
                "{body} must be MissingContent, got {result:?}"
            );
        }
    }

    /// AC 5 — plain text still comes back as text.
    #[test]
    fn plain_text_reply_is_returned() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(ok_text("G'day"))
            .create();

        let message = chat_completion(
            &server.url(),
            DUMMY_KEY,
            "m",
            &[ChatMessage::user("Hi")],
            &Value::Null,
        )
        .unwrap();
        assert_eq!(message.content.as_deref(), Some("G'day"));
        assert!(message.tool_calls.is_none());
    }

    // ── AC 6: the conversation is sent losslessly ────────────────────────────

    /// AC 6 — an assistant turn keeps its `tool_calls` and a `tool` turn keeps
    /// its `tool_call_id` all the way to the wire. The transport half of the
    /// lossless guarantee; `loop_.rs` covers the round-trip half.
    #[test]
    fn assistant_and_tool_turns_reach_the_wire_intact() {
        let mut server = mockito::Server::new();
        let bodies = Bodies::default();
        let _mock = server
            .mock("POST", "/chat/completions")
            .match_request(bodies.matcher())
            .with_status(200)
            .with_body(ok_text("ok"))
            .create();

        let call = ToolCall::function("call_42", "create_line", "{}");
        let conversation = [
            ChatMessage::system("s"),
            ChatMessage::user("draw"),
            ChatMessage::assistant_with_tool_calls(None, vec![call]),
            ChatMessage::tool_result("call_42", "Line created."),
        ];
        let result = chat_completion(&server.url(), DUMMY_KEY, "m", &conversation, &Value::Null);
        assert!(result.is_ok(), "got {result:?}");

        let body = bodies.json(0);
        let msgs = body["messages"].as_array().expect("messages array");
        assert_eq!(msgs.len(), 4, "no turn may be dropped, body was {body}");
        assert_eq!(msgs[2]["role"], "assistant");
        assert_eq!(msgs[2]["tool_calls"][0]["id"], "call_42");
        assert_eq!(msgs[3]["role"], "tool");
        assert_eq!(msgs[3]["tool_call_id"], "call_42");
        assert_eq!(msgs[3]["content"], "Line created.");
        assert!(
            msgs[1].get("tool_calls").is_none(),
            "a user turn must not carry a null tool_calls key, body was {body}"
        );
    }

    // ── AC 7 / AC 8: readable status errors that never leak the key ──────────

    /// AC 7 / AC 8 — 401 is `Unauthorized` and points at the settings dialog.
    #[test]
    fn status_401_is_unauthorized() {
        let err = status_case(401, "bad key");
        assert!(
            matches!(err, TransportError::Unauthorized),
            "401 must not fall through to the generic arm, got {err:?}"
        );
        let shown = err.to_string();
        assert!(
            shown.contains("Authentication failed (HTTP 401)"),
            "{shown}"
        );
        assert!(shown.contains("Check Help > Agent settings."), "{shown}");
        assert_no_key(&shown);
    }

    /// AC 7 / AC 8 — 429 is `RateLimited`.
    #[test]
    fn status_429_is_rate_limited() {
        let err = status_case(429, "slow down");
        assert!(
            matches!(err, TransportError::RateLimited),
            "429 must not fall through to the generic arm, got {err:?}"
        );
        let shown = err.to_string();
        assert!(shown.contains("Rate limited (HTTP 429)"), "{shown}");
        assert!(shown.contains("Wait a moment and try again."), "{shown}");
        assert_no_key(&shown);
    }

    /// AC 7 / AC 8 — 5xx is `ServerError` and shows the status and the body.
    #[test]
    fn status_500_is_server_error() {
        let err = status_case(503, "upstream on fire");
        match &err {
            TransportError::ServerError { status, body } => {
                assert_eq!(*status, 503);
                assert_eq!(body, "upstream on fire");
            }
            other => panic!("5xx must be ServerError, got {other:?}"),
        }
        let shown = err.to_string();
        assert_eq!(shown, "The endpoint failed (HTTP 503): upstream on fire");
        assert_no_key(&shown);
    }

    /// AC 7 / AC 8 — the control: any other non-2xx keeps the generic arm.
    #[test]
    fn status_418_stays_generic_http() {
        let err = status_case(418, "teapot");
        match &err {
            TransportError::Http { status, body } => {
                assert_eq!(*status, 418);
                assert_eq!(body, "teapot");
            }
            other => panic!("418 must stay generic Http, got {other:?}"),
        }
        assert_eq!(err.to_string(), "HTTP 418: teapot");
        assert_no_key(&err.to_string());
    }

    /// Drive one non-2xx response through the real transport with the dummy
    /// key, and hand back the error it produced.
    fn status_case(status: usize, body: &str) -> TransportError {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(status)
            .with_body(body)
            .create();

        chat_completion(
            &server.url(),
            DUMMY_KEY,
            "m",
            &[ChatMessage::user("x")],
            &Value::Null,
        )
        .expect_err("a non-2xx status must be an error")
    }

    /// AC 8 — the key must not be anywhere in what the operator is shown.
    fn assert_no_key(shown: &str) {
        assert!(
            !shown.contains(DUMMY_KEY),
            "the API key must never appear in an error: {shown}"
        );
    }

    /// AC 8 — the truncation that keeps a huge error body readable is still in
    /// force, and it does not turn into a panic on a multi-byte boundary.
    #[test]
    fn oversized_error_body_is_truncated_to_1024_chars() {
        let err = status_case(502, &"é".repeat(4000));
        match err {
            TransportError::ServerError { body, .. } => assert_eq!(body.chars().count(), 1024),
            other => panic!("expected ServerError, got {other:?}"),
        }
    }

    // ── AC 9: the model id comes from the caller ─────────────────────────────

    /// AC 9 — whatever model id the caller passes is what reaches the body.
    /// Two different values, so a hardcoded default cannot satisfy both.
    #[test]
    fn model_id_on_the_wire_is_the_one_passed_in() {
        for model in ["anthropic/claude-sonnet-4.6", "some/other-model"] {
            let mut server = mockito::Server::new();
            let bodies = Bodies::default();
            let _mock = server
                .mock("POST", "/chat/completions")
                .match_request(bodies.matcher())
                .with_status(200)
                .with_body(ok_text("ok"))
                .create();

            let result = chat_completion(
                &server.url(),
                DUMMY_KEY,
                model,
                &[ChatMessage::user("x")],
                &Value::Null,
            );
            assert!(result.is_ok(), "got {result:?}");
            assert_eq!(bodies.len(), 1);
            assert_eq!(bodies.json(0)["model"], model);
        }
    }

    // ── Regression cover carried over from LCV-077 ───────────────────────────

    /// A body that is not JSON is a parse error, not a panic.
    #[test]
    fn invalid_json_body_returns_parse_error() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body("not json at all")
            .create();

        let result = chat_completion(
            &server.url(),
            DUMMY_KEY,
            "m",
            &[ChatMessage::user("x")],
            &Value::Null,
        );
        assert!(matches!(result, Err(TransportError::Parse(_))));
    }

    /// A trailing slash on the endpoint is normalised away.
    #[test]
    fn trailing_slash_stripped_from_endpoint() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(ok_text("ok"))
            .expect(1)
            .create();

        let url_with_slash = format!("{}/", server.url());
        let result = chat_completion(
            &url_with_slash,
            DUMMY_KEY,
            "m",
            &[ChatMessage::user("x")],
            &Value::Null,
        );
        assert!(result.is_ok(), "got {result:?}");
        mock.assert();
    }

    /// An unreachable endpoint is a `Request` error, not a panic.
    #[test]
    fn unreachable_endpoint_is_a_request_error() {
        let result = chat_completion(
            "http://127.0.0.1:1",
            DUMMY_KEY,
            "m",
            &[ChatMessage::user("x")],
            &Value::Null,
        );
        assert!(matches!(result, Err(TransportError::Request(_))));
        assert_no_key(&result.unwrap_err().to_string());
    }

    // ── AC 2: the wire types are declared once, in wire.rs ───────────────────

    /// AC 2 — no duplicate wire struct survives in this file, and the shared
    /// declarations are imported instead. Bounded to the implementation
    /// section and built with `concat!`, so the scan cannot match the needles
    /// written here; pasting any of those structs back above the
    /// `#[cfg(test)]` marker turns this red.
    #[test]
    fn transport_declares_no_wire_structs_of_its_own() {
        let src = include_str!("transport.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("transport.rs must have a bare #[cfg(test)] marker");
        let implementation = &src[..at];

        let import = concat!("use crate::agent::", "wire");
        assert!(
            implementation.contains(import),
            "positive control: transport.rs must import the shared wire types"
        );
        for duplicate in [
            concat!("struct ", "ChatResponse"),
            concat!("struct ", "Choice"),
            concat!("struct ", "ChoiceMessage"),
            concat!("struct ", "AssistantMessage"),
            concat!("struct ", "ToolCall"),
            concat!("struct ", "ToolCallFunction"),
        ] {
            assert!(
                !implementation.contains(duplicate),
                "`{duplicate}` belongs in wire.rs, not transport.rs"
            );
        }
        assert!(
            !implementation.contains(concat!("struct ", "Message")),
            "transport::Message is retired; the conversation type is wire::ChatMessage"
        );
    }
}
