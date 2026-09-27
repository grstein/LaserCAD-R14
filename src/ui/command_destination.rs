//! LCV-139 — where would this exact line go, right now?
//!
//! [`destination_label`] answers the two-row command dock's context-row
//! question with no side effects at all: it opens no panel, arms no turn,
//! sends nothing, and touches neither the document nor the history. It is a
//! pure function of the four things that decide routing (ADR 0007 §D9): the
//! raw text, whether the active tool wants the command line as free text
//! (LCV-112), whether an agent is configured, and whether one is already
//! busy.
//!
//! This presents routing; it does not add any. [`crate::agent::classify`] is
//! called with the exact same `agent_available` definition
//! `src/app/cmdline.rs::agent_available` computes (AC 3, AC 5) — never a
//! second parser, a parallel prefix table, or an independent availability
//! check. `src/app/cmdline.rs::submit` and this module both read
//! [`crate::agent::Route`]; neither one decides it.
//!
//! Purity: no `egui` import. This file is testable as a table of pure
//! function calls, with no `App` and no UI context.

use crate::agent::{classify, Route};

/// [`Route::Cad`] — the grammar owns the line, whatever it is: a blank line,
/// a recognised word, or `Unknown command: "…"` waiting to happen (AC 4/5).
pub const LABEL_CAD: &str = "CAD";
/// [`Route::Agent`] with a non-empty prompt and a configured key (AC 4).
pub const LABEL_AI: &str = "AI";
/// Raw-input mode: the active tool wants the command line as free text, and
/// `classify` is never reached (AC 3, AC 4).
pub const LABEL_TOOL_INPUT: &str = "tool input";
/// [`Route::Unavailable`] — a `:` / `/ai` prompt with no configured key
/// (AC 4).
pub const LABEL_AI_UNAVAILABLE: &str = "AI unavailable";
/// A bare `:` or `/ai` prefix with nothing after it — refused whether or not
/// a key is configured, exactly as [`crate::agent::classify`] refuses it
/// before availability is even consulted (AC 4).
pub const LABEL_AI_PROMPT_EMPTY: &str = "AI prompt empty";
/// A `:` / `/ai`-prefixed non-empty prompt while a turn is already in flight
/// (AC 4). Never applied to a CAD line: only [`Route::Agent`] is sensitive to
/// `agent_busy` at all.
pub const LABEL_AI_BUSY: &str = "AI busy";

/// What would happen if `raw` were submitted right now, exactly as
/// `src/app/cmdline.rs::submit` would dispatch it.
///
/// Precedence (AC 3-5, mirroring `submit` and `to_agent`):
///
/// 1. `wants_raw_input` wins unconditionally — [`classify`] is not called at
///    all while it is `true` (ADR 0007 §D9 rule 1; the same reason `submit`
///    returns before reaching its own `classify` call).
/// 2. Otherwise [`classify`] decides. [`Route::Cad`] is always
///    [`LABEL_CAD`] — a blank line and any unrecognised word included — and
///    is never affected by `agent_busy` (AC 4's closing sentence).
/// 3. A [`Route::Agent`] prompt is checked empty-first, before `agent_busy`,
///    matching `to_agent`'s own order: a bare `:` is malformed whatever the
///    turn state is, and answering "busy" to it would send the operator
///    looking for a turn that has nothing to do with their typo.
pub fn destination_label(
    raw: &str,
    wants_raw_input: bool,
    agent_available: bool,
    agent_busy: bool,
) -> &'static str {
    if wants_raw_input {
        return LABEL_TOOL_INPUT;
    }
    match classify(raw, agent_available) {
        Route::Cad => LABEL_CAD,
        Route::Unavailable => LABEL_AI_UNAVAILABLE,
        Route::Agent(prompt) if prompt.is_empty() => LABEL_AI_PROMPT_EMPTY,
        Route::Agent(_) if agent_busy => LABEL_AI_BUSY,
        Route::Agent(_) => LABEL_AI,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC 3 — raw-input mode wins over everything, including a `:` prefix
    /// that would otherwise be a non-empty agent prompt, and regardless of
    /// availability or busy state.
    #[test]
    fn raw_input_mode_wins_before_classification() {
        for (available, busy) in [(false, false), (true, false), (true, true)] {
            assert_eq!(
                destination_label(":draw a line", true, available, busy),
                LABEL_TOOL_INPUT
            );
        }
    }

    /// AC 4/5 — the whole precedence table, every row with `agent_busy` both
    /// `true` and `false`, and `agent_available` both `true` and `false`.
    /// Since only a non-empty [`Route::Agent`] prompt is sensitive to either
    /// flag, every CAD row must answer [`LABEL_CAD`] in all four
    /// combinations, and a bare prefix must answer [`LABEL_AI_PROMPT_EMPTY`]
    /// in all four as well.
    #[test]
    fn the_precedence_table_holds_across_availability_and_busy() {
        let cad_rows = [
            "",
            "l",
            "line",
            "circle",
            "50,25",
            "@10,0",
            "37.5",
            "lien",
            "z",
            "zoom sideways",
            "/aim",
            "é",
        ];
        for raw in cad_rows {
            for available in [false, true] {
                for busy in [false, true] {
                    assert_eq!(
                        destination_label(raw, false, available, busy),
                        LABEL_CAD,
                        "`{raw}` (available={available}, busy={busy})"
                    );
                }
            }
        }

        let empty_prompt_rows = [":", "/ai", "/ai   ", ":   "];
        for raw in empty_prompt_rows {
            for available in [false, true] {
                for busy in [false, true] {
                    assert_eq!(
                        destination_label(raw, false, available, busy),
                        LABEL_AI_PROMPT_EMPTY,
                        "`{raw}` (available={available}, busy={busy})"
                    );
                }
            }
        }

        // A non-empty prompt: unavailable beats busy (busy is irrelevant —
        // `Route::Unavailable`, not `Route::Agent`, is what `classify`
        // returns without a key).
        for raw in [":draw a line", "/ai draw a line"] {
            for busy in [false, true] {
                assert_eq!(
                    destination_label(raw, false, false, busy),
                    LABEL_AI_UNAVAILABLE,
                    "`{raw}` with no key (busy={busy})"
                );
            }
            assert_eq!(destination_label(raw, false, true, false), LABEL_AI);
            assert_eq!(destination_label(raw, false, true, true), LABEL_AI_BUSY);
        }
    }

    /// AC 4 — a whitespace-only key is the same as no key at all: the
    /// definition lives in `src/app/cmdline.rs::agent_available`
    /// (`!key.trim().is_empty()`), and this function only ever receives the
    /// result of that check, never a key itself. A whitespace-only key
    /// therefore reaches this function exactly as `agent_available == false`.
    #[test]
    fn a_whitespace_only_key_is_unavailable_by_construction() {
        // `agent_available` here stands for `!"   ".trim().is_empty()`,
        // i.e. `false` — this function has no other way to see a key.
        assert_eq!(
            destination_label(":hello", false, false, false),
            LABEL_AI_UNAVAILABLE
        );
    }
}
