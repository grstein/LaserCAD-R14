# LCV-077 — Agent transport — blocking reqwest client for OpenAI-compatible API

- **Status**: Ready
- **Phase**: 7
- **Depends on**: LCV-076
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

The agent harness (consumed by LCV-079 multi-turn loop and LCV-080 command-line
wiring) needs to send JSON chat messages to an OpenAI-compatible endpoint —
default: OpenRouter — and retrieve the assistant's text reply. Without a
transport layer, the agent loop has no mechanism to reach the LLM, so the
`:` / `/ai` command prefixes the operator types would silently do nothing. This
demand delivers a self-contained, synchronous HTTP function that calls an
OpenAI-compatible `/chat/completions` endpoint, parses the response, and returns
the assistant content string. It is the only file in the codebase that touches
HTTP; every other agent module stays HTTP-unaware.

## Scope

### 1. `Cargo.toml` — new production dependency

Add exactly one new production dependency line:

```toml
reqwest = { version = "0.12", features = ["json", "blocking"] }
```

`reqwest::blocking` embeds its own Tokio runtime internally; no `tokio` entry is
needed in `[dependencies]` for this demand alone. `tokio` as a first-class
crate-level dependency is deferred to the demand that introduces async scheduling
(LCV-079 or LCV-059, whichever lands first).

Add one new dev-dependency for the HTTP mock server used in tests:

```toml
mockito = "1"
```

### 2. `src/agent/transport.rs` — new file

New file, ≤ 300 LOC. No imports from `egui`, `eframe`, or `rfd` (enforced by
AC#2). Module-level doc comment (`//!`) must reference LCV-077, state that this
module is the sole HTTP boundary of the agent subsystem, and name the three
public items.

#### 2a. `Message` — public struct

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}
```

Convenience: implement `Message::user(content: impl Into<String>) -> Self` and
`Message::assistant(content: impl Into<String>) -> Self` as associated functions
that hard-code `role` to `"user"` and `"assistant"` respectively.

#### 2b. `TransportError` — public error enum

```rust
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
```

#### 2c. `chat_completion` — public function

```rust
pub fn chat_completion(
    endpoint: &str,
    api_key:  &str,
    model:    &str,
    messages: &[Message],
) -> Result<String, TransportError>
```

> **Note on signature**: the initial brief listed three parameters
> (`endpoint`, `api_key`, `messages`). `model` has been added as a fourth
> because the OpenAI wire format requires it in the JSON request body; omitting
> it produces a server-side error on every call. The caller (LCV-079) reads
> `model` from `AgentSettings` (LCV-076) and passes it here.

Behaviour:

1. Construct the POST URL as
   `format!("{}/chat/completions", endpoint.trim_end_matches('/'))`.
2. Build the JSON request body:
   ```json
   { "model": "<model>", "messages": [{ "role": "…", "content": "…" }, …] }
   ```
   Use private structs (not `serde_json::json!`) to ensure compile-time type
   safety. Define `ChatRequest { model: &str, messages: &[Message] }` as a
   `Serialize`-only private struct.
3. Create a `reqwest::blocking::Client` (default settings) and POST with:
   - `Content-Type: application/json` (set automatically via `.json(body)`)
   - `Authorization: Bearer {api_key}`
4. Read the HTTP status code.
   - If not 2xx: read the body text (up to 1 024 bytes), return
     `Err(TransportError::Http { status, body })`.
5. Read the response body as text, then deserialize with `serde_json::from_str`
   into a private `ChatResponse` struct (see §2d).
   - Deserialization failure → `Err(TransportError::Parse(e))`.
6. Extract `response.choices[0].message.content`:
   - `choices` is empty → `Err(TransportError::MissingContent)`.
   - `content` is `None` → `Err(TransportError::MissingContent)`.
   - Otherwise → `Ok(content_string)`.

#### 2d. Private deserialization types

Define these inside `transport.rs`, not `pub`:

```rust
#[derive(serde::Deserialize)]
struct ChatResponse { choices: Vec<Choice> }

#[derive(serde::Deserialize)]
struct Choice { message: ChoiceMessage }

#[derive(serde::Deserialize)]
struct ChoiceMessage { content: Option<String> }
```

### 3. `src/agent/mod.rs` — register the new module

Add `pub mod transport;` to `src/agent/mod.rs`. The existing `MODULE` const
remains unchanged.

## Out of scope

- Multi-turn loop, retry logic, rate-limit back-off — LCV-079.
- Agent settings UI or API-key storage — LCV-076.
- Async reqwest or crate-level Tokio integration — LCV-079.
- Streaming / SSE / chunked responses.
- Authentication schemes other than Bearer token (OAuth, HMAC, etc.).
- HTTP timeouts beyond reqwest's defaults.
- Proxy configuration.
- Connection pooling or client reuse across calls — each `chat_completion` call
  creates its own `blocking::Client`; pooling is deferred.
- Validating the `model` string against a known list.
- Truncation beyond the 1 024-byte cap on error bodies.
- Any UI rendering — the function is pure Rust and returns a `Result`.

## Acceptance criteria

1. `Cargo.toml` contains `reqwest = { version = "0.12", features = ["json",
   "blocking"] }` under `[dependencies]`. No other production dep is added by
   this demand. `mockito = "1"` appears under `[dev-dependencies]`.

2. `src/agent/transport.rs` compiles and is ≤ 300 LOC.
   `grep -nE '^use (egui|eframe|rfd)' src/agent/transport.rs` returns no matches.

3. `Message` is `pub`, derives `Debug, Clone, PartialEq, Serialize, Deserialize`.
   `Message::user("hello").role == "user"` and
   `Message::user("hello").content == "hello"`.
   `Message::assistant("ok").role == "assistant"`.

4. `TransportError` has exactly the four variants `Request`, `Http`, `Parse`,
   `MissingContent` as specified in §2b.
   `TransportError::Http { status: 401, body: "Unauthorized".into() }` formats
   to `"HTTP 401: Unauthorized"` via `Display`.
   `TransportError::MissingContent` formats to
   `"response missing choices[0].message.content"` via `Display`.

5. `chat_completion` is the only `pub fn` exported by `transport.rs`.
   Its signature is exactly:
   ```rust
   pub fn chat_completion(
       endpoint: &str,
       api_key: &str,
       model: &str,
       messages: &[Message],
   ) -> Result<String, TransportError>
   ```

6. Given a mock server responding `200 OK` with body:
   ```json
   {"choices":[{"message":{"role":"assistant","content":"G'day"}}]}
   ```
   `chat_completion(server_url, "key", "gpt-4o-mini", &[Message::user("Hi")])`
   returns `Ok("G'day".to_string())`.

7. Given a mock server responding `200 OK` with body:
   ```json
   {"choices":[{"message":{"role":"assistant","content":"Hi"}}]}
   ```
   the POST body sent by the function deserializes to
   `{"model":"test-model","messages":[{"role":"user","content":"Hello"}]}`
   (verified via `mockito`'s `body_contains` matcher).

8. Given a mock server responding `401 Unauthorized` with body `"bad key"`:
   `chat_completion(…)` returns `Err(TransportError::Http { status: 401, body })`.
   `body` equals `"bad key"`.

9. Given a mock server responding `500 Internal Server Error` with an empty body:
   the function returns `Err(TransportError::Http { status: 500, body: "" })` or
   `body` is the empty string (not a panic).

10. Given a mock server responding `200 OK` with body `not json at all`:
    the function returns `Err(TransportError::Parse(_))`.

11. Given a mock server responding `200 OK` with body `{"choices":[]}`:
    the function returns `Err(TransportError::MissingContent)`.

12. Given a mock server responding `200 OK` with body:
    ```json
    {"choices":[{"message":{"role":"assistant","content":null}}]}
    ```
    the function returns `Err(TransportError::MissingContent)`.

13. The URL posted to is `<base>/chat/completions` regardless of whether
    `endpoint` has a trailing slash:
    - `endpoint = "https://api.example.com/v1"` → posts to
      `"https://api.example.com/v1/chat/completions"`.
    - `endpoint = "https://api.example.com/v1/"` → posts to
      `"https://api.example.com/v1/chat/completions"` (no double slash).

14. `src/agent/mod.rs` contains `pub mod transport;`.

15. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0.

## Expected tests

All unit tests live in `#[cfg(test)] mod tests` inside `src/agent/transport.rs`,
using `mockito` to create a local HTTP server.

- **`message_user_helper`** (AC#3): assert `Message::user("x").role == "user"`
  and `Message::user("x").content == "x"`.

- **`message_assistant_helper`** (AC#3): assert
  `Message::assistant("y").role == "assistant"`.

- **`error_http_display`** (AC#4): assert
  `format!("{}", TransportError::Http { status: 401, body: "Unauthorized".into() })`
  equals `"HTTP 401: Unauthorized"`.

- **`error_missing_content_display`** (AC#4): assert the `Display` output of
  `TransportError::MissingContent` contains `"choices[0].message.content"`.

- **`happy_path_returns_content`** (AC#6): spin up a mockito server with a 200
  response body containing `choices[0].message.content = "G'day"`, call
  `chat_completion(server_url, "key", "gpt-4o-mini", &[Message::user("Hi")])`,
  assert `Ok("G'day".to_string())`.

- **`request_body_contains_model_and_messages`** (AC#7): use mockito's request
  body matcher to verify the outgoing JSON body contains `"model":"test-model"`
  and `"content":"Hello"`.

- **`http_401_returns_http_error`** (AC#8): mockito returns 401 with body
  `"bad key"`, assert `Err(TransportError::Http { status: 401, body })` where
  `body == "bad key"`.

- **`http_500_empty_body_returns_http_error`** (AC#9): mockito returns 500 with
  empty body, assert `Err(TransportError::Http { status: 500, .. })` without
  panicking.

- **`invalid_json_body_returns_parse_error`** (AC#10): mockito returns 200 with
  body `"not json at all"`, assert `Err(TransportError::Parse(_))`.

- **`empty_choices_returns_missing_content`** (AC#11): mockito returns 200 with
  body `{"choices":[]}`, assert `Err(TransportError::MissingContent)`.

- **`null_content_returns_missing_content`** (AC#12): mockito returns 200 with
  `content: null`, assert `Err(TransportError::MissingContent)`.

- **`trailing_slash_stripped_from_endpoint`** (AC#13): start a mockito mock on
  path `/chat/completions`; call with `endpoint = server.url() + "/"`;
  assert the mock received the request (mockito asserts the call was made exactly
  once), confirming no double slash.

- **Static / CI — AC#2**: checked by `grep` in the CI run described in AC#15.

- **Static / CI — AC#15**: `cargo fmt --all -- --check &&
  cargo clippy --all-targets -- -D warnings && cargo test --all`.

## Open questions

*(none)*

## Notes

- **Why blocking, not async?** The egui `update()` loop runs on the UI thread.
  Calling an async function from it would require `block_on` or a channel
  round-trip. The multi-turn loop (LCV-079) will call `chat_completion` from a
  `std::thread::spawn` (or `tokio::task::spawn_blocking`) so the UI thread stays
  free. `reqwest::blocking` is the right primitive at this layer; the concurrency
  abstraction sits one level up.

- **Why `reqwest::blocking::Client` per call, not a shared client?** Connection
  reuse and pooling require shared mutable state (`Arc<Client>`), which belongs to
  LCV-079 (the loop that owns the runtime). This demand is intentionally minimal:
  one function, one client instance, one call. LCV-079 can pass a pre-built client
  if performance requires it — a future signature extension, not a change to this
  demand.

- **Why private structs for request/response, not `serde_json::json!`?**
  The macro produces `serde_json::Value`, which defers type errors to runtime.
  Private `#[derive(Serialize)]` structs catch field-name typos at compile time
  and avoid a `Value` allocation. A compile-time guarantee is worth four private
  lines of struct definitions.

- **Error body truncation at 1 024 bytes:** error bodies from OpenAI-compatible
  APIs are typically short JSON blobs (< 200 bytes). The cap prevents a
  pathological server returning megabytes of HTML from blowing up the log line.
  Implemented as `body.chars().take(1024).collect::<String>()` or equivalent.

- **`mockito` in CI**: `mockito` binds a random available port on localhost; no
  external network access is required. The test suite is hermetic and suitable for
  offline CI.

- **`model` parameter deviation**: the user's initial brief listed
  `chat_completion(endpoint, api_key, messages)` with three parameters. `model`
  was added as a fourth because the OpenAI wire format mandates it in the JSON
  body; a three-parameter function would produce a server-side 422 / 400 on every
  call to any standard endpoint. The caller (LCV-079) already reads `model` from
  `AgentSettings` (LCV-076).

- **OpenRouter base URL**: the default endpoint stored by LCV-076 is
  `https://openrouter.ai/api/v1`. Appending `/chat/completions` yields the
  standard completion path. No OpenRouter-specific headers (`HTTP-Referer`,
  `X-Title`) are required by this demand; the minimal OpenAI-compatible subset
  is sufficient for v0.1.0.

- **`thiserror` version**: `Cargo.toml` already carries `thiserror = "2"`;
  no version bump is needed.

- **LOC budget**: the file is expected to land at roughly 130–180 LOC:
  ~20 for public structs/enum, ~60 for the function body, ~20 for private
  deserialization types, ~80 for the test module. Well inside the 300-LOC cap.
