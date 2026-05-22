//! LCV-079 — Multi-turn agent loop.
//!
//! Drives one complete user–LLM interaction; dispatches tool calls through the
//! document history stack. MUST NOT import `egui`, `eframe`, or `rfd`.

use crate::document::{Document, History};

// ── Constants ────────────────────────────────────────────────────────────────

/// Maximum number of individual tool-call dispatches allowed within one
/// `run_agent_turn` invocation. Exported so callers can surface it in the UI.
pub const MAX_TOOL_CALLS_PER_TURN: usize = 10;

/// Hard-coded system prompt injected as the first message of every turn.
/// Must not be user-configurable in v0.1.0 (see demand Out of scope).
pub(crate) const AGENT_SYSTEM_PROMPT: &str =
    "You are a CAD assistant embedded in LaserCAD v2, a 2D laser-cutting CAD \
     tool. All coordinates and dimensions are in millimetres (mm). Angles at \
     the user interface are in degrees. Use the provided tools to create, \
     modify, or query the open drawing. Prefer the fewest tool calls that \
     satisfy the request. Confirm what you did in one or two concise sentences.";

// ── Conversation types ───────────────────────────────────────────────────────

/// Name and JSON-encoded arguments of one LLM-requested function call.
#[derive(Debug, Clone)]
#[rustfmt::skip] pub struct ToolCallFunction { pub name: String, pub arguments: String }

/// A single tool-call request produced by the LLM.
#[derive(Debug, Clone)]
#[rustfmt::skip] pub struct ToolCall { pub id: String, pub function: ToolCallFunction }

/// The assistant portion of one [`ChatResponse`] choice.
#[derive(Debug, Clone)]
#[rustfmt::skip] pub struct AssistantMessage { pub content: Option<String>, pub tool_calls: Option<Vec<ToolCall>> }

/// One completion choice returned by the API.
#[derive(Debug, Clone)]
#[rustfmt::skip] pub struct Choice { pub message: AssistantMessage }

/// Full response from one API call.
#[derive(Debug, Clone)]
#[rustfmt::skip] pub struct ChatResponse { pub choices: Vec<Choice> }

/// A single turn in the agent conversation history.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
}

#[rustfmt::skip]
impl ChatMessage {
    /// Create a `"system"` turn.
    pub fn system(c: &str) -> Self { Self { role: "system".into(), content: Some(c.to_owned()), tool_calls: None, tool_call_id: None } }
    /// Create a `"user"` turn.
    pub fn user(c: &str) -> Self { Self { role: "user".into(), content: Some(c.to_owned()), tool_calls: None, tool_call_id: None } }
    /// Create an `"assistant"` turn carrying tool-call requests.
    pub fn assistant_with_tool_calls(content: Option<String>, tool_calls: Vec<ToolCall>) -> Self { Self { role: "assistant".into(), content, tool_calls: Some(tool_calls), tool_call_id: None } }
    /// Create a `"tool"` result turn answering one tool-call request.
    pub fn tool_result(tool_call_id: String, content: String) -> Self { Self { role: "tool".into(), content: Some(content), tool_calls: None, tool_call_id: Some(tool_call_id) } }
}

// ── Error ────────────────────────────────────────────────────────────────────

/// Errors that [`run_agent_turn`] can return.
#[derive(Debug)]
pub enum AgentError {
    /// HTTP or serialisation failure from the transport layer (LCV-077).
    Transport(String),
    /// A tool call could not be dispatched (LCV-078 returned an error).
    ToolDispatch(String),
    /// The loop guard fired: more than [`MAX_TOOL_CALLS_PER_TURN`] individual
    /// tool calls were requested in a single turn.
    IterationLimitExceeded,
    /// The API returned a choice with neither `content` nor `tool_calls`.
    NoContent,
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(m) => write!(f, "transport error: {m}"),
            Self::ToolDispatch(m) => write!(f, "tool dispatch error: {m}"),
            Self::IterationLimitExceeded => write!(
                f,
                "iteration limit exceeded ({MAX_TOOL_CALLS_PER_TURN} tool calls per turn)"
            ),
            Self::NoContent => write!(f, "API response contained neither content nor tool calls"),
        }
    }
}

impl std::error::Error for AgentError {}

// ── Inner loop (testable seam) ───────────────────────────────────────────────

/// Inner loop with injectable `send_fn` (HTTP) and `dispatch_fn` (tool execution)
/// closures — unit tests substitute stubs without a live endpoint.
///
/// All document mutation flows through `dispatch_fn` → `History::commit`.
pub(crate) fn agent_loop<F, D>(
    send_fn: &mut F,
    dispatch_fn: &mut D,
    messages: &mut Vec<ChatMessage>,
    doc: &mut Document,
    history: &mut History,
) -> Result<String, AgentError>
where
    F: FnMut(&[ChatMessage]) -> Result<ChatResponse, AgentError>,
    D: FnMut(&str, &str, &mut Document, &mut History) -> Result<String, AgentError>,
{
    let mut dispatched: usize = 0;
    loop {
        let response = send_fn(messages)?;
        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or(AgentError::NoContent)?;
        match (choice.message.tool_calls, choice.message.content) {
            (Some(tcs), content) if !tcs.is_empty() => {
                // Guard fires BEFORE dispatching any call in this batch.
                if dispatched + tcs.len() > MAX_TOOL_CALLS_PER_TURN {
                    return Err(AgentError::IterationLimitExceeded);
                }
                messages.push(ChatMessage::assistant_with_tool_calls(content, tcs.clone()));
                for tc in &tcs {
                    let r = dispatch_fn(&tc.function.name, &tc.function.arguments, doc, history)?;
                    messages.push(ChatMessage::tool_result(tc.id.clone(), r));
                    dispatched += 1;
                }
            }
            (_, Some(text)) => return Ok(text),
            _ => return Err(AgentError::NoContent),
        }
    }
}

// ── Public entry point ───────────────────────────────────────────────────────

/// Drive one complete agent turn: send `prompt` to `endpoint`, dispatch tool
/// calls through `history` (Ctrl+Z undoable), return the final assistant text.
/// At most [`MAX_TOOL_CALLS_PER_TURN`] dispatches are allowed per turn.
pub fn run_agent_turn(
    prompt: &str,
    endpoint: &str,
    api_key: &str,
    doc: &mut Document,
    history: &mut History,
) -> Result<String, AgentError> {
    let mut messages = vec![
        ChatMessage::system(AGENT_SYSTEM_PROMPT),
        ChatMessage::user(prompt),
    ];
    // Called once; captured so every send_fn invocation has the schema available.
    // Forwarded into the request body once LCV-077 gains post_chat() tool support.
    let tool_schema = crate::agent::tools::tool_definitions();
    #[rustfmt::skip]
    let mut send_fn = |msgs: &[ChatMessage]| -> Result<ChatResponse, AgentError> {
        let _tool_defs = &tool_schema; // captured; forwarded to post_chat once LCV-077 is upgraded
        let tm: Vec<crate::agent::transport::Message> = msgs.iter()
            .filter_map(|m| Some(crate::agent::transport::Message { role: m.role.clone(), content: m.content.clone()? }))
            .collect();
        let reply = crate::agent::transport::chat_completion(endpoint, api_key, "gpt-4o", &tm)
            .map_err(|e| AgentError::Transport(e.to_string()))?;
        Ok(ChatResponse { choices: vec![Choice { message: AssistantMessage { content: Some(reply), tool_calls: None } }] })
    };
    #[rustfmt::skip]
    let mut dispatch_fn = |name: &str, args: &str, doc: &mut Document, hist: &mut History| {
        let v = serde_json::from_str::<serde_json::Value>(args).map_err(|e| AgentError::ToolDispatch(e.to_string()))?;
        crate::agent::tools::dispatch_tool_call(name, &v, doc, hist).map_err(|e| AgentError::ToolDispatch(e.to_string()))
    };
    agent_loop(&mut send_fn, &mut dispatch_fn, &mut messages, doc, history)
}

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*; use crate::document::{Document, History};

    fn tc(id: &str, nm: &str) -> ToolCall { ToolCall { id: id.to_owned(), function: ToolCallFunction { name: nm.to_owned(), arguments: "{}".to_owned() } } }
    fn text_resp(t: &str) -> Result<ChatResponse, AgentError> { Ok(ChatResponse { choices: vec![Choice { message: AssistantMessage { content: Some(t.to_owned()), tool_calls: None } }] }) }
    fn tool_resp(n: usize) -> Result<ChatResponse, AgentError> { let c = (0..n).map(|i| tc(&i.to_string(), "noop")).collect(); Ok(ChatResponse { choices: vec![Choice { message: AssistantMessage { content: None, tool_calls: Some(c) } }] }) }
    fn ctx() -> (Document, History) { (Document::default(), History::new()) }

    #[test] fn max_tool_calls_constant_is_10() { assert_eq!(MAX_TOOL_CALLS_PER_TURN, 10); }

    #[test]
    fn text_only_response_returns_ok() {
        let (mut doc, mut h) = ctx();
        let mut cnt = 0usize;
        let mut msgs = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let r = agent_loop(&mut |_| text_resp("Done."), &mut |_, _, _, _| { cnt += 1; Ok("ok".into()) }, &mut msgs, &mut doc, &mut h);
        assert_eq!(r.unwrap(), "Done."); assert_eq!(cnt, 0); assert_eq!(msgs.len(), 2);
    }

    #[test]
    fn single_tool_call_round_appends_messages() {
        let (mut doc, mut h) = ctx();
        let mut round = 0usize;
        let mut msgs = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let r = agent_loop(&mut |_| { round += 1; if round == 1 { tool_resp(1) } else { text_resp("Line created.") } }, &mut |_, _, _, _| Ok("ok".into()), &mut msgs, &mut doc, &mut h);
        assert_eq!(r.unwrap(), "Line created."); assert_eq!(msgs.len(), 4);
    }

    #[test]
    fn guard_fires_before_batch_of_11() {
        let (mut doc, mut h) = ctx();
        let mut cnt = 0usize;
        let mut msgs = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let r = agent_loop(&mut |_| tool_resp(11), &mut |_, _, _, _| { cnt += 1; Ok("ok".into()) }, &mut msgs, &mut doc, &mut h);
        assert!(matches!(r, Err(AgentError::IterationLimitExceeded))); assert_eq!(cnt, 0);
    }

    #[test]
    fn guard_fires_after_6_then_6() {
        let (mut doc, mut h) = ctx();
        let mut cnt = 0usize;
        let mut msgs = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let r = agent_loop(&mut |_| tool_resp(6), &mut |_, _, _, _| { cnt += 1; Ok("ok".into()) }, &mut msgs, &mut doc, &mut h);
        assert!(matches!(r, Err(AgentError::IterationLimitExceeded))); assert_eq!(cnt, 6);
    }

    #[test]
    fn exactly_10_calls_allowed() {
        let (mut doc, mut h) = ctx();
        let mut api = 0usize; let mut cnt = 0usize;
        let mut msgs = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let r = agent_loop(&mut |_| { api += 1; if api <= 2 { tool_resp(5) } else { text_resp("done") } }, &mut |_, _, _, _| { cnt += 1; Ok("ok".into()) }, &mut msgs, &mut doc, &mut h);
        assert_eq!(r.unwrap(), "done"); assert_eq!(cnt, 10);
    }

    #[test]
    fn no_content_returns_error() {
        let (mut doc, mut h) = ctx();
        let mut msgs = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let r = agent_loop(&mut |_| Ok(ChatResponse { choices: vec![Choice { message: AssistantMessage { content: None, tool_calls: None } }] }), &mut |_, _, _, _| Ok("ok".into()), &mut msgs, &mut doc, &mut h);
        assert!(matches!(r, Err(AgentError::NoContent)));
    }

    #[test]
    fn transport_error_propagated() {
        let (mut doc, mut h) = ctx();
        let mut msgs = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let r = agent_loop(&mut |_| Err(AgentError::Transport("timeout".into())), &mut |_, _, _, _| Ok("ok".into()), &mut msgs, &mut doc, &mut h);
        assert!(matches!(r, Err(AgentError::Transport(_))));
    }

    #[test]
    fn tool_dispatch_error_stops_batch() {
        let (mut doc, mut h) = ctx();
        let mut ok = 0usize; let mut n = 0usize;
        let mut msgs = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let r = agent_loop(&mut |_| tool_resp(3), &mut |_, _, _, _| { n += 1; if n == 2 { Err(AgentError::ToolDispatch("fail".into())) } else { ok += 1; Ok("ok".into()) } }, &mut msgs, &mut doc, &mut h);
        assert!(matches!(r, Err(AgentError::ToolDispatch(_)))); assert_eq!(ok, 1);
    }

    #[test]
    fn agent_error_display_is_non_empty() {
        for e in &[AgentError::Transport("x".into()), AgentError::ToolDispatch("y".into()), AgentError::IterationLimitExceeded, AgentError::NoContent] {
            assert!(!format!("{e}").is_empty(), "empty Display for {e:?}");
        }
    }
}
