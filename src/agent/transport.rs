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
//!
//! **Every call is bounded** (LCV-129). A client with no timeout blocks its
//! worker thread for as long as the socket stays open, which is forever
//! against a black-holed endpoint — and a worker that never returns never
//! sends a terminal event, so the turn never ends and the app never idles
//! again (ADR 0007 §D11). The two windows below are what stops that; the
//! operator's way out of a *slow* endpoint is the panel's Cancel button.

use crate::agent::wire::{AssistantMessage, ChatMessage, ChatResponse};
use std::time::Duration;

/// Wall-clock budget for one whole `/chat/completions` call — connect, send,
/// and read the entire response — deliberately generous because nothing is
/// streamed, so time-to-first-byte is a reasoning model's whole generation
/// time and Cancel, not this number, is what answers an impatient operator.
const AGENT_REQUEST_TIMEOUT_SECS: u64 = 120;

/// Budget for the TCP/TLS connect alone, so the commonest real failure — a
/// typo'd host, or one that black-holes packets — is readable in 10 s instead
/// of sitting for the OS default (~130 s on Linux).
const AGENT_CONNECT_TIMEOUT_SECS: u64 = 10;

/// Errors that can occur when calling the OpenAI-compatible chat endpoint.
///
/// Response bodies are truncated to the first 1 024 characters; the request —
/// which holds the API key — never appears in any of these.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// Network-level or connection error (DNS, TLS, refused socket, etc.).
    /// A timeout is **not** one of these — it is [`TransportError::Timeout`].
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// The call ran out of the window it was given, at the connect or at any
    /// later point up to the last byte of the body.
    #[error(
        "The endpoint did not answer within {secs} s. It may be slow, unreachable, or the endpoint URL may be wrong — check Help > Agent settings, or press Cancel and try a shorter prompt."
    )]
    Timeout {
        /// The window that elapsed, in whole seconds.
        secs: u64,
    },

    /// HTTP 401: the key is missing, wrong, or refused by this endpoint.
    #[error(
        "Authentication failed (HTTP 401): the API key is missing, invalid, or not accepted by this endpoint. Check Help > Agent settings."
    )]
    Unauthorized,

    /// HTTP 429: the endpoint is asking the caller to slow down.
    #[error(
        "Rate limited (HTTP 429): the endpoint asked you to slow down. Wait a moment and try again."
    )]
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

/// Map one `reqwest` failure onto the error the operator should read.
///
/// **Every** `reqwest::Error` raised by a bounded call goes through here — the
/// build, the `send` and the body read alike, because a response whose headers
/// arrived can still stall halfway down its body — so a timeout is never
/// reported as a generic request failure. That is also why no `?` is applied
/// directly to a `reqwest` result below.
///
/// Which window is named is the one that actually elapsed: a connect timeout
/// answers `is_connect()` as well as `is_timeout()`, a total timeout only the
/// latter. A connect that gave up after 10 s must not claim it waited 120.
fn request_error(error: reqwest::Error, request: Duration, connect: Duration) -> TransportError {
    if error.is_timeout() {
        let window = if error.is_connect() { connect } else { request };
        return TransportError::Timeout {
            secs: window.as_secs(),
        };
    }
    TransportError::Request(error)
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
/// (connection pooling is deliberately out of scope), bounded by
/// [`AGENT_REQUEST_TIMEOUT_SECS`] in total and [`AGENT_CONNECT_TIMEOUT_SECS`]
/// on the connect.
///
/// # Errors
///
/// [`TransportError::Timeout`] when either window elapses; see
/// [`TransportError`] for the rest.
pub fn chat_completion(
    endpoint: &str,
    api_key: &str,
    model: &str,
    messages: &[ChatMessage],
    tools: &serde_json::Value,
) -> Result<AssistantMessage, TransportError> {
    chat_completion_with_timeout(
        endpoint,
        api_key,
        model,
        messages,
        tools,
        Duration::from_secs(AGENT_REQUEST_TIMEOUT_SECS),
        Duration::from_secs(AGENT_CONNECT_TIMEOUT_SECS),
    )
}

/// [`chat_completion`] with both windows injected.
///
/// Private, and it stays private: the public surface is exactly one
/// `chat_completion`, whose windows are the two constants above and are not an
/// operator's to choose (they are a function of the model's latency, which
/// nobody knows in advance). The seam exists so a test can drive a 250 ms
/// window against a socket it owns and prove the bound fires without sleeping
/// for two minutes to do it.
fn chat_completion_with_timeout(
    endpoint: &str,
    api_key: &str,
    model: &str,
    messages: &[ChatMessage],
    tools: &serde_json::Value,
    request: Duration,
    connect: Duration,
) -> Result<AssistantMessage, TransportError> {
    let url = format!("{}/chat/completions", endpoint.trim_end_matches('/'));
    let body = ChatRequest {
        model,
        messages,
        tools: tools_for_request(tools),
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(request)
        .connect_timeout(connect)
        .build()
        .map_err(|error| request_error(error, request, connect))?;
    let response = client
        .post(&url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .map_err(|error| request_error(error, request, connect))?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().unwrap_or_default();
        let truncated: String = text.chars().take(1024).collect();
        return Err(status_error(status.as_u16(), truncated));
    }

    let text = response
        .text()
        .map_err(|error| request_error(error, request, connect))?;
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
mod tests;
