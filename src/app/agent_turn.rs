//! LCV-122 — the turn's UI-side driver: [`TurnFence`] and [`run_agent_turn`].
//!
//! ## Why the fence is here and not in `src/agent/bridge.rs`
//!
//! ADR 0007 §D4 puts `TurnFence` in this file on purpose. `bridge.rs` is the
//! protocol, and the protocol is visible to the worker thread; the fence is the
//! one value in this design the worker thread must never evaluate, because
//! evaluating it on the wrong side of the channel is exactly the stale-read bug
//! it exists to prevent. §D6's coalesce gate is the same revision arithmetic,
//! so it lives in the same type.
//!
//! ## What the fence is for
//!
//! `History::revision()` is a monotonic counter that the UI thread is the only
//! writer of (ADR 0002 §B), so it is a complete sequencer of every mutation in
//! the program. A turn records the revision it started at; before each action
//! it checks that the revision is still the one it expects. If the operator
//! drew, deleted or selected something in the meantime, the model's picture of
//! the drawing is stale — its index 7 may now be a different entity, which
//! nothing else in the system can detect — so the action is refused and the
//! turn ends. Already-applied actions stay applied and stay undoable.
//!
//! A tripped fence is **sticky**: it does not re-arm, not even after a query
//! that would give the model a fresh read. That is what keeps the coalesce gate
//! sound — once tripped the agent commits nothing more, so the entries the gate
//! wants to fold stay contiguous at the top of the undo stack.
//!
//! LCV-123 owns the instance: it constructs the fence at turn start and calls
//! it from the frame loop. This demand ships the type, tested against plain
//! `u64`s.

use crate::agent::wire::ChatMessage;
use crate::agent::{agent_loop, AgentError, AGENT_SYSTEM_PROMPT};
use crate::app::agent_apply;
use crate::document::{Document, History};

/// What the model is told when a foreign commit landed mid-turn (ADR 0007 §D4).
///
/// It says three things on purpose: what happened, that nothing was applied,
/// and that undo is unaffected — so the operator reading the transcript knows
/// the drawing is not in a half-finished state.
pub const AGENT_FENCE_REFUSAL: &str = "The drawing changed outside this turn — someone drew, deleted or selected something since I last looked. Nothing was applied. Undo is unaffected; ask again and I will re-read the drawing.";

/// Guards one agent turn against mutations it did not make.
///
/// Three `u64`-sized pieces of state and no borrow of anything: `start` is the
/// revision the turn began at, `expected` is the revision the next action
/// requires, and `tripped` records that the guard has already fired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnFence {
    start: u64,
    expected: u64,
    tripped: bool,
}

impl TurnFence {
    /// Arm a fence at the revision the turn is starting from.
    pub fn new(start_revision: u64) -> Self {
        Self {
            start: start_revision,
            expected: start_revision,
            tripped: false,
        }
    }

    /// Is `current` still the revision this turn expects?
    ///
    /// # Errors
    ///
    /// [`AGENT_FENCE_REFUSAL`] when the revision moved without this turn's
    /// knowledge — and on **every** later call, whatever `current` is, because
    /// the trip is sticky.
    pub fn check(&mut self, current: u64) -> Result<(), String> {
        if self.tripped || current != self.expected {
            self.tripped = true;
            return Err(AGENT_FENCE_REFUSAL.to_string());
        }
        Ok(())
    }

    /// Move the expectation to `current` after a successful apply, where
    /// `current` is `expected + 1`.
    pub fn advance(&mut self, current: u64) {
        self.expected = current;
    }

    /// Has this fence refused an action? Once true, always true.
    pub fn is_tripped(&self) -> bool {
        self.tripped
    }

    /// May the turn's `n` commits be folded into one undo entry?
    ///
    /// ADR 0007 §D6 step 3: only when the revision advanced by exactly `n`
    /// since the turn started, which proves the top `n` entries of the undo
    /// stack are contiguously this turn's and nobody else's. `n < 2` is not
    /// worth folding — a composite of one only relabels it.
    pub fn may_coalesce(&self, current: u64, n: usize) -> bool {
        !self.tripped && n >= 2 && current.saturating_sub(self.start) == n as u64
    }
}

/// Drive one complete agent turn: send `prompt` to `endpoint` as `model`,
/// apply the tool calls it asks for, return the final assistant text.
///
/// `model` and `step_budget` are the caller's to choose — they come from
/// `Settings`, and the budget must already have been through
/// `crate::agent::clamp_step_budget`. At most `step_budget` actions are applied
/// per turn.
///
/// **Intermediate state, LCV-122 only.** The `doc` / `history` pair is still
/// supplied by the caller, and `src/agent/panel.rs` still supplies a throwaway
/// one, so the geometry goes nowhere the operator can see. LCV-123 replaces
/// both parameters with the fenced rendezvous of ADR 0007 §D1–§D4; until then
/// ADR 0007 §"The 121→123 window" is the reason no release may be cut.
///
/// # Errors
///
/// [`AgentError`] for a transport failure, a malformed tool call, an exhausted
/// step budget, or a reply carrying neither text nor tool calls. An action the
/// document refuses is **not** an error: the refusal goes back to the model as
/// the tool result and the turn continues.
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
    let tools = crate::agent::tool_definitions();
    let mut send_fn = |msgs: &[ChatMessage]| {
        // The slice goes through untouched: filtering it would drop the
        // assistant turns that carry tool calls and the tool turns that answer
        // them, leaving holes in the conversation the model reads back.
        crate::agent::chat_completion(endpoint, api_key, model, msgs, &tools)
            .map_err(|e| AgentError::Transport(e.to_string()))
    };
    let mut dispatch_fn = |name: &str, args: &str| {
        let value = serde_json::from_str::<serde_json::Value>(args)
            .map_err(|e| AgentError::ToolDispatch(e.to_string()))?;
        let action = crate::agent::parse_tool_call(name, &value)
            .map_err(|e| AgentError::ToolDispatch(e.to_string()))?;
        // A refusal is a tool result, not a failure (ADR 0007 §D2a).
        Ok(agent_apply::apply_to_document(&action, doc, history).into_text())
    };
    agent_loop(&mut send_fn, &mut dispatch_fn, &mut messages, step_budget)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AGENT_STEP_BUDGET_DEFAULT;
    use serde_json::{json, Value};
    use std::sync::{Arc, Mutex};

    fn ctx() -> (Document, History) {
        (Document::default(), History::new())
    }

    // ── AC 13: the fence ───────────────────────────────────────────────────

    /// AC 13 — the revision the turn started at is the revision it expects,
    /// and checking it repeatedly does not drift.
    #[test]
    fn a_matching_revision_passes() {
        let mut fence = TurnFence::new(10);
        assert_eq!(fence.check(10), Ok(()));
        assert_eq!(fence.check(10), Ok(()));
        assert!(!fence.is_tripped());
    }

    /// AC 13 — a revision that moved without the turn's knowledge is refused
    /// with the ADR 0007 §D4 wording, character for character.
    #[test]
    fn a_foreign_revision_is_refused_with_the_adr_wording() {
        let mut fence = TurnFence::new(10);
        assert_eq!(fence.check(11), Err(AGENT_FENCE_REFUSAL.to_string()));
        assert!(AGENT_FENCE_REFUSAL.contains("Nothing was applied"));
        assert!(AGENT_FENCE_REFUSAL.contains("Undo is unaffected"));
    }

    /// AC 13 — the trip is **sticky**: after one mismatch, the revision the
    /// fence used to want is refused too. A fence that recomputed per call
    /// would answer `Ok` on the second line here (mutation (d)).
    #[test]
    fn a_tripped_fence_refuses_even_a_matching_revision() {
        let mut fence = TurnFence::new(10);
        assert!(fence.check(11).is_err());
        assert_eq!(fence.check(10), Err(AGENT_FENCE_REFUSAL.to_string()));
        assert_eq!(fence.check(11), Err(AGENT_FENCE_REFUSAL.to_string()));
        assert!(fence.is_tripped());
    }

    /// AC 13 — `advance` moves the expectation, so the revision the last apply
    /// produced becomes the one the next action requires. Before the advance
    /// that same revision would have tripped the fence.
    #[test]
    fn advance_moves_the_expectation() {
        let mut fence = TurnFence::new(10);
        fence.advance(11);
        assert_eq!(fence.check(11), Ok(()));
        assert!(
            fence.check(10).is_err(),
            "a rewound revision is still foreign"
        );
    }

    /// AC 13 / ADR 0007 §D6 step 3 — the gate is an equality, not a
    /// comparison: exactly `n` revisions since the start, nothing else.
    #[test]
    fn may_coalesce_is_an_exact_equality_over_at_least_two_entries() {
        let fence = TurnFence::new(10);
        assert!(fence.may_coalesce(14, 4), "four commits, four revisions");
        assert!(!fence.may_coalesce(14, 3), "a foreign commit hid in there");
        assert!(!fence.may_coalesce(14, 5), "one of them was not ours");
        assert!(!fence.may_coalesce(11, 1), "one entry is not worth folding");
        assert!(!fence.may_coalesce(10, 0), "nothing happened");
        assert!(fence.may_coalesce(12, 2), "two is the smallest fold");
    }

    /// AC 13 — a tripped fence never folds, whatever the arithmetic says.
    /// This is what keeps the agent's entries contiguous (ADR 0007 §D4).
    #[test]
    fn a_tripped_fence_declines_to_coalesce() {
        let mut fence = TurnFence::new(10);
        assert!(fence.may_coalesce(14, 4));
        assert!(fence.check(99).is_err());
        assert!(!fence.may_coalesce(14, 4), "a tripped fence must not fold");
    }

    /// The `current - start` in `may_coalesce` is unsigned subtraction one line
    /// away from a panic. A revision below the start cannot happen in practice
    /// — `revision()` is monotonic — but the arithmetic must survive it.
    #[test]
    fn may_coalesce_survives_a_revision_below_the_start() {
        let fence = TurnFence::new(10);
        assert!(!fence.may_coalesce(0, 2));
        assert!(!fence.may_coalesce(9, 2));
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
            "Line created: (0.000, 0.000) → (20.000, 0.000) mm. \
             The drawing now has 1 entities.",
            "the model reads the real outcome, count and all (AC 9)"
        );
        assert_eq!(
            second["tools"].as_array().map(|t| t.len()),
            Some(5),
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

        let (mut doc, mut history) = ctx();
        let before = history.revision();
        let reply = run_agent_turn(
            "delete entity 7",
            &server.url(),
            "k",
            "test/model",
            AGENT_STEP_BUDGET_DEFAULT,
            &mut doc,
            &mut history,
        )
        .expect("a refused tool call must not fail the turn");

        assert_eq!(reply, "There is nothing there.");
        assert_eq!(bodies.len(), 2, "the turn went another round");
        assert_eq!(doc.entities.len(), 0, "nothing was applied");
        assert_eq!(history.revision(), before, "nothing was committed");

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

    /// AC 15 / ADR 0007 §D4 — the fence stays where the thread cannot reach
    /// it, and this file stays out of the UI toolkit.
    ///
    /// Bounded at the bare `#[cfg(test)]` at column 0, needles built with
    /// `concat!`. The positive control names the one declaration that must be
    /// here, so a mis-sliced haystack fails instead of passing vacuously.
    #[test]
    fn the_fence_is_declared_here_and_this_file_imports_no_ui() {
        let src = include_str!("agent_turn.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("agent_turn.rs must have a bare #[cfg(test)] marker");
        let implementation = &src[..at];

        assert!(
            implementation.contains(concat!("pub struct ", "TurnFence")),
            "positive control: the fence must be declared in this file"
        );
        for forbidden in [
            concat!("e", "frame"),
            concat!("r", "fd"),
            concat!("use e", "gui"),
        ] {
            let hit = implementation
                .lines()
                .find(|l| l.contains(forbidden) && !l.trim_start().starts_with("//"));
            assert!(
                hit.is_none(),
                "agent_turn.rs must not name `{forbidden}`: {hit:?}"
            );
        }
    }
}
