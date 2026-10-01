//! LCV-200 — one agent turn run on the calling thread, for the evaluation
//! bench: armed like any turn, driven by the worker's own loop with an
//! injected `send_fn`, every action answered by the frame loop's own apply
//! path, and ended through the one turn end (ADR 0007 §D11). No thread, no
//! channel, no window.

use crate::agent::{AgentError, AssistantMessage, ChatMessage, TurnEnd, TurnMetrics};
use crate::app::agent_poll::{answer_act, end_turn};
use crate::app::agent_worker::drive_turn;
use crate::app::{App, TurnConfig, arm_turn};
use std::time::{Duration, Instant};

/// What [`run_turn_inline`] reports about the turn it ran.
#[derive(Debug)]
pub struct InlineTurn {
    /// The turn's final text, or why it stopped.
    pub result: Result<String, AgentError>,
    /// The turn's counts (LCV-193), read after the turn ended.
    pub metrics: TurnMetrics,
    /// Wall time from arming to the end of the turn.
    pub wall: Duration,
}

/// Arm a turn on `app` for `prompt` and run it to its end on this thread.
///
/// `send_fn` stands in for the endpoint: it gets the conversation and
/// answers with the model's next message. `config` is the turn's snapshot;
/// its `memory` and `image` ride as in a real turn. Every action goes
/// through the same `answer_act` the frame loop calls, so the fence, the
/// group, the tally and the verify bookkeeping all apply.
pub fn run_turn_inline<F>(
    app: &mut App,
    prompt: &str,
    config: &TurnConfig,
    send_fn: &mut F,
) -> InlineTurn
where
    F: FnMut(&[ChatMessage]) -> Result<AssistantMessage, AgentError>,
{
    let start = Instant::now();
    // The channel is never read: the turn end drops its receiver.
    drop(arm_turn(app, prompt));
    let user = app.agent.turn.user.clone();
    let (result, batches) = {
        let mut ask = |action| Ok(answer_act(app, &action));
        drive_turn(&user, config, send_fn, &mut ask)
    };
    let (row, end) = match &result {
        Ok(text) => (
            ("assistant", text.clone()),
            TurnEnd::Done { text: text.clone() },
        ),
        Err(error) => {
            let error = error.to_string();
            (("error", error.clone()), TurnEnd::Stopped { error })
        }
    };
    end_turn(app, Some(row), end, batches);
    InlineTurn {
        result,
        metrics: app.agent.turn.tally,
        wall: start.elapsed(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::ToolCall;
    use crate::document::Entity;

    /// LCV-200 AC 2 — two scripted replies, a line plus its check and then
    /// the text, draw the line on a fresh document through the real path:
    /// one undo group, a Done turn, and the counts the loop recorded.
    #[test]
    fn a_scripted_turn_draws_a_line_through_the_real_apply_path() {
        let mut app = App::default();
        let config = crate::app::config_for(&app);
        let line = r#"{"x1": 0, "y1": 0, "x2": 10, "y2": 0}"#;
        let mut replies = vec![
            AssistantMessage {
                content: None,
                tool_calls: Some(vec![
                    ToolCall::function("a", "create_line", line),
                    ToolCall::function("b", "check_drawing", "{}"),
                ]),
                reasoning_content: None,
            },
            AssistantMessage {
                content: Some("Drew it.".to_owned()),
                tool_calls: None,
                reasoning_content: None,
            },
        ]
        .into_iter();
        let mut send = |_: &[ChatMessage]| replies.next().ok_or(AgentError::Cancelled);

        let turn = run_turn_inline(&mut app, "draw a line", &config, &mut send);

        assert_eq!(turn.result.ok().as_deref(), Some("Drew it."));
        assert!(matches!(app.document.entities[..], [Entity::Line(_)]));
        let want = TurnMetrics {
            steps: 2,
            applied: 1,
            replies: 2,
            ..TurnMetrics::default()
        };
        assert_eq!(turn.metrics, want);
        assert!(!app.agent.busy && app.agent.rx.is_none(), "the turn ended");
        assert!(!app.history.group_open(), "and its group was sealed");
        assert_eq!(app.history.len(), 1, "one undo entry");
    }
}
