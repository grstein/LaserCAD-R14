//! LCV-079 / LCV-121 — Multi-turn agent loop.
//!
//! Control flow only: send the conversation, honour the step budget, feed tool
//! results back. The wire shapes come from [`crate::agent::wire`] and the HTTP
//! call from [`crate::agent::transport`]; nothing is declared twice.
//!
//! Since LCV-122 the loop has no idea what a drawing is: `dispatch_fn` takes a
//! tool name and a JSON argument string and hands back the sentence the model
//! reads. Whoever supplies that closure — `crate::app::run_agent_turn` — is the
//! only party that knows where the geometry goes (ADR 0007 §D1).
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use crate::agent::wire::{AssistantMessage, ChatMessage};

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
/// `dispatch_fn` receives `(tool_name, raw_json_arguments)` and returns the
/// `tool`-role result text. It is the caller's business whether that text came
/// from a real mutation, a refusal or a stub; this loop only sequences it.
pub(crate) fn agent_loop<F, D>(
    send_fn: &mut F,
    dispatch_fn: &mut D,
    messages: &mut Vec<ChatMessage>,
    step_budget: u8,
) -> Result<String, AgentError>
where
    F: FnMut(&[ChatMessage]) -> Result<AssistantMessage, AgentError>,
    D: FnMut(&str, &str) -> Result<String, AgentError>,
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
                    let result = dispatch_fn(&call.function.name, &call.function.arguments)?;
                    messages.push(ChatMessage::tool_result(call.id.clone(), result));
                    dispatched += 1;
                }
            }
            (_, Some(text)) => return Ok(text),
            _ => return Err(AgentError::NoContent),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::wire::ToolCall;

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
        let mut dispatches = 0usize;
        let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let result = agent_loop(
            &mut |_| call_reply(2),
            &mut |_, _| {
                dispatches += 1;
                Ok("ok".into())
            },
            &mut messages,
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
        let mut dispatches = 0usize;
        let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let result = agent_loop(
            &mut |_| call_reply(6),
            &mut |_, _| {
                dispatches += 1;
                Ok("ok".into())
            },
            &mut messages,
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
            &mut |_, _| {
                dispatches += 1;
                Ok("ok".into())
            },
            &mut messages,
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
        let mut dispatches = 0usize;
        let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
        let result = agent_loop(
            &mut |_| text_reply("Done."),
            &mut |_, _| {
                dispatches += 1;
                Ok("ok".into())
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
            &mut |_, _| Ok("Line created: ….".into()),
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
        assert_eq!(messages[3].content.as_deref(), Some("Line created: …."));
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
            &mut |_, _| Ok("2 entities.".into()),
            &mut messages,
            AGENT_STEP_BUDGET_DEFAULT,
        );
        assert_eq!(result.unwrap(), "Two lines.");
        assert_eq!(messages[2].role, "assistant");
        assert_eq!(
            messages[2].content.as_deref(),
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
            &mut |_, _| Ok("ok".into()),
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
            &mut |_, _| Ok("ok".into()),
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
            &mut |_, _| {
                seen += 1;
                if seen == 2 {
                    Err(AgentError::ToolDispatch("fail".into()))
                } else {
                    applied += 1;
                    Ok("ok".into())
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
