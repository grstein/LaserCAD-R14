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

use super::agent_worker::{ask_ui, run_agent_turn, TurnConfig};
use crate::agent::{prompt, AgentError, AgentEvent};
use crate::app::App;
use crate::io::settings::Settings;
use std::sync::mpsc::{channel, Sender};

/// What the model is told when a foreign commit landed mid-turn (ADR 0007 §D4).
///
/// It says three things on purpose: what happened, that nothing was applied,
/// and that undo is unaffected — so the operator reading the transcript knows
/// the drawing is not in a half-finished state.
pub const AGENT_FENCE_REFUSAL: &str = "The drawing changed outside this turn — someone drew, deleted or selected something since I last looked. Nothing was applied. Undo is unaffected; ask again and I will re-read the drawing.";

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
    /// The label of the turn's history group: `Agent:` plus the prompt.
    pub label: String,
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
    // Owned, never borrowed: the thread outlives this frame (ADR 0007 §D1).
    let config = turn_config(&app.settings);
    let prompt = prompt.to_owned();
    let tx = arm_with_limit(app, &prompt, config.step_limit);
    std::thread::spawn(move || {
        let result = {
            let mut ask = |action| ask_ui(&tx, action);
            run_agent_turn(&prompt, &config, &mut ask)
        };
        match result {
            Ok(reply) => drop(tx.send(AgentEvent::Done(reply))),
            // Nobody is listening — saying so would only surface in a later
            // turn's transcript (ADR 0007 §D2).
            Err(AgentError::Cancelled) => {}
            Err(error) => drop(tx.send(AgentEvent::Failed(error.to_string()))),
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

/// The undo-stack label for a turn: `Agent:` and the prompt, trimmed and cut to
/// [`LABEL_CHARS`] characters with an `…` when it did not fit (AC 10).
///
/// Counted in `char`s, not bytes: the prompt is whatever the operator typed,
/// and slicing a UTF-8 string at byte 40 panics on the first accented word.
fn turn_label(prompt: &str) -> String {
    let trimmed = prompt.trim();
    let mut label = String::from("Agent: ");
    label.extend(trimmed.chars().take(LABEL_CHARS));
    if trimmed.chars().nth(LABEL_CHARS).is_some() {
        label.push('…');
    }
    label
}

#[cfg(test)]
mod tests {
    use super::*;
    // ── AC 13: the fence ───────────────────────────────────────────────────

    /// AC 13 — the revision the turn started at is the revision it expects,
    /// and checking it repeatedly does not drift.
    #[test]
    fn a_matching_revision_passes() {
        let mut fence = TurnFence::new(10);
        assert_eq!(fence.check(10, true), Ok(()));
        assert_eq!(fence.check(10, true), Ok(()));
        assert!(!fence.is_tripped());
    }

    /// AC 13 — a revision that moved without the turn's knowledge is refused
    /// with the ADR 0007 §D4 wording, character for character.
    #[test]
    fn a_foreign_revision_is_refused_with_the_adr_wording() {
        let mut fence = TurnFence::new(10);
        assert_eq!(fence.check(11, true), Err(AGENT_FENCE_REFUSAL.to_string()));
        assert!(AGENT_FENCE_REFUSAL.contains("Nothing was applied"));
        assert!(AGENT_FENCE_REFUSAL.contains("Undo is unaffected"));
    }

    /// AC 13 — the trip is **sticky**: after one mismatch, the revision the
    /// fence used to want is refused too. A fence that recomputed per call
    /// would answer `Ok` on the second line here (mutation (d)).
    #[test]
    fn a_tripped_fence_refuses_even_a_matching_revision() {
        let mut fence = TurnFence::new(10);
        assert!(fence.check(11, true).is_err());
        assert_eq!(fence.check(10, true), Err(AGENT_FENCE_REFUSAL.to_string()));
        assert_eq!(fence.check(11, true), Err(AGENT_FENCE_REFUSAL.to_string()));
        assert!(fence.is_tripped());
    }

    /// AC 13 — `advance` moves the expectation, so the revision the last apply
    /// produced becomes the one the next action requires. Before the advance
    /// that same revision would have tripped the fence.
    #[test]
    fn advance_moves_the_expectation() {
        let mut fence = TurnFence::new(10);
        fence.advance(11);
        assert_eq!(fence.check(11, true), Ok(()));
        assert!(
            fence.check(10, true).is_err(),
            "a rewound revision is still foreign"
        );
    }

    /// LCV-142 AC 9 (ADR 0007 §D14) — the second witness: a matching revision
    /// with no group open trips the fence, and the trip is sticky even once a
    /// group is open again. This is the revision-0 replacement hole.
    #[test]
    fn a_closed_group_trips_the_fence_even_on_a_matching_revision() {
        let mut fence = TurnFence::new(0);
        assert_eq!(fence.check(0, false), Err(AGENT_FENCE_REFUSAL.to_string()));
        assert!(fence.is_tripped());
        assert_eq!(fence.check(0, true), Err(AGENT_FENCE_REFUSAL.to_string()));
    }

    // ── AC 3: arming, and refusing to arm twice ──────────────────────────────

    /// AC 3 — arming records the operator's words, raises the busy flag, parks
    /// a live `Receiver` on `App`, and hands back the `Sender` that feeds it.
    ///
    /// The `Sender` is exercised rather than merely returned: a channel whose
    /// other half never reached `App` would still type-check here.
    #[test]
    fn arm_turn_records_the_user_row_and_arms_a_live_channel() {
        let mut app = App::default();
        let tx = arm_turn(&mut app, "draw a 20 mm square");

        assert_eq!(
            app.agent.chat,
            vec![("user".to_owned(), "draw a 20 mm square".to_owned())]
        );
        assert!(app.agent.busy);
        assert!(app.agent.rx.is_some());
        assert_eq!(app.agent.turn.applied, 0);
        assert_eq!(app.agent.turn.label, "Agent: draw a 20 mm square");
        assert!(app.history.group_open(), "the turn's group is open");

        tx.send(AgentEvent::Done("hi".to_owned()))
            .expect("the returned Sender must reach the Receiver on App");
        match app
            .agent
            .rx
            .as_ref()
            .expect("armed")
            .try_recv()
            .expect("and the event must arrive on the Receiver App is holding")
        {
            AgentEvent::Done(text) => assert_eq!(text, "hi"),
            _ => panic!("the event must arrive intact"),
        }
    }

    /// AC 3 — the fence is armed at the revision the turn starts from, not at
    /// zero. A fence built on `0` would refuse the very first action of any
    /// turn in a session that had ever drawn anything.
    #[test]
    fn arm_turn_anchors_the_fence_on_the_current_revision() {
        let mut app = App::default();
        app.commit(Box::new(crate::document::CreateLine::new(
            crate::geometry::Line::new(
                crate::geometry::Vec2::new(0.0, 0.0),
                crate::geometry::Vec2::new(1.0, 0.0),
            ),
        )));
        let revision = app.history.revision();
        assert_ne!(revision, 0, "the fixture must have moved the revision");

        let _tx = arm_turn(&mut app, "go on");
        assert_eq!(app.agent.turn.fence, TurnFence::new(revision));
        assert_eq!(app.agent.turn.fence.check(revision, true), Ok(()));
    }

    /// AC 3 — a second turn is refused while one is in flight, and refused
    /// *silently*: `start_turn` touches nothing. One `agent.rx` and one fence
    /// mean one turn, and re-arming would strand the running thread.
    ///
    /// Asserted on a busy `App` whose channel was armed by hand, so no thread
    /// and no socket are involved.
    #[test]
    fn start_turn_on_a_busy_app_changes_nothing() {
        let mut app = App::default();
        let _tx = arm_turn(&mut app, "the first prompt");
        let chat = app.agent.chat.clone();
        let fence = app.agent.turn.fence.clone();
        let label = app.agent.turn.label.clone();
        let armed = app.agent.rx.as_ref().map(std::ptr::from_ref);

        start_turn(&mut app, "the second prompt");

        assert_eq!(app.agent.chat, chat, "no row for a turn that never started");
        assert_eq!(
            app.agent.turn.fence, fence,
            "the running turn keeps its fence"
        );
        assert_eq!(app.agent.turn.label, label);
        assert_eq!(
            app.agent.rx.as_ref().map(std::ptr::from_ref),
            armed,
            "the running turn keeps its receiver"
        );
        assert!(app.agent.busy);
    }

    // ── AC 18: the budget is read and clamped here ───────────────────────────

    /// AC 18 / LCV-142 AC 2 — the stored budget is clamped at this read site,
    /// and the armed turn snapshots the clamped value.
    #[test]
    fn the_budget_is_clamped_where_it_is_read() {
        for (stored, expected) in [
            (0, 1),
            (1, 1),
            (12, 12),
            (256, 256),
            (4096, 4096),
            (4097, 4096),
            (u32::MAX, 4096),
        ] {
            let mut app = App::default();
            app.settings.agent_step_budget = stored;
            assert_eq!(effective_step_limit(&app.settings), expected, "{stored}");
            assert_eq!(turn_config(&app.settings).step_limit, expected);
            let _tx = arm_turn(&mut app, "x");
            assert_eq!(app.agent.turn.limit, expected, "stored {stored}");
        }
    }

    /// LCV-142 AC 11 — the config is a snapshot: built from a budget of 7, it
    /// still says 7 after the settings change to 9, and so does the armed
    /// turn's limit.
    #[test]
    fn the_turn_config_is_a_turn_start_snapshot() {
        let mut app = App::default();
        app.settings.agent_step_budget = 7;
        app.settings.agent_model = "m/one".to_owned();
        let config = turn_config(&app.settings);
        let _tx = arm_with_limit(&mut app, "x", config.step_limit);

        app.settings.agent_step_budget = 9;
        app.settings.agent_model = "m/two".to_owned();

        assert_eq!(config.step_limit, 7);
        assert_eq!(config.model, "m/one");
        assert_eq!(app.agent.turn.limit, 7);
        assert_eq!(turn_config(&app.settings).step_limit, 9, "the next turn");
    }

    /// LCV-143 AC 5 — the prompt is resolved once, into the config: built
    /// with override A, the config still carries A verbatim after the
    /// settings move to B, and later configs carry B, then the default.
    #[test]
    fn the_turn_config_snapshots_the_resolved_prompt() {
        let mut app = App::default();
        let a = "  prompt A\n\twith edges  ";
        app.settings.agent_system_prompt = Some(a.to_owned());
        let first = turn_config(&app.settings);

        app.settings.agent_system_prompt = Some("prompt B".to_owned());
        let second = turn_config(&app.settings);
        app.settings.agent_system_prompt = None;
        let third = turn_config(&app.settings);

        assert_eq!(first.system_prompt, a, "the armed turn keeps A verbatim");
        assert_eq!(second.system_prompt, "prompt B");
        assert_eq!(third.system_prompt, crate::agent::DEFAULT_PROMPT);
        app.settings.agent_system_prompt = Some(String::new());
        assert_eq!(
            turn_config(&app.settings).system_prompt,
            "",
            "blank is kept"
        );
    }

    /// AC 18 — the settings module knows nothing about the agent. The clamp
    /// belongs to the reader, not to the store: `io/settings.rs` deserialises
    /// whatever the JSON says and hands it over unjudged, which is why the
    /// clamp above has to exist.
    #[test]
    fn the_settings_module_does_not_import_the_agent() {
        let src = include_str!("../io/settings.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("settings.rs must have a bare #[cfg(test)] marker");
        let implementation = &src[..at];
        assert!(
            implementation.contains(concat!("pub agent_step", "_budget: u32")),
            "positive control: settings must still hold the stored budget"
        );
        let needle = concat!("crate::", "agent");
        assert!(
            concat!("use crate::", "agent::clamp_step_budget;").contains(needle),
            "control: the needle must match a real import"
        );
        let hit = implementation
            .lines()
            .find(|l| l.contains(needle) && !l.trim_start().starts_with("//"));
        assert!(
            hit.is_none(),
            "AC 18: io/settings.rs must not reach into the agent: {hit:?}"
        );
    }

    // ── AC 10: the undo label ────────────────────────────────────────────────

    /// AC 10 — the label is `Agent:` plus the trimmed prompt, cut at 40
    /// characters with an `…` only when something was actually cut.
    #[test]
    fn the_turn_label_trims_and_truncates_at_forty_characters() {
        assert_eq!(turn_label("  draw a square  "), "Agent: draw a square");
        // Exactly 40 characters: kept whole, no ellipsis.
        let forty = "a".repeat(40);
        assert_eq!(turn_label(&forty), format!("Agent: {forty}"));
        // Forty-one: forty kept, one ellipsis.
        let forty_one = "b".repeat(41);
        assert_eq!(
            turn_label(&forty_one),
            format!("Agent: {}…", "b".repeat(40))
        );
    }

    /// AC 10 — the cut counts `char`s, not bytes. Slicing this prompt at byte
    /// 40 lands inside a multi-byte character and panics.
    #[test]
    fn the_turn_label_cuts_on_character_boundaries() {
        let prompt = "desenhe um quadrado de vinte milímetros no canto";
        let label = turn_label(prompt);
        assert!(label.ends_with('…'), "{label}");
        assert_eq!(label.chars().count(), "Agent: ".len() + 40 + 1);
        assert!(
            label.starts_with("Agent: desenhe um quadrado de vinte milí"),
            "{label}"
        );
    }

    /// AC 12 — arming, spawning and finishing a turn open no dialog and block
    /// nothing. The operator keeps drawing while the agent works.
    ///
    /// Every `*_open` flag on `App` is a modal or a panel; a turn that set one
    /// would take the canvas away, which ADR 0007 §D4 names a non-goal. The
    /// scan covers both files that run turn code.
    #[test]
    fn no_turn_function_opens_a_dialog() {
        let witness = "app.agent_settings_open = true; app.agent.panel_open = false;";
        for (name, src) in [
            ("agent_turn.rs", include_str!("agent_turn.rs")),
            ("agent_worker.rs", include_str!("agent_worker.rs")),
            ("agent_poll.rs", include_str!("agent_poll.rs")),
        ] {
            let at = src
                .find("\n#[cfg(test)]")
                .expect("a bare #[cfg(test)] marker");
            let needle = concat!("_open", " =");
            assert!(
                witness.contains(needle),
                "control: `{needle}` must match a real write"
            );
            let hit = src[..at]
                .lines()
                .find(|l| l.contains(needle) && !l.trim_start().starts_with("//"));
            assert!(
                hit.is_none(),
                "AC 12: {name} must not write a dialog flag: {hit:?}"
            );
        }
    }

    /// AC 15 / ADR 0007 §D4 — the fence stays where the thread cannot reach
    /// it, and this file stays out of the UI toolkit.
    ///
    /// Bounded at the bare `#[cfg(test)]` at column 0, needles built with
    /// `concat!`. The positive control names the one declaration that must be
    /// here, so a mis-sliced haystack fails instead of passing vacuously.
    #[test]
    fn the_fence_is_declared_here_and_this_file_imports_no_ui() {
        let src = include_str!("agent_turn.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("agent_turn.rs must have a bare #[cfg(test)] marker");
        let implementation = &src[..at];

        assert!(
            implementation.contains(concat!("pub struct ", "TurnFence")),
            "positive control: the fence must be declared in this file"
        );
        for forbidden in [
            concat!("e", "frame"),
            concat!("r", "fd"),
            concat!("use e", "gui"),
        ] {
            let hit = implementation
                .lines()
                .find(|l| l.contains(forbidden) && !l.trim_start().starts_with("//"));
            assert!(
                hit.is_none(),
                "agent_turn.rs must not name `{forbidden}`: {hit:?}"
            );
        }
    }
}
