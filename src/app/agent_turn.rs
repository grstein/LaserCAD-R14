//! LCV-122 / LCV-123 — the turn's UI-side driver: [`TurnFence`],
//! [`arm_turn`] and [`start_turn`]. The worker half is `agent_worker.rs`.
//!
//! ## Why the fence is here and not in `src/agent/bridge.rs`
//!
//! ADR 0007 §D4 puts `TurnFence` in this file on purpose. `bridge.rs` is the
//! protocol, and the protocol is visible to the worker thread; the fence is the
//! one value in this design the worker thread must never evaluate, because
//! evaluating it on the wrong side of the channel is exactly the stale-read bug
//! it exists to prevent.
//!
//! ## What the fence is for
//!
//! `History::revision()` is a monotonic counter that the UI thread is the only
//! writer of (ADR 0002 §B), so it is a complete sequencer of every mutation in
//! the program. A turn records the revision it started at; before each action
//! it checks that the revision is still the one it expects. If the operator
//! drew, deleted or selected something in the meantime, the model's picture of
//! the drawing is stale — its index 7 may now be a different entity, which
//! nothing else in the system can detect — so the action is refused and the
//! turn ends. Already-applied actions stay applied and stay undoable.
//!
//! A tripped fence is **sticky**: it does not re-arm, not even after a query
//! that would give the model a fresh read. That is what keeps the turn one
//! undo entry (ADR 0007 §D12, §D14) — a re-armed fence would commit after the
//! foreign event sealed the turn's group, and the turn would split around it.
//!
//! [`arm_turn`] constructs the fence and opens the turn's history group at
//! turn start; `agent_poll` checks the fence once per action and seals the
//! group once per turn. Those call sites sit there rather than here because
//! they are frame-loop work, and ADR 0007 §D8 gives `agent_poll` the drain,
//! the dispatch and the answer; what this file owns is the fence itself.
//!
//! ## What crosses the thread boundary
//!
//! [`start_turn`] is the only place a turn's thread is spawned. The prompt, one
//! owned `TurnConfig` (§D13) and one `Sender` cross into it — no `Document`,
//! no `History`, no `App`, nothing borrowed (ADR 0007 §D1). What runs there is
//! `agent_worker::run_agent_turn`, which asks the UI thread through `ask_ui`.

use super::agent_worker::{TurnConfig, ask_ui, run_agent_turn};
use crate::agent::{AgentError, AgentEvent, prompt};
use crate::app::{App, agent_memory};
use crate::io::settings::Settings;
use std::sync::mpsc::{Sender, channel};

/// What the model is told when a foreign commit landed mid-turn (ADR 0007 §D4).
///
/// It says three things on purpose: what happened, that nothing was applied,
/// and that undo is unaffected — so the operator reading the transcript knows
/// the drawing is not in a half-finished state.
pub const AGENT_FENCE_REFUSAL: &str = "The drawing changed outside this turn - someone drew, deleted or selected something since I last looked. Nothing was applied. Undo is unaffected; ask again and I will re-read the drawing.";

/// Guards one agent turn against mutations it did not make.
///
/// Two pieces of state and no borrow of anything: `expected` is the revision
/// the next action requires, and `tripped` records that the guard has already
/// fired.
///
/// `Default` derives to `{ expected: 0, tripped: false }`, exactly
/// [`TurnFence::new(0)`](TurnFence::new) — which is what lets
/// [`TurnState`] and [`AgentState`](crate::app::AgentState) derive `Default`
/// too (LCV-136).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TurnFence {
    expected: u64,
    tripped: bool,
}

impl TurnFence {
    /// Arm a fence at the revision the turn is starting from.
    pub fn new(start_revision: u64) -> Self {
        Self {
            expected: start_revision,
            tripped: false,
        }
    }

    /// Is `current` still the revision this turn expects, and is the turn's
    /// history group still open (ADR 0007 §D14)?
    ///
    /// The second witness catches what the revision alone cannot: document
    /// replacement restarts `History` at revision 0, so a turn armed at 0 would
    /// otherwise read `0 == 0` against a different file. Every seal and every
    /// replacement leaves no group open.
    ///
    /// # Errors
    ///
    /// [`AGENT_FENCE_REFUSAL`] when the revision moved without this turn's
    /// knowledge or the group is gone — and on **every** later call, whatever
    /// the arguments, because the trip is sticky.
    pub fn check(&mut self, current: u64, group_open: bool) -> Result<(), String> {
        if self.tripped || current != self.expected || !group_open {
            self.tripped = true;
            return Err(AGENT_FENCE_REFUSAL.to_string());
        }
        Ok(())
    }

    /// Move the expectation to `current` after a successful apply, where
    /// `current` is `expected + 1`.
    pub fn advance(&mut self, current: u64) {
        self.expected = current;
    }

    /// Has this fence refused an action? Once true, always true.
    pub fn is_tripped(&self) -> bool {
        self.tripped
    }
}

/// The in-flight turn's UI-side bookkeeping, reached as `app.agent.turn`
/// (ADR 0007 §D8, amendments 7 and 8). Re-armed whole by [`arm_turn`];
/// meaningless while `agent.busy` is false.
#[derive(Debug, Default)]
pub struct TurnState {
    /// Guards the turn against commits it did not make (ADR 0007 §D4, §D14).
    pub fence: TurnFence,
    /// How many of the turn's actions really changed the drawing. Taken once
    /// at turn end by the note row (LCV-142 AC 12).
    pub applied: usize,
    /// Step `Act`s received this turn (ADR 0007 §D13) — the `n` of the
    /// panel's `n of limit`.
    pub steps: u32,
    /// The turn's effective step limit, snapshotted when it was armed.
    pub limit: u32,
    /// The label of the turn's history group: `AI:` plus the prompt.
    pub label: String,
    /// The turn's user message as the model gets it: the prompt, prefixed
    /// when the drawing changed since the last turn (LCV-153 AC 7).
    pub user: String,
}

/// The longest prompt prefix an undo label carries (AC 10).
const LABEL_CHARS: usize = 40;

/// Put `app` into a turn and hand back the `Sender` the turn will report on.
///
/// Everything a turn needs before anything can arrive: the `user` row, the busy
/// flag, the channel, a fresh [`TurnState`] whose fence is armed at the
/// current revision, and the turn's history group, opened under the undo
/// label (ADR 0007 §D12) — every agent commit lands in it.
///
/// `pub` rather than `pub(crate)` because it is the seam the integration tests
/// drive: they arm a turn, keep the `Sender`, and push events into it by hand
/// instead of standing up a thread and a socket. That makes every assertion
/// about applying, fencing, grouping and ending a turn deterministic.
pub fn arm_turn(app: &mut App, prompt: &str) -> Sender<AgentEvent> {
    let limit = effective_step_limit(&app.settings);
    arm_with_limit(app, prompt, limit)
}

/// [`arm_turn`] with the limit the caller already snapshotted, so the panel's
/// `of {limit}` and the worker's [`TurnConfig`] can never disagree.
fn arm_with_limit(app: &mut App, prompt: &str, limit: u32) -> Sender<AgentEvent> {
    let (tx, rx) = channel::<AgentEvent>();
    // Before the group opens: the mark is the history the model last saw.
    let user = agent_memory::begin(app, prompt);
    app.agent.chat.push(("user".to_owned(), prompt.to_owned()));
    app.agent.busy = true;
    app.agent.rx = Some(rx);
    let label = turn_label(prompt);
    app.history.begin_group(&label);
    app.agent.turn = TurnState {
        fence: TurnFence::new(app.history.revision()),
        applied: 0,
        steps: 0,
        limit,
        label,
        user,
    };
    tx
}

/// Arm a turn and spawn the thread that runs it. The panel's Send button and
/// (from LCV-124) the command line both land here.
///
/// A no-op while a turn is in flight: one `agent.rx` and one fence mean one
/// turn (ADR 0007 §Revisit criteria), and a second `arm_turn` would drop the
/// first turn's receiver on the floor and strand its thread.
pub fn start_turn(app: &mut App, prompt: &str) {
    if app.agent.busy {
        return;
    }
    // Armed first, so the config carries the memory `begin` just trimmed.
    let tx = arm_with_limit(app, prompt, effective_step_limit(&app.settings));
    // Owned, never borrowed: the thread outlives this frame (ADR 0007 §D1).
    let config = config_for(app);
    let prompt = app.agent.turn.user.clone();
    std::thread::spawn(move || {
        let (result, batches) = {
            let mut ask = |action| ask_ui(&tx, action);
            run_agent_turn(&prompt, &config, &mut ask)
        };
        match result {
            Ok(reply) => drop(tx.send(AgentEvent::Done(reply, batches))),
            // Nobody is listening — saying so would only surface in a later
            // turn's transcript (ADR 0007 §D2).
            Err(AgentError::Cancelled) => {}
            Err(error) => drop(tx.send(AgentEvent::Failed(error.to_string(), batches))),
        }
    });
}

/// The turn-start snapshot of `settings` that crosses into the worker (ADR
/// 0007 §D13): cloned once, so a mid-turn edit reaches the next turn only.
/// The system prompt is resolved here too (LCV-143 AC 5), so the worker never
/// sees the override/default distinction.
fn turn_config(settings: &Settings) -> TurnConfig {
    TurnConfig {
        endpoint: settings.agent_endpoint.clone(),
        api_key: settings.agent_api_key.clone(),
        model: settings.agent_model.clone(),
        step_limit: effective_step_limit(settings),
        system_prompt: prompt::resolve(settings.agent_system_prompt.as_deref()).to_owned(),
        vision: settings.agent_allow_canvas_capture && settings.agent_model_supports_vision,
        memory: Vec::new(),
    }
}

/// The [`TurnConfig`] for the turn `app` has just armed (LCV-153): its
/// settings snapshot and a clone of the conversation memory.
pub fn config_for(app: &App) -> TurnConfig {
    TurnConfig {
        memory: app.agent.memory.flatten(),
        ..turn_config(&app.settings)
    }
}

/// The step budget a turn may spend, clamped **at the read site**.
///
/// ADR 0007 §D7/§D13: `settings.agent_step_budget` comes from a hand-editable
/// JSON file, so `0` and `5000` are both things a real file can say. Clamping
/// here, once, is what lets everything downstream treat the number as sane.
fn effective_step_limit(settings: &Settings) -> u32 {
    crate::agent::clamp_step_budget(settings.agent_step_budget)
}

/// The undo-stack label for a turn: `AI:` and the prompt, trimmed and cut to
/// [`LABEL_CHARS`] characters with an `…` when it did not fit (AC 10).
///
/// Counted in `char`s, not bytes: the prompt is whatever the operator typed,
/// and slicing a UTF-8 string at byte 40 panics on the first accented word.
fn turn_label(prompt: &str) -> String {
    let trimmed = prompt.trim();
    let mut label = String::from("AI: ");
    label.extend(trimmed.chars().take(LABEL_CHARS));
    if trimmed.chars().nth(LABEL_CHARS).is_some() {
        label.push('…');
    }
    label
}

#[cfg(test)]
mod tests;
