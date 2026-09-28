//! LCV-079 / LCV-121 — Multi-turn agent loop.
//!
//! Control flow only: send the conversation, honour the step budget, feed tool
//! results back. The wire shapes come from [`crate::agent::wire`] and the HTTP
//! call from [`crate::agent::transport`]; nothing is declared twice.
//!
//! Since LCV-122 the loop has no idea what a drawing is: `dispatch_fn` takes a
//! tool name and a JSON argument string and hands back the outcome whose
//! sentence the model reads. Whoever supplies that closure — `crate::app::run_agent_turn` — is the
//! only party that knows where the geometry goes (ADR 0007 §D1).
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use crate::agent::bridge::AgentOutcome;
use crate::agent::wire::{replace_images, AssistantMessage, ChatMessage, ContentPart};

// ── Step budget ──────────────────────────────────────────────────────────────

/// Tool-call dispatches (steps) allowed in one turn when the operator has not
/// chosen otherwise (ADR 0007 §D13, LCV-142).
pub const AGENT_STEP_BUDGET_DEFAULT: u32 = 256;

/// Smallest budget that still lets the agent do anything at all.
pub const AGENT_STEP_BUDGET_MIN: u32 = 1;

/// Largest budget. Not a structural limit — the flat history group (§D12)
/// holds any turn as one entry — but a runaway turn stays bounded in minutes
/// and in money.
pub const AGENT_STEP_BUDGET_MAX: u32 = 4096;

/// Hold a stored budget inside `AGENT_STEP_BUDGET_MIN..=AGENT_STEP_BUDGET_MAX`.
///
/// The value comes from a JSON file the operator can hand-edit, so every reader
/// goes through here rather than trusting the field: `0` becomes the minimum
/// and anything oversized becomes the maximum. Clamping deliberately lives with
/// the loop that enforces the budget, not in `io::settings` — that module must
/// not import `crate::agent` (ADR 0007 §D7).
pub fn clamp_step_budget(value: u32) -> u32 {
    value.clamp(AGENT_STEP_BUDGET_MIN, AGENT_STEP_BUDGET_MAX)
}

/// The tool result every call left in a batch gets once the fence has stopped
/// the turn (ADR 0007 §D14): not dispatched, but still answered, so every
/// `tool_call_id` the model sent stays paired.
pub(crate) const FENCE_STOP_PLACEHOLDER: &str =
    "not run: the turn stopped after the drawing changed outside it";

/// What an image part becomes once it has ridden its one request (ADR 0011
/// item 9), whether that send succeeded or failed.
pub(crate) const IMAGE_ELIDED: &str =
    "canvas image elided after one use; call capture_canvas to look again";

/// What an image part becomes when the UI did not authorise its upload (ADR
/// 0011 item 10): the request still goes out, text-only.
pub(crate) const IMAGE_WITHHELD: &str =
    "canvas image withheld: capture permission changed during the turn";

/// One request from the loop to whoever owns the drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dispatch<'a> {
    /// Run one tool call — a step.
    Tool {
        /// The tool name as the model sent it.
        name: &'a str,
        /// The raw JSON argument string.
        args: &'a str,
    },
    /// May the next request carry its canvas images? Not a step (ADR 0011
    /// item 10); `AgentOutcome::Ok` is yes, anything else is no.
    AuthorizeUpload,
}

// ── Error ────────────────────────────────────────────────────────────────────

/// Errors that [`crate::app::run_agent_turn`] can return.
#[derive(Debug)]
pub enum AgentError {
    /// HTTP or serialisation failure from the transport layer.
    Transport(String),
    /// A tool call could not be dispatched.
    ToolDispatch(String),
    /// The loop guard fired. Carries the budget that was in force, so the
    /// message names the real number rather than a constant.
    IterationLimitExceeded(u32),
    /// The endpoint answered with neither content nor tool calls.
    NoContent,
    /// The fence stopped the turn and the model's one last reply asked for
    /// more tool calls anyway (ADR 0007 §D14). None of them was dispatched.
    FenceStopped,
    /// The UI thread stopped answering: the app is closing, the turn was
    /// abandoned, or the frame loop is gone (ADR 0007 §D2). The worker returns
    /// on this **without** sending a terminal event — there is nobody left to
    /// read one, and inventing a row for a turn the app already forgot about
    /// would be the only thing that could show up in a *later* turn's
    /// transcript.
    Cancelled,
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
            Self::Cancelled => write!(f, "the turn was cancelled"),
            Self::FenceStopped => {
                write!(f, "the turn stopped after the drawing changed outside it")
            }
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
/// comparison is done in `usize` — narrower arithmetic could wrap on a large
/// batch and wave it through.
///
/// `dispatch_fn` receives [`Dispatch::Tool`] and returns the
/// outcome whose text becomes the `tool`-role result. It is the caller's
/// business whether that came from a real mutation, a refusal or a stub; this
/// loop only sequences it — with one exception it reads rather than decides.
/// On [`AgentOutcome::Fenced`] (ADR 0007 §D14) nothing more is dispatched: the
/// rest of the batch gets [`FENCE_STOP_PLACEHOLDER`], exactly one more
/// completion is sent, and its text ends the turn `Ok` while tool calls end it
/// [`AgentError::FenceStopped`].
///
/// An [`AgentOutcome::Observed`] (LCV-145) is a step like any other; its PNG
/// rides after **all** of the batch's tool results in one `user` message,
/// and every send goes through [`send_images`].
pub(crate) fn agent_loop<F, D>(
    send_fn: &mut F,
    dispatch_fn: &mut D,
    messages: &mut Vec<ChatMessage>,
    step_budget: u32,
) -> Result<String, AgentError>
where
    F: FnMut(&[ChatMessage]) -> Result<AssistantMessage, AgentError>,
    D: FnMut(Dispatch<'_>) -> Result<AgentOutcome, AgentError>,
{
    let budget = usize::try_from(step_budget).unwrap_or(usize::MAX);
    let mut dispatched: usize = 0;
    loop {
        let message = send_images(send_fn, dispatch_fn, messages)?;
        match (message.tool_calls, message.content) {
            (Some(calls), content) if !calls.is_empty() => {
                if dispatched + calls.len() > budget {
                    return Err(AgentError::IterationLimitExceeded(step_budget));
                }
                messages.push(ChatMessage::assistant_with_tool_calls(
                    content,
                    calls.clone(),
                ));
                let (mut fenced, mut images) = (false, Vec::new());
                for call in &calls {
                    let result = if fenced {
                        FENCE_STOP_PLACEHOLDER.to_owned()
                    } else {
                        let (name, args) = (&call.function.name, &call.function.arguments);
                        let outcome = dispatch_fn(Dispatch::Tool { name, args })?;
                        dispatched += 1;
                        fenced = outcome.is_fenced();
                        match outcome {
                            AgentOutcome::Observed { text, png } => {
                                let label = format!("canvas image for tool call {}", call.id);
                                images.extend([ContentPart::text(label), ContentPart::png(&png)]);
                                text
                            }
                            other => other.into_text(),
                        }
                    };
                    messages.push(ChatMessage::tool_result(call.id.clone(), result));
                }
                if !images.is_empty() {
                    messages.push(ChatMessage::user_parts(images));
                }
                if fenced {
                    return last_word(send_images(send_fn, dispatch_fn, messages)?);
                }
            }
            (_, Some(text)) => return Ok(text),
            _ => return Err(AgentError::NoContent),
        }
    }
}

/// Every send of the loop (ADR 0011 items 9–10). A request carrying an image
/// first asks [`Dispatch::AuthorizeUpload`], once: no — or anything but
/// `Ok` — withholds every image and sends text-only; a failed ask (cancel)
/// returns before anything is sent. After the send returns, success or error,
/// every image is elided, so none outlives its one request.
fn send_images<F, D>(
    send_fn: &mut F,
    dispatch_fn: &mut D,
    messages: &mut [ChatMessage],
) -> Result<AssistantMessage, AgentError>
where
    F: FnMut(&[ChatMessage]) -> Result<AssistantMessage, AgentError>,
    D: FnMut(Dispatch<'_>) -> Result<AgentOutcome, AgentError>,
{
    if messages.iter().any(ChatMessage::has_image) {
        let verdict = dispatch_fn(Dispatch::AuthorizeUpload)?;
        if !matches!(verdict, AgentOutcome::Ok(_)) {
            replace_images(messages, IMAGE_WITHHELD);
        }
    }
    let reply = send_fn(messages);
    replace_images(messages, IMAGE_ELIDED);
    reply
}

/// The one completion a fence-stopped turn is allowed (ADR 0007 §D14): text is
/// the model's report of what it got done; anything else is not dispatched.
fn last_word(message: AssistantMessage) -> Result<String, AgentError> {
    match (message.tool_calls, message.content) {
        (Some(calls), _) if !calls.is_empty() => Err(AgentError::FenceStopped),
        (_, Some(text)) => Ok(text),
        _ => Err(AgentError::NoContent),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::wire::ToolCall;

    /// The `Cancelled` variant exists, is distinct, and reads as an ended turn
    /// rather than as a failure the operator has to act on (AC 5).
    #[test]
    fn cancelled_is_its_own_error_with_its_own_wording() {
        let cancelled = AgentError::Cancelled;
        assert_ne!(
            std::mem::discriminant(&cancelled),
            std::mem::discriminant(&AgentError::NoContent)
        );
        assert_eq!(cancelled.to_string(), "the turn was cancelled");
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

    /// LCV-142 AC 2 — the three constants are the documented numbers.
    #[test]
    fn step_budget_constants_are_256_1_and_4096() {
        assert_eq!(AGENT_STEP_BUDGET_DEFAULT, 256);
        assert_eq!(AGENT_STEP_BUDGET_MIN, 1);
        assert_eq!(AGENT_STEP_BUDGET_MAX, 4096);
    }

    /// LCV-142 AC 2 — `clamp_step_budget` holds the range at both ends and
    /// leaves everything inside it alone, including the old default and the
    /// old maximum.
    #[test]
    fn clamp_step_budget_holds_the_range() {
        for (stored, expected) in [
            (0u32, 1u32),
            (1, 1),
            (12, 12),
            (32, 32),
            (33, 33),
            (256, 256),
            (4096, 4096),
            (4097, 4096),
            (u32::MAX, 4096),
        ] {
            assert_eq!(
                clamp_step_budget(stored),
                expected,
                "clamp_step_budget({stored}) must be {expected}"
            );
        }
    }

    // ── AC 11: the budget is a parameter, and the guard fires early ──────────

    /// Drive the loop with `send` and a counting dispatch; returns the result,
    /// the number of dispatches and the number of `send` calls.
    fn drive(
        mut send: impl FnMut(usize) -> Result<AssistantMessage, AgentError>,
        budget: u32,
    ) -> (Result<String, AgentError>, usize, usize) {
        let (mut sends, mut dispatches) = (0usize, 0usize);
        let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let result = agent_loop(
            &mut |_| {
                sends += 1;
                send(sends)
            },
            &mut |_| {
                dispatches += 1;
                Ok(AgentOutcome::Ok("ok".into()))
            },
            &mut messages,
            budget,
        );
        (result, dispatches, sends)
    }

    /// AC 11 — a batch that would cross the budget is refused **before** any of
    /// its calls is dispatched: budget 1, two calls in one response, zero
    /// dispatches.
    #[test]
    fn budget_of_one_refuses_a_two_call_batch_before_dispatching() {
        let (result, dispatches, _) = drive(|_| call_reply(2), 1);
        assert!(
            matches!(result, Err(AgentError::IterationLimitExceeded(1))),
            "got {result:?}"
        );
        assert_eq!(dispatches, 0, "not one call of the batch may be applied");
    }

    /// LCV-142 AC 3 — budget 5, one batch of 6: rejected whole, zero dispatches.
    #[test]
    fn budget_of_five_refuses_a_six_call_batch_whole() {
        let (result, dispatches, sends) = drive(|_| call_reply(6), 5);
        assert!(
            matches!(result, Err(AgentError::IterationLimitExceeded(5))),
            "got {result:?}"
        );
        assert_eq!((dispatches, sends), (0, 1));
    }

    /// LCV-142 AC 3 — the guard counts across rounds at the new scale: budget
    /// 300 takes three batches of 100, and a fourth batch of one is refused
    /// with nothing of it dispatched.
    #[test]
    fn budget_of_300_takes_three_batches_of_100_and_refuses_a_fourth() {
        let (result, dispatches, sends) = drive(
            |n| {
                if n <= 3 {
                    call_reply(100)
                } else {
                    call_reply(1)
                }
            },
            300,
        );
        assert!(
            matches!(result, Err(AgentError::IterationLimitExceeded(300))),
            "got {result:?}"
        );
        assert_eq!(dispatches, 300, "the three batches of 100 all dispatched");
        assert_eq!(sends, 4);
    }

    /// LCV-142 AC 4 — after a batch that lands exactly on the limit, exactly
    /// one more completion is sent: text ends the turn `Ok`, tool calls end it
    /// `IterationLimitExceeded(limit)` with nothing more dispatched.
    #[test]
    fn exact_exhaustion_allows_exactly_one_more_completion() {
        let (result, dispatches, sends) = drive(
            |n| {
                if n <= 2 {
                    call_reply(3)
                } else {
                    text_reply("done")
                }
            },
            6,
        );
        assert_eq!(result.unwrap(), "done");
        assert_eq!((dispatches, sends), (6, 3));

        let (result, dispatches, sends) = drive(|n| call_reply(if n <= 2 { 3 } else { 1 }), 6);
        assert!(
            matches!(result, Err(AgentError::IterationLimitExceeded(6))),
            "got {result:?}"
        );
        assert_eq!(dispatches, 6, "the dispatch count did not move");
        assert_eq!(sends, 3, "exactly one completion after exhaustion");
        assert_eq!(
            AgentError::IterationLimitExceeded(6).to_string(),
            "step budget exceeded (6 tool calls per turn)"
        );
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
        let mut dispatches = 0usize;
        let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let result = agent_loop(
            &mut |_| text_reply("Done."),
            &mut |_| {
                dispatches += 1;
                Ok(AgentOutcome::Ok("ok".into()))
            },
            &mut messages,
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
            &mut |_| Ok(AgentOutcome::Ok("Line created: ….".into())),
            &mut messages,
            AGENT_STEP_BUDGET_DEFAULT,
        );
        assert_eq!(result.unwrap(), "Line created.");
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[2].role, "assistant");
        let calls = messages[2].tool_calls.as_ref().expect("tool calls kept");
        assert_eq!(calls[0].id, "call_0");
        assert_eq!(messages[3].role, "tool");
        assert_eq!(messages[3].tool_call_id.as_deref(), Some("call_0"));
        assert_eq!(messages[3].text_content(), Some("Line created: …."));
    }

    /// LCV-121 carry-over, closed by LCV-122 — a model that narrates *and*
    /// calls a tool in the same turn ("Let me check…" followed by
    /// `query_entities`) must have its prose preserved on the assistant turn
    /// that goes back over the wire.
    ///
    /// `ChatMessage::assistant_with_tool_calls` has taken an `Option<String>`
    /// since LCV-121, but until now every call site and every test passed
    /// `None`, so an implementation that hardcoded `None` would have looked
    /// perfectly green. Dropping the prose makes the model's own reasoning
    /// vanish from its context between rounds.
    #[test]
    fn prose_alongside_a_tool_call_survives_the_round_trip() {
        let mut rounds = 0usize;
        let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let result = agent_loop(
            &mut |_| {
                rounds += 1;
                if rounds == 1 {
                    Ok(AssistantMessage {
                        content: Some("Let me look at the drawing first.".into()),
                        tool_calls: Some(vec![ToolCall::function("call_0", "noop", "{}")]),
                    })
                } else {
                    text_reply("Two lines.")
                }
            },
            &mut |_| Ok(AgentOutcome::Ok("2 entities.".into())),
            &mut messages,
            AGENT_STEP_BUDGET_DEFAULT,
        );
        assert_eq!(result.unwrap(), "Two lines.");
        assert_eq!(messages[2].role, "assistant");
        assert_eq!(
            messages[2].text_content(),
            Some("Let me look at the drawing first."),
            "the assistant's own words must go back with its tool calls"
        );
        assert!(messages[2].tool_calls.is_some(), "and so must the calls");
    }

    /// A reply with neither text nor tool calls ends the turn as an error.
    #[test]
    fn no_content_returns_error() {
        let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let result = agent_loop(
            &mut |_| {
                Ok(AssistantMessage {
                    content: None,
                    tool_calls: None,
                })
            },
            &mut |_| Ok(AgentOutcome::Ok("ok".into())),
            &mut messages,
            AGENT_STEP_BUDGET_DEFAULT,
        );
        assert!(matches!(result, Err(AgentError::NoContent)));
    }

    /// A transport failure is surfaced, not swallowed.
    #[test]
    fn transport_error_propagated() {
        let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let result = agent_loop(
            &mut |_| Err(AgentError::Transport("timeout".into())),
            &mut |_| Ok(AgentOutcome::Ok("ok".into())),
            &mut messages,
            AGENT_STEP_BUDGET_DEFAULT,
        );
        assert!(matches!(result, Err(AgentError::Transport(_))));
    }

    /// A failing dispatch stops the rest of its batch.
    #[test]
    fn tool_dispatch_error_stops_batch() {
        let (mut applied, mut seen) = (0usize, 0usize);
        let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let result = agent_loop(
            &mut |_| call_reply(3),
            &mut |_| {
                seen += 1;
                if seen == 2 {
                    Err(AgentError::ToolDispatch("fail".into()))
                } else {
                    applied += 1;
                    Ok(AgentOutcome::Ok("ok".into()))
                }
            },
            &mut messages,
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

    // ── LCV-145: canvas images ───────────────────────────────────────────────

    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 1, 2, 3];

    fn named_calls(names: &[&str]) -> Result<AssistantMessage, AgentError> {
        let calls = names
            .iter()
            .enumerate()
            .map(|(i, name)| ToolCall::function(format!("call_{i}"), *name, "{}"))
            .collect();
        Ok(AssistantMessage {
            content: None,
            tool_calls: Some(calls),
        })
    }

    /// What one scripted turn did: the serialised requests, the authorise
    /// count, the tool dispatch count and the result.
    struct Run {
        requests: Vec<String>,
        authorisations: usize,
        tools: usize,
        messages: Vec<ChatMessage>,
        result: Result<String, AgentError>,
    }

    /// Drive the loop: `reply(n)` answers the n-th send (1-based), a
    /// `capture_canvas` dispatch observes [`PNG`], anything else is `Ok`, and
    /// the upload check answers `verdict`.
    fn run(
        mut reply: impl FnMut(usize) -> Result<AssistantMessage, AgentError>,
        mut verdict: impl FnMut() -> Result<AgentOutcome, AgentError>,
        budget: u32,
    ) -> Run {
        let (mut requests, mut authorisations, mut tools) = (Vec::new(), 0, 0);
        let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let result = agent_loop(
            &mut |msgs| {
                requests.push(serde_json::to_string(msgs).unwrap());
                reply(requests.len())
            },
            &mut |dispatch| match dispatch {
                Dispatch::AuthorizeUpload => {
                    authorisations += 1;
                    verdict()
                }
                Dispatch::Tool { name, .. } => {
                    tools += 1;
                    Ok(if name == "capture_canvas" {
                        AgentOutcome::Observed {
                            text: format!("Canvas {tools}"),
                            png: PNG.to_vec(),
                        }
                    } else {
                        AgentOutcome::Ok("ok".into())
                    })
                }
            },
            &mut messages,
            budget,
        );
        Run {
            requests,
            authorisations,
            tools,
            messages,
            result,
        }
    }

    fn yes() -> Result<AgentOutcome, AgentError> {
        Ok(AgentOutcome::Ok("yes".into()))
    }

    /// AC 3 — a turn with no `capture_canvas` call sends no `image_url` part
    /// and never asks to authorise an upload.
    #[test]
    fn no_capture_sends_no_image_and_asks_nothing() {
        let r = run(
            |n| {
                if n == 1 {
                    named_calls(&["query_entities"])
                } else {
                    text_reply("done")
                }
            },
            yes,
            AGENT_STEP_BUDGET_DEFAULT,
        );
        assert_eq!(r.result.unwrap(), "done");
        assert_eq!(r.requests.len(), 2);
        assert!(r.requests.iter().all(|req| !req.contains("image_url")));
        assert_eq!(r.authorisations, 0);
    }

    /// AC 3 — each capture is one step: two captures fit a budget of two, and
    /// a batch of three captures is refused whole by it.
    #[test]
    fn each_capture_counts_as_one_step() {
        let r = run(
            |n| {
                if n == 1 {
                    named_calls(&["capture_canvas", "capture_canvas"])
                } else {
                    text_reply("ok")
                }
            },
            yes,
            2,
        );
        assert_eq!(r.result.unwrap(), "ok");
        assert_eq!(r.tools, 2);
        let r = run(|_| named_calls(&["capture_canvas"; 3]), yes, 2);
        assert!(matches!(
            r.result,
            Err(AgentError::IterationLimitExceeded(2))
        ));
        assert_eq!((r.tools, r.authorisations), (0, 0));
    }

    /// AC 9 — [query, capture, capture]: three `tool` results in call order,
    /// then one `user` message with two text+image pairs naming the ids; the
    /// tool results are text only.
    #[test]
    fn a_batch_appends_one_user_message_after_all_tool_results() {
        let r = run(
            |n| {
                if n == 1 {
                    named_calls(&["query_entities", "capture_canvas", "capture_canvas"])
                } else {
                    text_reply("seen")
                }
            },
            yes,
            AGENT_STEP_BUDGET_DEFAULT,
        );
        assert_eq!(r.result.unwrap(), "seen");
        let sent: Vec<ChatMessage> = serde_json::from_str(&r.requests[1]).unwrap();
        assert_eq!(sent.len(), 7);
        let roles: Vec<&str> = sent.iter().map(|m| m.role.as_str()).collect();
        assert_eq!(
            roles,
            [
                "system",
                "user",
                "assistant",
                "tool",
                "tool",
                "tool",
                "user"
            ]
        );
        for (i, text) in [(3, "ok"), (4, "Canvas 2"), (5, "Canvas 3")] {
            assert_eq!(
                sent[i].tool_call_id.as_deref(),
                Some(format!("call_{}", i - 3).as_str())
            );
            assert_eq!(sent[i].text_content(), Some(text));
        }
        let image = ContentPart::png(PNG);
        assert_eq!(
            sent[6],
            ChatMessage::user_parts(vec![
                ContentPart::text("canvas image for tool call call_1"),
                image.clone(),
                ContentPart::text("canvas image for tool call call_2"),
                image,
            ])
        );
        assert_eq!(r.authorisations, 1);
    }

    /// AC 10 — the image rides exactly one request: the next one carries the
    /// elided placeholder and no `image_url`.
    #[test]
    fn the_request_after_an_image_carries_the_elided_placeholder() {
        let r = run(
            |n| match n {
                1 => named_calls(&["capture_canvas"]),
                2 => named_calls(&["query_entities"]),
                _ => text_reply("done"),
            },
            yes,
            AGENT_STEP_BUDGET_DEFAULT,
        );
        assert_eq!(r.result.unwrap(), "done");
        assert!(r.requests[1].contains("image_url"));
        assert!(!r.requests[2].contains("image_url"), "{}", r.requests[2]);
        assert!(r.requests[2].contains(IMAGE_ELIDED));
        assert_eq!(r.authorisations, 1, "asked once, for the one image send");
    }

    /// AC 10 — a failed send elides too: nothing image-bearing survives it.
    #[test]
    fn a_send_error_still_elides_the_image() {
        let r = run(
            |n| {
                if n == 1 {
                    named_calls(&["capture_canvas"])
                } else {
                    Err(AgentError::Transport("down".into()))
                }
            },
            yes,
            AGENT_STEP_BUDGET_DEFAULT,
        );
        assert!(matches!(r.result, Err(AgentError::Transport(_))));
        assert!(
            r.requests[1].contains("image_url"),
            "the image was sent once"
        );
        assert!(!r.messages.iter().any(ChatMessage::has_image));
        let json = serde_json::to_string(&r.messages).unwrap();
        assert!(json.contains(IMAGE_ELIDED));
    }

    /// AC 11 — a "no" answer withholds the image; the request still goes out,
    /// text-only, carrying the withheld placeholder.
    #[test]
    fn a_refused_upload_sends_the_withheld_placeholder_text_only() {
        for verdict in [
            AgentOutcome::Refused("no".into()),
            AgentOutcome::Fenced("f".into()),
        ] {
            let r = run(
                |n| {
                    if n == 1 {
                        named_calls(&["capture_canvas"])
                    } else {
                        text_reply("blind")
                    }
                },
                || Ok(verdict.clone()),
                AGENT_STEP_BUDGET_DEFAULT,
            );
            assert_eq!(r.result.unwrap(), "blind");
            assert_eq!(r.authorisations, 1);
            assert!(!r.requests[1].contains("image_url"));
            assert!(r.requests[1].contains(IMAGE_WITHHELD));
            assert!(!r.requests[1].contains(IMAGE_ELIDED));
        }
    }

    /// AC 11 — a cancelled rendezvous sends nothing: `send_fn` is not called
    /// again after the image-bearing batch.
    #[test]
    fn a_cancelled_upload_check_sends_nothing() {
        let r = run(
            |n| {
                if n == 1 {
                    named_calls(&["capture_canvas"])
                } else {
                    text_reply("never")
                }
            },
            || Err(AgentError::Cancelled),
            AGENT_STEP_BUDGET_DEFAULT,
        );
        assert!(matches!(r.result, Err(AgentError::Cancelled)));
        assert_eq!(r.requests.len(), 1);
    }

    /// AC 11 — the fence's one last completion goes through the same check:
    /// a capture before the fence tripped still needs authorising.
    #[test]
    fn the_fence_stop_send_is_authorised_and_elided_too() {
        let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let (mut requests, mut authorisations, mut tools) = (Vec::<String>::new(), 0, 0);
        let result = agent_loop(
            &mut |msgs| {
                requests.push(serde_json::to_string(msgs).unwrap());
                if requests.len() == 1 {
                    named_calls(&["capture_canvas", "query_entities"])
                } else {
                    text_reply("stopped")
                }
            },
            &mut |dispatch| match dispatch {
                Dispatch::AuthorizeUpload => {
                    authorisations += 1;
                    Ok(AgentOutcome::Refused("no".into()))
                }
                Dispatch::Tool { .. } => {
                    tools += 1;
                    Ok(if tools == 1 {
                        AgentOutcome::Observed {
                            text: "Canvas".into(),
                            png: PNG.to_vec(),
                        }
                    } else {
                        AgentOutcome::Fenced("fenced".into())
                    })
                }
            },
            &mut messages,
            AGENT_STEP_BUDGET_DEFAULT,
        );
        assert_eq!(result.unwrap(), "stopped");
        assert_eq!(authorisations, 1);
        assert!(!requests[1].contains("image_url"));
        assert!(requests[1].contains(IMAGE_WITHHELD));
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
