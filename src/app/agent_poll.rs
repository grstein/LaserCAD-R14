//! LCV-080 — Testable helper for polling the agent background channel.
//!
//! Extracted from `App::update` so that AC#13 and AC#14 unit tests can drive
//! the polling logic headlessly without an egui context.

use crate::app::App;

/// Drain one message from `app.agent_rx` (if any) and update `agent_chat`,
/// `agent_busy`, and `agent_rx` accordingly.
///
/// Called once per frame at the top of `App::update`, before any panel is
/// drawn. When the channel is empty or `agent_rx` is `None`, this is a no-op.
pub fn poll_agent_rx(app: &mut App) {
    if let Some(rx) = &app.agent_rx {
        if let Ok(msg) = rx.try_recv() {
            match msg {
                crate::agent::AgentPanelMsg::Reply(text) => {
                    app.agent_chat.push(("assistant".into(), text));
                }
                crate::agent::AgentPanelMsg::Error(e) => {
                    app.agent_chat.push(("error".into(), e));
                }
            }
            app.agent_busy = false;
            app.agent_rx = None;
        }
    }
}
