//! The conversation's two UI-side moments (LCV-153, ADR 0007 §D16).
//!
//! [`begin`] runs when a turn is armed: it trims memory to the configured
//! window, decides whether the drawing changed since the model last saw it,
//! and stamps the mark. [`record`] runs once the turn has ended: it appends
//! the turn and, only on `Done`, advances the mark to the live history.

use crate::agent::memory::{clamp_context_tokens, turn_record, with_changed_prefix};
use crate::agent::{ChatMessage, TurnEnd};
use crate::app::App;

/// `(History::id(), History::revision())` of the live document.
fn live_mark(app: &App) -> (u64, u64) {
    (app.history.id(), app.history.revision())
}

/// Trim memory, stamp the mark and return the user message the model sees:
/// `prompt` itself, or `prompt` behind the drawing-changed line when memory
/// is non-empty and the live history moved since the mark.
pub(crate) fn begin(app: &mut App, prompt: &str) -> String {
    let context = clamp_context_tokens(app.settings.agent_context_tokens);
    app.agent.memory.trim(context);
    let live = live_mark(app);
    let changed = !app.agent.memory.is_empty() && app.agent.memory_mark != Some(live);
    app.agent.memory_mark = Some(live);
    if changed {
        with_changed_prefix(prompt)
    } else {
        prompt.to_owned()
    }
}

/// Append the ended turn to memory. Only `Done` advances the mark: after a
/// failed or cancelled turn the model has not seen its own applied actions
/// described, so the next turn is told the drawing changed.
pub(crate) fn record(app: &mut App, end: TurnEnd, batches: Vec<ChatMessage>) {
    let user = std::mem::take(&mut app.agent.turn.user);
    app.agent
        .memory
        .push_turn(turn_record(&user, batches, &end));
    if matches!(end, TurnEnd::Done { .. }) {
        app.agent.memory_mark = Some(live_mark(app));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::memory::DRAWING_CHANGED_PREFIX;

    #[test]
    fn begin_prefixes_only_with_memory_and_a_moved_mark() {
        let mut app = App::default();
        assert_eq!(begin(&mut app, "a"), "a");
        assert_eq!(app.agent.memory_mark, Some(live_mark(&app)));
        app.agent.turn.user = "a".into();
        record(&mut app, TurnEnd::Done { text: "ok".into() }, Vec::new());
        assert_eq!(begin(&mut app, "b"), "b");
        app.agent.memory_mark = Some((app.history.id(), app.history.revision() + 1));
        let prefixed = begin(&mut app, "c");
        assert_eq!(prefixed, format!("{DRAWING_CHANGED_PREFIX}\nc"));
    }

    #[test]
    fn record_advances_the_mark_only_on_done() {
        let mut app = App::default();
        app.agent.memory_mark = Some((0, 0));
        record(&mut app, TurnEnd::Cancelled, Vec::new());
        let stopped = TurnEnd::Stopped { error: "x".into() };
        record(&mut app, stopped, Vec::new());
        assert_eq!(app.agent.memory_mark, Some((0, 0)));
        record(&mut app, TurnEnd::Done { text: "ok".into() }, Vec::new());
        assert_eq!(app.agent.memory_mark, Some(live_mark(&app)));
        assert_eq!(app.agent.memory.turns().len(), 3);
    }
}
