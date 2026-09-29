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
    fn matcher(&self) -> impl Fn(&mockito::Request) -> bool + Send + Sync + 'static + use<> {
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

    let tools = crate::agent::tools::tool_definitions(false);
    assert_eq!(tools.as_array().map(|a| a.len()), Some(8), "fixture check");

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
        Some(8),
        "AC 4: all eight tool schemas must reach the wire, body was {body}"
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

// ── LCV-130: an address the test does not own is not isolation ─────────

/// LCV-130 — the endpoint this file named until now.
///
/// It used to be `http://127.0.0.1:1` — "port 1 on loopback, nothing
/// listens there, ever" — and that reasoning was wrong.
/// `reqwest::blocking::Client::new()` sets `auto_sys_proxy: true`, and
/// reqwest 0.12 has **no loopback bypass**: with `HTTP_PROXY` set (normal
/// on a corporate network, or a runner behind one) the closed port is
/// never dialled locally at all — the request is handed to the proxy
/// instead. A reviewer stood up a listener on the proxy address and
/// captured a real `POST` carrying `authorization: Bearer
/// sk-test-DO-NOT-LEAK` while this very test reported success. This
/// constant answers a different question: not "is the address
/// reachable", which the environment decides, but "can this ever be
/// sent", which the test decides by making the endpoint fail
/// `Url::parse` — no socket considered, no proxy consulted, in any
/// environment. [`owned_socket_closed_before_a_reply_is_a_request_error`]
/// below carries the socket-level half this replaced.
const UNPARSEABLE_ENDPOINT: &str = "not-a-url";

/// LCV-130 AC 1a — the unsendable endpoint is a `Request` error, not a
/// panic, and it costs nothing to prove: `Url::parse` rejects
/// [`UNPARSEABLE_ENDPOINT`] before `chat_completion` ever reaches
/// `reqwest::blocking::Client::builder()`, so no socket is opened and no
/// proxy is consulted — by construction, in every environment.
///
/// AC 4a's per-file guard rides on the same assertion: a module that
/// configures a transport endpoint from a constant proves, in the same
/// file, that the constant fails `Url::parse`.
#[test]
fn unparseable_endpoint_is_a_request_error() {
    assert!(
        reqwest::Url::parse(UNPARSEABLE_ENDPOINT).is_err(),
        "AC 4a: the constant these tests configure must fail URL parsing"
    );

    let result = chat_completion(
        UNPARSEABLE_ENDPOINT,
        DUMMY_KEY,
        "m",
        &[ChatMessage::user("x")],
        &Value::Null,
    );
    assert!(matches!(result, Err(TransportError::Request(_))));
    assert_no_key(&result.unwrap_err().to_string());
}

/// LCV-130 AC 1b — the socket the test binds, accepts on, and drops
/// without a byte, which is a `Request` error and **not** `Timeout`.
///
/// A bound-then-dropped port is explicitly rejected as a fixture: the
/// ephemeral port could be handed to an unrelated process between the
/// drop and the connect, which is a flake and, worse, a connection to
/// something else on the developer's machine. This binds, accepts one
/// connection and only then drops it — the socket is owned for the
/// call's entire lifetime, same shape as [`timeout_case`] below.
///
/// This is not a duplicate of (a) above: two mutations of
/// [`request_error`], measured, each prove it in one direction. Adding
/// `if error.is_request() { return TransportError::Timeout { .. } }`
/// turns this test red while (a) stays green; adding `if
/// error.is_builder() { return TransportError::Timeout { .. } }` turns
/// (a) red while this test stays green. If either direction does not
/// discriminate, this test is a duplicate of (a) and must be reported as
/// such, not kept for appearance (demand §Expected tests).
///
/// `is_connect()` cannot be that mutation: this fixture `accept()`s the
/// connection before dropping it (see above), so its failure is never a
/// connect-step failure. The only fixture shape whose failure would
/// answer `is_connect()` is a bound-then-dropped port with no `accept()`
/// at all — the flake this test's own fixture explicitly rejects.
#[test]
fn owned_socket_closed_before_a_reply_is_a_request_error() {
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback must be bindable");
    let url = format!(
        "http://{}",
        listener
            .local_addr()
            .expect("the fixture must have an address")
    );
    let fixture = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("the client must connect");
        // The whole fixture: accept, then drop with no byte written. The
        // client sees the connection close before any response arrives.
        drop(stream);
    });

    let result = chat_completion(
        &url,
        DUMMY_KEY,
        "m",
        &[ChatMessage::user("x")],
        &Value::Null,
    );

    fixture.join().expect("the fixture thread must not panic");
    match result {
        Err(TransportError::Request(_)) => {}
        other => {
            panic!("a connection dropped before a reply must be a Request error, got {other:?}")
        }
    }
}

// ── LCV-129: the call is bounded ─────────────────────────────────────────

/// The implementation section of this file: everything before the bare
/// `#[cfg(test)]` at column 0, so a scan can never match the test source
/// written next to it.
fn implementation_section() -> &'static str {
    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/agent/transport.rs"
    ));
    let at = src
        .find("\n#[cfg(test)]")
        .expect("transport.rs must have a bare #[cfg(test)] marker");
    &src[..at]
}

/// LCV-129 AC 1 — **bounded source scan**: the client is built through the
/// builder, with both windows, and the unbounded constructor is gone.
///
/// A runtime test cannot see this. Two of the three ways to lose the bound
/// — deleting `.timeout(..)`, or going back to `Client::new()` — leave
/// every other test in this file green, because mockito answers instantly
/// and an instant answer needs no window.
///
/// Shown to discriminate: every needle is first looked up in a witness that
/// spells out both the old line and the new block, through the same
/// `contains` that does the real work. A needle that had been silently
/// misspelt fails against the witness before the real haystack is consulted.
#[test]
fn ac1_the_client_is_built_with_both_windows_source_scan() {
    let witness = "let client = reqwest::blocking::Client::new(); \
                       reqwest::blocking::Client::builder() \
                           .timeout(request).connect_timeout(connect).build() \
                       const AGENT_REQUEST_TIMEOUT_SECS: u64 = 120; \
                       const AGENT_CONNECT_TIMEOUT_SECS: u64 = 10; \
                       Duration::from_secs(AGENT_REQUEST_TIMEOUT_SECS)";
    let required = [
        concat!("AGENT_REQUEST_TIMEOUT", "_SECS"),
        concat!("AGENT_CONNECT_TIMEOUT", "_SECS"),
        concat!("Client::build", "er()"),
        concat!(".time", "out("),
        concat!(".connect_time", "out("),
    ];
    let forbidden = concat!("Client::n", "ew()");
    for needle in required.iter().chain(std::iter::once(&forbidden)) {
        assert!(
            witness.contains(needle),
            "control: `{needle}` must be a needle that can match something"
        );
    }

    let implementation = implementation_section();
    for needle in required {
        assert!(
            implementation.contains(needle),
            "AC 1: the bounded client must be built with `{needle}`"
        );
    }
    assert!(
        !implementation.contains(forbidden),
        "AC 1: `{forbidden}` builds an unbounded client — the whole defect"
    );
}

/// LCV-129 AC 2 — the sentence an operator reads when a call runs out of
/// time, pinned character for character and carrying no key.
///
/// Assembled with `concat!` so the scan above — and any future one — cannot
/// match this literal, and asserted for both windows: a connect that gave
/// up after 10 s must not claim it waited 120.
#[test]
fn ac2_a_timeout_says_exactly_what_the_operator_must_read() {
    let shown = TransportError::Timeout { secs: 120 }.to_string();
    assert_eq!(
        shown,
        concat!(
            "The endpoint did not answer within 120 s. It may be slow, ",
            "unreachable, or the endpoint URL may be wrong — check ",
            "Help > Agent settings, or press Cancel and try a shorter prompt."
        )
    );
    assert_no_key(&shown);
    assert!(
        TransportError::Timeout { secs: 10 }
            .to_string()
            .starts_with("The endpoint did not answer within 10 s."),
        "the number in the sentence is the window that really elapsed"
    );
}

/// One loopback fixture, one bounded call, and whatever error it produced.
///
/// `reply` is what the fixture writes before it stalls: `None` accepts the
/// connection and never answers at all; `Some(head)` answers with headers
/// promising a body it then never sends. Either way the fixture thread ends
/// on its own — it blocks in a `read` that returns the moment the client
/// gives up and closes, so nothing here sleeps and nothing is left running.
///
/// The windows are deliberately different: 250 ms for the whole request and
/// seven seconds for the connect, which both fixtures complete instantly.
/// So a `Timeout` naming `0` seconds (250 ms, truncated) proves the request
/// window was the one reported, and a `7` would prove it was not.
///
/// **Ordering rule (LCV-149)**: the fixture reads the whole request (head
/// and `Content-Length` body) *before* it writes `reply`. Writing first
/// races the client: under CPU contention the head reaches hyper before it
/// has finished sending the request, and hyper rejects it as
/// `UnexpectedMessage` — a `Request` error, not the `Timeout` under test.
/// The bytes read before the reply are returned so the test can prove it.
fn timeout_case(reply: Option<&'static str>) -> (TransportError, Vec<u8>) {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback must be bindable");
    let url = format!(
        "http://{}",
        listener
            .local_addr()
            .expect("the fixture must have an address")
    );
    let fixture = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("the client must connect");
        let received = read_request(&mut stream);
        if let Some(head) = reply {
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.flush();
        }
        // Drain until the client hangs up. This is the stall: the request
        // is read and never answered.
        let mut sink = [0u8; 1024];
        while matches!(stream.read(&mut sink), Ok(n) if n > 0) {}
        received
    });

    let (done, result) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let answer = chat_completion_with_timeout(
            &url,
            "",
            "m",
            &[ChatMessage::user("x")],
            &Value::Null,
            std::time::Duration::from_millis(250),
            std::time::Duration::from_secs(7),
        );
        drop(done.send(answer));
    });
    let answer = result
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("AC 3: an unbounded client hangs here — the call never gave up");
    let received = fixture.join().expect("the fixture thread must not panic");
    let error = answer.expect_err("a fixture that never answers cannot produce a message");
    (error, received)
}

/// Reads one HTTP request from `stream`: the head through `\r\n\r\n`,
/// then exactly `Content-Length` body bytes. Returns everything read.
/// Stops early, returning what it has, if the client hangs up.
fn read_request(stream: &mut std::net::TcpStream) -> Vec<u8> {
    use std::io::Read;

    let mut received = Vec::new();
    let mut chunk = [0u8; 1024];
    let head_end = loop {
        if let Some(at) = received.windows(4).position(|w| w == b"\r\n\r\n") {
            break at + 4;
        }
        match stream.read(&mut chunk) {
            Ok(n) if n > 0 => received.extend_from_slice(&chunk[..n]),
            _ => return received,
        }
    };
    let body_len = String::from_utf8_lossy(&received[..head_end])
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim()
                .eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    while received.len() < head_end + body_len {
        match stream.read(&mut chunk) {
            Ok(n) if n > 0 => received.extend_from_slice(&chunk[..n]),
            _ => break,
        }
    }
    received
}

/// LCV-129 AC 3 — the bound really fires, on both halves of a call.
///
/// Two fixtures this test binds and owns: one that accepts and never
/// writes (the send stalls), one that writes a header promising 100 bytes
/// and then sends none (the *body read* stalls — a response can arrive
/// half-way and stop, which is why the mapper is on `text()` too).
///
/// **Nothing here passes because of a sleep.** The pass comes from the
/// implementation giving up after its injected 250 ms; the
/// `recv_timeout(5 s)` bounds a *failure*, so a regression that drops
/// `.timeout(..)` fails this test in five seconds instead of hanging the
/// whole suite on a socket nothing will ever close. The green path costs
/// about half a second in total and sleeps nowhere.
///
/// **Inventory note for LCV-130**: these two sockets are reached through
/// `reqwest`, which honours `HTTP_PROXY`, so under a proxy they are dialled
/// through it like every other request in this file. They carry no key (the
/// call is made with an empty one) and no system prompt, but they belong on
/// LCV-130's list all the same.
#[test]
fn ac3_both_halves_of_a_call_give_up_when_the_endpoint_stalls() {
    for (what, reply) in [
        ("a socket that accepts and never answers", None),
        (
            "a header promising a body that never comes",
            Some("HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n"),
        ),
    ] {
        let (error, received) = timeout_case(reply);
        let received = String::from_utf8_lossy(&received);
        assert!(
            received.starts_with("POST /chat/completions "),
            "{what}: the fixture must read the request head before replying, read {received:?}"
        );
        assert!(
            received.contains(r#""model":"m""#),
            "{what}: the fixture must read the JSON body before replying, read {received:?}"
        );
        match error {
            TransportError::Timeout { secs } => assert_eq!(
                secs, 0,
                "{what}: the request window (250 ms) is the one that elapsed, not the connect one"
            ),
            other => panic!("{what} must be a Timeout, got {other:?}"),
        }
    }
}

// ── AC 2: the wire types are declared once, in wire.rs ───────────────────

/// AC 2 — no duplicate wire struct survives in this file, and the shared
/// declarations are imported instead. Bounded to the implementation
/// section and built with `concat!`, so the scan cannot match the needles
/// written here; pasting any of those structs back above the
/// `#[cfg(test)]` marker turns this red.
#[test]
fn transport_declares_no_wire_structs_of_its_own() {
    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/agent/transport.rs"
    ));
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

// ── LCV-130 AC 3: the production client carries no proxy config ─────────

/// LCV-130 AC 3 — the production transport is untouched. `.no_proxy()`,
/// in any form, is out of scope for `chat_completion` (demand §Out of
/// scope): it would break every real operator behind a corporate proxy
/// to buy this suite a test convenience. AC 2's `.cargo/config.toml`
/// entry is the only mechanism this demand adds, and it never reaches
/// this file — bounded to the implementation section, so the test
/// module these two needles necessarily discuss cannot trip its own scan.
///
/// Shown to discriminate: both needles are first looked up in a witness
/// built from the very words this test forbids, through the same
/// `contains` that scans the real implementation section.
#[test]
fn ac3_the_production_client_carries_no_proxy_configuration() {
    let witness = concat!("client.", "no_proxy", "(); client.", "proxy", "(p)");
    let needles = [concat!("no_", "proxy"), concat!("pro", "xy")];
    for needle in needles {
        assert!(
            witness.contains(needle),
            "control: `{needle}` must be a needle that can match something"
        );
    }

    let implementation = implementation_section();
    for needle in needles {
        assert!(
            !implementation.contains(needle),
            "AC 3: `{needle}` must not appear in the production transport"
        );
    }
}
