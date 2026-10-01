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

use crate::agent::loop_::{Dispatch, IMAGE_ELIDED};
use crate::agent::memory::whole_batches;
use crate::agent::wire::replace_images;
use crate::agent::{
    AgentAction, AgentError, AgentEvent, AgentOutcome, AssistantMessage, ChatMessage, RefusedCalls,
    ToolCallError, agent_loop,
};
use std::sync::mpsc::{Sender, channel};

/// Everything that crosses into the worker thread, as one owned value (ADR
/// 0007 §D13). Built once, in `start_turn`, from `Settings`: that is the
/// turn-start snapshot, so settings edited mid-turn affect the next turn only.
///
/// Carries the API key and the prompt, so its `Debug` is written by hand: it
/// prints `api_key: "<redacted>"` (ADR 0007 §D10) and the prompt's length
/// only (LCV-143 AC 7).
#[derive(Clone, PartialEq)]
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
    /// Both canvas opt-ins were on when the turn was armed (LCV-145): the
    /// turn advertises `capture_canvas`. Execution re-checks them live.
    pub vision: bool,
    /// The conversation so far (LCV-153, ADR 0007 §D16), flattened: sent
    /// between the system prompt and the new user message. Never printed.
    pub memory: Vec<ChatMessage>,
}

impl std::fmt::Debug for TurnConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TurnConfig")
            .field("endpoint", &self.endpoint)
            .field("api_key", &"<redacted>")
            .field("model", &self.model)
            .field("step_limit", &self.step_limit)
            .field("system_prompt_len", &self.system_prompt.len())
            .field("vision", &self.vision)
            .field("memory_len", &self.memory.len())
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
/// `config.memory` goes between the system prompt and `prompt`. Returns the
/// result together with the whole tool-call batches that followed `prompt`,
/// images elided, whether the turn succeeded or not (LCV-153).
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
) -> (Result<String, AgentError>, Vec<ChatMessage>)
where
    A: FnMut(AgentAction) -> Result<AgentOutcome, AgentError>,
{
    // Built once; every round offers the model the same schemas.
    let tools = crate::agent::tool_definitions(config.vision);
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
    drive_turn(prompt, config, &mut send_fn, ask)
}

/// [`run_agent_turn`] minus the network: the conversation, the parse and the
/// `ask`, with `send_fn` injected so the worker's own rules — the fence stop
/// (§D14) and malformed calls (§D15) — are testable without an endpoint.
/// `config.system_prompt` is the turn's system message, verbatim.
fn drive_turn<F, A>(
    prompt: &str,
    config: &TurnConfig,
    send_fn: &mut F,
    ask: &mut A,
) -> (Result<String, AgentError>, Vec<ChatMessage>)
where
    F: FnMut(&[ChatMessage]) -> Result<AssistantMessage, AgentError>,
    A: FnMut(AgentAction) -> Result<AgentOutcome, AgentError>,
{
    // `[system, memory…, user]` (ADR 0007 §D16): only appended to from here,
    // so each request is a prefix of the next, and of the next turn's.
    let mut messages = vec![ChatMessage::system(config.system_prompt.as_str())];
    messages.extend(config.memory.iter().cloned());
    messages.push(ChatMessage::user(prompt));
    let first_batch = messages.len();
    // A refusal is a tool result, not a failure (ADR 0007 §D2a), and so is a
    // malformed call (§D15); a `Fenced` answer is read by `agent_loop` (§D14).
    // An upload check names the turn's endpoint and model, never its key
    // (ADR 0011 item 10); a note, a model reply and the after-batch feedback
    // ask ride as non-step actions (LCV-187, LCV-193, LCV-195). A call repeating a refused one is answered from the
    // first refusal, still as a step (LCV-192 AC 4).
    let mut refused = RefusedCalls::default();
    let mut dispatch_fn = |dispatch: Dispatch<'_>| match dispatch {
        Dispatch::Tool { name, args } => {
            let action = match refused.check(name, args) {
                Some(reason) => AgentAction::Malformed {
                    tool: name.to_owned(),
                    reason,
                },
                None => to_action(name, args),
            };
            let outcome = ask(action)?;
            refused.record(name, args, &outcome);
            Ok(outcome)
        }
        Dispatch::AuthorizeUpload => ask(AgentAction::AuthorizeUpload {
            endpoint: config.endpoint.clone(),
            model: config.model.clone(),
        }),
        Dispatch::Note(text) => ask(AgentAction::Note(text.to_owned())),
        Dispatch::Replied { captures } => ask(AgentAction::Replied { captures }),
        Dispatch::Feedback => ask(AgentAction::Feedback),
    };
    let result = agent_loop(send_fn, &mut dispatch_fn, &mut messages, config.step_limit);
    // What memory keeps of this turn: whole batches, no image (§D3, ADR 0011).
    let mut batches = whole_batches(&messages[first_batch..]);
    replace_images(&mut batches, IMAGE_ELIDED);
    (result, batches)
}

/// The raw argument string of any tool call is refused above this many bytes,
/// before `serde_json` parses it (ADR 0010 §3.1).
const MAX_TOOL_ARGUMENT_BYTES: usize = 1_048_576;

/// Shape-check one tool call. Anything that fails — JSON syntax, unknown tool,
/// any `ToolCallError` — becomes [`AgentAction::Malformed`] and still goes to
/// the UI as an ordinary `Act` (ADR 0007 §D15). The reason never quotes `args`.
fn to_action(name: &str, args: &str) -> AgentAction {
    let malformed = |reason: String| AgentAction::Malformed {
        tool: name.to_owned(),
        reason,
    };
    // LCV-192 AC 3: the whole argument string is the path `(root)`.
    let root = |reason: String, expected: String| {
        malformed(
            ToolCallError::Arg {
                tool: name.to_owned(),
                path: "(root)".to_owned(),
                reason,
                expected,
            }
            .to_string(),
        )
    };
    if args.len() > MAX_TOOL_ARGUMENT_BYTES {
        return root(
            format!("arguments exceed {MAX_TOOL_ARGUMENT_BYTES} bytes"),
            format!("at most {MAX_TOOL_ARGUMENT_BYTES} bytes"),
        );
    }
    // The argument-free queries are routinely called with `""` rather than
    // `"{}"`, which is not JSON; both mean the same empty object here.
    let value = if args.trim().is_empty() {
        serde_json::Value::Null
    } else {
        match serde_json::from_str::<serde_json::Value>(args) {
            Ok(value) => value,
            Err(e) => {
                return root(format!("not valid JSON ({e})"), "a JSON object".to_owned());
            }
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
mod tests;
