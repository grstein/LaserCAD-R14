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

use crate::agent::AgentEvent;
use crate::app::TurnFence;

/// The agent's UI-side state: the chat transcript, the in-flight turn's
/// channel and fence, the applied-action counter and undo label, and the
/// panel's own visibility and input draft.
///
/// `Default` matches what `App::default()` used to spell out field by field:
/// an empty transcript, no receiver, `busy == false`, and
/// [`TurnFence::default()`] equal to `TurnFence::new(0)`.
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
    /// Guards the in-flight turn against commits it did not make (ADR 0007
    /// §D4). Re-armed by `arm_turn`; meaningless while `busy` is false.
    pub fence: TurnFence,
    /// How many of the in-flight turn's actions really changed the drawing.
    /// Read once at turn end by the coalesce and the note row (AC 10, AC 11).
    pub applied: usize,
    /// The undo-stack label a coalesced turn gets: `Agent:` plus the prompt.
    pub turn_label: String,
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
        assert_eq!(state.fence, TurnFence::new(0));
        assert_eq!(state.applied, 0);
        assert!(state.turn_label.is_empty());
    }
}
