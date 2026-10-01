use super::*;
use crate::agent::wire::{ContentPart, ToolCall};

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
        reasoning_content: None,
    })
}

fn call_reply(n: usize) -> Result<AssistantMessage, AgentError> {
    let calls = (0..n)
        .map(|i| ToolCall::function(format!("call_{i}"), "noop", "{}"))
        .collect();
    Ok(AssistantMessage {
        content: None,
        tool_calls: Some(calls),
        reasoning_content: None,
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
        &mut |dispatch| {
            dispatches += usize::from(matches!(dispatch, Dispatch::Tool { .. }));
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

/// LCV-142 AC 3 — budget 5, one batch of 6: rejected whole, zero
/// dispatches; the repeat after the grace reply (LCV-189) ends the turn.
#[test]
fn budget_of_five_refuses_a_six_call_batch_whole() {
    let (result, dispatches, sends) = drive(|_| call_reply(6), 5);
    assert!(
        matches!(result, Err(AgentError::IterationLimitExceeded(5))),
        "got {result:?}"
    );
    assert_eq!((dispatches, sends), (0, 2));
}

/// LCV-142 AC 3 — the guard counts across rounds at the new scale: budget
/// 300 takes three batches of 100, and a fourth batch of one is refused
/// with nothing of it dispatched, nor its repeat after the grace reply.
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
    assert_eq!(sends, 5);
}

/// LCV-142 AC 4, as amended by LCV-189 — after a batch that lands exactly
/// on the limit, text ends the turn `Ok`; tool calls are answered "not run"
/// and get one more completion, and tool calls again end it
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
    assert_eq!(sends, 4, "exactly one grace completion after the overrun");
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
        &mut |dispatch| {
            dispatches += usize::from(matches!(dispatch, Dispatch::Tool { .. }));
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
        &mut |dispatch| {
            Ok(AgentOutcome::Ok(match dispatch {
                Dispatch::Tool { .. } => "Line created: ….".into(),
                _ => String::new(),
            }))
        },
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
                    reasoning_content: None,
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
                reasoning_content: None,
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
        &mut |dispatch| {
            if !matches!(dispatch, Dispatch::Tool { .. }) {
                return Ok(AgentOutcome::Ok(String::new()));
            }
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
        reasoning_content: None,
    })
}

/// What one scripted turn did: the serialised requests, the authorise
/// count, the tool dispatch count, the result, and the order of sends,
/// authorisations and notes (LCV-187).
struct Run {
    requests: Vec<String>,
    authorisations: usize,
    tools: usize,
    messages: Vec<ChatMessage>,
    result: Result<String, AgentError>,
    events: Vec<String>,
}

/// Drive the loop: `reply(n)` answers the n-th send (1-based), a
/// `capture_canvas` dispatch observes [`PNG`], anything else is `Ok`, and
/// the upload check answers `verdict`. Feedback (LCV-195) answers `Ok("")`
/// and leaves no event.
fn run(
    reply: impl FnMut(usize) -> Result<AssistantMessage, AgentError>,
    verdict: impl FnMut() -> Result<AgentOutcome, AgentError>,
    budget: u32,
) -> Run {
    run_fed(reply, verdict, None, budget)
}

/// The answer a scripted turn gives to `Dispatch::Feedback` (LCV-195).
type Feedback<'a> = &'a mut dyn FnMut() -> Result<AgentOutcome, AgentError>;

/// [`run`] with `feedback` answering `Dispatch::Feedback`, if given; then
/// every tool call and every feedback ask is an event too. A tool named
/// `fenced` is answered `Fenced`.
fn run_fed(
    mut reply: impl FnMut(usize) -> Result<AssistantMessage, AgentError>,
    mut verdict: impl FnMut() -> Result<AgentOutcome, AgentError>,
    mut feedback: Option<Feedback<'_>>,
    budget: u32,
) -> Run {
    let fed = feedback.is_some();
    let (mut requests, mut authorisations, mut tools) = (Vec::new(), 0, 0);
    let events = std::cell::RefCell::new(Vec::new());
    let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
    let result = agent_loop(
        &mut |msgs| {
            requests.push(serde_json::to_string(msgs).unwrap());
            events.borrow_mut().push(format!("send {}", requests.len()));
            reply(requests.len())
        },
        &mut |dispatch| match dispatch {
            Dispatch::AuthorizeUpload => {
                authorisations += 1;
                events.borrow_mut().push("authorise".to_owned());
                verdict()
            }
            Dispatch::Note(text) => {
                events.borrow_mut().push(format!("note {text}"));
                Ok(AgentOutcome::Ok(text.to_owned()))
            }
            Dispatch::Replied { captures } => {
                events.borrow_mut().push(format!("replied {captures}"));
                Ok(AgentOutcome::Ok(String::new()))
            }
            Dispatch::Feedback => match feedback.as_mut() {
                Some(answer) => {
                    events.borrow_mut().push("feedback".to_owned());
                    answer()
                }
                None => Ok(AgentOutcome::Ok(String::new())),
            },
            Dispatch::Tool { name, .. } => {
                tools += 1;
                if fed {
                    events.borrow_mut().push(format!("tool {name}"));
                }
                Ok(match name {
                    "capture_canvas" => AgentOutcome::Observed {
                        text: format!("Canvas {tools}"),
                        png: PNG.to_vec(),
                    },
                    "fenced" => AgentOutcome::Fenced("fenced".into()),
                    _ => AgentOutcome::Ok("ok".into()),
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
        events: events.into_inner(),
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
    assert!(r.messages.iter().all(|m| m.image_count() == 0));
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
    let mut notes = Vec::new();
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
            Dispatch::Note(text) => {
                notes.push(text.to_owned());
                Ok(AgentOutcome::Ok(text.to_owned()))
            }
            Dispatch::Replied { .. } | Dispatch::Feedback => Ok(AgentOutcome::Ok(String::new())),
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
    assert_eq!(
        notes,
        ["Canvas image for call call_0 withheld (permission changed)."],
        "LCV-187 AC 6: the fence's last send notes its image too"
    );
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

/// LCV-189 AC 2 — an overrunning reply runs none of its calls, answers
/// each one "not run" (no steps-left line), keeps every id paired, and the
/// model gets one more reply.
#[test]
fn an_overrun_is_answered_not_run_and_gets_one_more_reply() {
    let r = run(
        |n| {
            if n == 1 {
                named_calls(&["query_entities"; 6])
            } else {
                text_reply("too big")
            }
        },
        yes,
        5,
    );
    assert_eq!(r.result.unwrap(), "too big");
    assert_eq!((r.tools, r.requests.len()), (0, 2));
    let not_run = "not run: this reply has 6 tool calls but 5 steps are left";
    assert_eq!(tool_texts(&r.messages), [not_run; 6]);
    let calls = r.messages[2].tool_calls.as_ref().expect("calls kept");
    assert_eq!(calls.len(), 6);
    for (i, tool) in r.messages[3..].iter().enumerate() {
        assert_eq!(
            tool.tool_call_id.as_deref(),
            Some(format!("call_{i}").as_str())
        );
    }
}

/// LCV-189 AC 2 — the grace reply may spend what is left: budget 5, three
/// run, a batch of three (one over) is refused naming the two left, and a
/// batch of exactly two then runs.
#[test]
fn the_grace_reply_can_spend_the_steps_left() {
    let r = run(
        |n| match n {
            1 | 2 => named_calls(&["query_entities"; 3]),
            3 => named_calls(&["query_entities"; 2]),
            _ => text_reply("done"),
        },
        yes,
        5,
    );
    assert_eq!(r.result.unwrap(), "done");
    assert_eq!((r.tools, r.requests.len()), (5, 4));
    let not_run = "not run: this reply has 3 tool calls but 2 steps are left";
    assert_eq!(
        tool_texts(&r.messages),
        [
            "ok",
            "ok",
            "ok\nSteps left this turn: 2 of 5.",
            not_run,
            not_run,
            not_run,
            "ok",
            "ok\nSteps left this turn: 0 of 5.",
        ]
    );
}

/// LCV-189 AC 3 — a second consecutive overrun ends the turn
/// `IterationLimitExceeded`, with nothing dispatched and no third send.
#[test]
fn a_second_consecutive_overrun_ends_the_turn() {
    let r = run(|_| named_calls(&["query_entities"; 3]), yes, 2);
    assert!(
        matches!(r.result, Err(AgentError::IterationLimitExceeded(2))),
        "got {:?}",
        r.result
    );
    assert_eq!((r.tools, r.requests.len()), (0, 2));
}

/// LCV-189 AC 2, AC 3 — a batch that runs clears the grace: a later
/// overrun gets its own one more reply.
#[test]
fn a_run_batch_between_overruns_resets_the_grace() {
    let r = run(
        |n| match n {
            1 => named_calls(&["query_entities"; 5]),
            2 => named_calls(&["query_entities"; 2]),
            3 => named_calls(&["query_entities"; 3]),
            4 => named_calls(&["query_entities"]),
            _ => text_reply("done"),
        },
        yes,
        4,
    );
    assert_eq!(r.result.unwrap(), "done");
    assert_eq!((r.tools, r.requests.len()), (3, 5));
    let texts = tool_texts(&r.messages);
    assert_eq!(
        texts[7],
        "not run: this reply has 3 tool calls but 2 steps are left"
    );
    assert_eq!(texts[10], "ok\nSteps left this turn: 1 of 4.");
}

/// LCV-189 AC 5 — asking to upload an image is not a step: two captures
/// and a query spend all of a budget of three, even though two sends were
/// authorised along the way.
#[test]
fn an_upload_authorisation_is_not_counted_in_steps_left() {
    let r = run(
        |n| match n {
            1 | 2 => named_calls(&["capture_canvas"]),
            3 => named_calls(&["query_entities"]),
            _ => text_reply("done"),
        },
        yes,
        3,
    );
    assert_eq!(r.result.unwrap(), "done");
    assert_eq!((r.tools, r.authorisations), (3, 2));
    assert_eq!(
        tool_texts(&r.messages),
        [
            "Canvas 1\nSteps left this turn: 2 of 3.",
            "Canvas 2\nSteps left this turn: 1 of 3.",
            "ok\nSteps left this turn: 0 of 3.",
        ]
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

// ── LCV-154: reasoning_content rides with its tool-call turn ─────────────

/// `reply` with `reasoning_content` set to `reasoning`.
fn reasoned(
    reply: Result<AssistantMessage, AgentError>,
    reasoning: &str,
) -> Result<AssistantMessage, AgentError> {
    reply.map(|message| AssistantMessage {
        reasoning_content: Some(reasoning.to_owned()),
        ..message
    })
}

/// The `n`-th message of a serialised request.
fn request_message(request: &str, n: usize) -> serde_json::Value {
    let messages: serde_json::Value = serde_json::from_str(request).unwrap();
    messages[n].clone()
}

/// LCV-154 AC 2 — batch 1's `reasoning_content` goes back verbatim on its
/// assistant message in every later request of the turn; batch 2, which
/// had none, carries no key.
#[test]
fn reasoning_content_rides_every_later_request_of_the_turn() {
    let r = run(
        |n| match n {
            1 => reasoned(named_calls(&["query_entities"]), "R1 \"verbatim\"\n"),
            2 => named_calls(&["query_entities"]),
            _ => text_reply("done"),
        },
        yes,
        AGENT_STEP_BUDGET_DEFAULT,
    );
    assert_eq!(r.result.unwrap(), "done");
    assert_eq!(r.requests.len(), 3);
    assert!(!r.requests[0].contains("reasoning_content"));
    for request in &r.requests[1..] {
        let first = request_message(request, 2);
        assert_eq!(first["role"], "assistant");
        assert_eq!(first["reasoning_content"], "R1 \"verbatim\"\n");
    }
    let second = request_message(&r.requests[2], 4);
    assert_eq!(second["role"], "assistant");
    assert!(second.get("reasoning_content").is_none(), "got {second}");
    assert_eq!(r.requests[2].matches("reasoning_content").count(), 1);
}

/// LCV-154 AC 2 — an overrun batch answered "not run" (LCV-189) is still a
/// tool-call turn and keeps its `reasoning_content`.
#[test]
fn an_overrun_batch_keeps_its_reasoning_content() {
    let r = run(
        |n| match n {
            1 => reasoned(call_reply(2), "R2"),
            _ => text_reply("done"),
        },
        yes,
        1,
    );
    assert_eq!(r.result.unwrap(), "done");
    assert_eq!(r.tools, 0);
    assert_eq!(r.messages[2].reasoning_content.as_deref(), Some("R2"));
    assert_eq!(
        request_message(&r.requests[1], 2)["reasoning_content"],
        "R2"
    );
}

// ── LCV-187: what happened to each image ─────────────────────────────────

/// The note events of a run, in order.
fn notes(r: &Run) -> Vec<&str> {
    r.events
        .iter()
        .filter_map(|e| e.strip_prefix("note "))
        .collect()
}

/// A batch with captures at `call_0` and `call_2`, then `second`.
fn two_captures(second: Result<AssistantMessage, AgentError>) -> Run {
    let mut second = Some(second);
    run(
        move |n| match n {
            1 => named_calls(&["capture_canvas", "query_entities", "capture_canvas"]),
            _ => second.take().unwrap_or_else(|| text_reply("again")),
        },
        yes,
        AGENT_STEP_BUDGET_DEFAULT,
    )
}

/// LCV-187 AC 6 — an authorised send that returns notes `sent` once per
/// image, in call order, after the authorisation and after the send.
#[test]
fn a_delivered_image_is_noted_sent_after_the_send() {
    let r = two_captures(text_reply("seen"));
    assert_eq!(r.result.unwrap(), "seen");
    assert_eq!(
        r.events,
        [
            "send 1",
            "replied 0",
            "authorise",
            "send 2",
            "note Canvas image for call call_0 sent.",
            "note Canvas image for call call_2 sent.",
            "replied 2",
        ]
    );
}

/// LCV-187 AC 6 — a refused upload notes `withheld` per image, whether the
/// text-only send then succeeds or fails.
#[test]
fn a_withheld_image_is_noted_withheld() {
    for second in [
        text_reply("blind"),
        Err(AgentError::Transport("down".into())),
    ] {
        let ok = second.is_ok();
        let r = run(
            {
                let mut second = Some(second);
                move |n| match n {
                    1 => named_calls(&["capture_canvas"]),
                    _ => second.take().unwrap_or_else(|| text_reply("again")),
                }
            },
            || Ok(AgentOutcome::Refused("no".into())),
            AGENT_STEP_BUDGET_DEFAULT,
        );
        assert_eq!(r.result.is_ok(), ok);
        assert_eq!(
            notes(&r),
            ["Canvas image for call call_0 withheld (permission changed)."],
            "send ok: {ok}"
        );
        let last_note = r.events.iter().rev().find(|e| e.starts_with("note "));
        assert_eq!(
            last_note.map(String::as_str),
            Some("note Canvas image for call call_0 withheld (permission changed).")
        );
    }
}

/// LCV-187 AC 6 — an authorised send that fails notes `not delivered` per
/// image before the error returns.
#[test]
fn a_failed_send_notes_the_image_not_delivered() {
    let r = two_captures(Err(AgentError::Transport("503".into())));
    assert!(matches!(r.result, Err(AgentError::Transport(_))));
    assert_eq!(
        r.events,
        [
            "send 1",
            "replied 0",
            "authorise",
            "send 2",
            "note Canvas image for call call_0 not delivered (request failed).",
            "note Canvas image for call call_2 not delivered (request failed).",
        ]
    );
}

/// LCV-187 AC 6 — no image, no note; a cancelled authorisation sends
/// nothing and notes nothing.
#[test]
fn no_image_or_a_cancelled_check_notes_nothing() {
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
    assert_eq!(r.result.as_deref().unwrap(), "done");
    assert!(notes(&r).is_empty());
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
    assert_eq!(r.events, ["send 1", "replied 0", "authorise"]);
}

/// LCV-187 AC 6 — a note is not a step: after two noted images the next
/// batch still reports the steps the tool calls alone left.
#[test]
fn a_note_is_not_a_step() {
    let r = two_captures(named_calls(&["query_entities"]));
    assert_eq!(r.result.as_deref().unwrap(), "again");
    assert_eq!(notes(&r).len(), 2);
    assert_eq!(r.tools, 4);
    assert_eq!(
        tool_texts(&r.messages)[3],
        "ok\nSteps left this turn: 252 of 256."
    );
}

// ── LCV-193: every model reply is counted ────────────────────────────────

/// The `Replied` captures of a run, in order.
fn replies(r: &Run) -> Vec<&str> {
    r.events
        .iter()
        .filter_map(|e| e.strip_prefix("replied "))
        .collect()
}

/// LCV-193 AC 3 — one `Replied` per successful send, dispatched after the
/// send returns and after its image notes, carrying the image parts that
/// request carried (here: none, then two).
#[test]
fn each_successful_send_dispatches_one_replied_after_it_returns() {
    let r = two_captures(text_reply("seen"));
    assert_eq!(r.result.as_deref().unwrap(), "seen");
    assert_eq!(replies(&r), ["0", "2"]);
    assert_eq!(r.events.last().map(String::as_str), Some("replied 2"));
    assert_eq!(r.events[..2], ["send 1", "replied 0"]);
}

/// LCV-193 AC 3 — a withheld upload sent no image: its reply has zero
/// captures.
#[test]
fn a_withheld_upload_replies_with_zero_captures() {
    let r = run(
        |n| match n {
            1 => named_calls(&["capture_canvas"]),
            _ => text_reply("blind"),
        },
        || Ok(AgentOutcome::Refused("no".into())),
        AGENT_STEP_BUDGET_DEFAULT,
    );
    assert_eq!(r.result.as_deref().unwrap(), "blind");
    assert_eq!(replies(&r), ["0", "0"]);
}

/// LCV-193 AC 3 — a failed send is not a reply: no `Replied` for it.
#[test]
fn a_failed_send_dispatches_no_replied() {
    let r = run(
        |_| Err(AgentError::Transport("down".into())),
        yes,
        AGENT_STEP_BUDGET_DEFAULT,
    );
    assert!(matches!(r.result, Err(AgentError::Transport(_))));
    assert!(replies(&r).is_empty(), "{:?}", r.events);
    let r = two_captures(Err(AgentError::Transport("503".into())));
    assert_eq!(replies(&r), ["0"], "only the first send got a reply");
}

/// LCV-193 AC 3 — a reply is not a step: three replies and three steps
/// still leave the budget the tool calls alone left.
#[test]
fn a_reply_is_not_a_step() {
    let r = run(
        |n| match n {
            1 | 2 => named_calls(&["query_entities"]),
            _ => text_reply("done"),
        },
        yes,
        3,
    );
    assert_eq!(r.result.as_deref().unwrap(), "done");
    assert_eq!(replies(&r).len(), 3);
    assert_eq!(
        tool_texts(&r.messages),
        [
            "ok\nSteps left this turn: 2 of 3.",
            "ok\nSteps left this turn: 1 of 3."
        ]
    );
}

// ── LCV-195: feedback after a batch ──────────────────────────────────────

/// The events of a fed run without the `replied` bookkeeping.
fn fed_events(r: &Run) -> Vec<&str> {
    r.events
        .iter()
        .map(String::as_str)
        .filter(|e| !e.starts_with("replied "))
        .collect()
}

/// LCV-195 AC 1 — a batch that ran asks `Feedback` exactly once, after its
/// last call and before the next send.
#[test]
fn a_run_batch_asks_feedback_once_after_its_last_call() {
    let mut feedback = || Ok(AgentOutcome::Ok(String::new()));
    let r = run_fed(
        |n| match n {
            1 => named_calls(&["query_entities", "create_line"]),
            2 => named_calls(&["delete_entity"]),
            _ => text_reply("done"),
        },
        yes,
        Some(&mut feedback),
        10,
    );
    assert_eq!(r.result.as_deref().unwrap(), "done");
    assert_eq!(
        fed_events(&r),
        [
            "send 1",
            "tool query_entities",
            "tool create_line",
            "feedback",
            "send 2",
            "tool delete_entity",
            "feedback",
            "send 3",
        ]
    );
}

/// LCV-195 AC 4, AC 6 — an empty feedback leaves every result as today.
#[test]
fn an_empty_feedback_leaves_the_results_as_today() {
    let mut feedback = || Ok(AgentOutcome::Ok(String::new()));
    let r = run_fed(
        |n| match n {
            1 => named_calls(&["query_entities"; 2]),
            _ => text_reply("done"),
        },
        yes,
        Some(&mut feedback),
        10,
    );
    assert_eq!(
        tool_texts(&r.messages),
        ["ok", "ok\nSteps left this turn: 8 of 10."]
    );
}

/// LCV-195 AC 1, AC 5 — a feedback text goes on the last result only,
/// before the steps-left line, which still counts the tool calls alone.
#[test]
fn a_feedback_text_goes_before_the_steps_left_line() {
    let mut feedback = || Ok(AgentOutcome::Ok("Drawing now: 1 entity.".into()));
    let r = run_fed(
        |n| match n {
            1 => named_calls(&["create_line"; 2]),
            2 => named_calls(&["create_line"]),
            _ => text_reply("done"),
        },
        yes,
        Some(&mut feedback),
        10,
    );
    assert_eq!(r.result.unwrap(), "done");
    assert_eq!(r.tools, 3, "feedback is not a step");
    assert_eq!(
        tool_texts(&r.messages),
        [
            "ok",
            "ok\nDrawing now: 1 entity.\nSteps left this turn: 8 of 10.",
            "ok\nDrawing now: 1 entity.\nSteps left this turn: 7 of 10.",
        ]
    );
}

/// LCV-195 AC 4 — a fence-stopped batch asks no feedback, whether the
/// fence answered a middle call or the last one, and neither does an
/// over-budget batch.
#[test]
fn a_fenced_or_over_budget_batch_asks_no_feedback() {
    for calls in [
        &["create_line", "fenced", "create_line"][..],
        &["create_line", "fenced"][..],
    ] {
        let mut feedback = || Ok(AgentOutcome::Ok("Drawing now: 1 entity.".into()));
        let r = run_fed(
            |n| match n {
                1 => named_calls(calls),
                _ => text_reply("stopped"),
            },
            yes,
            Some(&mut feedback),
            10,
        );
        assert_eq!(r.result.unwrap(), "stopped");
        assert!(!r.events.iter().any(|e| e == "feedback"), "{:?}", r.events);
        assert!(
            tool_texts(&r.messages)
                .iter()
                .all(|t| !t.contains("Drawing"))
        );
    }
    let mut feedback = || Ok(AgentOutcome::Ok("Drawing now: 1 entity.".into()));
    let r = run_fed(
        |n| match n {
            1 => named_calls(&["create_line"; 3]),
            _ => text_reply("too big"),
        },
        yes,
        Some(&mut feedback),
        2,
    );
    assert_eq!(r.result.unwrap(), "too big");
    assert_eq!(r.tools, 0);
    assert!(!r.events.iter().any(|e| e == "feedback"), "{:?}", r.events);
}

/// LCV-195 AC 5 — a cancelled feedback ask ends the turn like any other
/// cancelled rendezvous, before the next send.
#[test]
fn a_cancelled_feedback_ends_the_turn() {
    let mut feedback = || Err(AgentError::Cancelled);
    let r = run_fed(
        |_| named_calls(&["create_line"]),
        yes,
        Some(&mut feedback),
        10,
    );
    assert!(
        matches!(r.result, Err(AgentError::Cancelled)),
        "{:?}",
        r.result
    );
    assert_eq!(r.requests.len(), 1);
}

/// A feedback that observes: the summary text and [`PNG`].
fn observed_feedback() -> Result<AgentOutcome, AgentError> {
    Ok(AgentOutcome::Observed {
        text: "Drawing now: 1 entity.".into(),
        png: PNG.to_vec(),
    })
}

/// LCV-195 AC 3, AC 5 — an observed feedback appends its text like any
/// feedback and rides its image under the last call's id: the upload is
/// authorised before the next send, a `sent` note follows it, and the steps
/// count only the tool calls.
#[test]
fn an_observed_feedback_rides_under_the_last_call_id() {
    let mut feedback = observed_feedback;
    let r = run_fed(
        |n| match n {
            1 => named_calls(&["query_entities", "create_line"]),
            _ => text_reply("seen"),
        },
        yes,
        Some(&mut feedback),
        10,
    );
    assert_eq!(r.result.as_deref().unwrap(), "seen");
    assert_eq!(r.tools, 2);
    assert_eq!(
        tool_texts(&r.messages),
        [
            "ok",
            "ok\nDrawing now: 1 entity.\nSteps left this turn: 8 of 10."
        ]
    );
    let sent: Vec<ChatMessage> = serde_json::from_str(&r.requests[1]).unwrap();
    assert_eq!(
        sent[5],
        ChatMessage::user_parts(vec![
            ContentPart::text("canvas image for tool call call_1"),
            ContentPart::png(PNG),
        ])
    );
    assert_eq!(
        fed_events(&r),
        [
            "send 1",
            "tool query_entities",
            "tool create_line",
            "feedback",
            "authorise",
            "send 2",
            "note Canvas image for call call_1 sent.",
        ]
    );
}

/// LCV-195 AC 3 — when the last call itself captured, both images ride
/// under its id, each labelled and each noted.
#[test]
fn a_capture_and_an_observed_feedback_are_both_labelled_and_noted() {
    let mut feedback = observed_feedback;
    let r = run_fed(
        |n| match n {
            1 => named_calls(&["capture_canvas"]),
            _ => text_reply("seen"),
        },
        yes,
        Some(&mut feedback),
        10,
    );
    assert_eq!(r.result.as_deref().unwrap(), "seen");
    let sent: Vec<ChatMessage> = serde_json::from_str(&r.requests[1]).unwrap();
    let label = || ContentPart::text("canvas image for tool call call_0");
    assert_eq!(
        sent[4],
        ChatMessage::user_parts(vec![
            label(),
            ContentPart::png(PNG),
            label(),
            ContentPart::png(PNG)
        ])
    );
    assert_eq!(r.authorisations, 1);
    assert_eq!(
        notes(&r),
        [
            "Canvas image for call call_0 sent.",
            "Canvas image for call call_0 sent."
        ]
    );
}

// ── LCV-197: verify before reply ─────────────────────────────────────────

/// What one scripted verify turn did.
struct Verified {
    result: Result<String, AgentError>,
    asks: usize,
    tools: usize,
    sends: usize,
    messages: Vec<ChatMessage>,
}

/// Drive the loop through `replies` in order (a text once they run out);
/// `Dispatch::VerifyDue` answers `verify`, a tool named `fenced` is
/// answered `Fenced`, everything else `Ok`.
fn run_verify(
    replies: Vec<Result<AssistantMessage, AgentError>>,
    mut verify: impl FnMut() -> Result<AgentOutcome, AgentError>,
    budget: u32,
) -> Verified {
    let mut replies = replies.into_iter();
    let (mut asks, mut tools, mut sends) = (0, 0, 0);
    let mut messages = vec![ChatMessage::system("s"), ChatMessage::user("u")];
    let result = agent_loop(
        &mut |_| {
            sends += 1;
            replies.next().unwrap_or_else(|| text_reply("spare"))
        },
        &mut |dispatch| match dispatch {
            Dispatch::VerifyDue => {
                asks += 1;
                verify()
            }
            Dispatch::Tool { name, .. } => {
                tools += 1;
                Ok(match name {
                    "fenced" => AgentOutcome::Fenced("fenced".into()),
                    _ => AgentOutcome::Ok("ok".into()),
                })
            }
            _ => Ok(AgentOutcome::Ok(String::new())),
        },
        &mut messages,
        budget,
    );
    Verified {
        result,
        asks,
        tools,
        sends,
        messages,
    }
}

fn granted() -> Result<AgentOutcome, AgentError> {
    Ok(AgentOutcome::Ok(String::new()))
}

/// LCV-197 AC 3 — a yes pushes the interim text and the reminder, sends
/// again and ends with the second text; AC 5 — that second text is never
/// asked about.
#[test]
fn a_granted_reminder_sends_once_more_and_returns_the_second_text() {
    let r = run_verify(
        vec![call_reply(1), text_reply("first"), text_reply("second")],
        granted,
        AGENT_STEP_BUDGET_DEFAULT,
    );
    assert_eq!(r.result.unwrap(), "second");
    assert_eq!((r.asks, r.sends), (1, 3));
    let tail: Vec<_> = r.messages[r.messages.len() - 2..]
        .iter()
        .map(|m| (m.role.as_str(), m.text_content()))
        .collect();
    assert_eq!(
        tail,
        [
            ("assistant", Some("first")),
            ("user", Some(verify::VERIFY_REMINDER)),
        ]
    );
}

/// LCV-197 AC 3 — the reminder is the spec's text, verbatim.
#[test]
fn the_reminder_is_the_spec_text() {
    assert_eq!(
        verify::VERIFY_REMINDER,
        "Before you finish: verify the drawing against the request with measure, \
         check_drawing or capture_canvas, fix what fails, then report each check as \
         pass or fail."
    );
}

/// LCV-197 AC 5 — a no ends the turn with the first text, unchanged.
#[test]
fn a_refused_reminder_ends_with_the_first_text() {
    let r = run_verify(
        vec![call_reply(1), text_reply("first")],
        || Ok(AgentOutcome::Refused(String::new())),
        AGENT_STEP_BUDGET_DEFAULT,
    );
    assert_eq!(r.result.unwrap(), "first");
    assert_eq!((r.asks, r.sends), (1, 2));
    assert_eq!(r.messages.len(), 4, "no interim text, no reminder");
}

/// LCV-197 AC 6 — with no step left the loop never asks.
#[test]
fn no_step_left_is_never_asked() {
    let r = run_verify(vec![call_reply(1), text_reply("first")], granted, 1);
    assert_eq!(r.result.unwrap(), "first");
    assert_eq!((r.asks, r.tools), (0, 1));
}

/// LCV-197 AC 6 — one step left is enough to be asked.
#[test]
fn one_step_left_is_asked() {
    let r = run_verify(
        vec![call_reply(1), text_reply("first"), text_reply("second")],
        granted,
        2,
    );
    assert_eq!(r.result.unwrap(), "second");
    assert_eq!(r.asks, 1);
}

/// LCV-197 AC 6 — after a fence stop the last word ends the turn unasked.
#[test]
fn a_fence_stop_is_never_asked() {
    let r = run_verify(
        vec![named_calls(&["fenced"]), text_reply("stopped")],
        granted,
        AGENT_STEP_BUDGET_DEFAULT,
    );
    assert_eq!(r.result.unwrap(), "stopped");
    assert_eq!(r.asks, 0);
}

/// LCV-197 AC 6 — a cancelled ask ends the turn `Cancelled`.
#[test]
fn a_cancelled_ask_returns_cancelled() {
    let r = run_verify(
        vec![call_reply(1), text_reply("first")],
        || Err(AgentError::Cancelled),
        AGENT_STEP_BUDGET_DEFAULT,
    );
    assert!(matches!(r.result, Err(AgentError::Cancelled)), "{:?}", r.result);
    assert_eq!(r.sends, 2);
}

/// LCV-197 AC 5 — tool calls after the reminder dispatch as usual, and the
/// next text ends the turn without a second ask.
#[test]
fn tool_calls_after_the_reminder_dispatch_normally() {
    let r = run_verify(
        vec![
            call_reply(1),
            text_reply("first"),
            named_calls(&["measure", "check_drawing"]),
            text_reply("verified"),
        ],
        granted,
        AGENT_STEP_BUDGET_DEFAULT,
    );
    assert_eq!(r.result.unwrap(), "verified");
    assert_eq!((r.asks, r.tools, r.sends), (1, 3, 4));
}
