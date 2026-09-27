//! LCV-122 / LCV-123 — the turn's UI-side driver: [`TurnFence`],
//! [`arm_turn`], [`start_turn`] and [`run_agent_turn`].
//!
//! ## Why the fence is here and not in `src/agent/bridge.rs`
//!
//! ADR 0007 §D4 puts `TurnFence` in this file on purpose. `bridge.rs` is the
//! protocol, and the protocol is visible to the worker thread; the fence is the
//! one value in this design the worker thread must never evaluate, because
//! evaluating it on the wrong side of the channel is exactly the stale-read bug
//! it exists to prevent. §D6's coalesce gate is the same revision arithmetic,
//! so it lives in the same type.
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
//! that would give the model a fresh read. That is what keeps the coalesce gate
//! sound — once tripped the agent commits nothing more, so the entries the gate
//! wants to fold stay contiguous at the top of the undo stack.
//!
//! [`arm_turn`] constructs the fence at turn start; `agent_poll` checks it once
//! per action and reads its coalesce gate once per turn. The two call sites sit
//! there rather than here because they are frame-loop work, and ADR 0007 §D8
//! gives `agent_poll` the drain, the dispatch and the answer; what this file
//! owns is the fence itself and the arithmetic it answers with.
//!
//! ## What crosses the thread boundary
//!
//! [`start_turn`] is the only place a turn's thread is spawned. Four owned
//! `String`s, one `u8` and one `Sender` cross into it — no `Document`, no
//! `History`, no `App`, nothing borrowed (ADR 0007 §D1). [`run_agent_turn`] is
//! then control flow only: it knows how to talk to an endpoint and how to feed
//! a tool result back, and it learns what a tool call *did* by asking, through
//! the `ask` seam, whoever owns the drawing. In the app that is [`ask_ui`],
//! which blocks on the UI thread's answer; in a test it is a closure.

use crate::agent::{
    agent_loop, AgentAction, AgentError, AgentEvent, AgentOutcome, ChatMessage, AGENT_SYSTEM_PROMPT,
};
use crate::app::App;
use std::sync::mpsc::{channel, Sender};

/// What the model is told when a foreign commit landed mid-turn (ADR 0007 §D4).
///
/// It says three things on purpose: what happened, that nothing was applied,
/// and that undo is unaffected — so the operator reading the transcript knows
/// the drawing is not in a half-finished state.
pub const AGENT_FENCE_REFUSAL: &str = "The drawing changed outside this turn — someone drew, deleted or selected something since I last looked. Nothing was applied. Undo is unaffected; ask again and I will re-read the drawing.";

/// Guards one agent turn against mutations it did not make.
///
/// Three `u64`-sized pieces of state and no borrow of anything: `start` is the
/// revision the turn began at, `expected` is the revision the next action
/// requires, and `tripped` records that the guard has already fired.
///
/// `Default` derives to `{ start: 0, expected: 0, tripped: false }`, exactly
/// [`TurnFence::new(0)`](TurnFence::new) — which is what lets
/// [`AgentState`](crate::app::AgentState) derive `Default` too (LCV-136).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TurnFence {
    start: u64,
    expected: u64,
    tripped: bool,
}

impl TurnFence {
    /// Arm a fence at the revision the turn is starting from.
    pub fn new(start_revision: u64) -> Self {
        Self {
            start: start_revision,
            expected: start_revision,
            tripped: false,
        }
    }

    /// Is `current` still the revision this turn expects?
    ///
    /// # Errors
    ///
    /// [`AGENT_FENCE_REFUSAL`] when the revision moved without this turn's
    /// knowledge — and on **every** later call, whatever `current` is, because
    /// the trip is sticky.
    pub fn check(&mut self, current: u64) -> Result<(), String> {
        if self.tripped || current != self.expected {
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

    /// May the turn's `n` commits be folded into one undo entry?
    ///
    /// ADR 0007 §D6 step 3: only when the revision advanced by exactly `n`
    /// since the turn started, which proves the top `n` entries of the undo
    /// stack are contiguously this turn's and nobody else's. `n < 2` is not
    /// worth folding — a composite of one only relabels it.
    pub fn may_coalesce(&self, current: u64, n: usize) -> bool {
        !self.tripped && n >= 2 && current.saturating_sub(self.start) == n as u64
    }
}

/// The longest prompt prefix an undo label carries (AC 10).
const LABEL_CHARS: usize = 40;

/// Drive one complete agent turn: send `prompt` to `endpoint` as `model`, ask
/// `ask` to carry out every tool call the model requests, return its final
/// text.
///
/// Owns no document state of any kind (ADR 0007 §D1). `ask` is the whole seam:
/// it receives one [`AgentAction`] and answers with what really happened to the
/// real drawing. In the app that answer comes from another thread; here it is
/// just a call.
///
/// `model` and `step_budget` are the caller's to choose — they come from
/// `Settings`, and the budget must already have been through
/// [`crate::agent::clamp_step_budget`]. At most `step_budget` actions are
/// applied per turn.
///
/// # Errors
///
/// [`AgentError`] for a transport failure, a malformed tool call, an exhausted
/// step budget, a reply carrying neither text nor tool calls, or
/// [`AgentError::Cancelled`] when `ask` reports that nobody is left to answer.
/// An action the document *refuses* is **not** an error: the refusal goes back
/// to the model as the tool result and the turn continues (ADR 0007 §D2a).
pub fn run_agent_turn<A>(
    prompt: &str,
    endpoint: &str,
    api_key: &str,
    model: &str,
    step_budget: u8,
    ask: &mut A,
) -> Result<String, AgentError>
where
    A: FnMut(AgentAction) -> Result<AgentOutcome, AgentError>,
{
    let mut messages = vec![
        ChatMessage::system(AGENT_SYSTEM_PROMPT),
        ChatMessage::user(prompt),
    ];
    // Built once; every round offers the model the same schemas.
    let tools = crate::agent::tool_definitions();
    let mut send_fn = |msgs: &[ChatMessage]| {
        // The slice goes through untouched: filtering it would drop the
        // assistant turns that carry tool calls and the tool turns that answer
        // them, leaving holes in the conversation the model reads back.
        crate::agent::chat_completion(endpoint, api_key, model, msgs, &tools)
            .map_err(|e| AgentError::Transport(e.to_string()))
    };
    let mut dispatch_fn = |name: &str, args: &str| {
        // The argument-free queries are routinely called with `""` rather than
        // `"{}"`, which is not JSON; both mean the same empty object here.
        let value = if args.trim().is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_str::<serde_json::Value>(args)
                .map_err(|e| AgentError::ToolDispatch(e.to_string()))?
        };
        let action = crate::agent::parse_tool_call(name, &value)
            .map_err(|e| AgentError::ToolDispatch(e.to_string()))?;
        // A refusal is a tool result, not a failure (ADR 0007 §D2a).
        Ok(ask(action)?.into_text())
    };
    agent_loop(&mut send_fn, &mut dispatch_fn, &mut messages, step_budget)
}

/// The worker thread's `ask`: one rendezvous with the UI thread (ADR 0007 §D2).
///
/// A fresh one-shot channel per action, so the reply `Sender` rides inside the
/// message and nothing is ever parked on `App` (§D3). Both failures mean the
/// same thing — the UI is gone, or has abandoned this turn — and both are
/// [`AgentError::Cancelled`], on which the caller returns without sending a
/// terminal event, because there is nobody left to read one.
fn ask_ui(tx: &Sender<AgentEvent>, action: AgentAction) -> Result<AgentOutcome, AgentError> {
    let (reply, answer) = channel::<AgentOutcome>();
    tx.send(AgentEvent::Act { action, reply })
        .map_err(|_| AgentError::Cancelled)?;
    answer.recv().map_err(|_| AgentError::Cancelled)
}

/// Put `app` into a turn and hand back the `Sender` the turn will report on.
///
/// Everything a turn needs before anything can arrive: the `user` row, the busy
/// flag, the channel, a fence armed at the current revision, a zeroed
/// applied-action counter and the undo label the coalesce will use.
///
/// `pub` rather than `pub(crate)` because it is the seam the integration tests
/// drive: they arm a turn, keep the `Sender`, and push events into it by hand
/// instead of standing up a thread and a socket. That makes every assertion
/// about applying, fencing, coalescing and ending a turn deterministic.
pub fn arm_turn(app: &mut App, prompt: &str) -> Sender<AgentEvent> {
    let (tx, rx) = channel::<AgentEvent>();
    app.agent.chat.push(("user".to_owned(), prompt.to_owned()));
    app.agent.busy = true;
    app.agent.rx = Some(rx);
    app.agent.fence = TurnFence::new(app.history.revision());
    app.agent.applied = 0;
    app.agent.turn_label = turn_label(prompt);
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
    // Cloned, never borrowed: the thread outlives this frame (ADR 0007 §D1).
    let endpoint = app.settings.agent_endpoint.clone();
    let api_key = app.settings.agent_api_key.clone();
    let model = app.settings.agent_model.clone();
    let step_budget = turn_step_budget(app);
    let prompt = prompt.to_owned();
    let tx = arm_turn(app, &prompt);
    std::thread::spawn(move || {
        let result = {
            let mut ask = |action| ask_ui(&tx, action);
            run_agent_turn(&prompt, &endpoint, &api_key, &model, step_budget, &mut ask)
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

/// The step budget this turn may spend, clamped **at the read site**.
///
/// ADR 0007 §D7: `settings.agent_step_budget` comes from a hand-editable JSON
/// file, so `0` and `200` are both things a real file can say. Clamping here,
/// once, is what lets everything downstream treat the number as sane.
fn turn_step_budget(app: &App) -> u8 {
    crate::agent::clamp_step_budget(app.settings.agent_step_budget)
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
    use crate::agent::AGENT_STEP_BUDGET_DEFAULT;
    use crate::app::agent_apply;
    use serde_json::{json, Value};
    use std::sync::{Arc, Mutex};

    /// The `ask` seam, wired to a real `App`, recording what it was asked.
    ///
    /// This is the shape ADR 0007 §D1 buys: the turn is exercised end to end
    /// against a live document with no thread, no channel and no socket
    /// between them, because the only thing the turn knows how to do with an
    /// action is hand it to whoever owns the drawing.
    struct Applier {
        app: App,
        seen: Vec<AgentAction>,
        cancel: bool,
    }

    impl Applier {
        fn new() -> Self {
            // ADR 0002 §A2: `App::default()` never touches the developer's
            // real settings or autosave paths.
            Self {
                app: App::default(),
                seen: Vec::new(),
                cancel: false,
            }
        }

        /// Report, on every ask, that nobody is left to answer (AC 5).
        fn cancelling() -> Self {
            Self {
                cancel: true,
                ..Self::new()
            }
        }

        fn ask(&mut self, action: AgentAction) -> Result<AgentOutcome, AgentError> {
            self.seen.push(action.clone());
            if self.cancel {
                return Err(AgentError::Cancelled);
            }
            Ok(agent_apply::apply(&mut self.app, &action))
        }

        fn entities(&self) -> usize {
            self.app.document.entity_count()
        }
    }

    // ── AC 13: the fence ───────────────────────────────────────────────────

    /// AC 13 — the revision the turn started at is the revision it expects,
    /// and checking it repeatedly does not drift.
    #[test]
    fn a_matching_revision_passes() {
        let mut fence = TurnFence::new(10);
        assert_eq!(fence.check(10), Ok(()));
        assert_eq!(fence.check(10), Ok(()));
        assert!(!fence.is_tripped());
    }

    /// AC 13 — a revision that moved without the turn's knowledge is refused
    /// with the ADR 0007 §D4 wording, character for character.
    #[test]
    fn a_foreign_revision_is_refused_with_the_adr_wording() {
        let mut fence = TurnFence::new(10);
        assert_eq!(fence.check(11), Err(AGENT_FENCE_REFUSAL.to_string()));
        assert!(AGENT_FENCE_REFUSAL.contains("Nothing was applied"));
        assert!(AGENT_FENCE_REFUSAL.contains("Undo is unaffected"));
    }

    /// AC 13 — the trip is **sticky**: after one mismatch, the revision the
    /// fence used to want is refused too. A fence that recomputed per call
    /// would answer `Ok` on the second line here (mutation (d)).
    #[test]
    fn a_tripped_fence_refuses_even_a_matching_revision() {
        let mut fence = TurnFence::new(10);
        assert!(fence.check(11).is_err());
        assert_eq!(fence.check(10), Err(AGENT_FENCE_REFUSAL.to_string()));
        assert_eq!(fence.check(11), Err(AGENT_FENCE_REFUSAL.to_string()));
        assert!(fence.is_tripped());
    }

    /// AC 13 — `advance` moves the expectation, so the revision the last apply
    /// produced becomes the one the next action requires. Before the advance
    /// that same revision would have tripped the fence.
    #[test]
    fn advance_moves_the_expectation() {
        let mut fence = TurnFence::new(10);
        fence.advance(11);
        assert_eq!(fence.check(11), Ok(()));
        assert!(
            fence.check(10).is_err(),
            "a rewound revision is still foreign"
        );
    }

    /// AC 13 / ADR 0007 §D6 step 3 — the gate is an equality, not a
    /// comparison: exactly `n` revisions since the start, nothing else.
    #[test]
    fn may_coalesce_is_an_exact_equality_over_at_least_two_entries() {
        let fence = TurnFence::new(10);
        assert!(fence.may_coalesce(14, 4), "four commits, four revisions");
        assert!(!fence.may_coalesce(14, 3), "a foreign commit hid in there");
        assert!(!fence.may_coalesce(14, 5), "one of them was not ours");
        assert!(!fence.may_coalesce(11, 1), "one entry is not worth folding");
        assert!(!fence.may_coalesce(10, 0), "nothing happened");
        assert!(fence.may_coalesce(12, 2), "two is the smallest fold");
    }

    /// AC 13 — a tripped fence never folds, whatever the arithmetic says.
    /// This is what keeps the agent's entries contiguous (ADR 0007 §D4).
    #[test]
    fn a_tripped_fence_declines_to_coalesce() {
        let mut fence = TurnFence::new(10);
        assert!(fence.may_coalesce(14, 4));
        assert!(fence.check(99).is_err());
        assert!(!fence.may_coalesce(14, 4), "a tripped fence must not fold");
    }

    /// AC 13 — the gate anchors on the **turn's start revision**, never on the
    /// last advanced expectation.
    ///
    /// Every other `may_coalesce` test above runs on a fence that was never
    /// advanced, where `start` and `expected` hold the same number and so are
    /// indistinguishable. The real LCV-123 flow advances the fence after each
    /// apply, so by the end `expected == current`, their difference is `0`, and
    /// a gate anchored on `expected` would return `false` for the rest of time:
    /// the turn would quietly leave `n` separate undo entries behind instead of
    /// one, which is AC 13's whole point, and nothing would say a word. So this
    /// test advances first, then coalesces.
    #[test]
    fn the_gate_anchors_on_the_start_revision_not_the_last_advance() {
        let mut fence = TurnFence::new(10);
        fence.advance(11);
        fence.advance(12);
        assert!(
            fence.may_coalesce(12, 2),
            "two commits since revision 10 fold into one, however often the \
             fence was advanced along the way"
        );
        assert!(
            !fence.may_coalesce(13, 2),
            "a third revision the turn did not make still breaks the run"
        );

        // And the same fence, advanced all the way to a four-commit turn.
        let mut fence = TurnFence::new(100);
        for revision in 101..=104 {
            assert_eq!(fence.check(revision - 1), Ok(()));
            fence.advance(revision);
        }
        assert!(fence.may_coalesce(104, 4), "four commits, four revisions");
        assert!(!fence.may_coalesce(104, 3), "a foreign commit hid in there");
    }

    /// The `current - start` in `may_coalesce` is unsigned subtraction one line
    /// away from a panic. A revision below the start cannot happen in practice
    /// — `revision()` is monotonic — but the arithmetic must survive it.
    #[test]
    fn may_coalesce_survives_a_revision_below_the_start() {
        let fence = TurnFence::new(10);
        assert!(!fence.may_coalesce(0, 2));
        assert!(!fence.may_coalesce(9, 2));
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
        assert_eq!(app.agent.applied, 0);
        assert_eq!(app.agent.turn_label, "Agent: draw a 20 mm square");

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
        assert_eq!(app.agent.fence, TurnFence::new(revision));
        assert_eq!(app.agent.fence.check(revision), Ok(()));
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
        let fence = app.agent.fence.clone();
        let label = app.agent.turn_label.clone();
        let armed = app.agent.rx.as_ref().map(std::ptr::from_ref);

        start_turn(&mut app, "the second prompt");

        assert_eq!(app.agent.chat, chat, "no row for a turn that never started");
        assert_eq!(app.agent.fence, fence, "the running turn keeps its fence");
        assert_eq!(app.agent.turn_label, label);
        assert_eq!(
            app.agent.rx.as_ref().map(std::ptr::from_ref),
            armed,
            "the running turn keeps its receiver"
        );
        assert!(app.agent.busy);
    }

    // ── AC 18: the budget is read and clamped here ───────────────────────────

    /// AC 18 / ADR 0007 §D7 — the stored budget is clamped at this read site.
    /// `0` and `200` are both things a hand-edited settings file can say.
    #[test]
    fn the_budget_is_clamped_where_it_is_read() {
        for (stored, expected) in [(0, 1), (1, 1), (12, 12), (32, 32), (200, 32), (255, 32)] {
            let mut app = App::default();
            app.settings.agent_step_budget = stored;
            assert_eq!(turn_step_budget(&app), expected, "stored {stored}");
        }
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
            implementation.contains(concat!("pub agent_step", "_budget: u8")),
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

    // ── The turn, over a real socket ─────────────────────────────────────────

    // ── mockito: the whole turn, over a real socket ──────────────────────────

    /// Captures every request body the mock server receives, in arrival order.
    ///
    /// mockito evaluates **every** mock's request matcher against **every**
    /// request, so this is attached to exactly one mock of a sequence and still
    /// sees all of them, once each. The `len()` assertions in the tests below
    /// are what would catch it if that ever changed.
    #[derive(Clone, Default)]
    struct Bodies(Arc<Mutex<Vec<String>>>);

    impl Bodies {
        fn matcher(&self) -> impl Fn(&mockito::Request) -> bool + Send + Sync + 'static {
            let sink = self.0.clone();
            move |request| {
                let body = request
                    .utf8_lossy_body()
                    .map(|b| b.into_owned())
                    .unwrap_or_default();
                sink.lock().expect("recorder mutex").push(body);
                true
            }
        }

        fn json(&self, n: usize) -> Value {
            let captured = self.0.lock().expect("recorder mutex");
            let raw = captured.get(n).unwrap_or_else(|| {
                panic!(
                    "expected at least {} request(s), saw {}",
                    n + 1,
                    captured.len()
                )
            });
            serde_json::from_str(raw).expect("the request body must be JSON")
        }

        fn len(&self) -> usize {
            self.0.lock().expect("recorder mutex").len()
        }
    }

    fn named_tool_call_body(id: &str, name: &str, arguments: &str) -> String {
        json!({"choices": [{"message": {
            "role": "assistant",
            "content": Value::Null,
            "tool_calls": [{
                "id": id,
                "type": "function",
                "function": {"name": name, "arguments": arguments}
            }]
        }}]})
        .to_string()
    }

    fn tool_call_body(id: &str, arguments: &str) -> String {
        named_tool_call_body(id, "create_line", arguments)
    }

    const LINE_ARGS: &str = r#"{"x1":0,"y1":0,"x2":20,"y2":0}"#;

    /// LCV-121 AC 6, moved here with the turn — a two-round turn sends the
    /// whole conversation back on the second request: system, user, the
    /// assistant turn **with** its `tool_calls`, and the `tool` turn with the
    /// matching id and the real outcome, in that order. This is the test that
    /// fails if `send_fn` ever filters the slice on `content` again, and it is
    /// also the one live-path proof that `parse_tool_call` → `agent_apply`
    /// really is wired end to end.
    #[test]
    fn multi_step_turn_sends_the_whole_conversation_back() {
        let mut server = mockito::Server::new();
        let bodies = Bodies::default();
        let _first = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(tool_call_body("call_xyz", LINE_ARGS))
            .expect(1)
            .create();
        let _second = server
            .mock("POST", "/chat/completions")
            .match_request(bodies.matcher())
            .with_status(200)
            .with_body(r#"{"choices":[{"message":{"role":"assistant","content":"Done."}}]}"#)
            .create();

        let mut applier = Applier::new();
        let reply = run_agent_turn(
            "draw a line",
            &server.url(),
            "k",
            "test/model",
            AGENT_STEP_BUDGET_DEFAULT,
            &mut |action| applier.ask(action),
        )
        .expect("the turn must finish");

        assert_eq!(reply, "Done.");
        assert_eq!(applier.entities(), 1, "the tool call really was dispatched");
        assert_eq!(
            applier.seen,
            [AgentAction::CreateLine {
                x1: 0.0,
                y1: 0.0,
                x2: 20.0,
                y2: 0.0
            }],
            "AC 4: one ask per tool call, carrying the parsed action"
        );
        assert_eq!(bodies.len(), 2, "exactly two round trips");

        let second = bodies.json(1);
        let msgs = second["messages"].as_array().expect("messages array");
        let roles: Vec<&str> = msgs.iter().filter_map(|m| m["role"].as_str()).collect();
        assert_eq!(
            roles,
            ["system", "user", "assistant", "tool"],
            "no turn may be dropped or reordered, body was {second}"
        );
        assert_eq!(msgs[2]["tool_calls"][0]["id"], "call_xyz");
        assert_eq!(msgs[2]["tool_calls"][0]["function"]["arguments"], LINE_ARGS);
        assert_eq!(msgs[3]["tool_call_id"], "call_xyz");
        assert_eq!(
            msgs[3]["content"],
            "Line created: (0.000, 0.000) → (20.000, 0.000) mm. \
             The drawing now has 1 entities.",
            "the model reads the real outcome, count and all (AC 9)"
        );
        assert_eq!(
            second["tools"].as_array().map(|t| t.len()),
            Some(7),
            "every round offers the tools, body was {second}"
        );
    }

    /// AC 11 / ADR 0007 §D2a — a refusal is a **tool result**, not a turn
    /// failure.
    ///
    /// The range check lives at the apply site now, so the only thing that can
    /// refuse an out-of-range index is `agent_apply`, and the only useful place
    /// to put its answer is back in the model's hands: it can then apologise,
    /// re-read the drawing, or pick a real index. Failing the turn instead
    /// would throw the conversation away over a recoverable mistake, and the
    /// user would see a red error row for what is really the model asking a
    /// question badly.
    #[test]
    fn a_refusal_is_fed_back_as_a_tool_result_and_the_turn_survives() {
        let mut server = mockito::Server::new();
        let bodies = Bodies::default();
        let _first = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(named_tool_call_body(
                "call_bad",
                "delete_entity",
                r#"{"index":7}"#,
            ))
            .expect(1)
            .create();
        let _second = server
            .mock("POST", "/chat/completions")
            .match_request(bodies.matcher())
            .with_status(200)
            .with_body(
                r#"{"choices":[{"message":{"role":"assistant","content":"There is nothing there."}}]}"#,
            )
            .create();

        let mut applier = Applier::new();
        let before = applier.app.history.revision();
        let reply = run_agent_turn(
            "delete entity 7",
            &server.url(),
            "k",
            "test/model",
            AGENT_STEP_BUDGET_DEFAULT,
            &mut |action| applier.ask(action),
        )
        .expect("a refused tool call must not fail the turn");

        assert_eq!(reply, "There is nothing there.");
        assert_eq!(bodies.len(), 2, "the turn went another round");
        assert_eq!(applier.entities(), 0, "nothing was applied");
        assert_eq!(
            applier.app.history.revision(),
            before,
            "nothing was committed"
        );

        let second = bodies.json(1);
        let msgs = second["messages"].as_array().expect("messages array");
        let roles: Vec<&str> = msgs.iter().filter_map(|m| m["role"].as_str()).collect();
        assert_eq!(roles, ["system", "user", "assistant", "tool"]);
        assert_eq!(msgs[3]["tool_call_id"], "call_bad");
        assert_eq!(
            msgs[3]["content"], "index 7 is out of range (the drawing has 0 entities)",
            "the model must be told what went wrong, in the apply site's words"
        );
    }

    /// LCV-121 AC 9 — `run_agent_turn` forwards the model id it was given, unchanged.
    /// Two different ids, so a hardcoded default cannot satisfy both.
    #[test]
    fn run_agent_turn_sends_the_model_it_was_given() {
        for model in ["anthropic/claude-sonnet-4.6", "test/some-other-model"] {
            let mut server = mockito::Server::new();
            let bodies = Bodies::default();
            let _mock = server
                .mock("POST", "/chat/completions")
                .match_request(bodies.matcher())
                .with_status(200)
                .with_body(r#"{"choices":[{"message":{"role":"assistant","content":"hi"}}]}"#)
                .create();

            let mut applier = Applier::new();
            let reply = run_agent_turn(
                "hello",
                &server.url(),
                "k",
                model,
                AGENT_STEP_BUDGET_DEFAULT,
                &mut |action| applier.ask(action),
            );
            assert_eq!(reply.unwrap(), "hi");
            assert_eq!(bodies.len(), 1);
            assert_eq!(bodies.json(0)["model"], model);
        }
    }

    /// LCV-121 AC 11 — against an endpoint that only ever asks for another tool call,
    /// the turn stops at the budget, says the real number, and has applied no
    /// more than `budget` actions.
    #[test]
    fn a_relentless_endpoint_stops_at_the_budget() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(tool_call_body("call_loop", LINE_ARGS))
            .create();

        let mut applier = Applier::new();
        let result = run_agent_turn(
            "draw forever",
            &server.url(),
            "k",
            "test/model",
            2,
            &mut |action| applier.ask(action),
        );

        match result {
            Err(AgentError::IterationLimitExceeded(budget)) => {
                assert_eq!(budget, 2);
                assert!(AgentError::IterationLimitExceeded(budget)
                    .to_string()
                    .contains('2'));
            }
            other => panic!("expected the budget to stop the turn, got {other:?}"),
        }
        assert_eq!(
            applier.entities(),
            2,
            "at most the budget may be dispatched"
        );
    }

    /// A transport-level status error reaches the caller as `Transport`, with
    /// the readable text the transport produced.
    #[test]
    fn a_401_reaches_the_caller_as_a_transport_error() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("POST", "/chat/completions")
            .with_status(401)
            .with_body("nope")
            .create();

        let mut applier = Applier::new();
        let result = run_agent_turn(
            "hello",
            &server.url(),
            "k",
            "test/model",
            AGENT_STEP_BUDGET_DEFAULT,
            &mut |action| applier.ask(action),
        );
        match result {
            Err(AgentError::Transport(message)) => {
                assert!(
                    message.contains("Authentication failed (HTTP 401)"),
                    "{message}"
                );
            }
            other => panic!("expected a transport error, got {other:?}"),
        }
    }

    // ── AC 4, AC 5: the ask seam ─────────────────────────────────────────────

    /// AC 4 — one ask per tool call, in the order the model asked, and the
    /// string the ask answered with is what the model reads back.
    ///
    /// Two calls in **one** assistant message, so an implementation that
    /// handled only the first — or that reordered them — cannot pass. The
    /// arguments differ in every field, so a transposition is visible too.
    #[test]
    fn every_tool_call_becomes_one_ask_in_order() {
        let mut server = mockito::Server::new();
        let bodies = Bodies::default();
        let _first = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(
                json!({"choices": [{"message": {
                    "role": "assistant",
                    "content": Value::Null,
                    "tool_calls": [
                        {"id": "a", "type": "function", "function":
                            {"name": "create_line",
                             "arguments": r#"{"x1":0,"y1":0,"x2":20,"y2":0}"#}},
                        {"id": "b", "type": "function", "function":
                            {"name": "create_circle",
                             "arguments": r#"{"cx":5,"cy":6,"r":3}"#}}
                    ]
                }}]})
                .to_string(),
            )
            .expect(1)
            .create();
        let _second = server
            .mock("POST", "/chat/completions")
            .match_request(bodies.matcher())
            .with_status(200)
            .with_body(r#"{"choices":[{"message":{"role":"assistant","content":"Done."}}]}"#)
            .create();

        let mut applier = Applier::new();
        let reply = run_agent_turn(
            "draw",
            &server.url(),
            "k",
            "test/model",
            AGENT_STEP_BUDGET_DEFAULT,
            &mut |action| applier.ask(action),
        )
        .expect("the turn must finish");

        assert_eq!(reply, "Done.");
        assert_eq!(
            applier.seen,
            [
                AgentAction::CreateLine {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 20.0,
                    y2: 0.0
                },
                AgentAction::CreateCircle {
                    cx: 5.0,
                    cy: 6.0,
                    r: 3.0
                },
            ],
            "AC 4: one ask per tool call, in order"
        );
        assert_eq!(applier.entities(), 2);

        // And each ask's answer is the tool result the model reads back.
        let second = bodies.json(1);
        let msgs = second["messages"].as_array().expect("messages array");
        assert_eq!(msgs[3]["tool_call_id"], "a");
        assert!(
            msgs[3]["content"]
                .as_str()
                .is_some_and(|c| c.starts_with("Line created:")),
            "body was {second}"
        );
        assert_eq!(msgs[4]["tool_call_id"], "b");
        assert!(
            msgs[4]["content"]
                .as_str()
                .is_some_and(|c| c.starts_with("Circle created:")),
            "body was {second}"
        );
    }

    /// AC 5 — an ask that reports cancellation ends the turn **immediately**.
    ///
    /// The UI is gone, or has abandoned this turn, so there is nothing left to
    /// answer any further question and nothing to spend another request on. The
    /// mock is `.expect(1)`: a turn that carried on would send a second one and
    /// this assertion is what says so.
    #[test]
    fn a_cancelled_ask_returns_at_once_and_sends_nothing_more() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_body(tool_call_body("call_gone", LINE_ARGS))
            .expect(1)
            .create();

        let mut applier = Applier::cancelling();
        let result = run_agent_turn(
            "draw a line",
            &server.url(),
            "k",
            "test/model",
            AGENT_STEP_BUDGET_DEFAULT,
            &mut |action| applier.ask(action),
        );

        assert!(
            matches!(result, Err(AgentError::Cancelled)),
            "got {result:?}"
        );
        assert_eq!(applier.seen.len(), 1, "it stops at the first dead ask");
        assert_eq!(applier.entities(), 0, "and nothing was applied");
        mock.assert();
    }

    /// AC 4 / AC 8 — the real ask seam **asks**. Every test above this one
    /// substitutes its own callback for [`ask_ui`], so nothing there would
    /// notice a seam that fabricated an answer and never sent an `Act` — the
    /// mutation that silently disconnects the model from the drawing.
    ///
    /// Two halves, both deterministic and neither one timed:
    ///
    /// - a `Sender` whose `Receiver` is already gone must report cancellation
    ///   rather than success, and needs no second thread at all;
    /// - a live rendezvous, where this thread plays the UI: it blocks on the
    ///   channel until the `Act` arrives, checks the action crossed intact,
    ///   answers, and joins. A seam that fabricated would drop its `Sender`
    ///   without sending, so `recv` fails here instead of hanging.
    #[test]
    fn the_ask_seam_sends_the_action_and_waits_for_the_answer() {
        let action = AgentAction::CreateLine {
            x1: 1.0,
            y1: 2.0,
            x2: 3.0,
            y2: 4.0,
        };

        let (dead_tx, dead_rx) = channel::<AgentEvent>();
        drop(dead_rx);
        assert!(
            matches!(ask_ui(&dead_tx, action.clone()), Err(AgentError::Cancelled)),
            "no listener means cancelled, never a fabricated success"
        );

        let (tx, rx) = channel::<AgentEvent>();
        let sent = action.clone();
        let worker = std::thread::spawn(move || ask_ui(&tx, sent));

        let event = rx.recv().expect("the ask must put an Act on the channel");
        let AgentEvent::Act {
            action: crossed,
            reply,
        } = event
        else {
            panic!("the ask must send an Act, not a terminal event");
        };
        assert_eq!(crossed, action, "the action crosses the channel intact");
        reply
            .send(AgentOutcome::Ok("the UI answered".to_owned()))
            .expect("the ask must still be waiting on its reply channel");

        let answered = worker.join().expect("the ask thread must not panic");
        assert_eq!(
            answered.expect("an answered ask succeeds").text(),
            "the UI answered",
            "the seam returns the UI's answer, not one of its own"
        );
    }

    /// AC 1 — `run_agent_turn` owns no document state: this file's
    /// implementation section names neither type, nor a snapshot of one, nor a
    /// shared handle to one.
    ///
    /// The needles are checked against a witness first, so a misspelt one
    /// fails here instead of passing over any haystack at all.
    #[test]
    fn the_turn_owns_no_document_state() {
        let src = include_str!("agent_turn.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("agent_turn.rs must have a bare #[cfg(test)] marker");
        let implementation = &src[..at];

        let witness = "use crate::document::{Document, History}; \
                       let shared: Arc<Mutex<Document>> = x; \
                       let snapshot: Vec<Entity> = y; let d = Document::default();";
        for forbidden in [
            concat!("crate::", "document"),
            concat!("Doc", "ument"),
            concat!("His", "tory"),
            concat!("Arc<", "Mutex"),
            concat!("Vec<", "Entity>"),
        ] {
            assert!(
                witness.contains(forbidden),
                "control: `{forbidden}` must be able to match real document state"
            );
            let hit = implementation
                .lines()
                .find(|l| l.contains(forbidden) && !l.trim_start().starts_with("//"));
            assert!(
                hit.is_none(),
                "AC 1: run_agent_turn's file must not name `{forbidden}`: {hit:?}"
            );
        }
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

    // ── LCV-129 AC 10: the worker can always notice and unwind ─────────────

    /// LCV-129 AC 10, first case — the receiver is already gone when the
    /// worker tries to send.
    ///
    /// This is the easy half, and it exercises the real [`ask_ui`]: a `send`
    /// on a channel whose `Receiver` has been dropped returns `Err`, which the
    /// mapper turns into `Cancelled`. No thread and no sleep — dropping `rx`
    /// here is exactly what `end_turn` does one frame after a cancel.
    #[test]
    fn ac10_a_send_with_nobody_listening_is_a_cancel() {
        let (tx, rx) = channel::<AgentEvent>();
        drop(rx);

        let outcome = ask_ui(&tx, AgentAction::QuerySelection);

        assert!(
            matches!(outcome, Err(AgentError::Cancelled)),
            "a dead channel must unwind the turn, got {outcome:?}"
        );
    }

    /// LCV-129 AC 10, second case — the receiver is dropped while an `Act` is
    /// queued and nobody ever read it.
    ///
    /// The genuinely hard half, and the property the whole cancel design rests
    /// on: the worker has already *succeeded* at sending and is parked in
    /// `answer.recv()`, so nothing about the event channel can wake it. What
    /// wakes it is that dropping a `std::sync::mpsc::Receiver` drops the
    /// messages still queued in it, and the reply `Sender` rides *inside* the
    /// `Act` (ADR 0007 §D3) — so the one-shot channel closes as a side effect
    /// and `recv()` returns `Err`. Park this design on a channel that stored
    /// its reply handle on `App` instead and a cancelled worker would block
    /// forever.
    ///
    /// [`ask_ui`] cannot be called here: it would park this very test thread
    /// on `recv()`. So its second half is replicated line for line — same
    /// one-shot channel, same `Act`, same `map_err` — and the drop that a real
    /// cancel performs happens in between, with the `Act` still unread. No
    /// thread, no sleep; if the property ever stopped holding this test would
    /// hang rather than pass, and `send` succeeding is asserted first so a
    /// failure says which half broke.
    #[test]
    fn ac10_dropping_the_receiver_with_an_act_queued_is_a_cancel() {
        let (tx, rx) = channel::<AgentEvent>();
        let (reply, answer) = channel::<AgentOutcome>();
        tx.send(AgentEvent::Act {
            action: AgentAction::QuerySelection,
            reply,
        })
        .expect("the UI is still there when the worker sends");

        // The cancel: `end_turn` drops the receiver, and the unread `Act`
        // — carrying the only `Sender` to `answer` — goes with it.
        drop(rx);

        let outcome: Result<AgentOutcome, AgentError> =
            answer.recv().map_err(|_| AgentError::Cancelled);
        assert!(
            matches!(outcome, Err(AgentError::Cancelled)),
            "a worker parked on an answer nobody will give must unwind, got {outcome:?}"
        );
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
