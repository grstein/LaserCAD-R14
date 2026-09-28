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
    value
}

/// `prompt` with [`DRAWING_CHANGED_PREFIX`] on its own line before it.
pub fn with_changed_prefix(prompt: &str) -> String {
    prompt.to_owned()
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
pub fn turn_record(user: &str, batches: Vec<ChatMessage>, end: &TurnEnd) -> Vec<ChatMessage> {
    let _ = (user, batches, end);
    Vec::new()
}

/// The whole tool-call batches of `messages`: each assistant `tool_calls`
/// message with everything up to the next assistant message, kept only when
/// every call it made has its `tool` result. A batch missing a result is
/// dropped whole (AC 5), so the model never reads an unanswered call.
pub fn whole_batches(messages: &[ChatMessage]) -> Vec<ChatMessage> {
    let _ = messages;
    Vec::new()
}

/// The token estimate of AC 8: UTF-8 bytes of every text and tool-call
/// argument string, summed, then divided by four once.
pub fn estimate_tokens(messages: &[ChatMessage]) -> usize {
    let _ = messages;
    0
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
    /// turn is always kept whole.
    pub fn trim(&mut self, context_tokens: u32) {
        let _ = context_tokens;
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
        let record = turn_record("draw it", batches.clone(), &end);
        let mut expected = vec![ChatMessage::user("draw it")];
        expected.extend(batches);
        expected.push(reply("drawn"));
        assert_eq!(record, expected);
    }

    /// AC 5 — a failed turn keeps its batches and closes with the error.
    #[test]
    fn a_stopped_turn_closes_with_the_error_sentence() {
        let batches = turn("x", "r")[1..3].to_vec();
        let end = TurnEnd::Stopped {
            error: "transport error: 503".to_owned(),
        };
        let record = turn_record("go", batches.clone(), &end);
        let mut expected = vec![ChatMessage::user("go")];
        expected.extend(batches);
        expected.push(reply("Turn stopped: transport error: 503."));
        assert_eq!(record, expected);
        // An error that already ends a sentence is not doubled up.
        let end = TurnEnd::Stopped {
            error: "Agent turn ended without a reply.".to_owned(),
        };
        let record = turn_record("go", Vec::new(), &end);
        assert_eq!(
            record.last(),
            Some(&reply("Turn stopped: Agent turn ended without a reply."))
        );
    }

    /// AC 6 — a cancelled turn is its user message and the fixed text only.
    #[test]
    fn a_cancelled_turn_is_user_and_the_cancelled_text() {
        let batches = turn("x", "r")[1..3].to_vec();
        let record = turn_record("go", batches, &TurnEnd::Cancelled);
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
            let record = turn_record("u", turn("x", "r")[1..3].to_vec(), &end);
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
