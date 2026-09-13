//! LCV-124 — the command line can reach the agent (ADR 0007 §D9).
//!
//! Every test here drives a real `App`. Most of them need no turn at all: a
//! line that is refused — for want of a key, for want of a prompt, or because
//! a turn is already in flight — is answered synchronously by `submit`, so the
//! assertions are exact and there is no race to lose.
//!
//! The two tests that do start a turn **never reach a real endpoint**. The
//! endpoint is [`DEAD_ENDPOINT`], a closed port on the loopback interface, so
//! the worker thread the app spawns fails its connection locally and exits; and
//! `a_turn_started_from_the_command_line_draws_on_the_real_bed` then takes the
//! turn over through LCV-123's `arm_turn` seam — it replaces the receiver with
//! one it owns and pushes the events by hand, so the geometry assertion depends
//! on no thread and no socket (demand §Expected tests, ADR 0007 §D10).
//!
//! ADR 0002 §A2: `App::default()` only, never `App::new()`. §A4 rule 1: no test
//! here sends `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S`. Every `App` leaves
//! `settings_path` and `autosave_path` at `None`, so the dummy key below can
//! never be written to a real per-user path (ADR 0006, ADR 0007 §D10).

mod harness;

use harness::{frame, raw_input, submit_command, tap, type_command};
use lasercad::agent::{AgentAction, AgentEvent};
use lasercad::app::{arm_turn, submit, App};
use lasercad::document::Entity;
use lasercad::geometry::Vec2;
use lasercad::text::layout_text;
use lasercad::tools::{PolylineTool, TextTool};
use std::sync::mpsc::channel;

/// A key that is obviously not a key, assembled from fragments so a grep for
/// the whole string finds no copy of it. Its only job is to make
/// `agent_available` true.
const DUMMY_KEY: &str = concat!("sk-test-", "DO-NOT-LEAK");

/// Port 1 on the loopback interface: nothing listens there, ever. A turn these
/// tests start gets `ECONNREFUSED` from the local network stack before a byte
/// leaves the machine, which is how "no CI test may reach a real endpoint" is
/// honoured without standing up a mockito server for a reply nobody asserts on.
const DEAD_ENDPOINT: &str = "http://127.0.0.1:1";

/// An `App` with no injected paths and an agent that is configured but
/// unreachable.
fn app_with_a_key() -> App {
    let mut app = App::default();
    assert!(
        app.settings_path.is_none() && app.autosave_path.is_none(),
        "ADR 0006: no real per-user path may be injected next to a key"
    );
    app.settings.agent_api_key = DUMMY_KEY.to_owned();
    app.settings.agent_endpoint = DEAD_ENDPOINT.to_owned();
    app
}

/// A headless context and an `App` after one frame, which registers every
/// widget rect and syncs the camera away from its zero-area sentinel.
fn boot(app: App) -> (egui::Context, App, egui::Rect) {
    let ctx = egui::Context::default();
    let mut app = app;
    let mut viewport = egui::Rect::NOTHING;
    let _ = ctx.run(raw_input(vec![]), |c| {
        app.update_ui(c);
        viewport = c.available_rect();
    });
    (ctx, app, viewport)
}

/// A complete primary-button click at `pos`: move, press, release.
fn click_events(pos: egui::Pos2) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

/// Click the viewport's centre: a warm-up `PointerMoved` frame (ADR 0003 §F3
/// trap 3), then the frame carrying the click itself.
fn click_anchor(ctx: &egui::Context, app: &mut App, viewport: egui::Rect) {
    let p = viewport.center();
    frame(ctx, app, vec![egui::Event::PointerMoved(p)]);
    frame(ctx, app, click_events(p));
}

fn lines_of(entities: &[Entity]) -> Vec<(Vec2, Vec2)> {
    entities
        .iter()
        .filter_map(|e| match e {
            Entity::Line(l) => Some((l.p1, l.p2)),
            _ => None,
        })
        .collect()
}

// ── AC 2 rule 1: raw means raw ──────────────────────────────────────────────

/// AC 2 rule 1 — with `TextTool` awaiting its string, `:hello` is *text*.
///
/// This is the test the demand's mutation (a) targets: move the `classify`
/// call above `submit`'s `wants_raw_input` early return and the string an
/// operator is typing into a TEXT entity is posted to a language model
/// instead. A key is configured here on purpose — without one the prefixed
/// line would be refused for the wrong reason and the mutation would survive.
#[test]
fn raw_input_wins_over_the_agent_prefix() {
    let (ctx, mut app, viewport) = boot(app_with_a_key());
    app.tool_manager.set_tool(Box::new(TextTool::default()));
    click_anchor(&ctx, &mut app, viewport);
    let anchor = app
        .last_cursor_world
        .expect("the click must have moved the cursor into the viewport");
    let ring_before = app.command_history.len();

    submit(&mut app, ":hello");

    assert_eq!(app.tool_manager.active_tool_name(), "TEXT");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "TEXT Specify height <5>:",
        "the prefixed line was consumed as the string, not routed"
    );
    assert!(!app.agent_busy, "no turn may be armed from raw input");
    assert!(app.agent_rx.is_none());
    assert!(app.agent_chat.is_empty(), "and nothing reached the panel");
    assert!(!app.agent_panel_open);
    assert_eq!(
        app.command_history.len(),
        ring_before,
        "raw-mode content never enters the recall ring (LCV-112)"
    );

    // And the string really is the one that gets cut: `:hello`, verbatim.
    submit(&mut app, "10");
    let expected = layout_text(":hello", anchor, 10.0, 1.0);
    assert_eq!(lines_of(&app.document.entities), lines_of(&expected));
}

/// AC 2 rule 1 — the same for `/ai`, and for the height prompt as well as the
/// text prompt: raw mode is not about which prefix, it is about the mode.
#[test]
fn raw_input_wins_for_the_slash_ai_prefix_too() {
    let (ctx, mut app, viewport) = boot(app_with_a_key());
    app.tool_manager.set_tool(Box::new(TextTool::default()));
    click_anchor(&ctx, &mut app, viewport);

    submit(&mut app, "/ai hello");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "TEXT Specify height <5>:"
    );
    submit(&mut app, ":10");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "TEXT Height must be between 0.1 and 2000 mm. Specify height <5>:",
        "`:10` was offered to the height parser, not to the agent"
    );
    assert!(!app.agent_busy);
    assert!(app.agent_chat.is_empty());
}

// ── AC 4: an empty prompt is refused, not sent ──────────────────────────────

/// AC 4 — a bare `:` must not reach the active tool. An implementation that
/// let the empty prompt through — or that let `:` fall through to the CAD
/// dispatch as a blank line — would send `Enter` to `PolylineTool` and finish a
/// chain the operator never meant to finish. PLINE's "finished" is its anchor
/// going away: the chain stops being open and the next point starts a new one.
///
/// The control at the end is the real blank line, which *does* finish it: it is
/// what proves this assertion can fail.
#[test]
fn a_bare_colon_does_not_finish_a_polyline() {
    let mut app = app_with_a_key();
    app.tool_manager.set_tool(Box::new(PolylineTool::default()));
    submit(&mut app, "0,0");
    submit(&mut app, "20,0");
    assert_eq!(
        app.tool_manager.anchor(),
        Some(Vec2::new(20.0, 0.0)),
        "two points in, the chain is open at the second"
    );

    submit(&mut app, ":");

    assert_eq!(app.command_feedback, "Agent prompt is empty.");
    assert_eq!(
        app.tool_manager.anchor(),
        Some(Vec2::new(20.0, 0.0)),
        "a bare `:` must not finish the polyline"
    );
    assert!(!app.agent_busy, "and nothing was sent");
    assert!(app.agent_rx.is_none());
    assert!(app.agent_chat.is_empty());
    assert!(!app.agent_panel_open);
    assert_eq!(app.document.entity_count(), 1, "and committed nothing new");

    // Control: the blank line the operator really does mean.
    submit(&mut app, "");
    assert_eq!(
        app.tool_manager.anchor(),
        None,
        "control: Enter on a blank line still finishes the polyline"
    );
}

// ── AC 6: no key, no turn ───────────────────────────────────────────────────

/// AC 6 — without a key a prefixed line says exactly that, through a real
/// frame, and does nothing else. The needle is built with `concat!` so this
/// assertion cannot be satisfied by the constant it is checking.
#[test]
fn without_a_key_a_prefixed_line_is_refused_verbatim() {
    let (ctx, mut app) = {
        let (ctx, app, _) = boot(App::default());
        (ctx, app)
    };
    assert!(app.settings.agent_api_key.is_empty());

    submit_command(&ctx, &mut app, ":draw a square");

    assert_eq!(
        app.command_feedback,
        concat!(
            "! Agent unavailable: set the API key in ",
            "Help > Agent settings"
        )
    );
    assert!(!app.agent_busy);
    assert!(app.agent_rx.is_none());
    assert!(app.agent_chat.is_empty());
    assert!(!app.agent_panel_open);
    assert_eq!(app.document.entity_count(), 0);
}

// ── AC 7: the existing answer for unknown text is unchanged ─────────────────

/// AC 7 — without a key, `lien` still answers the way LCV-111 shipped it.
///
/// The literal is deliberately **not** copied here: it is taken from the same
/// message the word `bogus` produces — the one `tests/lcv111.rs` pins in
/// `escape_clears_the_field_and_the_feedback_and_cancels_the_tool` — and
/// re-spelt for `lien`. If the shape of the message ever changes, that test
/// fails and this one follows it rather than contradicting it.
#[test]
fn without_a_key_a_typo_is_still_a_local_error() {
    let (ctx, mut app, _) = boot(App::default());

    submit_command(&ctx, &mut app, "bogus");
    let pinned = app.command_feedback.clone();
    assert!(
        pinned.contains("bogus"),
        "positive control: the pinned message names the word, got {pinned:?}"
    );

    submit_command(&ctx, &mut app, "lien");
    assert_eq!(app.command_feedback, pinned.replace("bogus", "lien"));
    assert!(!app.agent_busy, "nothing was sent");
    assert!(app.agent_chat.is_empty());
    assert!(!app.agent_panel_open);
}

// ── AC 8: a send is unmistakable ────────────────────────────────────────────

/// AC 8 — all three effects of a send, asserted synchronously so no thread can
/// race them: the echo in the command line, the panel opening by itself, and
/// the operator's own row in the transcript (written by `arm_turn`, LCV-123
/// AC 3 — asserted here, not duplicated).
#[test]
fn a_send_is_unmistakable() {
    let mut app = app_with_a_key();
    assert!(!app.agent_panel_open, "the panel starts closed");

    submit(&mut app, ":draw a 20 mm square at 10,10");

    assert_eq!(
        app.command_feedback,
        concat!("\u{2192} agent: ", "\"draw a 20 mm square at 10,10\"")
    );
    assert!(app.agent_panel_open, "the panel opens itself");
    assert_eq!(
        app.agent_chat.first(),
        Some(&("user".to_owned(), "draw a 20 mm square at 10,10".to_owned())),
        "the prompt is the transcript's first row"
    );
    assert!(app.agent_busy, "and a turn really is in flight");
}

/// AC 8 — the echo is cut at 60 characters with an `…`, so a pasted paragraph
/// cannot push the command line's own feedback off the screen.
#[test]
fn a_long_prompt_is_echoed_truncated() {
    let mut app = app_with_a_key();
    let long = "x".repeat(75);

    submit(&mut app, &format!(":{long}"));

    let echoed = app
        .command_feedback
        .trim_start_matches(concat!("\u{2192} ", "agent: \""))
        .trim_end_matches('"');
    assert_eq!(echoed.chars().count(), 61, "60 characters and the …");
    assert!(echoed.ends_with('…'));
    assert_eq!(&echoed[..60], &long[..60]);
    assert_eq!(
        app.agent_chat.first().map(|(_, text)| text.len()),
        Some(75),
        "the prompt itself is sent whole — only the echo is cut"
    );
}

/// AC 8, and the demand's "not over HTTP" — a turn the command line started is
/// a real turn: the fence, the busy flag and the channel are the ones
/// `arm_turn` sets up, and driving them by hand puts the model's geometry on
/// the operator's own bed.
///
/// Replacing `agent_rx` drops the worker's receiver, so the thread that
/// `start_turn` spawned (against [`DEAD_ENDPOINT`], where nothing listens)
/// exits `Cancelled` without a word. Everything below is this test's own
/// channel.
#[test]
fn a_turn_started_from_the_command_line_draws_on_the_real_bed() {
    let (ctx, mut app, _) = boot(app_with_a_key());
    submit(&mut app, ":draw a 20 mm line");
    assert!(app.agent_busy);

    let (tx, rx) = channel::<AgentEvent>();
    app.agent_rx = Some(rx);

    let (reply, answer) = channel();
    tx.send(AgentEvent::Act {
        action: AgentAction::CreateLine {
            x1: 0.0,
            y1: 0.0,
            x2: 20.0,
            y2: 0.0,
        },
        reply,
    })
    .expect("the app holds the receiver");
    frame(&ctx, &mut app, Vec::new());

    assert!(answer.try_recv().is_ok(), "the UI answered the action");
    assert_eq!(
        lines_of(&app.document.entities),
        [(Vec2::new(0.0, 0.0), Vec2::new(20.0, 0.0))],
        "the exact millimetres, on the live document"
    );

    tx.send(AgentEvent::Done("Drew it.".to_owned()))
        .expect("the app holds the receiver");
    frame(&ctx, &mut app, Vec::new());
    assert!(!app.agent_busy, "and the turn ends");
    assert_eq!(
        app.agent_chat.first(),
        Some(&("user".to_owned(), "draw a 20 mm line".to_owned()))
    );
}

// ── AC 9: the recall ring is unchanged ──────────────────────────────────────

/// AC 9 — an agent-routed line is in the ring like any other, so `ArrowUp`
/// brings a mistyped prompt back for editing.
#[test]
fn an_agent_routed_line_is_recalled_by_arrow_up() {
    let (ctx, mut app, _) = boot(app_with_a_key());

    submit_command(&ctx, &mut app, ":draw a squre");
    assert!(app.command_feedback.starts_with('\u{2192}'), "it was sent");

    // Focus the field and let one frame pass: `command_line_focused` mirrors
    // the *previous* frame's `has_focus()` (LCV-111 AC 23).
    type_command(&ctx, &mut app, "9");
    frame(&ctx, &mut app, Vec::new());
    assert!(app.command_line_focused);

    tap(&ctx, &mut app, egui::Key::ArrowUp, egui::Modifiers::NONE);
    assert_eq!(
        app.command_line_input, ":draw a squre",
        "the whole line, prefix included, comes back for editing"
    );
}

// ── AC 10: one turn at a time ───────────────────────────────────────────────

/// AC 10 — a second agent-routed line while a turn is in flight is refused,
/// not queued, and the in-flight turn is left exactly as it was.
#[test]
fn a_second_turn_is_refused_not_queued() {
    let mut app = app_with_a_key();
    let tx = arm_turn(&mut app, "the first prompt");
    let chat_before = app.agent_chat.clone();

    submit(&mut app, "/ai the second prompt");

    assert_eq!(
        app.command_feedback,
        concat!(
            "Agent is busy ",
            "\u{2014} wait for the current turn to finish."
        )
    );
    assert_eq!(app.agent_chat, chat_before, "the transcript is untouched");
    assert!(app.agent_busy, "the first turn is still in flight");
    assert!(app.agent_rx.is_some(), "and still holds its receiver");

    // The first turn is still drivable, which is the real claim: nothing about
    // the refusal disturbed it.
    tx.send(AgentEvent::Done("done".to_owned()))
        .expect("the armed receiver must still be on App");
    lasercad::app::poll_agent_rx(&mut app);
    assert!(!app.agent_busy);
    assert_eq!(
        app.agent_chat.last(),
        Some(&("assistant".to_owned(), "done".to_owned()))
    );
}

// ── The bare typo hazard, end to end ────────────────────────────────────────

/// The §Risks decision, made visible: with a key configured `lien` is sent,
/// loudly; without one it is a free local error. One app each, one line each.
#[test]
fn a_typo_reaches_the_agent_only_when_a_key_is_configured() {
    let mut without = App::default();
    submit(&mut without, "lien");
    assert!(!without.agent_busy);
    assert!(without.command_feedback.contains("lien"));
    assert!(!without.command_feedback.starts_with('\u{2192}'));

    let mut with = app_with_a_key();
    submit(&mut with, "lien");
    assert!(with.agent_busy, "with a key, the same typo is a turn");
    assert_eq!(
        with.command_feedback,
        concat!("\u{2192} agent: ", "\"lien\""),
        "and the command line says so at a glance"
    );
    assert!(with.agent_panel_open);
}
