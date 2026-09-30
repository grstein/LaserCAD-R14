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
    assert_eq!(
        messages[3].text_content(),
        Some("Line created: ….\nSteps left this turn: 255 of 256.")
    );
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
    let last = "Canvas 3\nSteps left this turn: 253 of 256.";
    for (i, text) in [(3, "ok"), (4, "Canvas 2"), (5, last)] {
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

// ── LCV-189: the step budget is visible ──────────────────────────────────

/// The text of every `tool` message, in order.
fn tool_texts(messages: &[ChatMessage]) -> Vec<&str> {
    messages
        .iter()
        .filter(|m| m.role == "tool")
        .map(|m| m.text_content().expect("tool results are text"))
        .collect()
}

/// LCV-189 AC 1 — each run batch ends its **last** tool result with the
/// steps left; the earlier results of the batch are untouched.
#[test]
fn the_last_tool_result_of_a_batch_carries_the_steps_left() {
    let r = run(
        |n| match n {
            1 => named_calls(&["query_entities"; 3]),
            2 => named_calls(&["query_entities"]),
            _ => text_reply("done"),
        },
        yes,
        10,
    );
    assert_eq!(r.result.unwrap(), "done");
    assert_eq!(
        tool_texts(&r.messages),
        [
            "ok",
            "ok",
            "ok\nSteps left this turn: 7 of 10.",
            "ok\nSteps left this turn: 6 of 10.",
        ]
    );
}

/// LCV-189 AC 1 — a batch of exactly the budget still runs whole and
/// reports zero steps left.
#[test]
fn an_exact_budget_batch_runs_and_reports_zero_left() {
    let r = run(
        |n| {
            if n == 1 {
                named_calls(&["query_entities"; 4])
            } else {
                text_reply("done")
            }
        },
        yes,
        4,
    );
    assert_eq!(r.result.unwrap(), "done");
    assert_eq!(r.tools, 4);
    assert_eq!(
        tool_texts(&r.messages)[3],
        "ok\nSteps left this turn: 0 of 4."
    );
}

/// LCV-189 AC 1 — when the last call observed the canvas, the line goes on
/// its tool result text; the image message keeps its own label.
#[test]
fn the_steps_left_line_goes_on_the_text_of_an_observed_result() {
    let r = run(
        |n| {
            if n == 1 {
                named_calls(&["query_entities", "capture_canvas"])
            } else {
                text_reply("seen")
            }
        },
        yes,
        5,
    );
    assert_eq!(r.result.unwrap(), "seen");
    assert_eq!(
        tool_texts(&r.messages),
        ["ok", "Canvas 2\nSteps left this turn: 3 of 5."]
    );
    let sent: Vec<ChatMessage> = serde_json::from_str(&r.requests[1]).unwrap();
    let image = ContentPart::png(PNG);
    assert_eq!(
        sent[5],
        ChatMessage::user_parts(vec![
            ContentPart::text("canvas image for tool call call_1"),
            image,
        ])
    );
}

/// LCV-189 AC 1 — a fence-stopped batch did not run to its end: its
/// results stay verbatim, the placeholder included, with no steps-left
/// line (the turn has no more steps to spend).
#[test]
fn a_fence_stopped_batch_gets_no_steps_left_line() {
    let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
    let mut sends = 0;
    let result = agent_loop(
        &mut |_| {
            sends += 1;
            if sends == 1 {
                named_calls(&["query_entities"; 3])
            } else {
                text_reply("stopped")
            }
        },
        &mut |_| Ok(AgentOutcome::Fenced("fenced".into())),
        &mut messages,
        AGENT_STEP_BUDGET_DEFAULT,
    );
    assert_eq!(result.unwrap(), "stopped");
    assert_eq!(
        tool_texts(&messages),
        ["fenced", FENCE_STOP_PLACEHOLDER, FENCE_STOP_PLACEHOLDER]
    );
}

// ── AC 2: the wire types are declared once, in wire.rs ───────────────────

/// AC 2 — no duplicate wire struct survives in this file, and the shared
/// declarations are imported instead. Bounded to the implementation
/// section and built with `concat!`, so the scan cannot match the needles
/// written here; pasting any of those structs back above the
/// `#[cfg(test)]` marker turns this red.
#[test]
fn loop_declares_no_wire_structs_of_its_own() {
    let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/agent/loop_.rs"));
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
