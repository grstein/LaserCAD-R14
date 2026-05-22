//! LCV-077 — Agent HTTP transport.
//!
//! This module is the **sole HTTP boundary** of the agent subsystem.  Every
//! other agent module stays HTTP-unaware and calls into here.
//!
//! Public items:
//! - [`Message`] — a single chat turn (role + content).
//! - [`TransportError`] — typed error returned by the transport function.
//! - [`chat_completion`] — synchronous POST to an OpenAI-compatible
//!   `/chat/completions` endpoint; returns the assistant content string.

/// A single message in a chat conversation.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    /// Create a `"user"` turn.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }

    /// Create an `"assistant"` turn.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }
}

/// Errors that can occur when calling the OpenAI-compatible chat endpoint.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// Network-level or connection error (DNS, TLS, timeout, etc.).
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// Server responded with a non-2xx status.
    /// `body` is the response body text, truncated to 1 024 bytes.
    #[error("HTTP {status}: {body}")]
    Http { status: u16, body: String },

    /// Response body could not be parsed as the expected JSON shape.
    #[error("JSON parse error: {0}")]
    Parse(#[from] serde_json::Error),

    /// `choices` array is empty or `choices[0].message.content` is null/absent.
    #[error("response missing choices[0].message.content")]
    MissingContent,
}

#[derive(serde::Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [Message],
}

#[derive(serde::Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(serde::Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(serde::Deserialize)]
struct ChoiceMessage {
    content: Option<String>,
}

/// POST `messages` to an OpenAI-compatible `/chat/completions` endpoint and
/// return the assistant's reply text.
///
/// `endpoint` may or may not have a trailing slash; it is normalised
/// automatically.  A new [`reqwest::blocking::Client`] is created per call
/// (connection pooling is deferred to LCV-079).
pub fn chat_completion(
    endpoint: &str,
    api_key: &str,
    model: &str,
    messages: &[Message],
) -> Result<String, TransportError> {
    let url = format!("{}/chat/completions", endpoint.trim_end_matches('/'));
    let body = ChatRequest { model, messages };

    let client = reqwest::blocking::Client::new();
    let response = client.post(&url).bearer_auth(api_key).json(&body).send()?;

    let status = response.status();
    if !status.is_success() {
        let code = status.as_u16();
        let text = response.text().unwrap_or_default();
        let body_str: String = text.chars().take(1024).collect();
        return Err(TransportError::Http {
            status: code,
            body: body_str,
        });
    }

    let text = response.text()?;
    let parsed: ChatResponse = serde_json::from_str(&text)?;

    match parsed.choices.into_iter().next() {
        Some(choice) => choice.message.content.ok_or(TransportError::MissingContent),
        None => Err(TransportError::MissingContent),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── AC#3: Message helpers ────────────────────────────────────────────────

    #[test]
    fn message_user_helper() {
        let m = Message::user("x");
        assert_eq!(m.role, "user");
        assert_eq!(m.content, "x");
    }

    #[test]
    fn message_assistant_helper() {
        let m = Message::assistant("y");
        assert_eq!(m.role, "assistant");
        assert_eq!(m.content, "y");
    }

    // ── AC#4: TransportError Display ────────────────────────────────────────

    #[test]
    fn error_http_display() {
        let e = TransportError::Http {
            status: 401,
            body: "Unauthorized".into(),
        };
        assert_eq!(format!("{e}"), "HTTP 401: Unauthorized");
    }

    #[test]
    fn error_missing_content_display() {
        let e = TransportError::MissingContent;
        assert!(format!("{e}").contains("choices[0].message.content"));
    }

    // ── AC#6: happy path ─────────────────────────────────────────────────────

    #[test]
    fn happy_path_returns_content() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"choices":[{"message":{"role":"assistant","content":"G'day"}}]}"#)
            .create();

        let result = chat_completion(&server.url(), "key", "gpt-4o-mini", &[Message::user("Hi")]);
        assert_eq!(result.unwrap(), "G'day");
    }

    // AC#7: request body contains model and messages
    #[test]
    fn request_body_contains_model_and_messages() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"choices":[{"message":{"role":"assistant","content":"Hi"}}]}"#)
            .match_body(mockito::Matcher::AllOf(vec![
                mockito::Matcher::PartialJsonString(r#"{"model":"test-model"}"#.into()),
                mockito::Matcher::PartialJsonString(
                    r#"{"messages":[{"role":"user","content":"Hello"}]}"#.into(),
                ),
            ]))
            .create();

        let result = chat_completion(
            &server.url(),
            "key",
            "test-model",
            &[Message::user("Hello")],
        );
        assert!(result.is_ok());
        _mock.assert();
    }

    // ── AC#8: 401 returns Http error ─────────────────────────────────────────

    #[test]
    fn http_401_returns_http_error() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(401)
            .with_body("bad key")
            .create();

        let result = chat_completion(&server.url(), "bad", "m", &[Message::user("x")]);
        match result {
            Err(TransportError::Http { status, body }) => {
                assert_eq!(status, 401);
                assert_eq!(body, "bad key");
            }
            other => panic!("expected Http error, got {other:?}"),
        }
    }

    // ── AC#9: 500 with empty body does not panic ──────────────────────────────

    #[test]
    fn http_500_empty_body_returns_http_error() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(500)
            .with_body("")
            .create();

        let result = chat_completion(&server.url(), "k", "m", &[Message::user("x")]);
        match result {
            Err(TransportError::Http { status, .. }) => assert_eq!(status, 500),
            other => panic!("expected Http error, got {other:?}"),
        }
    }

    // ── AC#10: bad JSON returns Parse error ───────────────────────────────────

    #[test]
    fn invalid_json_body_returns_parse_error() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body("not json at all")
            .create();

        let result = chat_completion(&server.url(), "k", "m", &[Message::user("x")]);
        assert!(matches!(result, Err(TransportError::Parse(_))));
    }

    // ── AC#11: empty choices array ────────────────────────────────────────────

    #[test]
    fn empty_choices_returns_missing_content() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"choices":[]}"#)
            .create();

        let result = chat_completion(&server.url(), "k", "m", &[Message::user("x")]);
        assert!(matches!(result, Err(TransportError::MissingContent)));
    }

    // ── AC#12: null content returns MissingContent ────────────────────────────

    #[test]
    fn null_content_returns_missing_content() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"choices":[{"message":{"role":"assistant","content":null}}]}"#)
            .create();

        let result = chat_completion(&server.url(), "k", "m", &[Message::user("x")]);
        assert!(matches!(result, Err(TransportError::MissingContent)));
    }

    // AC#13: trailing slash stripped from endpoint
    #[test]
    fn trailing_slash_stripped_from_endpoint() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"choices":[{"message":{"role":"assistant","content":"ok"}}]}"#)
            .expect(1)
            .create();

        let url_with_slash = format!("{}/", server.url());
        let result = chat_completion(&url_with_slash, "k", "m", &[Message::user("x")]);
        assert!(result.is_ok());
        mock.assert();
    }
}
