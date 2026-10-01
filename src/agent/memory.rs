//! LCV-153 — the conversation memory's policy (ADR 0007 §D16).
//!
//! Memory is a list of turns, each the `Vec<ChatMessage>` the model saw for
//! it: the user message, every whole tool-call batch, and one closing
//! assistant text. It lives on the UI side (`AgentState::memory`) and crosses
//! into the worker by value once per turn; this file owns every rule about
//! what goes in and what comes out — the record, the whole-batch filter, the
//! token estimate and the trim. `app::agent_memory` is only glue.
//!
//! Kernel-pure (AGENTS.md §Purity rule): no UI toolkit, no HTTP client and
//! no document type. The system prompt is never stored (LCV-143 resolves it per
//! turn) and nothing here is ever persisted.

use crate::agent::attachment::ATTACHED_IMAGE_ELIDED;
use crate::agent::wire::{ChatMessage, Content, ContentPart};

/// `Settings::agent_context_tokens` when the operator has not chosen.
pub const CONTEXT_TOKENS_DEFAULT: u32 = 128_000;

/// Smallest context window a turn is sized for.
pub const CONTEXT_TOKENS_MIN: u32 = 8_000;

/// Largest context window a turn is sized for.
pub const CONTEXT_TOKENS_MAX: u32 = 2_000_000;

/// What an old tool result becomes once the trim elides it (AC 9).
pub const ELIDED_TOOL_RESULT: &str = "[old tool result elided]";

/// The closing assistant text of a turn the operator cancelled (AC 6).
pub const CANCELLED_TEXT: &str = "Turn cancelled by the operator.";

/// The line put before a user message when the drawing changed since the
/// model last saw it (AC 7).
pub const DRAWING_CHANGED_PREFIX: &str =
    "[The drawing changed since your last turn; re-read indices with query_entities.]";

/// Hold a stored context size inside
/// `CONTEXT_TOKENS_MIN..=CONTEXT_TOKENS_MAX`; the settings file is
/// hand-editable, so every reader clamps.
pub fn clamp_context_tokens(value: u32) -> u32 {
    value.clamp(CONTEXT_TOKENS_MIN, CONTEXT_TOKENS_MAX)
}

/// `prompt` with [`DRAWING_CHANGED_PREFIX`] on its own line before it.
pub fn with_changed_prefix(prompt: &str) -> String {
    format!("{DRAWING_CHANGED_PREFIX}\n{prompt}")
}

/// How a turn ended, as far as memory cares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnEnd {
    /// The model answered with a final text.
    Done {
        /// That text.
        text: String,
    },
    /// The turn failed, or its worker was lost.
    Stopped {
        /// The human-readable error.
        error: String,
    },
    /// The operator cancelled the turn.
    Cancelled,
}

/// One turn's memory entry: `user`, then `batches`, then the closing
/// assistant text `end` dictates. A cancelled turn keeps no batches (AC 6).
/// A stopped turn's error loses a trailing full stop, so the sentence that
/// wraps it ends in exactly one. A turn that carried an attached `image`
/// keeps `[user, image elided]` parts in its place (LCV-199 AC 6).
pub fn turn_record(
    user: &str,
    image: bool,
    batches: Vec<ChatMessage>,
    end: &TurnEnd,
) -> Vec<ChatMessage> {
    let mut record = vec![if image {
        ChatMessage::user_parts(vec![
            ContentPart::text(user),
            ContentPart::text(ATTACHED_IMAGE_ELIDED),
        ])
    } else {
        ChatMessage::user(user)
    }];
    let closing = match end {
        TurnEnd::Done { text } => text.clone(),
        TurnEnd::Stopped { error } => format!("Turn stopped: {}.", error.trim_end_matches('.')),
        TurnEnd::Cancelled => CANCELLED_TEXT.to_owned(),
    };
    if *end != TurnEnd::Cancelled {
        record.extend(batches);
    }
    record.push(ChatMessage::assistant(closing));
    record
}

/// The whole tool-call batches of `messages`: each assistant `tool_calls`
/// message with everything up to the next assistant message, kept only when
/// every call it made has its `tool` result. A batch missing a result is
/// dropped whole (AC 5), so the model never reads an unanswered call.
pub fn whole_batches(messages: &[ChatMessage]) -> Vec<ChatMessage> {
    let mut kept = Vec::new();
    let mut start = 0;
    while start < messages.len() {
        let end = messages[start + 1..]
            .iter()
            .position(|m| m.role == "assistant")
            .map_or(messages.len(), |at| start + 1 + at);
        let batch = &messages[start..end];
        let answered = |id: &String| {
            batch
                .iter()
                .any(|m| m.role == "tool" && m.tool_call_id.as_ref() == Some(id))
        };
        if let Some(calls) = &batch[0].tool_calls
            && calls.iter().all(|call| answered(&call.id))
        {
            kept.extend_from_slice(batch);
        }
        start = end;
    }
    kept
}

/// The token estimate of AC 8: UTF-8 bytes of every text, tool-call
/// argument string and `reasoning_content` (LCV-154), summed, then divided
/// by four once.
pub fn estimate_tokens(messages: &[ChatMessage]) -> usize {
    messages.iter().map(message_bytes).sum::<usize>() / 4
}

/// The bytes one message adds to the estimate: its text (every text part of
/// a parts message), the arguments of every tool call it carries and its
/// `reasoning_content`.
fn message_bytes(message: &ChatMessage) -> usize {
    let text = match &message.content {
        Some(Content::Text(text)) => text.len(),
        Some(Content::Parts(parts)) => parts
            .iter()
            .map(|part| match part {
                ContentPart::Text { text } => text.len(),
                ContentPart::ImageUrl { .. } => 0,
            })
            .sum(),
        None => 0,
    };
    let arguments: usize = message
        .tool_calls
        .iter()
        .flatten()
        .map(|call| call.function.arguments.len())
        .sum();
    let reasoning = message.reasoning_content.as_ref().map_or(0, String::len);
    text + arguments + reasoning
}

/// The conversation so far, one entry per turn, oldest first.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Memory {
    turns: Vec<Vec<ChatMessage>>,
}

impl Memory {
    /// Append one turn's record.
    pub fn push_turn(&mut self, turn: Vec<ChatMessage>) {
        self.turns.push(turn);
    }

    /// Every stored message, in order — what goes between the system prompt
    /// and the new user message.
    pub fn flatten(&self) -> Vec<ChatMessage> {
        self.turns.iter().flatten().cloned().collect()
    }

    /// The stored turns, oldest first.
    pub fn turns(&self) -> &[Vec<ChatMessage>] {
        &self.turns
    }

    /// No turn stored yet.
    pub fn is_empty(&self) -> bool {
        self.turns.is_empty()
    }

    /// Forget everything (LCV-150).
    pub fn clear(&mut self) {
        self.turns.clear();
    }

    /// AC 9: when the estimate exceeds `context_tokens / 2`, shrink memory to
    /// at most half that — first eliding tool results outside the newest
    /// turn, oldest first, then dropping the oldest whole turns. The newest
    /// turn is always kept whole. A result no longer than
    /// [`ELIDED_TOOL_RESULT`] is left as it is: replacing it would not shrink
    /// anything.
    pub fn trim(&mut self, context_tokens: u32) {
        let cap = usize::try_from(context_tokens / 2).unwrap_or(usize::MAX);
        let target = cap / 2;
        let mut bytes: usize = self.turns.iter().flatten().map(message_bytes).sum();
        if bytes / 4 <= cap {
            return;
        }
        let newest = self.turns.len().saturating_sub(1);
        let old_results = self.turns[..newest]
            .iter_mut()
            .flatten()
            .filter(|m| m.role == "tool");
        for message in old_results {
            if bytes / 4 <= target {
                break;
            }
            let before = message_bytes(message);
            if before > ELIDED_TOOL_RESULT.len() {
                message.content = Some(Content::Text(ELIDED_TOOL_RESULT.to_owned()));
                bytes = bytes - before + ELIDED_TOOL_RESULT.len();
            }
        }
        while bytes / 4 > target && self.turns.len() > 1 {
            let dropped = self.turns.remove(0);
            bytes -= dropped.iter().map(message_bytes).sum::<usize>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::wire::ToolCall;

    fn reply(text: &str) -> ChatMessage {
        ChatMessage {
            role: "assistant".to_owned(),
            content: Some(Content::Text(text.to_owned())),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }
    }

    fn calls(ids: &[&str], args: &str) -> ChatMessage {
        let calls = ids
            .iter()
            .map(|id| ToolCall::function(*id, "query_entities", args))
            .collect();
        ChatMessage::assistant_with_tool_calls(None, calls)
    }

    /// One recorded turn: `user`, one call with `{}`, its result, `ok`.
    fn turn(user: &str, result: &str) -> Vec<ChatMessage> {
        vec![
            ChatMessage::user(user),
            calls(&["c"], "{}"),
            ChatMessage::tool_result("c", result),
            reply("ok"),
        ]
    }

    fn memory(turns: Vec<Vec<ChatMessage>>) -> Memory {
        let mut memory = Memory::default();
        for t in turns {
            memory.push_turn(t);
        }
        memory
    }

    fn tool_text(memory: &Memory, turn: usize) -> &str {
        memory.turns()[turn][2].text_content().unwrap_or_default()
    }

    /// AC 1 — a `Done` turn is user, every batch in order, then the text.
    #[test]
    fn a_done_turn_records_user_batches_then_the_final_text() {
        let batches = vec![
            calls(&["a", "b"], "{}"),
            ChatMessage::tool_result("a", "ra"),
            ChatMessage::tool_result("b", "rb"),
            calls(&["c"], "{}"),
            ChatMessage::tool_result("c", "rc"),
        ];
        let end = TurnEnd::Done {
            text: "drawn".to_owned(),
        };
        let record = turn_record("draw it", false, batches.clone(), &end);
        let mut expected = vec![ChatMessage::user("draw it")];
        expected.extend(batches);
        expected.push(reply("drawn"));
        assert_eq!(record, expected);
    }

    /// LCV-199 AC 6 — a turn that carried an attached image keeps the
    /// placeholder beside its prompt, never the image.
    #[test]
    fn an_image_turn_keeps_the_placeholder() {
        let end = TurnEnd::Done { text: "ok".into() };
        let record = turn_record("from the sketch", true, Vec::new(), &end);
        assert_eq!(
            record[0],
            ChatMessage::user_parts(vec![
                ContentPart::text("from the sketch"),
                ContentPart::text("image elided"),
            ])
        );
        assert_eq!(record[0].image_count(), 0);
    }

    /// AC 5 — a failed turn keeps its batches and closes with the error.
    #[test]
    fn a_stopped_turn_closes_with_the_error_sentence() {
        let batches = turn("x", "r")[1..3].to_vec();
        let end = TurnEnd::Stopped {
            error: "transport error: 503".to_owned(),
        };
        let record = turn_record("go", false, batches.clone(), &end);
        let mut expected = vec![ChatMessage::user("go")];
        expected.extend(batches);
        expected.push(reply("Turn stopped: transport error: 503."));
        assert_eq!(record, expected);
        // An error that already ends a sentence is not doubled up.
        let end = TurnEnd::Stopped {
            error: "Agent turn ended without a reply.".to_owned(),
        };
        let record = turn_record("go", false, Vec::new(), &end);
        assert_eq!(
            record.last(),
            Some(&reply("Turn stopped: Agent turn ended without a reply."))
        );
    }

    /// AC 6 — a cancelled turn is its user message and the fixed text only.
    #[test]
    fn a_cancelled_turn_is_user_and_the_cancelled_text() {
        let batches = turn("x", "r")[1..3].to_vec();
        let record = turn_record("go", false, batches, &TurnEnd::Cancelled);
        assert_eq!(record, vec![ChatMessage::user("go"), reply(CANCELLED_TEXT)]);
    }

    /// AC 2 — no record, whatever its end, holds a system message.
    #[test]
    fn no_record_holds_a_system_message() {
        for end in [
            TurnEnd::Done { text: "t".into() },
            TurnEnd::Stopped { error: "e".into() },
            TurnEnd::Cancelled,
        ] {
            let record = turn_record("u", false, turn("x", "r")[1..3].to_vec(), &end);
            assert!(!record.is_empty());
            assert!(record.iter().all(|m| m.role != "system"), "{end:?}");
        }
    }

    /// AC 5 — a batch missing a result is dropped whole; complete batches,
    /// with the elided image message that followed them, are kept.
    #[test]
    fn whole_batches_drops_a_batch_missing_any_result() {
        let first = vec![
            calls(&["a"], "{}"),
            ChatMessage::tool_result("a", "seen"),
            ChatMessage::user_parts(vec![ContentPart::text("image elided")]),
        ];
        let mut messages = first.clone();
        messages.extend([
            calls(&["b", "c"], "{}"),
            ChatMessage::tool_result("b", "only b"),
        ]);
        assert_eq!(whole_batches(&messages), first);
        messages.push(ChatMessage::tool_result("c", "and c"));
        assert_eq!(whole_batches(&messages), messages);
        assert!(whole_batches(&[]).is_empty());
    }

    /// AC 8 — bytes are summed first and divided once; arguments count.
    #[test]
    fn the_estimate_sums_bytes_then_divides_by_four() {
        // 3 + 3 bytes: per-message division would give 0.
        assert_eq!(estimate_tokens(&[reply("abc"), reply("def")]), 1);
        // 4 bytes of arguments, no content.
        assert_eq!(estimate_tokens(&[calls(&["a"], "{\"\"}")]), 1);
        assert_eq!(estimate_tokens(&[calls(&["a", "b"], "{\"\"}")]), 2);
        // UTF-8 bytes, not chars: "é" is two bytes.
        assert_eq!(estimate_tokens(&[reply("éé")]), 1);
        assert_eq!(estimate_tokens(&[reply("abcdefg")]), 1);
        let parts = ChatMessage::user_parts(vec![ContentPart::text("abcd")]);
        assert_eq!(estimate_tokens(&[parts]), 1);
        assert_eq!(estimate_tokens(&[]), 0);
    }

    /// LCV-154 AC 6 — `reasoning_content` bytes count like any text: 8 bytes
    /// of arguments and 8 of reasoning are 16 bytes, 4 tokens.
    #[test]
    fn the_estimate_counts_reasoning_content() {
        let reasoned = calls(&["a"], "{\"ab\":1}").with_reasoning(Some("thinking".into()));
        assert_eq!(estimate_tokens(&[reasoned]), 4);
        let short = calls(&["a"], "{}").with_reasoning(Some("abcdef".into()));
        assert_eq!(estimate_tokens(&[short]), 2);
    }

    /// LCV-154 AC 6 — the trim counts reasoning toward the cap: without the
    /// 200 reasoning bytes the two turns (410 bytes) sit under a 150-token
    /// cap; with them (610 bytes) the trim elides and then drops turn 0.
    #[test]
    fn the_trim_counts_reasoning_toward_the_cap() {
        let plain = vec![turn("u", &"r".repeat(200)), turn("u", &"r".repeat(200))];
        let mut untouched = memory(plain.clone());
        untouched.trim(300);
        assert_eq!(untouched, memory(plain.clone()));
        let mut reasoned = plain.clone();
        reasoned[0][1] = reasoned[0][1].clone().with_reasoning(Some("t".repeat(200)));
        let mut m = memory(reasoned);
        assert_eq!(estimate_tokens(&m.flatten()), 152);
        m.trim(300);
        assert_eq!(m, memory(plain[1..].to_vec()));
    }

    /// AC 8 — default 128 000, clamped to 8 000..=2 000 000.
    #[test]
    fn the_context_size_is_clamped() {
        assert_eq!(CONTEXT_TOKENS_DEFAULT, 128_000);
        assert_eq!(clamp_context_tokens(0), 8_000);
        assert_eq!(clamp_context_tokens(7_999), 8_000);
        assert_eq!(clamp_context_tokens(8_001), 8_001);
        assert_eq!(clamp_context_tokens(2_000_001), 2_000_000);
        assert_eq!(clamp_context_tokens(u32::MAX), 2_000_000);
    }

    /// AC 7 — the prefix sits on its own line before the prompt.
    #[test]
    fn the_changed_prefix_is_its_own_line() {
        assert_eq!(
            with_changed_prefix("wider"),
            format!("{DRAWING_CHANGED_PREFIX}\nwider")
        );
    }

    /// AC 9 — at the cap nothing moves; one token over, it trims.
    #[test]
    fn memory_at_the_cap_is_untouched() {
        // Four turns of 205 bytes: 820 bytes, 205 tokens.
        let turns: Vec<_> = (0..4).map(|_| turn("u", &"r".repeat(200))).collect();
        let mut at_cap = memory(turns.clone());
        assert_eq!(estimate_tokens(&at_cap.flatten()), 205);
        at_cap.trim(410);
        assert_eq!(at_cap, memory(turns.clone()));
        let mut over = memory(turns);
        over.trim(408);
        assert_eq!(tool_text(&over, 0), ELIDED_TOOL_RESULT);
    }

    /// AC 9 phase 1 — tool results are elided oldest first, and the trim
    /// stops as soon as memory is at or below half the cap.
    #[test]
    fn phase_one_elides_old_tool_results_oldest_first_and_stops() {
        // 5 × 205 bytes = 256 tokens > cap 250; target 125 tokens.
        let mut m = memory((0..5).map(|_| turn("u", &"r".repeat(200))).collect());
        m.trim(500);
        assert_eq!(m.turns().len(), 5);
        for t in 0..3 {
            assert_eq!(tool_text(&m, t), ELIDED_TOOL_RESULT, "turn {t}");
        }
        assert_eq!(tool_text(&m, 3), "r".repeat(200));
        assert_eq!(tool_text(&m, 4), "r".repeat(200));
        assert!(estimate_tokens(&m.flatten()) <= 125);
    }

    /// AC 9 phase 2 — when eliding is not enough, the oldest whole turns go,
    /// until memory is at or below half the cap.
    #[test]
    fn phase_two_drops_the_oldest_turns_until_under_target() {
        // 5 turns of 405 bytes, results too short to elide: 506 tokens.
        let turns: Vec<_> = (0..5)
            .map(|n| turn(&format!("{n}{}", "u".repeat(399)), "r"))
            .collect();
        let mut m = memory(turns.clone());
        m.trim(1000);
        assert_eq!(m, memory(turns[3..].to_vec()));
    }

    /// AC 9 phase 1 — a result as short as the placeholder is kept verbatim:
    /// replacing it would lose its text and shrink nothing.
    #[test]
    fn phase_one_keeps_a_result_no_longer_than_the_placeholder() {
        let short = "x".repeat(ELIDED_TOOL_RESULT.len());
        let turns = vec![
            turn("u", &short),
            turn("u", &"r".repeat(2000)),
            turn("u", "r"),
        ];
        let mut m = memory(turns);
        m.trim(1000);
        assert_eq!(m.turns().len(), 3);
        assert_eq!(tool_text(&m, 0), short);
        assert_eq!(tool_text(&m, 1), ELIDED_TOOL_RESULT);
    }

    /// AC 9 phase 2 — memory exactly at half the cap is done: no extra turn goes.
    #[test]
    fn phase_two_stops_exactly_at_the_target() {
        // 5 turns of 400 bytes: 500 tokens > cap 400; two turns are 200 = target.
        let turns: Vec<_> = (0..5)
            .map(|n| turn(&format!("{n}{}", "u".repeat(394)), "r"))
            .collect();
        let mut m = memory(turns.clone());
        m.trim(800);
        assert_eq!(estimate_tokens(&m.flatten()), 200);
        assert_eq!(m, memory(turns[3..].to_vec()));
    }

    /// AC 9 — both phases, and the newest turn's results stay whole even
    /// when that leaves memory over the target.
    #[test]
    fn the_newest_turn_is_always_kept_whole() {
        let turns: Vec<_> = (0..3).map(|_| turn("u", &"r".repeat(200))).collect();
        let mut m = memory(turns.clone());
        m.trim(200);
        assert_eq!(m, memory(turns[2..].to_vec()));
        let big = vec![turn(&"u".repeat(4000), &"r".repeat(4000))];
        let mut single = memory(big.clone());
        single.trim(100);
        assert_eq!(single, memory(big));
        let mut empty = Memory::default();
        empty.trim(0);
        assert!(empty.is_empty());
    }

    /// The container itself: order in, order out, and clear forgets.
    #[test]
    fn flatten_keeps_order_and_clear_forgets() {
        let (a, b) = (turn("a", "ra"), turn("b", "rb"));
        let mut m = memory(vec![a.clone(), b.clone()]);
        assert!(!m.is_empty());
        assert_eq!(m.flatten(), [a, b].concat());
        m.clear();
        assert!(m.is_empty());
        assert!(m.flatten().is_empty());
    }

    /// Kernel-pure (AGENTS.md §Purity rule), with a positive control over the
    /// same slice.
    #[test]
    fn memory_is_kernel_pure() {
        let src = include_str!("memory.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("a bare #[cfg(test)] marker");
        let implementation = &src[..at];
        assert!(implementation.contains(concat!("Chat", "Message")));
        for forbidden in [
            concat!("req", "west"),
            concat!("eg", "ui"),
            concat!("ef", "rame"),
            concat!("rf", "d::"),
            concat!("crate::", "document"),
            concat!("crate::", "app"),
        ] {
            assert!(!implementation.contains(forbidden), "{forbidden}");
        }
    }
}
