//! LCV-129 — the bounded call, and the way out of a turn that will not end.
//!
//! Two halves of one defect. A call with no window hangs the worker; a worker
//! that never reports leaves `agent.busy` latched; a latched flag repaints for
//! the rest of the session (LCV-120, reopened through a different door). The
//! timeout closes the common case and Cancel closes the rest, and both end the
//! turn through the *same* exit — `agent_poll::end_turn` — because ADR 0007
//! §D11 is a closure property, not a list of arms.
//!
//! **No test in this file reaches a real endpoint.** The timeout itself is
//! proven in `src/agent/transport.rs`'s own tests against two loopback
//! fixtures; here a timed-out call is replayed as the `AgentEvent::Failed` the
//! worker would have sent, by hand, with no thread and no socket. The one turn
//! that really spawns a worker (AC 9's second prompt) points at
//! [`UNPARSEABLE_ENDPOINT`], which `Url::parse` rejects before a connection is
//! built or a proxy consulted — the remedy LCV-124's reviewer established after
//! reproducing a genuine credential leak through `HTTP_PROXY` against an
//! address that merely *looked* unreachable.
//!
//! ADR 0002 §A2: `App::default()` only, never `App::new()`. §A4 rule 1: no test
//! here sends `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S`. Every `App` leaves
//! `settings_path` and `autosave_path` at `None`, so nothing here can write to
//! a real per-user path (ADR 0006, ADR 0007 §D10).
//!
//! ## Why this file reads the paint list
//!
//! AC 8 is about a **button that exists only while a turn does**, and AC 7's
//! second sentence is about the **order of two rows**. A source scan can prove
//! the `Cancel` literal is written — the inline scan in `src/agent/panel.rs`
//! does exactly that — and it can prove neither of those two things. So the
//! two helpers below read the frame's paint list through
//! `tests/harness/paint.rs`, which is where the traps that decide whether such
//! an assertion means anything are written down. The scan half calls
//! `tests/harness/scan.rs`'s walker and its count-returning matcher, because
//! AC 5's claim is a count and not a presence.

use crate::harness;

use harness::paint::{lines_on_surface_of, painted_runs, texts, Run};
use harness::raw_input;
use harness::scan::{is_test_file, occurrences, rs_files};
use lasercad::agent::{AgentAction, AgentEvent, AgentOutcome, TransportError};
use lasercad::app::{arm_turn, cancel_turn, poll_agent_rx, App, AGENT_CANCELLED_MESSAGE};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

/// A recognisable key that must never reach a wire or a transcript. Built with
/// `concat!` so a grep for the whole string finds no copy of it.
const DUMMY_KEY: &str = concat!("sk-test-", "DO-NOT-LEAK");

/// An endpoint that fails in `Url::parse`, before any socket and before any
/// proxy lookup (LCV-124). `127.0.0.1:1` is **not** a substitute: with
/// `HTTP_PROXY` set, reqwest hands even a loopback address to the proxy, and a
/// reviewer captured a bearer token leaving the machine that way while the
/// suite reported all green.
const UNPARSEABLE_ENDPOINT: &str = "not-a-url";

// ── Plumbing ────────────────────────────────────────────────────────────────

/// A headless egui context and an `App` with no injected paths.
fn ctx_and_app() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let app = App::default();
    assert!(
        app.settings_path.is_none() && app.autosave_path.is_none(),
        "ADR 0006: no real per-user path may be injected"
    );
    (ctx, app)
}

/// Push one `Act` and hand back the reply `Receiver` (ADR 0007 §D3, by hand).
fn push_act(tx: &Sender<AgentEvent>, action: AgentAction) -> Receiver<AgentOutcome> {
    let (reply, answer) = channel::<AgentOutcome>();
    tx.send(AgentEvent::Act { action, reply })
        .expect("the armed Receiver must still be on App");
    answer
}

fn line(x2: f64) -> AgentAction {
    AgentAction::CreateLine {
        x1: 0.0,
        y1: 0.0,
        x2,
        y2: 0.0,
    }
}

/// `Ctrl` **as egui reports it on Linux**: `ctrl` and `command` both set.
///
/// `egui::Modifiers::CTRL` leaves `command` false and `dispatch_shortcuts`
/// gates undo on `command_only()`, so a tap built from the bare constant is
/// silently swallowed and the undo assertion reads as a broken coalesce.
fn ctrl() -> egui::Modifiers {
    egui::Modifiers {
        ctrl: true,
        command: true,
        ..egui::Modifiers::NONE
    }
}

// ── The paint list ──────────────────────────────────────────────────────────

/// The agent panel, open, after two frames — egui sizes a layout on the first
/// and paints it settled on the second (ADR 0002).
fn settled_panel(ctx: &egui::Context, app: &mut App) -> Vec<Run> {
    app.agent.panel_open = true;
    let _ = painted_runs(ctx, app);
    painted_runs(ctx, app)
}

/// The panel's visual lines, top to bottom, each read left to right.
///
/// Scoped on the `AI Assistant` heading, a run genuinely inside the panel —
/// never on a window title, which is clipped to the whole screen and would
/// select every surface in the frame.
fn panel_lines(runs: &[Run]) -> Vec<Vec<String>> {
    texts(&lines_on_surface_of(runs, "AI Assistant"))
}

// ── Part A — the timeout ────────────────────────────────────────────────────

/// AC 4 — a timed-out call ends the turn through the exit that already exists.
///
/// The worker's `Err` becomes `AgentError::Transport(..)` and then
/// `AgentEvent::Failed(..)`, which is ADR 0007 §D11 exit (2) — no new event,
/// no new variant. So this test replays exactly that event, by hand, and
/// asserts the three things a turn owes on its way out. No thread, no socket,
/// no sleep: what the transport does with a stalled connection is pinned in
/// its own tests, and what the *app* does with the result is pinned here.
#[test]
fn ac4_a_timed_out_call_ends_the_turn_through_failed() {
    let mut app = App::default();
    let tx = arm_turn(&mut app, "draw a very slow circle");
    let sentence = TransportError::Timeout { secs: 120 }.to_string();

    tx.send(AgentEvent::failed(sentence.clone()))
        .expect("the armed Receiver must still be on App");
    poll_agent_rx(&mut app);

    assert!(
        !app.agent.busy,
        "AC 4: the busy flag falls — LCV-120's latch"
    );
    assert!(app.agent.rx.is_none(), "AC 4: and the channel goes with it");
    let (role, text) = app.agent.chat.last().expect("a terminal row");
    assert_eq!(role, "error", "a timeout is a failure, not a note");
    assert_eq!(text, &sentence);
    assert!(
        text.contains("did not answer within 120 s"),
        "the operator reads the window, not a reqwest debug string: {text}"
    );
    assert!(!text.contains(DUMMY_KEY), "and never a key");
}

// ── AC 5 — the sole-writer scan ─────────────────────────────────────────────

/// `src/`'s `.rs` files as (relative path, implementation text) pairs, sorted
/// on the rendered path.
///
/// Each haystack stops at the bare `#[cfg(test)]` at column 0, so inline test
/// code is never searched. Paths are rebuilt from `components()` joined with
/// `/` rather than `Path::display()`, which emits `\` on Windows and has broken
/// this repository's CI twice (AGENTS.md).
fn implementation_sections() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rs_files(&root, &mut files);
    files.retain(|path| !is_test_file(path));
    assert!(
        files.len() > 30,
        "positive control: the walk must see the whole tree, saw {}",
        files.len()
    );
    let mut sections: Vec<(String, String)> = files
        .iter()
        .map(|path| {
            let relative = path
                .strip_prefix(&root)
                .expect("every walked file is under src/")
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            let src = std::fs::read_to_string(path).expect("a readable source file");
            let body = match src.find("\n#[cfg(test)]") {
                Some(at) => &src[..at],
                None => &src[..],
            };
            (relative, body.to_owned())
        })
        .collect();
    sections.sort();
    sections
}

/// AC 5 / ADR 0007 §D11 — `end_turn` is the only place in the program that
/// clears the busy flag, and now something says so out loud.
///
/// This is the guard that makes the rest of the demand safe. The tempting way
/// to write Cancel is `app.agent.busy = false` in the panel's click handler: it
/// passes every manual test, and it skips the coalesce, the note row and the
/// channel drop. AC 12 catches the undo shape; this catches the *shape of the
/// code*, which is what ADR 0007 promised and what prose alone cannot keep.
///
/// The positive control is not decorative: an absence assertion over a
/// mis-sliced or empty haystack passes for the wrong reason, so the same
/// counter is first run over a synthetic tree carrying the needle twice.
///
/// **LCV-128 AC 3 extends this test in place** rather than adding a second
/// scan of the same tree with the same hygiene (the demand's own sequencing
/// note: this file landed first, so it owns the invariant). The other two
/// single-writer claims in the same AGENTS.md §Event flow paragraph —
/// `agent.rx = None` cleared only by `end_turn`, and `agent.busy = true` set
/// only by `arm_turn` — are asserted by the same loop, over the same
/// `implementation_sections()`, with the same witness technique.
#[test]
fn ac5_only_agent_poll_clears_the_busy_flag() {
    let sections = implementation_sections();

    for (needle, owner) in [
        (concat!("agent.busy", " = false"), "app/agent_poll.rs"),
        (concat!("agent.rx", " = None"), "app/agent_poll.rs"),
        (concat!("agent.busy", " = true"), "app/agent_turn.rs"),
    ] {
        let witness = vec![
            (
                "agent/panel.rs".to_owned(),
                format!("if ui.button(\"X\").clicked() {{ app.{needle}; }}"),
            ),
            (
                owner.to_owned(),
                format!("// app.{needle}\n    app.{needle};\n"),
            ),
        ];
        assert_eq!(
            occurrences(&witness, needle),
            [("agent/panel.rs".to_owned(), 1), (owner.to_owned(), 1)],
            "control: the counter must find a second writer of `{needle}`, and must not \
             count a commented one"
        );

        let hits = occurrences(&sections, needle);
        assert_eq!(
            hits,
            [(owner.to_owned(), 1)],
            "AGENTS.md §Event flow: `{needle}` belongs to {owner} and to nothing else"
        );
    }
}

// ── Part B — the cancel ─────────────────────────────────────────────────────

/// AC 8 — **painted output**: the Cancel button is on the screen while a turn
/// is running, and off it when none is.
///
/// The inline scan in `src/agent/panel.rs` proves the literal is written inside
/// the `agent.busy` block. It cannot prove the block is reached, that the
/// button is laid out rather than clipped to nothing, or that the row really
/// disappears — three ways to ship a Cancel nobody can click. The two frames
/// below differ in exactly one field, so the comparison is the discrimination.
#[test]
fn ac8_the_cancel_button_is_painted_only_while_a_turn_runs() {
    let (ctx, mut app) = ctx_and_app();

    // Idle: no turn, so no row and no button.
    let idle = settled_panel(&ctx, &mut app);
    let idle_lines = panel_lines(&idle);
    assert!(
        !idle_lines.iter().flatten().any(|t| t == "Cancel"),
        "AC 8: nothing to cancel, so nothing offers to: {idle_lines:?}"
    );

    // Busy: the same panel, one field different.
    let _tx = arm_turn(&mut app, "draw a slow thing");
    let busy = settled_panel(&ctx, &mut app);
    let busy_lines = panel_lines(&busy);
    let thinking = busy_lines
        .iter()
        .find(|line| line.iter().any(|t| t == "Thinking… 0 of 256 steps"))
        .unwrap_or_else(|| panic!("the thinking row must be painted: {busy_lines:?}"));
    assert!(
        thinking.iter().any(|t| t == "Cancel"),
        "AC 8: the way out sits on the thinking row itself: {thinking:?}"
    );
}

/// AC 7 — **painted output**: after a cancel the operator reads two rows, in
/// one order, and that order is what makes them make sense.
///
/// The cancel note says work already applied stays applied; the undo note says
/// in how many steps it comes back. Reversed, the second sentence answers a
/// question the first has not asked yet. `agent.chat` order is asserted too,
/// but only the paint list can say the panel renders them in that order: the
/// loop that feeds `draw_chat_row` could be reversed and every state assertion
/// in this repository would stay green.
#[test]
fn ac7_the_cancel_note_is_painted_above_the_undo_note() {
    let (ctx, mut app) = ctx_and_app();
    let tx = arm_turn(&mut app, "draw three lines");
    let _answers = [
        push_act(&tx, line(10.0)),
        push_act(&tx, line(20.0)),
        push_act(&tx, line(30.0)),
    ];
    let _ = settled_panel(&ctx, &mut app);

    cancel_turn(&mut app);

    let undo_note = "Applied 3 actions — Ctrl+Z undoes the whole turn.";
    assert_eq!(
        app.agent
            .chat
            .iter()
            .map(|(r, _)| r.as_str())
            .collect::<Vec<_>>(),
        ["user", "tool", "tool", "tool", "note", "note"],
        "AC 7: no seventh role — a cancel is a note like the undo note is"
    );
    assert_eq!(
        app.agent.chat[4].1, AGENT_CANCELLED_MESSAGE,
        "the cancel row comes first"
    );
    assert_eq!(app.agent.chat[5].1, undo_note, "and the undo row after it");

    let runs = settled_panel(&ctx, &mut app);
    let lines = panel_lines(&runs);
    let index = |text: &str| {
        lines
            .iter()
            .position(|line| line.iter().any(|t| t == text))
            .unwrap_or_else(|| panic!("`{text}` must be painted: {lines:?}"))
    };
    assert_eq!(
        index(undo_note),
        index(AGENT_CANCELLED_MESSAGE) + 1,
        "AC 7: the two notes are painted as consecutive lines, cancel first: {lines:?}"
    );
}

/// AC 9 — after a cancel the app is idle, and it is *usable*.
///
/// Idle is behavioural, not a flag read: the frame after the cancel tells egui
/// it may sleep (`Duration::MAX`), where the frame before it demanded an
/// immediate wake-up. That is the whole defect — a repaint condition that can
/// no longer fall — measured at the seam LCV-120 established rather than
/// inferred from `agent.busy`.
///
/// Then the second half: a prompt typed after a cancel is *accepted*, not
/// refused by LCV-124 AC 10's busy gate. The turn it starts is real — a worker
/// thread does spawn — but its endpoint fails in `Url::parse`, so no socket is
/// opened, no proxy is consulted, and the thread dies with a request error.
#[test]
fn ac9_after_a_cancel_the_app_idles_and_takes_a_new_turn() {
    let (ctx, mut app) = ctx_and_app();
    app.settings.agent_api_key = DUMMY_KEY.to_owned();
    app.settings.agent_endpoint = UNPARSEABLE_ENDPOINT.to_owned();
    assert!(
        reqwest::Url::parse(UNPARSEABLE_ENDPOINT).is_err(),
        "control: the endpoint must fail at parse, before any socket"
    );

    let _tx = arm_turn(&mut app, "draw a slow thing");
    let _ = settled_panel(&ctx, &mut app);
    assert_eq!(
        delay(&ctx, &mut app),
        Duration::ZERO,
        "control: a live turn keeps frames turning (ADR 0007 §D2)"
    );

    cancel_turn(&mut app);

    assert!(!app.agent.busy);
    assert!(app.agent.rx.is_none());
    let _ = delay(&ctx, &mut app); // the frame that observes the change
    assert_eq!(
        delay(&ctx, &mut app),
        Duration::MAX,
        "AC 9: the repaint gate no longer fires — the app may sleep"
    );

    // And the command line takes a second prompt.
    harness::submit_command(&ctx, &mut app, ": draw a circle");
    assert!(
        app.command_feedback.starts_with("→ agent:"),
        "AC 9: a cancelled turn does not hold the busy gate shut: {}",
        app.command_feedback
    );
    assert!(app.agent.busy, "the second turn really started");
    cancel_turn(&mut app);
}

/// One frame, and the repaint egui was asked for.
fn delay(ctx: &egui::Context, app: &mut App) -> Duration {
    let out = ctx.run(raw_input(Vec::new()), |ctx| app.update_ui(ctx));
    out.viewport_output
        .get(&egui::ViewportId::ROOT)
        .expect("the root viewport is always present")
        .repaint_delay
}

/// AC 11 — a cancelled turn can never speak into a later one.
///
/// The zombie thread §Out of scope accepts is only harmless if its `Sender` is
/// inert. Each turn owns its own channel, so the old one is closed the moment
/// `end_turn` drops the receiver — and the new turn, armed a line later, does
/// not inherit it. This is what makes "the worker may still be draining an
/// HTTP call" an acceptable cost rather than a race.
#[test]
fn ac11_the_old_sender_cannot_speak_into_the_new_turn() {
    let mut app = App::default();
    let ghost = arm_turn(&mut app, "the cancelled turn");
    cancel_turn(&mut app);

    let _live = arm_turn(&mut app, "the new turn");
    let chat = app.agent.chat.clone();
    let revision = app.history.revision();

    assert!(
        ghost.send(AgentEvent::done("ghost")).is_err(),
        "AC 11: the old channel died with the turn that owned it"
    );
    poll_agent_rx(&mut app);

    assert_eq!(app.agent.chat, chat, "the new turn heard nothing");
    assert!(app.agent.busy, "and is still running");
    assert!(app.agent.rx.is_some(), "on its own channel");
    assert_eq!(app.agent.turn.applied, 0);
    assert_eq!(app.history.revision(), revision);
}

/// AC 12 — a cancelled turn has the undo shape every other exit has.
///
/// Three actions, a cancel, one `Ctrl+Z`, and the bed is as it was. This is the
/// criterion that makes the tempting implementation visible: a `cancel_turn`
/// that cleared `agent.busy` itself instead of tail-calling `end_turn` would
/// leave three separate undo entries behind a note claiming one, and every
/// manual test would look fine.
#[test]
fn ac12_a_cancelled_turn_still_folds_into_one_undo_entry() {
    let (ctx, mut app) = ctx_and_app();
    let entities_before = app.document.entities.len();
    let stack_before = app.history.len();

    let tx = arm_turn(&mut app, "draw three lines");
    let _answers = [
        push_act(&tx, line(10.0)),
        push_act(&tx, line(20.0)),
        push_act(&tx, line(30.0)),
    ];
    harness::frame(&ctx, &mut app, Vec::new());
    assert_eq!(app.document.entities.len(), entities_before + 3);

    cancel_turn(&mut app);

    assert_eq!(
        app.history.len(),
        stack_before + 1,
        "AC 12: three commits, one undo entry — the cancel went through end_turn"
    );
    assert_eq!(
        app.agent.chat.last().map(|(r, t)| (r.as_str(), t.as_str())),
        Some(("note", "Applied 3 actions — Ctrl+Z undoes the whole turn.")),
        "and the note says the shape the stack really has"
    );

    harness::tap(&ctx, &mut app, egui::Key::Z, ctrl());
    assert_eq!(
        app.document.entities.len(),
        entities_before,
        "AC 12: one Ctrl+Z takes the whole partial turn back"
    );
}

// ── Part C — purity and the settings that were not added ────────────────────

/// AC 13 — the agent module stays a pure-Rust kernel plus two UI files.
///
/// `only_the_transport_imports_reqwest` in `tests/it/repo/tree_scans.rs`
/// covers the HTTP half and is not duplicated here. LCV-128 will widen the
/// egui/eframe/rfd half to the whole tree; until it lands this file states it
/// for `src/agent/`, which is the half LCV-129 touched.
#[test]
fn ac13_only_the_two_ui_files_under_agent_import_egui() {
    let sections: Vec<(String, String)> = implementation_sections()
        .into_iter()
        .filter(|(path, _)| path.starts_with("agent/"))
        .collect();
    assert!(
        sections.len() > 5,
        "positive control: the agent module has more than a couple of files"
    );

    // The needle is the crate name on a code line, not `use egui`: this file
    // reaches egui through fully-qualified paths (`egui::Ui`) and imports
    // nothing, so a `use`-shaped needle would report the panel as pure.
    let egui_importers: Vec<String> = occurrences(&sections, concat!("e", "gui"))
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    assert_eq!(
        egui_importers,
        ["agent/panel.rs", "agent/settings_ui.rs"],
        "AC 13: the agent kernel is pure Rust; only these two files render"
    );

    for forbidden in [concat!("e", "frame"), concat!("r", "fd")] {
        let hits = occurrences(&sections, forbidden);
        assert!(
            hits.is_empty(),
            "AC 13: nothing under src/agent/ may name `{forbidden}`: {hits:?}"
        );
    }
}

/// AC 14 — the two windows are constants in the transport, not a setting.
///
/// A timeout the operator can set is a timeout the operator can set to zero,
/// and it would have to be persisted, migrated and explained. The demand chose
/// two constants; this is what keeps that choice from eroding one row at a
/// time.
#[test]
fn ac14_no_timeout_setting_was_added() {
    let sections = implementation_sections();
    let needle = concat!("time", "out");

    let settings: Vec<(String, String)> = sections
        .iter()
        .filter(|(path, _)| path == "io/settings.rs" || path == "agent/settings_ui.rs")
        .cloned()
        .collect();
    assert_eq!(
        settings.len(),
        2,
        "positive control: both settings files must be in the walk"
    );
    assert!(
        settings
            .iter()
            .any(|(_, body)| body.contains(concat!("agent_step", "_budget"))),
        "positive control: the settings really do carry agent fields"
    );
    let hits = occurrences(&settings, needle);
    assert!(
        hits.is_empty(),
        "AC 14: no setting was added for `{needle}`: {hits:?}"
    );

    let owners: Vec<String> = occurrences(&sections, concat!("AGENT_REQUEST_TIME", "OUT_SECS"))
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    assert_eq!(
        owners,
        ["agent/transport.rs"],
        "AC 14: the windows live in the transport and nowhere else"
    );
}
