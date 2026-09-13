//! LCV-079 / LCV-121 — Multi-turn agent loop.
//!
//! Control flow only: send the conversation, honour the step budget, feed tool
//! results back. The wire shapes come from [`crate::agent::wire`] and the HTTP
//! call from [`crate::agent::transport`]; nothing is declared twice.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use crate::agent::wire::{AssistantMessage, ChatMessage};
use crate::document::{Document, History};

// ── Step budget ──────────────────────────────────────────────────────────────

/// Tool-call dispatches allowed in one turn when the operator has not chosen
/// otherwise (ADR 0007 §D7).
pub const AGENT_STEP_BUDGET_DEFAULT: u8 = 12;

/// Smallest budget that still lets the agent do anything at all.
pub const AGENT_STEP_BUDGET_MIN: u8 = 1;

/// Largest budget. Caps how long a runaway turn can keep drawing.
pub const AGENT_STEP_BUDGET_MAX: u8 = 32;

/// Hold a stored budget inside `AGENT_STEP_BUDGET_MIN..=AGENT_STEP_BUDGET_MAX`.
///
/// The value comes from a JSON file the operator can hand-edit, so every reader
/// goes through here rather than trusting the field: `0` becomes the minimum
/// and anything oversized becomes the maximum. Clamping deliberately lives with
/// the loop that enforces the budget, not in `io::settings` — that module must
/// not import `crate::agent` (ADR 0007 §D7).
pub fn clamp_step_budget(value: u8) -> u8 {
    value.clamp(AGENT_STEP_BUDGET_MIN, AGENT_STEP_BUDGET_MAX)
}

/// Hard-coded system prompt injected as the first message of every turn.
/// Must not be user-configurable in v0.1.0 (see demand Out of scope).
pub(crate) const AGENT_SYSTEM_PROMPT: &str =
    "You are a CAD assistant embedded in LaserCAD v2, a 2D laser-cutting CAD \
     tool. All coordinates and dimensions are in millimetres (mm). Angles at \
     the user interface are in degrees. Use the provided tools to create, \
     modify, or query the open drawing. Prefer the fewest tool calls that \
     satisfy the request. Confirm what you did in one or two concise sentences.";

// ── Error ────────────────────────────────────────────────────────────────────

/// Errors that [`run_agent_turn`] can return.
#[derive(Debug)]
pub enum AgentError {
    /// HTTP or serialisation failure from the transport layer.
    Transport(String),
    /// A tool call could not be dispatched.
    ToolDispatch(String),
    /// The loop guard fired. Carries the budget that was in force, so the
    /// message names the real number rather than a constant.
    IterationLimitExceeded(u8),
    /// The endpoint answered with neither content nor tool calls.
    NoContent,
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(m) => write!(f, "transport error: {m}"),
            Self::ToolDispatch(m) => write!(f, "tool dispatch error: {m}"),
            Self::IterationLimitExceeded(budget) => {
                write!(f, "step budget exceeded ({budget} tool calls per turn)")
            }
            Self::NoContent => write!(f, "API response contained neither content nor tool calls"),
        }
    }
}

impl std::error::Error for AgentError {}

// ── Inner loop (testable seam) ───────────────────────────────────────────────

/// Inner loop with injectable `send_fn` (HTTP) and `dispatch_fn` (tool
/// execution) closures — unit tests substitute stubs without a live endpoint.
///
/// `step_budget` is the number of individual tool-call dispatches this turn may
/// make; the guard fires **before** dispatching any call of a batch that would
/// cross it, so a turn never half-applies a batch it cannot finish. The
/// comparison is done in `usize` — `u8` arithmetic would wrap on a large batch
/// and wave it through.
///
/// All document mutation flows through `dispatch_fn` → `History::commit`.
pub(crate) fn agent_loop<F, D>(
    send_fn: &mut F,
    dispatch_fn: &mut D,
    messages: &mut Vec<ChatMessage>,
    doc: &mut Document,
    history: &mut History,
    step_budget: u8,
) -> Result<String, AgentError>
where
    F: FnMut(&[ChatMessage]) -> Result<AssistantMessage, AgentError>,
    D: FnMut(&str, &str, &mut Document, &mut History) -> Result<String, AgentError>,
{
    let budget = step_budget as usize;
    let mut dispatched: usize = 0;
    loop {
        let message = send_fn(messages)?;
        match (message.tool_calls, message.content) {
            (Some(calls), content) if !calls.is_empty() => {
                if dispatched + calls.len() > budget {
                    return Err(AgentError::IterationLimitExceeded(step_budget));
                }
                messages.push(ChatMessage::assistant_with_tool_calls(
                    content,
                    calls.clone(),
                ));
                for call in &calls {
                    let result =
                        dispatch_fn(&call.function.name, &call.function.arguments, doc, history)?;
                    messages.push(ChatMessage::tool_result(call.id.clone(), result));
                    dispatched += 1;
                }
            }
            (_, Some(text)) => return Ok(text),
            _ => return Err(AgentError::NoContent),
        }
    }
}

// ── Public entry point ───────────────────────────────────────────────────────

/// Drive one complete agent turn: send `prompt` to `endpoint` as `model`,
/// dispatch tool calls through `history` (Ctrl+Z undoable), return the final
/// assistant text.
///
/// `model` and `step_budget` are the caller's to choose — they come from
/// `Settings`, and the budget must already have been through
/// [`clamp_step_budget`]. At most `step_budget` dispatches happen per turn.
pub fn run_agent_turn(
    prompt: &str,
    endpoint: &str,
    api_key: &str,
    model: &str,
    step_budget: u8,
    doc: &mut Document,
    history: &mut History,
) -> Result<String, AgentError> {
    let mut messages = vec![
        ChatMessage::system(AGENT_SYSTEM_PROMPT),
        ChatMessage::user(prompt),
    ];
    // Built once; every round offers the model the same schemas.
    let tools = crate::agent::tools::tool_definitions();
    let mut send_fn = |msgs: &[ChatMessage]| -> Result<AssistantMessage, AgentError> {
        // The slice goes through untouched: filtering it would drop the
        // assistant turns that carry tool calls and the tool turns that answer
        // them, leaving holes in the conversation the model reads back.
        crate::agent::transport::chat_completion(endpoint, api_key, model, msgs, &tools)
            .map_err(|e| AgentError::Transport(e.to_string()))
    };
    let mut dispatch_fn = |name: &str, args: &str, doc: &mut Document, hist: &mut History| {
        let value = serde_json::from_str::<serde_json::Value>(args)
            .map_err(|e| AgentError::ToolDispatch(e.to_string()))?;
        crate::agent::tools::dispatch_tool_call(name, &value, doc, hist)
            .map_err(|e| AgentError::ToolDispatch(e.to_string()))
    };
    agent_loop(
        &mut send_fn,
        &mut dispatch_fn,
        &mut messages,
        doc,
        history,
        step_budget,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::wire::ToolCall;
    use crate::document::{Document, History};
    use serde_json::{json, Value};
    use std::sync::{Arc, Mutex};

    fn ctx() -> (Document, History) {
        (Document::default(), History::new())
    }

    fn text_reply(text: &str) -> Result<AssistantMessage, AgentError> {
        Ok(AssistantMessage {
            content: Some(text.to_owned()),
            tool_calls: None,
        })
    }

    fn call_reply(n: usize) -> Result<AssistantMessage, AgentError> {
        let calls = (0..n)
            .map(|i| ToolCall::function(format!("call_{i}"), "noop", "{}"))
            .collect();
        Ok(AssistantMessage {
            content: None,
            tool_calls: Some(calls),
        })
    }

    // ── AC 10: the step budget replaces the constant ─────────────────────────

    /// AC 10 — the three constants are the documented numbers.
    #[test]
    fn step_budget_constants_are_12_1_and_32() {
        assert_eq!(AGENT_STEP_BUDGET_DEFAULT, 12);
        assert_eq!(AGENT_STEP_BUDGET_MIN, 1);
        assert_eq!(AGENT_STEP_BUDGET_MAX, 32);
    }

    /// AC 10 — `clamp_step_budget` holds the range at both ends and leaves
    /// everything inside it alone. The `0 → 1` and `200 → 32` rows are what
    /// fail if the clamp is ever replaced by the identity function.
    #[test]
    fn clamp_step_budget_holds_the_range() {
        for (stored, expected) in [
            (0u8, 1u8),
            (1, 1),
            (2, 2),
            (12, 12),
            (32, 32),
            (33, 32),
            (200, 32),
            (255, 32),
        ] {
            assert_eq!(
                clamp_step_budget(stored),
                expected,
                "clamp_step_budget({stored}) must be {expected}"
            );
        }
    }

    // ── AC 11: the budget is a parameter, and the guard fires early ──────────

    /// AC 11 — a batch that would cross the budget is refused **before** any of
    /// its calls is dispatched: budget 1, two calls in one response, zero
    /// dispatches.
    #[test]
    fn budget_of_one_refuses_a_two_call_batch_before_dispatching() {
        let (mut doc, mut history) = ctx();
        let mut dispatches = 0usize;
        let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let result = agent_loop(
            &mut |_| call_reply(2),
            &mut |_, _, _, _| {
                dispatches += 1;
                Ok("ok".into())
            },
            &mut messages,
            &mut doc,
            &mut history,
            1,
        );
        assert!(
            matches!(result, Err(AgentError::IterationLimitExceeded(1))),
            "got {result:?}"
        );
        assert_eq!(dispatches, 0, "not one call of the batch may be applied");
    }

    /// AC 11 — the guard counts across rounds: two batches of 6 fit a budget of
    /// 12, the third is refused with 12 dispatches already done.
    #[test]
    fn guard_counts_dispatches_across_rounds() {
        let (mut doc, mut history) = ctx();
        let mut dispatches = 0usize;
        let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let result = agent_loop(
            &mut |_| call_reply(6),
            &mut |_, _, _, _| {
                dispatches += 1;
                Ok("ok".into())
            },
            &mut messages,
            &mut doc,
            &mut history,
            AGENT_STEP_BUDGET_DEFAULT,
        );
        assert!(
            matches!(result, Err(AgentError::IterationLimitExceeded(12))),
            "got {result:?}"
        );
        assert_eq!(dispatches, 12);
    }

    /// AC 11 — a turn that uses exactly the budget still succeeds.
    #[test]
    fn exactly_the_budget_is_allowed() {
        let (mut doc, mut history) = ctx();
        let (mut rounds, mut dispatches) = (0usize, 0usize);
        let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let result = agent_loop(
            &mut |_| {
                rounds += 1;
                if rounds <= 2 {
                    call_reply(3)
                } else {
                    text_reply("done")
                }
            },
            &mut |_, _, _, _| {
                dispatches += 1;
                Ok("ok".into())
            },
            &mut messages,
            &mut doc,
            &mut history,
            6,
        );
        assert_eq!(result.unwrap(), "done");
        assert_eq!(dispatches, 6);
    }

    /// AC 11 — the error names the budget that was in force, not a constant.
    #[test]
    fn iteration_limit_display_names_the_budget() {
        assert_eq!(
            AgentError::IterationLimitExceeded(7).to_string(),
            "step budget exceeded (7 tool calls per turn)"
        );
        assert_eq!(
            AgentError::IterationLimitExceeded(2).to_string(),
            "step budget exceeded (2 tool calls per turn)"
        );
    }

    // ── Loop control flow ────────────────────────────────────────────────────

    /// A text-only reply ends the turn without dispatching anything.
    #[test]
    fn text_only_response_returns_ok() {
        let (mut doc, mut history) = ctx();
        let mut dispatches = 0usize;
        let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let result = agent_loop(
            &mut |_| text_reply("Done."),
            &mut |_, _, _, _| {
                dispatches += 1;
                Ok("ok".into())
            },
            &mut messages,
            &mut doc,
            &mut history,
            AGENT_STEP_BUDGET_DEFAULT,
        );
        assert_eq!(result.unwrap(), "Done.");
        assert_eq!(dispatches, 0);
        assert_eq!(messages.len(), 2);
    }

    /// AC 6 — the appended turns keep the shape the model needs to read back:
    /// an assistant turn carrying the tool calls, then a `tool` turn whose
    /// `tool_call_id` is the id that came down the wire.
    #[test]
    fn a_tool_round_appends_an_assistant_turn_and_a_matching_tool_turn() {
        let (mut doc, mut history) = ctx();
        let mut rounds = 0usize;
        let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let result = agent_loop(
            &mut |_| {
                rounds += 1;
                if rounds == 1 {
                    call_reply(1)
                } else {
                    text_reply("Line created.")
                }
            },
            &mut |_, _, _, _| Ok("Line created: ….".into()),
            &mut messages,
            &mut doc,
            &mut history,
            AGENT_STEP_BUDGET_DEFAULT,
        );
        assert_eq!(result.unwrap(), "Line created.");
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[2].role, "assistant");
        let calls = messages[2].tool_calls.as_ref().expect("tool calls kept");
        assert_eq!(calls[0].id, "call_0");
        assert_eq!(messages[3].role, "tool");
        assert_eq!(messages[3].tool_call_id.as_deref(), Some("call_0"));
        assert_eq!(messages[3].content.as_deref(), Some("Line created: …."));
    }

    /// A reply with neither text nor tool calls ends the turn as an error.
    #[test]
    fn no_content_returns_error() {
        let (mut doc, mut history) = ctx();
        let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let result = agent_loop(
            &mut |_| {
                Ok(AssistantMessage {
                    content: None,
                    tool_calls: None,
                })
            },
            &mut |_, _, _, _| Ok("ok".into()),
            &mut messages,
            &mut doc,
            &mut history,
            AGENT_STEP_BUDGET_DEFAULT,
        );
        assert!(matches!(result, Err(AgentError::NoContent)));
    }

    /// A transport failure is surfaced, not swallowed.
    #[test]
    fn transport_error_propagated() {
        let (mut doc, mut history) = ctx();
        let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let result = agent_loop(
            &mut |_| Err(AgentError::Transport("timeout".into())),
            &mut |_, _, _, _| Ok("ok".into()),
            &mut messages,
            &mut doc,
            &mut history,
            AGENT_STEP_BUDGET_DEFAULT,
        );
        assert!(matches!(result, Err(AgentError::Transport(_))));
    }

    /// A failing dispatch stops the rest of its batch.
    #[test]
    fn tool_dispatch_error_stops_batch() {
        let (mut doc, mut history) = ctx();
        let (mut applied, mut seen) = (0usize, 0usize);
        let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let result = agent_loop(
            &mut |_| call_reply(3),
            &mut |_, _, _, _| {
                seen += 1;
                if seen == 2 {
                    Err(AgentError::ToolDispatch("fail".into()))
                } else {
                    applied += 1;
                    Ok("ok".into())
                }
            },
            &mut messages,
            &mut doc,
            &mut history,
            AGENT_STEP_BUDGET_DEFAULT,
        );
        assert!(matches!(result, Err(AgentError::ToolDispatch(_))));
        assert_eq!(applied, 1);
    }

    /// Every error variant says something.
    #[test]
    fn agent_error_display_is_non_empty() {
        for e in &[
            AgentError::Transport("x".into()),
            AgentError::ToolDispatch("y".into()),
            AgentError::IterationLimitExceeded(3),
            AgentError::NoContent,
        ] {
            assert!(!format!("{e}").is_empty(), "empty Display for {e:?}");
        }
    }

    // ── mockito: the whole turn, over a real socket ──────────────────────────

    /// Captures every request body the mock server receives, in arrival order.
    ///
    /// mockito evaluates **every** mock's request matcher against **every**
    /// request, so this is attached to exactly one mock of a sequence and still
    /// sees all of them, once each. The `len()` assertions in the tests below
    /// are what would catch it if that ever changed.
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

    fn tool_call_body(id: &str, arguments: &str) -> String {
        json!({"choices": [{"message": {
            "role": "assistant",
            "content": Value::Null,
            "tool_calls": [{
                "id": id,
                "type": "function",
                "function": {"name": "create_line", "arguments": arguments}
            }]
        }}]})
        .to_string()
    }

    const LINE_ARGS: &str = r#"{"x1":0,"y1":0,"x2":20,"y2":0}"#;

    /// AC 6 — a two-round turn sends the whole conversation back on the second
    /// request: system, user, the assistant turn **with** its `tool_calls`, and
    /// the `tool` turn with the matching id and the real dispatch outcome, in
    /// that order. This is the test that fails if `send_fn` ever filters the
    /// slice on `content` again.
    #[test]
    fn multi_step_turn_sends_the_whole_conversation_back() {
        let mut server = mockito::Server::new();
        let bodies = Bodies::default();
        let _first = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(tool_call_body("call_xyz", LINE_ARGS))
            .expect(1)
            .create();
        let _second = server
            .mock("POST", "/chat/completions")
            .match_request(bodies.matcher())
            .with_status(200)
            .with_body(r#"{"choices":[{"message":{"role":"assistant","content":"Done."}}]}"#)
            .create();

        let (mut doc, mut history) = ctx();
        let reply = run_agent_turn(
            "draw a line",
            &server.url(),
            "k",
            "test/model",
            AGENT_STEP_BUDGET_DEFAULT,
            &mut doc,
            &mut history,
        )
        .expect("the turn must finish");

        assert_eq!(reply, "Done.");
        assert_eq!(doc.entities.len(), 1, "the tool call really was dispatched");
        assert_eq!(bodies.len(), 2, "exactly two round trips");

        let second = bodies.json(1);
        let msgs = second["messages"].as_array().expect("messages array");
        let roles: Vec<&str> = msgs.iter().filter_map(|m| m["role"].as_str()).collect();
        assert_eq!(
            roles,
            ["system", "user", "assistant", "tool"],
            "no turn may be dropped or reordered, body was {second}"
        );
        assert_eq!(msgs[2]["tool_calls"][0]["id"], "call_xyz");
        assert_eq!(msgs[2]["tool_calls"][0]["function"]["arguments"], LINE_ARGS);
        assert_eq!(msgs[3]["tool_call_id"], "call_xyz");
        assert_eq!(
            msgs[3]["content"],
            "Line created: (0.000, 0.000) → (20.000, 0.000) mm."
        );
        assert_eq!(
            second["tools"].as_array().map(|t| t.len()),
            Some(5),
            "every round offers the tools, body was {second}"
        );
    }

    /// AC 9 — `run_agent_turn` forwards the model id it was given, unchanged.
    /// Two different ids, so a hardcoded default cannot satisfy both.
    #[test]
    fn run_agent_turn_sends_the_model_it_was_given() {
        for model in ["anthropic/claude-sonnet-4.6", "test/some-other-model"] {
            let mut server = mockito::Server::new();
            let bodies = Bodies::default();
            let _mock = server
                .mock("POST", "/chat/completions")
                .match_request(bodies.matcher())
                .with_status(200)
                .with_body(r#"{"choices":[{"message":{"role":"assistant","content":"hi"}}]}"#)
                .create();

            let (mut doc, mut history) = ctx();
            let reply = run_agent_turn(
                "hello",
                &server.url(),
                "k",
                model,
                AGENT_STEP_BUDGET_DEFAULT,
                &mut doc,
                &mut history,
            );
            assert_eq!(reply.unwrap(), "hi");
            assert_eq!(bodies.len(), 1);
            assert_eq!(bodies.json(0)["model"], model);
        }
    }

    /// AC 11 — against an endpoint that only ever asks for another tool call,
    /// the turn stops at the budget, says the real number, and has applied no
    /// more than `budget` actions.
    #[test]
    fn a_relentless_endpoint_stops_at_the_budget() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(tool_call_body("call_loop", LINE_ARGS))
            .create();

        let (mut doc, mut history) = ctx();
        let result = run_agent_turn(
            "draw forever",
            &server.url(),
            "k",
            "test/model",
            2,
            &mut doc,
            &mut history,
        );

        match result {
            Err(AgentError::IterationLimitExceeded(budget)) => {
                assert_eq!(budget, 2);
                assert!(AgentError::IterationLimitExceeded(budget)
                    .to_string()
                    .contains('2'));
            }
            other => panic!("expected the budget to stop the turn, got {other:?}"),
        }
        assert_eq!(
            doc.entities.len(),
            2,
            "at most the budget may be dispatched"
        );
    }

    /// A transport-level status error reaches the caller as `Transport`, with
    /// the readable text the transport produced.
    #[test]
    fn a_401_reaches_the_caller_as_a_transport_error() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(401)
            .with_body("nope")
            .create();

        let (mut doc, mut history) = ctx();
        let result = run_agent_turn(
            "hello",
            &server.url(),
            "k",
            "test/model",
            AGENT_STEP_BUDGET_DEFAULT,
            &mut doc,
            &mut history,
        );
        match result {
            Err(AgentError::Transport(message)) => {
                assert!(
                    message.contains("Authentication failed (HTTP 401)"),
                    "{message}"
                );
            }
            other => panic!("expected a transport error, got {other:?}"),
        }
    }

    // ── AC 2: the wire types are declared once, in wire.rs ───────────────────

    /// AC 2 — no duplicate wire struct survives in this file, and the shared
    /// declarations are imported instead. Bounded to the implementation
    /// section and built with `concat!`, so the scan cannot match the needles
    /// written here; pasting any of those structs back above the
    /// `#[cfg(test)]` marker turns this red.
    #[test]
    fn loop_declares_no_wire_structs_of_its_own() {
        let src = include_str!("loop_.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("loop_.rs must have a bare #[cfg(test)] marker");
        let implementation = &src[..at];

        let import = concat!("use crate::agent::", "wire");
        assert!(
            implementation.contains(import),
            "positive control: loop_.rs must import the shared wire types"
        );
        for duplicate in [
            concat!("struct ", "ChatResponse"),
            concat!("struct ", "Choice"),
            concat!("struct ", "ChoiceMessage"),
            concat!("struct ", "AssistantMessage"),
            concat!("struct ", "ToolCall"),
            concat!("struct ", "ToolCallFunction"),
            concat!("struct ", "ChatMessage"),
        ] {
            assert!(
                !implementation.contains(duplicate),
                "`{duplicate}` belongs in wire.rs, not loop_.rs"
            );
        }
    }
}
