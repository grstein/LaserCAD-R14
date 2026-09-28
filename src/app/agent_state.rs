//! [`AgentState`] — the agent's UI-side fields, split out of `struct App`.
//!
//! ADR 0004 §"The `src/app/mod.rs` seam, pre-decided per rule 4" named this
//! split before the demand that finally crossed the cap arrived (LCV-136):
//! eight agent fields moved off `App` into one `pub agent: AgentState` field,
//! landing `src/app/mod.rs` back under the 300-LOC implementation cap. Pure
//! data, no behaviour: every method that used to read or write one of these
//! fields directly on `App` now goes through `app.agent.<field>` instead, and
//! nothing about when or how those fields change moved with them.
//!
//! `agent_settings_open` stays on `App` itself, not here — it sits with
//! `about_open` and `shortcuts_open` in the dialog-visibility cluster; the
//! seam follows the turn, not every flag whose name starts with `agent`.

use std::sync::mpsc::Receiver;

use crate::agent::{AgentEvent, Memory};
use crate::app::TurnState;

/// The agent's UI-side state: the chat transcript, the in-flight turn's
/// channel and [`TurnState`], and the panel's own visibility and input draft.
///
/// `Default` matches what `App::default()` used to spell out field by field:
/// an empty transcript, no receiver, `busy == false`, and a default
/// [`TurnState`] whose fence equals `TurnFence::new(0)`.
#[derive(Default)]
pub struct AgentState {
    /// Whether the AI assistant side panel is visible (LCV-080).
    /// Toggled by the 🤖 toolbar button.
    pub panel_open: bool,
    /// Chat history as `(role, content)` pairs (LCV-080).
    /// Role is one of `"user"`, `"assistant"`, `"error"`, `"tool"`,
    /// `"refused"` or `"note"` (LCV-123 AC 23; LCV-125 renders them).
    pub chat: Vec<(String, String)>,
    /// Live contents of the AI text-input widget; cleared on submit (LCV-080).
    pub input_draft: String,
    /// `true` while a background agent thread is in flight (LCV-080).
    /// The Send button is disabled and a spinner is shown when this is `true`.
    pub busy: bool,
    /// Receiver polled every frame; `Some` while a turn is in flight (LCV-080).
    pub rx: Option<Receiver<AgentEvent>>,
    /// The in-flight turn's fence, counters and undo label (ADR 0007 §D8,
    /// amendment 8). `busy` and `rx` stay outside it: they are §D11's
    /// single-writer pair. Re-armed by `arm_turn`.
    pub turn: TurnState,
    /// The conversation so far (LCV-153, ADR 0007 §D16). Starts empty and is
    /// never persisted.
    pub memory: Memory,
    /// `(History::id(), History::revision())` up to which the model has seen
    /// every change; `None` before the first turn (ADR 0007 §D16).
    pub memory_mark: Option<(u64, u64)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default `AgentState` matches every value `App::default()` used to
    /// spell out for these eight fields by hand, including the fence: LCV-136
    /// relies on `TurnFence::default() == TurnFence::new(0)`.
    #[test]
    fn default_matches_the_values_app_default_used_to_spell_out() {
        let state = AgentState::default();
        assert!(!state.panel_open);
        assert!(state.chat.is_empty());
        assert!(state.input_draft.is_empty());
        assert!(!state.busy);
        assert!(state.rx.is_none());
        assert_eq!(state.turn.fence, crate::app::TurnFence::new(0));
        assert_eq!(state.turn.applied, 0);
        assert!(state.turn.label.is_empty());
    }

    /// Three transcript rows and two remembered turns.
    fn talked() -> AgentState {
        use crate::agent::wire::ChatMessage;
        let mut state = AgentState::default();
        for role in ["user", "tool", "assistant"] {
            state.chat.push((role.to_owned(), "text".to_owned()));
        }
        for prompt in ["one", "two"] {
            state.memory.push_turn(vec![
                ChatMessage::user(prompt),
                ChatMessage::assistant("ok"),
            ]);
        }
        state.input_draft = "half typed".to_owned();
        state.panel_open = true;
        state
    }

    /// LCV-150 AC 2 — the clear empties the transcript and the memory and
    /// leaves every other field where it was.
    #[test]
    fn clear_conversation_empties_the_transcript_and_the_memory() {
        let mut state = talked();
        state.memory_mark = Some((1, 7));
        state.clear_conversation();
        assert!(state.chat.is_empty());
        assert!(state.memory.is_empty());
        assert_eq!(state.input_draft, "half typed");
        assert!(state.panel_open);
        assert!(!state.busy);
        assert!(state.rx.is_none());
        assert_eq!(state.memory_mark, Some((1, 7)));
    }

    /// LCV-150 AC 3 — while a turn is in flight the clear does nothing.
    #[test]
    fn clear_conversation_is_a_no_op_while_busy() {
        let mut state = talked();
        state.busy = true;
        state.clear_conversation();
        assert_eq!(state.chat.len(), 3);
        assert_eq!(state.memory.turns().len(), 2);
        assert!(state.busy);
    }
}
