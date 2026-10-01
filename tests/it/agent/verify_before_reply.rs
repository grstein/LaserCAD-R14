//! LCV-197 — the agent is reminded once to verify a drawing it changed.
//!
//! Each test runs a whole turn on one thread: `run_agent_turn` sends real
//! requests to a mockito socket whose replies are scripted in order, and its
//! `ask` pushes every `Act` onto a turn armed with `arm_turn` and drains it
//! with `poll_agent_rx`, so the live `App` answers exactly as in a frame.
//!
//! ADR 0002 §A2: `App::default()` only. ADR 0005: no dialog is armed and no
//! test sends `Ctrl+O` / `Ctrl+S`. ADR 0006: no per-user path is injected.

use lasercad::agent::{AgentAction, AgentError, AgentEvent, AgentOutcome};
use lasercad::app::{App, arm_turn, config_for, poll_agent_rx, run_agent_turn};
use lasercad::document::CreateLine;
use lasercad::geometry::{Line, Vec2};
use serde_json::json;
use std::sync::mpsc::channel;

/// The reminder's opening words, as the request body carries them.
const REMINDER: &str = "Before you finish: verify the drawing against the request";

/// The note row a granted reminder leaves.
const NOTE: &str = "Asked the agent to verify its work.";

/// A completion body carrying `message`.
fn completion(message: serde_json::Value) -> String {
    json!({"choices": [{"message": message}]}).to_string()
}

/// A reply with one tool call per `(name, arguments)`.
fn calls(list: &[(&str, &str)]) -> String {
    let calls: Vec<_> = list
        .iter()
        .enumerate()
        .map(|(i, (name, args))| {
            json!({"id": format!("c{i}"), "type": "function",
                   "function": {"name": name, "arguments": args}})
        })
        .collect();
    completion(json!({"role": "assistant", "content": null, "tool_calls": calls}))
}

/// A text-only reply.
fn text(content: &str) -> String {
    completion(json!({"role": "assistant", "content": content}))
}

const LINE: &str = r#"{"x1":0,"y1":0,"x2":10,"y2":0}"#;

/// One scripted turn: its server, its app and what it ended with.
struct Turn {
    /// Kept alive for the turn; dropping it closes the socket.
    _server: mockito::ServerGuard,
    app: App,
    mocks: Vec<mockito::Mock>,
}

impl Turn {
    /// A server that answers the n-th request with `replies[n]`, each
    /// expected once; `reminded` marks the reply that must answer the
    /// request carrying the reminder.
    fn new(replies: &[&str], reminded: Option<usize>) -> Self {
        let mut server = mockito::Server::new();
        let mocks = replies
            .iter()
            .enumerate()
            .map(|(i, body)| {
                let mock = server
                    .mock("POST", "/chat/completions")
                    .with_status(200)
                    .with_body(*body)
                    .expect(1);
                match reminded {
                    Some(at) if at == i => {
                        mock.match_body(mockito::Matcher::Regex(REMINDER.to_owned()))
                    }
                    _ => mock,
                }
                .create()
            })
            .collect();
        let mut app = App::default();
        app.settings.agent_endpoint = server.url();
        app.settings.agent_model = "test/model".to_owned();
        app.settings.agent_api_key = "test-key".to_owned();
        Self {
            _server: server,
            app,
            mocks,
        }
    }

    /// Run the turn to its end; `before` runs on the app before each `Act`
    /// is answered. Returns the worker's result and the transcript rows
    /// after the user row.
    fn run(
        &mut self,
        mut before: impl FnMut(&mut App, &AgentAction),
    ) -> (Result<String, String>, Vec<(String, String)>) {
        let app = &mut self.app;
        let tx = arm_turn(app, "draw a 10 mm line");
        let user = app.agent.chat.len();
        let config = config_for(app);
        let prompt = app.agent.turn.user.clone();
        let (result, batches) = {
            let mut ask = |action: AgentAction| {
                before(app, &action);
                let (reply, answer) = channel::<AgentOutcome>();
                tx.send(AgentEvent::Act { action, reply })
                    .map_err(|_| AgentError::Cancelled)?;
                poll_agent_rx(app);
                answer.try_recv().map_err(|_| AgentError::Cancelled)
            };
            run_agent_turn(&prompt, &config, &mut ask)
        };
        let result = result.map_err(|e| e.to_string());
        let end = match &result {
            Ok(reply) => AgentEvent::Done(reply.clone(), batches),
            Err(error) => AgentEvent::Failed(error.clone(), batches),
        };
        tx.send(end).expect("the turn is still armed");
        poll_agent_rx(app);
        assert!(!app.agent.busy, "the turn ended");
        (result, app.agent.chat[user..].to_vec())
    }

    /// Every scripted reply was requested exactly once.
    fn assert_all_sent(&self) {
        for mock in &self.mocks {
            mock.assert();
        }
    }
}

fn notes(rows: &[(String, String)]) -> Vec<&str> {
    rows.iter()
        .filter(|(role, _)| role == "note")
        .map(|(_, text)| text.as_str())
        .collect()
}

/// AC 3 / AC 4 — `create_line` then a text reply: the reminder is sent once,
/// the note row appears, the steps are the tool calls alone, the extra reply
/// is counted, and the reply that ends the turn is the second answer.
#[test]
fn a_create_then_text_is_reminded_once_and_ends_with_the_second_answer() {
    let replies = [
        calls(&[("create_line", LINE)]),
        text("Drew a line."),
        text("Drew a line. Length 10 mm: pass."),
    ];
    let replies: Vec<&str> = replies.iter().map(String::as_str).collect();
    let mut turn = Turn::new(&replies, Some(2));
    let (result, rows) = turn.run(|_, _| {});
    turn.assert_all_sent();
    assert_eq!(result.as_deref(), Ok("Drew a line. Length 10 mm: pass."));
    let notes = notes(&rows);
    assert_eq!(notes.iter().filter(|n| **n == NOTE).count(), 1, "{rows:?}");
    assert_eq!(
        notes.last().copied(),
        Some(
            "Turn: 1 steps, 1 actions applied, 0 refused (0 repeated), \
             0 captures sent, 3 model replies."
        ),
        "{rows:?}"
    );
    let assistant: Vec<&str> = rows
        .iter()
        .filter(|(role, _)| role == "assistant")
        .map(|(_, text)| text.as_str())
        .collect();
    assert_eq!(assistant, ["Drew a line. Length 10 mm: pass."]);
}

/// Run `replies` as a turn whose app gets `setup` first; assert every reply
/// was requested once, the turn ended with `want` and wrote no reminder note.
fn unreminded(
    replies: &[String],
    setup: impl FnOnce(&mut App),
    before: impl FnMut(&mut App, &AgentAction),
    want: &str,
) {
    let replies: Vec<&str> = replies.iter().map(String::as_str).collect();
    let mut turn = Turn::new(&replies, None);
    setup(&mut turn.app);
    let (result, rows) = turn.run(before);
    turn.assert_all_sent();
    assert_eq!(result.as_deref(), Ok(want));
    assert!(!notes(&rows).contains(&NOTE), "{rows:?}");
}

/// AC 7 — a query-only turn is never reminded.
#[test]
fn a_query_only_turn_is_not_reminded() {
    let replies = [calls(&[("query_entities", "{}")]), text("Empty.")];
    unreminded(&replies, |_| {}, |_, _| {}, "Empty.");
}

/// AC 3 — a `create_line` followed by `check_drawing` already verified.
#[test]
fn a_create_then_check_drawing_is_not_reminded() {
    let replies = [
        calls(&[("create_line", LINE)]),
        calls(&[("check_drawing", "{}")]),
        text("Drew a line; check: pass."),
    ];
    unreminded(&replies, |_| {}, |_, _| {}, "Drew a line; check: pass.");
}

/// AC 6 — a turn the fence stopped after it applied a line ends with its
/// last word, unreminded.
#[test]
fn a_fenced_turn_is_not_reminded() {
    let replies = [
        calls(&[("create_line", LINE)]),
        calls(&[("create_line", LINE)]),
        text("Stopped after one line."),
    ];
    let mut creates = 0;
    let foreign = |app: &mut App, action: &AgentAction| {
        if matches!(action, AgentAction::CreateLine { .. }) {
            creates += 1;
            if creates == 2 {
                let line = Line::new(Vec2::new(0.0, 50.0), Vec2::new(5.0, 50.0));
                app.commit(Box::new(CreateLine::new(line)));
            }
        }
    };
    unreminded(&replies, |_| {}, foreign, "Stopped after one line.");
}

/// AC 6 — a step budget of 1 spent on the create leaves no step for the
/// reminder.
#[test]
fn a_spent_step_budget_is_not_reminded() {
    let replies = [calls(&[("create_line", LINE)]), text("Drew a line.")];
    let budget = |app: &mut App| app.settings.agent_step_budget = 1;
    unreminded(&replies, budget, |_, _| {}, "Drew a line.");
}
