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

/// Hard-coded system prompt injected as the first message of every turn.
/// Must not be user-configurable in v0.1.0 (see demand Out of scope).
///
/// The second half is the index contract of ADR 0007 §D5. Entity handles are
/// positional until stable ids land, so the two things that can silently
/// corrupt a drawing — the model's own deletes renumbering what it is about to
/// touch, and the operator drawing between the model's read and its write —
/// are disclosed in words rather than left to be discovered. The refusal
/// sentence is the fence's (`crate::app::AGENT_FENCE_REFUSAL`) seen from the
/// model's side: retrying cannot help, because the fence is sticky.
pub(crate) const AGENT_SYSTEM_PROMPT: &str =
    "You are a CAD assistant embedded in LaserCAD v2, a 2D laser-cutting CAD \
     tool. All coordinates and dimensions are in millimetres (mm). Angles at \
     the user interface are in degrees. Use the provided tools to create, \
     modify, or query the open drawing. Prefer the fewest tool calls that \
     satisfy the request. Confirm what you did in one or two concise \
     sentences. Entity handles are positional indices into the drawing: entity \
     0 is the first entity, and deleting an entity renumbers every higher \
     index down by one. The drawing may also have changed since you last read \
     it, so call query_entities before any delete_entity or move_entity whose \
     index you did not read during this turn. If an action is refused because \
     the drawing changed, stop and tell the operator what happened instead of \
     retrying.";

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
/// `dispatch_fn` receives `(tool_name, raw_json_arguments)` and returns the
/// `tool`-role result text. It is the caller's business whether that text came
/// from a real mutation, a refusal or a stub; this loop only sequences it.
pub(crate) fn agent_loop<F, D>(
    send_fn: &mut F,
    dispatch_fn: &mut D,
    messages: &mut Vec<ChatMessage>,
    step_budget: u32,
) -> Result<String, AgentError>
where
    F: FnMut(&[ChatMessage]) -> Result<AssistantMessage, AgentError>,
    D: FnMut(&str, &str) -> Result<String, AgentError>,
{
    let budget = usize::try_from(step_budget).unwrap_or(usize::MAX);
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

    // ── LCV-123 AC 17: the index contract is disclosed ───────────────────────

    /// AC 17 / ADR 0007 §D5 — the prompt tells the model, in words, that entity
    /// handles are **positional**, that a delete **renumbers** what comes after
    /// it, and that `query_entities` is how it re-reads the drawing.
    ///
    /// The haystack is the constant itself, not this file's source, so the
    /// scan cannot match its own needles however they are spelt — the failure
    /// mode ADR 0004 keeps catching. The control is the other direction: a term
    /// that must be **absent**, proving `contains` is really being evaluated
    /// against the prompt and not against something that says yes to anything.
    #[test]
    fn the_system_prompt_discloses_the_index_contract() {
        for needle in ["positional", "renumber", "query_entities"] {
            assert!(
                AGENT_SYSTEM_PROMPT.contains(needle),
                "the prompt must say `{needle}`: {AGENT_SYSTEM_PROMPT}"
            );
        }
        assert!(
            !AGENT_SYSTEM_PROMPT.contains("stable id"),
            "control: entity ids are explicitly not stable yet (ADR 0007 §D5), \
             so a prompt that promised them would be lying to the model"
        );
        // The pre-existing unit statement survives the rewrite.
        assert!(
            AGENT_SYSTEM_PROMPT.contains("millimetres (mm)"),
            "the unit statement must survive any prompt rewrite"
        );
        assert!(
            AGENT_SYSTEM_PROMPT.contains("degrees"),
            "angles are named at the UI in degrees, and the prompt says so"
        );
        // And the refusal advice is "stop", not "try again". One spelling, not
        // two: the constant is built from `\`-continuations and holds no
        // newline at all, so a `"instead of\nretrying"` alternative could never
        // match and would quietly carry none of this assertion.
        assert!(
            AGENT_SYSTEM_PROMPT.contains("instead of retrying"),
            "a sticky fence cannot be retried, so the prompt must not suggest it"
        );
        assert!(
            !AGENT_SYSTEM_PROMPT.contains('\n'),
            "the reason the single spelling above is enough"
        );
    }

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
            &mut |_, _| {
                dispatches += 1;
                Ok("ok".into())
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
