# LCV-079 — Multi-turn agent loop (run AI agent conversation with tool calls)

- **Status**: Ready
- **Phase**: 7
- **Depends on**: LCV-077 (HTTP transport), LCV-078 (agent tool registry)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

The HTTP transport (LCV-077) and the CAD tool registry (LCV-078) are
standalone components; without a coordinating loop they cannot serve a user
request end-to-end. When a laser-cutting operator types `/ai add a 100 × 50 mm
rectangle centred at 0,0`, the system must send that intent to the LLM, receive
one or more tool calls (e.g. `create_line` four times), execute each call
against the live document through the history stack so that Ctrl+Z can undo the
entire batch, and return a concise confirmation sentence to the command line —
all within a single interaction. Without a driving loop, LLM responses that
include tool calls produce no document changes and the feature is broken.

This demand ships `src/agent/loop_.rs`, the function that orchestrates the
multi-step tool-call conversation, caps iteration at 10 dispatched tool calls
per user turn to prevent runaway API spend, and exposes a testable internal
seam so unit tests can exercise the loop logic without a live HTTP endpoint.

## Scope

### 1 — New file `src/agent/loop_.rs`

The file exposes exactly the following public items:

```rust
/// Maximum number of individual tool-call dispatches allowed within one
/// `run_agent_turn` invocation. Exported so callers can surface it in the UI.
pub const MAX_TOOL_CALLS_PER_TURN: usize = 10;

/// Errors that `run_agent_turn` can return.
#[derive(Debug)]
pub enum AgentError {
    /// HTTP or serialisation failure from the transport layer (LCV-077).
    Transport(String),
    /// A tool call could not be dispatched (LCV-078 returned an error).
    ToolDispatch(String),
    /// The loop guard fired: more than MAX_TOOL_CALLS_PER_TURN individual
    /// tool calls were requested in a single turn.
    IterationLimitExceeded,
    /// The API returned a choice whose `content` is None and whose
    /// `tool_calls` is also None/empty — a protocol violation.
    NoContent,
}

impl std::fmt::Display for AgentError { … }
impl std::error::Error for AgentError {}

/// Drive one complete agent turn: send `prompt` to the OpenAI-compatible
/// endpoint, dispatch any tool calls via LCV-078, and return the final
/// assistant text. May execute up to MAX_TOOL_CALLS_PER_TURN individual
/// tool dispatches before returning `Err(AgentError::IterationLimitExceeded)`.
///
/// `doc` and `history` are mutated via `dispatch_tool_call` (LCV-078), which
/// routes every document change through `History::commit` so Ctrl+Z can undo
/// the whole batch.
pub fn run_agent_turn(
    prompt: &str,
    endpoint: &str,
    api_key: &str,
    doc: &mut Document,
    history: &mut History,
) -> Result<String, AgentError>
```

All other items in the file are `pub(crate)` or private.

### 2 — System prompt constant

```rust
/// Hard-coded system prompt injected as the first message of every turn.
/// Must not be user-configurable in v0.1.0 (see Out of scope).
pub(crate) const AGENT_SYSTEM_PROMPT: &str =
    "You are a CAD assistant embedded in LaserCAD v2, a 2D laser-cutting CAD \
     tool. All coordinates and dimensions are in millimetres (mm). Angles at \
     the user interface are in degrees. Use the provided tools to create, \
     modify, or query the open drawing. Prefer the fewest tool calls that \
     satisfy the request. Confirm what you did in one or two concise sentences.";
```

### 3 — Testable inner loop

To allow unit testing without a live HTTP endpoint, implement the loop in a
package-private generic function and have `run_agent_turn` delegate to it:

```rust
/// Inner loop: `send_fn` encapsulates the HTTP round-trip.
/// `F: FnMut(&[ChatMessage]) -> Result<ChatResponse, AgentError>`
///
/// Called from `run_agent_turn` (which wraps the real transport) and from
/// unit tests (which supply a closure returning canned responses).
pub(crate) fn agent_loop<F>(
    send_fn: &mut F,
    messages: &mut Vec<ChatMessage>,
    doc: &mut Document,
    history: &mut History,
) -> Result<String, AgentError>
where
    F: FnMut(&[ChatMessage]) -> Result<ChatResponse, AgentError>,
```

`run_agent_turn` constructs the initial `messages` vec
(`[ChatMessage::system(AGENT_SYSTEM_PROMPT), ChatMessage::user(prompt)]`),
wraps `transport::post_chat(endpoint, api_key, …)` in a closure that maps
`TransportError` → `AgentError::Transport`, and calls `agent_loop`.

### 4 — Loop algorithm (inside `agent_loop`)

```
let mut dispatched: usize = 0;

loop {
    let response = send_fn(messages)?;
    let choice   = response.choices.into_iter().next()
                       .ok_or(AgentError::NoContent)?;

    match (choice.message.tool_calls, choice.message.content) {
        (Some(tool_calls), _) if !tool_calls.is_empty() => {
            // Guard BEFORE dispatching.
            if dispatched + tool_calls.len() > MAX_TOOL_CALLS_PER_TURN {
                return Err(AgentError::IterationLimitExceeded);
            }

            // Append the assistant's tool-call message to the conversation.
            messages.push(ChatMessage::assistant_with_tool_calls(
                choice.message.content.clone(),
                tool_calls.clone(),
            ));

            // Dispatch each tool call, append its result message.
            for tc in &tool_calls {
                let result = dispatch_tool_call(
                    &tc.function.name,
                    &tc.function.arguments,
                    doc,
                    history,
                )
                .map_err(|e| AgentError::ToolDispatch(e.to_string()))?;

                messages.push(ChatMessage::tool_result(tc.id.clone(), result));
                dispatched += 1;
            }
        }

        (_, Some(text)) => return Ok(text),
        _               => return Err(AgentError::NoContent),
    }
}
```

Key invariants:
- The guard fires **before** dispatch: if `dispatched + new_batch > 10`, return
  the error immediately without calling `dispatch_tool_call` even once for that
  batch.
- Each dispatched tool call is counted individually (a batch of three tool calls
  in one API response increments `dispatched` by 3).
- The guard is checked at the top of each iteration, so a perfectly sized batch
  of exactly 10 individual calls is allowed.

### 5 — Interfaces consumed from dependencies

These contracts must be fulfilled by LCV-077 and LCV-078. `loop_.rs` imports
from `crate::agent::transport` and `crate::agent::tools`:

**From LCV-077** (`src/agent/transport.rs`):
```rust
pub struct ChatMessage   { /* role + content + optional tool_calls/tool_call_id */ }
pub struct ChatResponse  { pub choices: Vec<Choice> }
pub struct Choice        { pub message: AssistantMessage }
pub struct AssistantMessage {
    pub content:    Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}
pub struct ToolCall {
    pub id:       String,
    pub function: ToolCallFunction,
}
pub struct ToolCallFunction { pub name: String, pub arguments: String }

pub struct TransportError(pub String);

pub fn post_chat(
    endpoint: &str,
    api_key:  &str,
    messages: &[ChatMessage],
    tools:    &[crate::agent::tools::ToolDef],
) -> Result<ChatResponse, TransportError>;
```

`ChatMessage` must provide at minimum three constructors:
- `ChatMessage::system(content: &str) -> Self`
- `ChatMessage::user(content: &str) -> Self`
- `ChatMessage::assistant_with_tool_calls(content: Option<String>, tool_calls: Vec<ToolCall>) -> Self`
- `ChatMessage::tool_result(tool_call_id: String, content: String) -> Self`

**From LCV-078** (`src/agent/tools.rs`):
```rust
pub struct ToolDef  { /* JSON Schema for one tool exposed to the LLM */ }
pub struct ToolDispatchError(pub String);
impl std::fmt::Display for ToolDispatchError { … }

pub fn dispatch_tool_call(
    name:      &str,
    arguments: &str,
    doc:       &mut Document,
    history:   &mut History,
) -> Result<String, ToolDispatchError>;

/// Returns the list of ToolDefs to include in every ChatRequest.
/// Called once per `run_agent_turn` invocation.
pub fn tool_definitions() -> Vec<ToolDef>;
```

`loop_.rs` calls `tool_definitions()` once inside `run_agent_turn` and passes
the slice to `post_chat`. The inner `agent_loop` receives `messages` already
populated; it never calls `tool_definitions` itself.

### 6 — Module wiring

Add to `src/agent/mod.rs`:

```rust
pub mod loop_;
pub use loop_::{run_agent_turn, AgentError, MAX_TOOL_CALLS_PER_TURN};
```

### 7 — Purity and size constraints

- `src/agent/loop_.rs` MUST NOT import `egui`, `eframe`, or `rfd`.
- File MUST remain ≤ 300 LOC (blank lines and comments included).
- No `unwrap()` or `expect()` in non-test code. Every `?` maps to `AgentError`.

## Out of scope

- **Persistent conversation history across turns.** Each call to
  `run_agent_turn` starts a fresh message list. Multi-turn memory is a future
  demand.
- **Configurable system prompt.** The prompt is a compile-time constant in
  v0.1.0. No UI knob.
- **Streaming responses.** The transport delivers a complete `ChatResponse`.
  Streaming is a future demand.
- **Cancellation / timeout.** The caller (App's tokio runtime) owns the
  task lifecycle; `run_agent_turn` has no cancel handle.
- **Model selection inside this function.** The caller passes `endpoint` and
  `api_key`; model is embedded in the endpoint or supplied by LCV-077's
  transport layer.
- **Retry on transient HTTP errors.** Single-attempt per loop iteration; retry
  policy is a future demand.
- **Command-line wiring (`:` / `/ai` prefixes).** That is LCV-080.
- **Agent settings UI.** That is LCV-076.
- **Partial-result return when the guard fires.** `IterationLimitExceeded` is a
  hard error; any document changes already committed via `History::commit`
  before the guard fired are **not** rolled back (they are undoable via Ctrl+Z).
  Atomic batch semantics are out of scope for v0.1.0.

## Acceptance criteria

1. `src/agent/loop_.rs` exists and compiles. `pub use` in `src/agent/mod.rs`
   re-exports `run_agent_turn`, `AgentError`, and `MAX_TOOL_CALLS_PER_TURN`.

2. `MAX_TOOL_CALLS_PER_TURN == 10`. Verified by
   `assert_eq!(MAX_TOOL_CALLS_PER_TURN, 10)` in a unit test.

3. **Text-only response (no tool calls):** When `send_fn` returns a
   `ChatResponse` with one `Choice` whose `content = Some("Done.".to_string())`
   and `tool_calls = None`, `agent_loop` returns `Ok("Done.".to_string())`.
   `dispatch_tool_call` is never called. `messages.len() == 2` (system +
   user; nothing appended).

4. **Single tool-call round:** When `send_fn` returns a tool call
   `{id:"tc1", name:"create_line", arguments:"{}"}` on the first invocation and
   `content = Some("Line created.")` (no tool calls) on the second,
   `agent_loop` returns `Ok("Line created.")`. `messages` after completion
   contains exactly 4 entries: system, user, assistant-with-tool-call,
   tool-result.

5. **Guard fires before dispatch:** Construct a `send_fn` that always returns a
   batch of 11 tool calls. `agent_loop` returns
   `Err(AgentError::IterationLimitExceeded)` on the first iteration without
   ever calling `dispatch_tool_call`.

6. **Guard fires after partial batches:** `send_fn` returns 6 tool calls on the
   first invocation (all 6 dispatched, `dispatched = 6`) and then 6 more on
   the second. The second iteration returns
   `Err(AgentError::IterationLimitExceeded)` before any of the second batch is
   dispatched.

7. **Exactly 10 individual calls are allowed:** `send_fn` returns 5 tool calls
   per round, twice (10 total), then text on the third call. `agent_loop`
   returns `Ok(text)` without error; `dispatched == 10`.

8. **`NoContent` propagated:** When `send_fn` returns a `Choice` with
   `content = None` and `tool_calls = None` (or `Some(vec![])`), `agent_loop`
   returns `Err(AgentError::NoContent)`.

9. **Transport error propagated:** When `send_fn` returns
   `Err(AgentError::Transport("timeout".into()))`, `agent_loop` returns
   `Err(AgentError::Transport(_))`.

10. **`ToolDispatch` error propagated:** When a `dispatch_tool_call` stub
    returns `Err(ToolDispatchError("unknown tool".into()))`, `agent_loop`
    returns `Err(AgentError::ToolDispatch(_))` and stops processing the
    remaining tool calls in that batch.

11. `AgentError` implements `Display`, `Debug`, and `std::error::Error`.
    `format!("{}", AgentError::IterationLimitExceeded)` is non-empty. Verified
    in a unit test.

12. `run_agent_turn` passes the `Vec<ToolDef>` from `tool_definitions()` to
    the transport on every API call, not just the first. Verified by
    code review (each `send_fn` invocation inside the loop receives the tools
    slice).

13. Purity: `grep -nE '^use (egui|eframe|rfd)' src/agent/loop_.rs` returns no
    matches.

14. Size: `wc -l src/agent/loop_.rs` reports `<= 300`.

15. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0.

## Expected tests

All in `#[cfg(test)] mod tests` inside `src/agent/loop_.rs` unless noted.
Tests use a `MockDispatch` stub (a simple counter + optional error flag) in
place of the real `dispatch_tool_call`; the `agent_loop` seam accepts a
closure, so no runtime trait injection is needed.

- **`max_tool_calls_constant_is_10`** (AC 2): `assert_eq!(MAX_TOOL_CALLS_PER_TURN, 10)`.

- **`text_only_response_returns_ok`** (AC 3): `send_fn` closure returns a canned
  `ChatResponse` with content `"Done."` and no tool calls. Assert
  `agent_loop(…) == Ok("Done.".to_string())` and dispatch counter is 0.

- **`single_tool_call_round_appends_messages`** (AC 4): `send_fn` returns one
  tool call on the first call, text on the second. Assert `Ok("Line created.")`
  and `messages.len() == 4`.

- **`guard_fires_before_batch_of_11`** (AC 5): `send_fn` always returns 11 tool
  calls. Assert `Err(AgentError::IterationLimitExceeded)` and dispatch counter
  is 0.

- **`guard_fires_after_6_then_6`** (AC 6): two rounds of 6. Assert error after
  round 2 fires before dispatch; dispatch counter stays at 6 (only the first
  round was processed).

- **`exactly_10_calls_allowed`** (AC 7): two rounds of 5, then text. Assert
  `Ok(text)` and dispatch counter == 10.

- **`no_content_returns_error`** (AC 8): `send_fn` returns `content = None`,
  `tool_calls = None`. Assert `Err(AgentError::NoContent)`.

- **`transport_error_propagated`** (AC 9): `send_fn` returns
  `Err(AgentError::Transport("x".into()))`. Assert matching error variant.

- **`tool_dispatch_error_stops_batch`** (AC 10): batch of 3 tool calls; stub
  dispatch errors on the second one. Assert `Err(AgentError::ToolDispatch(_))`
  and dispatch counter == 1 (only the first call succeeded).

- **`agent_error_display_is_non_empty`** (AC 11): for each `AgentError` variant,
  assert `!format!("{}", e).is_empty()`.

- **Static (AC 13)**: `grep -nE '^use (egui|eframe|rfd)' src/agent/loop_.rs`
  returns no matches.

- **Static (AC 14)**: `wc -l src/agent/loop_.rs` reports `<= 300`.

- **Build gate (AC 15)**: `cargo fmt --all -- --check &&
  cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **Filename `loop_.rs`**: the trailing underscore avoids the Rust keyword
  `loop`. This matches the `move_.rs` convention already in `src/tools/`.

- **Testability seam — `agent_loop` with a closure**: the public
  `run_agent_turn` is not easily testable without a live HTTP server because it
  calls `transport::post_chat` directly. The `agent_loop` inner function accepts
  a `&mut impl FnMut(…) -> Result<…>` closure, which tests replace with a
  canned-response stub. The pattern is identical to `handle_undo` / `handle_redo`
  in LCV-075: free functions that accept explicit arguments and can be exercised
  without an egui or network context.

- **Guard fires before dispatch, not after.** Checking `dispatched >= MAX`
  after dispatching would allow the 11th call to execute, then fail. The
  algorithm checks `dispatched + batch.len() > MAX` before entering the
  dispatch loop, so no call beyond the cap ever reaches `dispatch_tool_call`.

- **Partial undo of an aborted turn.** When `IterationLimitExceeded` fires
  after, say, 7 tool calls, those 7 changes are already in the history stack
  and are individually undoable by Ctrl+Z. Atomic batch semantics (undo the
  whole agent turn in one Ctrl+Z) are deferred; this matches v1 behaviour and
  avoids complexity in Phase 7.

- **Concurrency model.** `run_agent_turn` takes `&mut Document` and
  `&mut History` by exclusive reference, so it cannot be `Send` as written.
  The expected call pattern (established by `AGENTS.md` §"Event flow") is:
  `App` spawns this via `tokio::task::spawn_blocking` (or runs it inside
  a `std::thread` channelled back to the UI frame). If LCV-077 delivers a
  blocking `post_chat` API, `run_agent_turn` is a plain blocking function. If
  LCV-077 is async, `run_agent_turn` must be `async fn` and the callee adjusts
  accordingly — implementer should align with the final LCV-077 interface.
  If the signatures diverge, route to `architect` before proceeding.

- **`tool_definitions()` called once per turn.** The current design calls
  `tool_definitions()` inside `run_agent_turn` and passes the same slice to
  every `post_chat` invocation within the loop. This is intentional: the
  available tools do not change mid-turn, and computing the list once avoids
  repeated allocations.

- **`AGENT_SYSTEM_PROMPT` is exported `pub(crate)`** so LCV-080 (command-line
  wiring) and LCV-076 (agent settings dialog) can display it in a tooltip or
  debug view without the constant leaking into the public crate API.

- **`AgentError` in `loop_.rs`, not a shared `error.rs`.** With only one
  consumer in Phase 7 there is no justification for a central error module.
  If Phase 8+ demands aggregate errors across multiple agent files, extraction
  to `src/agent/error.rs` is a clean refactor at that time.
