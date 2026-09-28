//! LCV-142 — the worker-side half of an agent turn (ADR 0007 §D8, amendment
//! 7): [`TurnConfig`], [`run_agent_turn`] and [`ask_ui`].
//!
//! Split out of `agent_turn.rs`, which keeps the UI-side half (the fence,
//! arming and spawning). Everything here runs on the turn's own thread and
//! owns no document state of any kind (ADR 0007 §D1): it knows how to talk to
//! an endpoint and how to feed a tool result back, and it learns what a tool
//! call *did* by asking, through the `ask` seam, whoever owns the drawing. In
//! the app that is [`ask_ui`], which blocks on the UI thread's answer; in a
//! test it is a closure.

use crate::agent::{
    agent_loop, AgentAction, AgentError, AgentEvent, AgentOutcome, AssistantMessage, ChatMessage,
};
use std::sync::mpsc::{channel, Sender};

/// Everything that crosses into the worker thread, as one owned value (ADR
/// 0007 §D13). Built once, in `start_turn`, from `Settings`: that is the
/// turn-start snapshot, so settings edited mid-turn affect the next turn only.
///
/// Carries the API key and the prompt, so its `Debug` is written by hand: it
/// prints `api_key: "<redacted>"` (ADR 0007 §D10) and the prompt's length
/// only (LCV-143 AC 7).
#[derive(Clone, PartialEq, Eq)]
pub struct TurnConfig {
    /// OpenAI-compatible base URL.
    pub endpoint: String,
    /// Bearer token. Never printed.
    pub api_key: String,
    /// Model id sent with every request.
    pub model: String,
    /// The turn's effective step limit, already clamped at the read site.
    pub step_limit: u32,
    /// The turn's system message, resolved from `Settings` when the turn was
    /// armed (LCV-143). Never printed.
    pub system_prompt: String,
}

impl std::fmt::Debug for TurnConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TurnConfig")
            .field("endpoint", &self.endpoint)
            .field("api_key", &"<redacted>")
            .field("model", &self.model)
            .field("step_limit", &self.step_limit)
            .field("system_prompt_len", &self.system_prompt.len())
            .finish()
    }
}

/// Drive one complete agent turn: send `prompt` to `config.endpoint` as
/// `config.model`, ask `ask` to carry out every tool call the model requests,
/// return its final text.
///
/// Owns no document state of any kind (ADR 0007 §D1). `ask` is the whole seam:
/// it receives one [`AgentAction`] and answers with what really happened to the
/// real drawing. In the app that answer comes from another thread; here it is
/// just a call.
///
/// `config` is the caller's snapshot of `Settings`; its `step_limit` has
/// already been through [`crate::agent::clamp_step_budget`]. At most
/// `step_limit` actions are dispatched per turn.
///
/// # Errors
///
/// [`AgentError`] for a transport failure, an exhausted step budget, a reply carrying neither text nor tool calls, tool calls asked
/// for after a `Fenced` answer stopped the turn ([`AgentError::FenceStopped`],
/// §D14), or [`AgentError::Cancelled`] when `ask` reports that nobody is left
/// to answer.
/// An action the document *refuses* is **not** an error, and neither is a
/// malformed tool call: either goes back to the model as the tool result and
/// the turn continues (ADR 0007 §D2a, §D15).
pub fn run_agent_turn<A>(
    prompt: &str,
    config: &TurnConfig,
    ask: &mut A,
) -> Result<String, AgentError>
where
    A: FnMut(AgentAction) -> Result<AgentOutcome, AgentError>,
{
    // Built once; every round offers the model the same schemas.
    let tools = crate::agent::tool_definitions();
    let mut send_fn = |msgs: &[ChatMessage]| {
        // The slice goes through untouched: filtering it would drop the
        // assistant turns that carry tool calls and the tool turns that answer
        // them, leaving holes in the conversation the model reads back.
        let TurnConfig {
            endpoint,
            api_key,
            model,
            ..
        } = config;
        crate::agent::chat_completion(endpoint, api_key, model, msgs, &tools)
            .map_err(|e| AgentError::Transport(e.to_string()))
    };
    drive_turn(
        prompt,
        &config.system_prompt,
        config.step_limit,
        &mut send_fn,
        ask,
    )
}

/// [`run_agent_turn`] minus the network: the conversation, the parse and the
/// `ask`, with `send_fn` injected so the worker's own rules — the fence stop
/// (§D14) and malformed calls (§D15) — are testable without an endpoint.
/// `system` is the turn's system message, verbatim.
fn drive_turn<F, A>(
    prompt: &str,
    system: &str,
    step_limit: u32,
    send_fn: &mut F,
    ask: &mut A,
) -> Result<String, AgentError>
where
    F: FnMut(&[ChatMessage]) -> Result<AssistantMessage, AgentError>,
    A: FnMut(AgentAction) -> Result<AgentOutcome, AgentError>,
{
    let mut messages = vec![ChatMessage::system(system), ChatMessage::user(prompt)];
    // A refusal is a tool result, not a failure (ADR 0007 §D2a), and so is a
    // malformed call (§D15); a `Fenced` answer is read by `agent_loop` (§D14).
    let mut dispatch_fn = |name: &str, args: &str| ask(to_action(name, args));
    agent_loop(send_fn, &mut dispatch_fn, &mut messages, step_limit)
}

/// Shape-check one tool call. Anything that fails — JSON syntax, unknown tool,
/// any `ToolCallError` — becomes [`AgentAction::Malformed`] and still goes to
/// the UI as an ordinary `Act` (ADR 0007 §D15). The reason never quotes `args`.
fn to_action(name: &str, args: &str) -> AgentAction {
    let malformed = |reason: String| AgentAction::Malformed {
        tool: name.to_owned(),
        reason,
    };
    // The argument-free queries are routinely called with `""` rather than
    // `"{}"`, which is not JSON; both mean the same empty object here.
    let value = if args.trim().is_empty() {
        serde_json::Value::Null
    } else {
        match serde_json::from_str::<serde_json::Value>(args) {
            Ok(value) => value,
            Err(e) => return malformed(format!("tool `{name}` arguments are not valid JSON: {e}")),
        }
    };
    crate::agent::parse_tool_call(name, &value).unwrap_or_else(|e| malformed(e.to_string()))
}

/// The worker thread's `ask`: one rendezvous with the UI thread (ADR 0007 §D2).
///
/// A fresh one-shot channel per action, so the reply `Sender` rides inside the
/// message and nothing is ever parked on `App` (§D3). Both failures mean the
/// same thing — the UI is gone, or has abandoned this turn — and both are
/// [`AgentError::Cancelled`], on which the caller returns without sending a
/// terminal event, because there is nobody left to read one.
pub(super) fn ask_ui(
    tx: &Sender<AgentEvent>,
    action: AgentAction,
) -> Result<AgentOutcome, AgentError> {
    let (reply, answer) = channel::<AgentOutcome>();
    tx.send(AgentEvent::Act { action, reply })
        .map_err(|_| AgentError::Cancelled)?;
    answer.recv().map_err(|_| AgentError::Cancelled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AGENT_STEP_BUDGET_DEFAULT;
    use crate::app::{agent_apply, App};
    use serde_json::{json, Value};
    use std::sync::{Arc, Mutex};
    /// The `ask` seam, wired to a real `App`, recording what it was asked.
    ///
    /// This is the shape ADR 0007 §D1 buys: the turn is exercised end to end
    /// against a live document with no thread, no channel and no socket
    /// between them, because the only thing the turn knows how to do with an
    /// action is hand it to whoever owns the drawing.
    struct Applier {
        app: App,
        seen: Vec<AgentAction>,
        cancel: bool,
    }

    impl Applier {
        fn new() -> Self {
            // ADR 0002 §A2: `App::default()` never touches the developer's
            // real settings or autosave paths.
            Self {
                app: App::default(),
                seen: Vec::new(),
                cancel: false,
            }
        }

        /// Report, on every ask, that nobody is left to answer (AC 5).
        fn cancelling() -> Self {
            Self {
                cancel: true,
                ..Self::new()
            }
        }

        fn ask(&mut self, action: AgentAction) -> Result<AgentOutcome, AgentError> {
            self.seen.push(action.clone());
            if self.cancel {
                return Err(AgentError::Cancelled);
            }
            Ok(agent_apply::apply(&mut self.app, &action))
        }

        fn entities(&self) -> usize {
            self.app.document.entity_count()
        }
    }

    // ── The turn, over a real socket ─────────────────────────────────────────

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

    fn named_tool_call_body(id: &str, name: &str, arguments: &str) -> String {
        json!({"choices": [{"message": {
            "role": "assistant",
            "content": Value::Null,
            "tool_calls": [{
                "id": id,
                "type": "function",
                "function": {"name": name, "arguments": arguments}
            }]
        }}]})
        .to_string()
    }

    fn tool_call_body(id: &str, arguments: &str) -> String {
        named_tool_call_body(id, "create_line", arguments)
    }

    fn config(endpoint: &str, model: &str, step_limit: u32) -> TurnConfig {
        TurnConfig {
            endpoint: endpoint.to_owned(),
            api_key: "k".to_owned(),
            model: model.to_owned(),
            step_limit,
            system_prompt: "test system prompt".to_owned(),
        }
    }

    /// LCV-142 AC 11 — `Debug` never prints the key, and says it withheld it.
    #[test]
    fn turn_config_debug_redacts_the_api_key() {
        let config = TurnConfig {
            api_key: "sk-test-DO-NOT-LEAK".to_owned(),
            ..config("https://example.invalid", "m", 7)
        };
        let printed = format!("{config:?}");
        assert!(!printed.contains("sk-test-DO-NOT-LEAK"), "{printed}");
        assert!(!printed.contains("DO-NOT-LEAK"), "{printed}");
        assert!(printed.contains(r#"api_key: "<redacted>""#), "{printed}");
        assert!(printed.contains("step_limit: 7"), "{printed}");
        let pretty = format!("{config:#?}");
        assert!(!pretty.contains("sk-test"), "{pretty}");
    }

    /// LCV-143 AC 5 / AC 7 — `Debug` omits the prompt text and prints its
    /// length only.
    #[test]
    fn turn_config_debug_omits_the_system_prompt() {
        let config = TurnConfig {
            system_prompt: "PROMPT-SENTINEL-91c2".to_owned(),
            ..config("https://example.invalid", "m", 7)
        };
        for printed in [format!("{config:?}"), format!("{config:#?}")] {
            assert!(!printed.contains("SENTINEL"), "{printed}");
            assert!(printed.contains("system_prompt_len"), "{printed}");
            assert!(printed.contains("20"), "{printed}");
        }
        assert!(format!("{config:?}").contains("system_prompt_len: 20"));
    }

    /// LCV-143 AC 5 — `drive_turn` seeds the conversation with exactly the
    /// system text it is handed, then the user's prompt.
    #[test]
    fn drive_turn_seeds_the_given_system_prompt() {
        const SENTINEL: &str = "SYSTEM-SENTINEL\n  kept  verbatim ";
        let mut seen = Vec::new();
        let mut send_fn = |msgs: &[ChatMessage]| {
            seen = msgs.to_vec();
            Ok(text("ok"))
        };
        let mut ask = |_: AgentAction| Ok(AgentOutcome::Ok(String::new()));
        let result = drive_turn("hi", SENTINEL, 3, &mut send_fn, &mut ask);
        assert_eq!(result.expect("text ends the turn"), "ok");
        assert_eq!(seen[0].role, "system");
        assert_eq!(seen[0].content.as_deref(), Some(SENTINEL));
        assert_eq!(seen[1].role, "user");
        assert_eq!(seen[1].content.as_deref(), Some("hi"));
    }

    /// LCV-143 AC 5 — `run_agent_turn` sends `config.system_prompt` as the
    /// first message, not a built-in constant.
    #[test]
    fn run_agent_turn_sends_the_configured_system_prompt() {
        let mut server = mockito::Server::new();
        let bodies = Bodies::default();
        let _mock = server
            .mock("POST", "/chat/completions")
            .match_request(bodies.matcher())
            .with_status(200)
            .with_body(r#"{"choices":[{"message":{"role":"assistant","content":"hi"}}]}"#)
            .create();
        let config = TurnConfig {
            system_prompt: "CONFIGURED-SENTINEL".to_owned(),
            ..config(&server.url(), "m", AGENT_STEP_BUDGET_DEFAULT)
        };
        let mut applier = Applier::new();
        let reply = run_agent_turn("hello", &config, &mut |a| applier.ask(a));
        assert_eq!(reply.expect("the turn finishes"), "hi");
        let first = &bodies.json(0)["messages"][0];
        assert_eq!(first["role"], "system");
        assert_eq!(first["content"], "CONFIGURED-SENTINEL");
    }

    const LINE_ARGS: &str = r#"{"x1":0,"y1":0,"x2":20,"y2":0}"#;

    /// LCV-121 AC 6, moved here with the turn — a two-round turn sends the
    /// whole conversation back on the second request: system, user, the
    /// assistant turn **with** its `tool_calls`, and the `tool` turn with the
    /// matching id and the real outcome, in that order. This is the test that
    /// fails if `send_fn` ever filters the slice on `content` again, and it is
    /// also the one live-path proof that `parse_tool_call` → `agent_apply`
    /// really is wired end to end.
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

        let mut applier = Applier::new();
        let reply = run_agent_turn(
            "draw a line",
            &config(&server.url(), "test/model", AGENT_STEP_BUDGET_DEFAULT),
            &mut |action| applier.ask(action),
        )
        .expect("the turn must finish");

        assert_eq!(reply, "Done.");
        assert_eq!(applier.entities(), 1, "the tool call really was dispatched");
        assert_eq!(
            applier.seen,
            [AgentAction::CreateLine {
                x1: 0.0,
                y1: 0.0,
                x2: 20.0,
                y2: 0.0
            }],
            "AC 4: one ask per tool call, carrying the parsed action"
        );
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
            "Line created: (0.000, 0.000) → (20.000, 0.000) mm. \
             The drawing now has 1 entities.",
            "the model reads the real outcome, count and all (AC 9)"
        );
        assert_eq!(
            second["tools"].as_array().map(|t| t.len()),
            Some(7),
            "every round offers the tools, body was {second}"
        );
    }

    /// AC 11 / ADR 0007 §D2a — a refusal is a **tool result**, not a turn
    /// failure.
    ///
    /// The range check lives at the apply site now, so the only thing that can
    /// refuse an out-of-range index is `agent_apply`, and the only useful place
    /// to put its answer is back in the model's hands: it can then apologise,
    /// re-read the drawing, or pick a real index. Failing the turn instead
    /// would throw the conversation away over a recoverable mistake, and the
    /// user would see a red error row for what is really the model asking a
    /// question badly.
    #[test]
    fn a_refusal_is_fed_back_as_a_tool_result_and_the_turn_survives() {
        let mut server = mockito::Server::new();
        let bodies = Bodies::default();
        let _first = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(named_tool_call_body(
                "call_bad",
                "delete_entity",
                r#"{"index":7}"#,
            ))
            .expect(1)
            .create();
        let _second = server
            .mock("POST", "/chat/completions")
            .match_request(bodies.matcher())
            .with_status(200)
            .with_body(
                r#"{"choices":[{"message":{"role":"assistant","content":"There is nothing there."}}]}"#,
            )
            .create();

        let mut applier = Applier::new();
        let before = applier.app.history.revision();
        let reply = run_agent_turn(
            "delete entity 7",
            &config(&server.url(), "test/model", AGENT_STEP_BUDGET_DEFAULT),
            &mut |action| applier.ask(action),
        )
        .expect("a refused tool call must not fail the turn");

        assert_eq!(reply, "There is nothing there.");
        assert_eq!(bodies.len(), 2, "the turn went another round");
        assert_eq!(applier.entities(), 0, "nothing was applied");
        assert_eq!(
            applier.app.history.revision(),
            before,
            "nothing was committed"
        );

        let second = bodies.json(1);
        let msgs = second["messages"].as_array().expect("messages array");
        let roles: Vec<&str> = msgs.iter().filter_map(|m| m["role"].as_str()).collect();
        assert_eq!(roles, ["system", "user", "assistant", "tool"]);
        assert_eq!(msgs[3]["tool_call_id"], "call_bad");
        assert_eq!(
            msgs[3]["content"], "index 7 is out of range (the drawing has 0 entities)",
            "the model must be told what went wrong, in the apply site's words"
        );
    }

    /// LCV-121 AC 9 — `run_agent_turn` forwards the model id it was given, unchanged.
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

            let mut applier = Applier::new();
            let reply = run_agent_turn(
                "hello",
                &config(&server.url(), model, AGENT_STEP_BUDGET_DEFAULT),
                &mut |action| applier.ask(action),
            );
            assert_eq!(reply.unwrap(), "hi");
            assert_eq!(bodies.len(), 1);
            assert_eq!(bodies.json(0)["model"], model);
        }
    }

    /// LCV-121 AC 11 — against an endpoint that only ever asks for another tool call,
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

        let mut applier = Applier::new();
        let result = run_agent_turn(
            "draw forever",
            &config(&server.url(), "test/model", 2),
            &mut |action| applier.ask(action),
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
            applier.entities(),
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

        let mut applier = Applier::new();
        let result = run_agent_turn(
            "hello",
            &config(&server.url(), "test/model", AGENT_STEP_BUDGET_DEFAULT),
            &mut |action| applier.ask(action),
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

    // ── AC 4, AC 5: the ask seam ─────────────────────────────────────────────

    /// AC 4 — one ask per tool call, in the order the model asked, and the
    /// string the ask answered with is what the model reads back.
    ///
    /// Two calls in **one** assistant message, so an implementation that
    /// handled only the first — or that reordered them — cannot pass. The
    /// arguments differ in every field, so a transposition is visible too.
    #[test]
    fn every_tool_call_becomes_one_ask_in_order() {
        let mut server = mockito::Server::new();
        let bodies = Bodies::default();
        let _first = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(
                json!({"choices": [{"message": {
                    "role": "assistant",
                    "content": Value::Null,
                    "tool_calls": [
                        {"id": "a", "type": "function", "function":
                            {"name": "create_line",
                             "arguments": r#"{"x1":0,"y1":0,"x2":20,"y2":0}"#}},
                        {"id": "b", "type": "function", "function":
                            {"name": "create_circle",
                             "arguments": r#"{"cx":5,"cy":6,"r":3}"#}}
                    ]
                }}]})
                .to_string(),
            )
            .expect(1)
            .create();
        let _second = server
            .mock("POST", "/chat/completions")
            .match_request(bodies.matcher())
            .with_status(200)
            .with_body(r#"{"choices":[{"message":{"role":"assistant","content":"Done."}}]}"#)
            .create();

        let mut applier = Applier::new();
        let reply = run_agent_turn(
            "draw",
            &config(&server.url(), "test/model", AGENT_STEP_BUDGET_DEFAULT),
            &mut |action| applier.ask(action),
        )
        .expect("the turn must finish");

        assert_eq!(reply, "Done.");
        assert_eq!(
            applier.seen,
            [
                AgentAction::CreateLine {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 20.0,
                    y2: 0.0
                },
                AgentAction::CreateCircle {
                    cx: 5.0,
                    cy: 6.0,
                    r: 3.0
                },
            ],
            "AC 4: one ask per tool call, in order"
        );
        assert_eq!(applier.entities(), 2);

        // And each ask's answer is the tool result the model reads back.
        let second = bodies.json(1);
        let msgs = second["messages"].as_array().expect("messages array");
        assert_eq!(msgs[3]["tool_call_id"], "a");
        assert!(
            msgs[3]["content"]
                .as_str()
                .is_some_and(|c| c.starts_with("Line created:")),
            "body was {second}"
        );
        assert_eq!(msgs[4]["tool_call_id"], "b");
        assert!(
            msgs[4]["content"]
                .as_str()
                .is_some_and(|c| c.starts_with("Circle created:")),
            "body was {second}"
        );
    }

    /// AC 5 — an ask that reports cancellation ends the turn **immediately**.
    ///
    /// The UI is gone, or has abandoned this turn, so there is nothing left to
    /// answer any further question and nothing to spend another request on. The
    /// mock is `.expect(1)`: a turn that carried on would send a second one and
    /// this assertion is what says so.
    #[test]
    fn a_cancelled_ask_returns_at_once_and_sends_nothing_more() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(tool_call_body("call_gone", LINE_ARGS))
            .expect(1)
            .create();

        let mut applier = Applier::cancelling();
        let result = run_agent_turn(
            "draw a line",
            &config(&server.url(), "test/model", AGENT_STEP_BUDGET_DEFAULT),
            &mut |action| applier.ask(action),
        );

        assert!(
            matches!(result, Err(AgentError::Cancelled)),
            "got {result:?}"
        );
        assert_eq!(applier.seen.len(), 1, "it stops at the first dead ask");
        assert_eq!(applier.entities(), 0, "and nothing was applied");
        mock.assert();
    }

    /// AC 4 / AC 8 — the real ask seam **asks**. Every test above this one
    /// substitutes its own callback for [`ask_ui`], so nothing there would
    /// notice a seam that fabricated an answer and never sent an `Act` — the
    /// mutation that silently disconnects the model from the drawing.
    ///
    /// Two halves, both deterministic and neither one timed:
    ///
    /// - a `Sender` whose `Receiver` is already gone must report cancellation
    ///   rather than success, and needs no second thread at all;
    /// - a live rendezvous, where this thread plays the UI: it blocks on the
    ///   channel until the `Act` arrives, checks the action crossed intact,
    ///   answers, and joins. A seam that fabricated would drop its `Sender`
    ///   without sending, so `recv` fails here instead of hanging.
    #[test]
    fn the_ask_seam_sends_the_action_and_waits_for_the_answer() {
        let action = AgentAction::CreateLine {
            x1: 1.0,
            y1: 2.0,
            x2: 3.0,
            y2: 4.0,
        };

        let (dead_tx, dead_rx) = channel::<AgentEvent>();
        drop(dead_rx);
        assert!(
            matches!(ask_ui(&dead_tx, action.clone()), Err(AgentError::Cancelled)),
            "no listener means cancelled, never a fabricated success"
        );

        let (tx, rx) = channel::<AgentEvent>();
        let sent = action.clone();
        let worker = std::thread::spawn(move || ask_ui(&tx, sent));

        let event = rx.recv().expect("the ask must put an Act on the channel");
        let AgentEvent::Act {
            action: crossed,
            reply,
        } = event
        else {
            panic!("the ask must send an Act, not a terminal event");
        };
        assert_eq!(crossed, action, "the action crosses the channel intact");
        reply
            .send(AgentOutcome::Ok("the UI answered".to_owned()))
            .expect("the ask must still be waiting on its reply channel");

        let answered = worker.join().expect("the ask thread must not panic");
        assert_eq!(
            answered.expect("an answered ask succeeds").text(),
            "the UI answered",
            "the seam returns the UI's answer, not one of its own"
        );
    }

    /// AC 1 — `run_agent_turn` owns no document state: this file's
    /// implementation section names neither type, nor a snapshot of one, nor a
    /// shared handle to one.
    ///
    /// The needles are checked against a witness first, so a misspelt one
    /// fails here instead of passing over any haystack at all.
    #[test]
    fn the_turn_owns_no_document_state() {
        let src = include_str!("agent_worker.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("agent_worker.rs must have a bare #[cfg(test)] marker");
        let implementation = &src[..at];

        let witness = "use crate::document::{Document, History}; \
                       let shared: Arc<Mutex<Document>> = x; \
                       let snapshot: Vec<Entity> = y; let d = Document::default();";
        for forbidden in [
            concat!("crate::", "document"),
            concat!("Doc", "ument"),
            concat!("His", "tory"),
            concat!("Arc<", "Mutex"),
            concat!("Vec<", "Entity>"),
        ] {
            assert!(
                witness.contains(forbidden),
                "control: `{forbidden}` must be able to match real document state"
            );
            let hit = implementation
                .lines()
                .find(|l| l.contains(forbidden) && !l.trim_start().starts_with("//"));
            assert!(
                hit.is_none(),
                "AC 1: run_agent_turn's file must not name `{forbidden}`: {hit:?}"
            );
        }
    }

    // ── LCV-129 AC 10: the worker can always notice and unwind ─────────────

    /// LCV-129 AC 10, first case — the receiver is already gone when the
    /// worker tries to send.
    ///
    /// This is the easy half, and it exercises the real [`ask_ui`]: a `send`
    /// on a channel whose `Receiver` has been dropped returns `Err`, which the
    /// mapper turns into `Cancelled`. No thread and no sleep — dropping `rx`
    /// here is exactly what `end_turn` does one frame after a cancel.
    #[test]
    fn ac10_a_send_with_nobody_listening_is_a_cancel() {
        let (tx, rx) = channel::<AgentEvent>();
        drop(rx);

        let outcome = ask_ui(&tx, AgentAction::QuerySelection);

        assert!(
            matches!(outcome, Err(AgentError::Cancelled)),
            "a dead channel must unwind the turn, got {outcome:?}"
        );
    }

    /// LCV-129 AC 10, second case — the receiver is dropped while an `Act` is
    /// queued and nobody ever read it.
    ///
    /// The genuinely hard half, and the property the whole cancel design rests
    /// on: the worker has already *succeeded* at sending and is parked in
    /// `answer.recv()`, so nothing about the event channel can wake it. What
    /// wakes it is that dropping a `std::sync::mpsc::Receiver` drops the
    /// messages still queued in it, and the reply `Sender` rides *inside* the
    /// `Act` (ADR 0007 §D3) — so the one-shot channel closes as a side effect
    /// and `recv()` returns `Err`. Park this design on a channel that stored
    /// its reply handle on `App` instead and a cancelled worker would block
    /// forever.
    ///
    /// [`ask_ui`] cannot be called here: it would park this very test thread
    /// on `recv()`. So its second half is replicated line for line — same
    /// one-shot channel, same `Act`, same `map_err` — and the drop that a real
    /// cancel performs happens in between, with the `Act` still unread. No
    /// thread, no sleep; if the property ever stopped holding this test would
    /// hang rather than pass, and `send` succeeding is asserted first so a
    /// failure says which half broke.
    #[test]
    fn ac10_dropping_the_receiver_with_an_act_queued_is_a_cancel() {
        let (tx, rx) = channel::<AgentEvent>();
        let (reply, answer) = channel::<AgentOutcome>();
        tx.send(AgentEvent::Act {
            action: AgentAction::QuerySelection,
            reply,
        })
        .expect("the UI is still there when the worker sends");

        // The cancel: `end_turn` drops the receiver, and the unread `Act`
        // — carrying the only `Sender` to `answer` — goes with it.
        drop(rx);

        let outcome: Result<AgentOutcome, AgentError> =
            answer.recv().map_err(|_| AgentError::Cancelled);
        assert!(
            matches!(outcome, Err(AgentError::Cancelled)),
            "a worker parked on an answer nobody will give must unwind, got {outcome:?}"
        );
    }

    // ── LCV-142 AC 9: a `Fenced` answer stops dispatch (ADR 0007 §D14) ───────

    fn batch(calls: &[(&str, &str)]) -> AssistantMessage {
        AssistantMessage {
            content: None,
            tool_calls: Some(
                calls
                    .iter()
                    .enumerate()
                    .map(|(i, (name, args))| {
                        crate::agent::ToolCall::function(format!("call_{i}"), *name, *args)
                    })
                    .collect(),
            ),
        }
    }

    fn text(content: &str) -> AssistantMessage {
        AssistantMessage {
            content: Some(content.to_owned()),
            tool_calls: None,
        }
    }

    /// Drive a fake turn: first reply a batch of three, then `last`, then
    /// (if asked) anything. `ask` answers `Fenced` every time. Returns the
    /// result, the ask count, the send count and the messages the final send
    /// saw.
    fn fenced_turn(
        last: AssistantMessage,
    ) -> (Result<String, AgentError>, usize, usize, Vec<ChatMessage>) {
        let three = batch(&[
            ("query_entities", "{}"),
            ("delete_entity", r#"{"index":0}"#),
            ("query_selection", ""),
        ]);
        let (mut sends, mut asks) = (0usize, 0usize);
        let mut seen = Vec::new();
        let mut send_fn = |msgs: &[ChatMessage]| {
            sends += 1;
            seen = msgs.to_vec();
            Ok(match sends {
                1 => three.clone(),
                2 => last.clone(),
                _ => text("a third send must never happen"),
            })
        };
        let mut ask = |_action: AgentAction| {
            asks += 1;
            Ok(AgentOutcome::Fenced(
                crate::app::AGENT_FENCE_REFUSAL.to_owned(),
            ))
        };
        let result = drive_turn(
            "go",
            "sys",
            AGENT_STEP_BUDGET_DEFAULT,
            &mut send_fn,
            &mut ask,
        );
        (result, asks, sends, seen)
    }

    /// AC 9 — the first `Fenced` answer stops dispatch: `ask` is called once,
    /// the other two calls get the fixed placeholder so every id is answered,
    /// and exactly one more completion is sent, whose text ends the turn `Ok`.
    #[test]
    fn a_fenced_answer_placeholders_the_rest_and_sends_once_more() {
        let (result, asks, sends, seen) = fenced_turn(text("I stopped."));
        assert_eq!(result.expect("text ends the turn Ok"), "I stopped.");
        assert_eq!((asks, sends), (1, 2));

        let tools: Vec<_> = seen.iter().filter(|m| m.role == "tool").collect();
        assert_eq!(tools.len(), 3, "every tool_call_id is answered");
        let ids: Vec<_> = tools.iter().map(|m| m.tool_call_id.clone()).collect();
        let want = ["call_0", "call_1", "call_2"].map(|id| Some(id.to_owned()));
        assert_eq!(ids, want);
        assert_eq!(
            tools[0].content.as_deref(),
            Some(crate::app::AGENT_FENCE_REFUSAL)
        );
        let placeholder = "not run: the turn stopped after the drawing changed outside it";
        assert_eq!(crate::agent::loop_::FENCE_STOP_PLACEHOLDER, placeholder);
        for tool in &tools[1..] {
            assert_eq!(tool.content.as_deref(), Some(placeholder));
        }
    }

    /// AC 9 — if the one last reply asks for more tool calls, none is
    /// dispatched and the turn ends `FenceStopped` with the exact sentence.
    #[test]
    fn tool_calls_after_a_fence_stop_end_the_turn_failed_undispatched() {
        let (result, asks, sends, _) = fenced_turn(batch(&[("query_entities", "{}")]));
        let error = result.expect_err("tool calls after the stop must fail");
        assert!(matches!(error, AgentError::FenceStopped), "got {error:?}");
        assert_eq!(
            error.to_string(),
            "the turn stopped after the drawing changed outside it"
        );
        assert_eq!((asks, sends), (1, 2), "no further ask, no third send");
    }

    // ── LCV-142 AC 10: a malformed call is an `Act` too (ADR 0007 §D15) ──────

    /// AC 10 — invalid JSON, an unknown tool and a missing field each reach
    /// `ask` as `Malformed`, and the turn continues to `Ok`. A sentinel inside
    /// the arguments never reaches a reason.
    #[test]
    fn malformed_calls_reach_ask_and_the_turn_continues() {
        const SENTINEL: &str = "SENTINEL-7f3a";
        let bad_json = format!(r#"{{"x1": {SENTINEL}"#);
        let missing = format!(r#"{{"x1": 0, "y1": 0, "x2": "{SENTINEL}"}}"#);
        let unknown = format!(r#"{{"note": "{SENTINEL}"}}"#);
        let three = batch(&[
            ("create_line", &bad_json),
            ("draw_unicorn", &unknown),
            ("create_line", &missing),
        ]);
        let mut sends = 0usize;
        let mut send_fn = |_: &[ChatMessage]| {
            sends += 1;
            Ok(if sends == 1 {
                three.clone()
            } else {
                text("fixed")
            })
        };
        let mut asked = Vec::new();
        let mut ask = |action: AgentAction| {
            asked.push(action.clone());
            match action {
                AgentAction::Malformed { reason, .. } => Ok(AgentOutcome::Refused(reason)),
                other => panic!("expected Malformed, got {other:?}"),
            }
        };
        let result = drive_turn(
            "go",
            "sys",
            AGENT_STEP_BUDGET_DEFAULT,
            &mut send_fn,
            &mut ask,
        );
        assert_eq!(result.expect("the turn continues"), "fixed");

        let reasons: Vec<(String, String)> = asked
            .into_iter()
            .map(|a| match a {
                AgentAction::Malformed { tool, reason } => (tool, reason),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(reasons.len(), 3);
        assert_eq!(reasons[0].0, "create_line");
        assert!(
            reasons[0]
                .1
                .starts_with("tool `create_line` arguments are not valid JSON: "),
            "{:?}",
            reasons[0]
        );
        assert_eq!(
            reasons[1],
            ("draw_unicorn".into(), "unknown tool: `draw_unicorn`".into())
        );
        assert_eq!(
            reasons[2].1,
            "tool `create_line` missing required argument `x2`"
        );
        for (_, reason) in &reasons {
            assert!(!reason.contains(SENTINEL), "the raw args leaked: {reason}");
        }
    }

    /// AC 3 / AC 10 — a malformed call counts as a step: budget 2 and a batch
    /// of two malformed calls exhausts it, so a third call is refused whole.
    #[test]
    fn a_malformed_call_counts_as_a_step() {
        let mut sends = 0usize;
        let mut send_fn = |_: &[ChatMessage]| {
            sends += 1;
            Ok(match sends {
                1 => batch(&[("nope", "{}"), ("create_line", "{")]),
                _ => batch(&[("query_entities", "{}")]),
            })
        };
        let mut asks = 0usize;
        let mut ask = |_: AgentAction| {
            asks += 1;
            Ok(AgentOutcome::Refused("bad".into()))
        };
        let result = drive_turn("go", "sys", 2, &mut send_fn, &mut ask);
        assert!(
            matches!(result, Err(AgentError::IterationLimitExceeded(2))),
            "got {result:?}"
        );
        assert_eq!(asks, 2);
    }
}
