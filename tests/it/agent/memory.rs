//! LCV-153 — the agent remembers earlier turns of the conversation (ADR 0007
//! §D16). The policy is unit-tested in `src/agent/memory.rs` and the worker's
//! request in `src/app/agent_worker.rs`; this file drives the UI side: the
//! Context tokens setting, and memory through `arm_turn` and real events.

use crate::harness;
use harness::paint::{self, Run};
use harness::raw_input_at;
use lasercad::agent::memory::{
    estimate_tokens, CANCELLED_TEXT, DRAWING_CHANGED_PREFIX, ELIDED_TOOL_RESULT,
};
use lasercad::agent::{AgentAction, AgentEvent, AgentOutcome, ChatMessage, ToolCall};
use lasercad::app::{arm_turn, cancel_turn, config_for, poll_agent_rx, App, AGENT_LOST_MESSAGE};
use lasercad::document::CreateLine;
use lasercad::geometry::{Line, Vec2};
use std::sync::mpsc::{channel, Sender};

const SCREEN: [f32; 2] = [1280.0, 800.0];

/// A complete primary-button click at `pos`.
fn click_events(pos: egui::Pos2) -> Vec<egui::Event> {
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    vec![egui::Event::PointerMoved(pos), button(true), button(false)]
}

/// A point inside the one painted run reading `label`.
fn locate(runs: &[Run], label: &str) -> egui::Pos2 {
    let found: Vec<&Run> = runs.iter().filter(|r| r.text.trim() == label).collect();
    assert_eq!(found.len(), 1, "`{label}` must be painted once");
    egui::pos2(found[0].pos.x + 2.0, found[0].pos.y + found[0].height / 2.0)
}

/// Hover `pos` for a frame, then click it.
fn click(ctx: &egui::Context, app: &mut App, pos: egui::Pos2) {
    let hover = vec![egui::Event::PointerMoved(pos)];
    let _ = ctx.run(raw_input_at(SCREEN, hover), |c| app.update_ui(c));
    let _ = ctx.run(raw_input_at(SCREEN, click_events(pos)), |c| {
        app.update_ui(c)
    });
}

fn frame(ctx: &egui::Context, app: &mut App, events: Vec<egui::Event>) {
    let _ = ctx.run(raw_input_at(SCREEN, events), |c| app.update_ui(c));
}

/// AC 8 — "Context tokens" is one integer field in Agent Settings: a real
/// click and typed value, closed with Done, is persisted by the LCV-141 path.
#[test]
fn ac8_the_context_tokens_edit_persists_through_done() {
    let dir = std::env::temp_dir().join("lcv153_context_tokens");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("settings.json");
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let mut app = App {
        settings_path: Some(path.clone()),
        ..App::default()
    };
    app.agent_settings_open = true;
    frame(&ctx, &mut app, Vec::new());

    let runs = paint::painted_runs(&ctx, &mut app);
    assert!(runs.iter().any(|r| r.text.trim() == "Context tokens"));
    click(&ctx, &mut app, locate(&runs, "128000"));
    frame(&ctx, &mut app, vec![egui::Event::Text("64000".to_owned())]);
    frame(
        &ctx,
        &mut app,
        harness::key_events(egui::Key::Enter, egui::Modifiers::NONE),
    );
    assert_eq!(app.settings.agent_context_tokens, 64_000);

    let runs = paint::painted_runs(&ctx, &mut app);
    click(&ctx, &mut app, locate(&runs, "Done"));
    assert!(!app.agent_settings_open, "Done closes the dialog");
    let saved = std::fs::read_to_string(&path).expect("Done persists the settings");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(saved.contains("\"agent_context_tokens\": 64000"), "{saved}");
}

// ── Memory through real turn exits ──────────────────────────────────────────

/// One whole batch: a call and its result.
fn one_batch(id: &str) -> Vec<ChatMessage> {
    vec![
        ChatMessage::assistant_with_tool_calls(
            None,
            vec![ToolCall::function(id, "query_entities", "{}")],
        ),
        ChatMessage::tool_result(id, "No entities."),
    ]
}

fn record(user: &str, batches: Vec<ChatMessage>, closing: &str) -> Vec<ChatMessage> {
    let mut turn = vec![ChatMessage::user(user)];
    turn.extend(batches);
    turn.push(ChatMessage::assistant(closing));
    turn
}

/// Arm a turn, send `event`, drain it.
fn run_turn(app: &mut App, prompt: &str, event: AgentEvent) {
    let tx = arm_turn(app, prompt);
    tx.send(event).expect("the armed receiver is on App");
    poll_agent_rx(app);
    assert!(!app.agent.busy);
}

/// Apply one line through the turn's rendezvous, as a worker would.
fn act_line(app: &mut App, tx: &Sender<AgentEvent>) {
    let (reply, answer) = channel::<AgentOutcome>();
    let action = AgentAction::CreateLine {
        layer: None,
        x1: 0.0,
        y1: 0.0,
        x2: 5.0,
        y2: 0.0,
    };
    tx.send(AgentEvent::Act { action, reply }).unwrap();
    poll_agent_rx(app);
    assert!(matches!(answer.try_recv(), Ok(AgentOutcome::Ok(_))));
}

fn human_line(app: &mut App) {
    let line = Line::new(Vec2::new(0.0, 1.0), Vec2::new(5.0, 1.0));
    app.commit(Box::new(CreateLine::new(line)));
}

fn prefixed(prompt: &str) -> String {
    format!("{DRAWING_CHANGED_PREFIX}\n{prompt}")
}

/// AC 11 — memory starts empty, with no mark.
#[test]
fn ac11_app_default_starts_with_empty_memory() {
    let app = App::default();
    assert!(app.agent.memory.is_empty());
    assert_eq!(app.agent.memory_mark, None);
    assert!(config_for(&app).memory.is_empty());
}

/// AC 1 / AC 10 — `Done` appends user, the event's batches in order, then
/// the final text.
#[test]
fn ac1_a_done_turn_is_recorded_whole() {
    let mut app = App::default();
    let batches = [one_batch("a"), one_batch("b")].concat();
    run_turn(
        &mut app,
        "draw",
        AgentEvent::Done("drawn".into(), batches.clone()),
    );
    assert_eq!(app.agent.memory.turns(), [record("draw", batches, "drawn")]);
}

/// AC 5 — `Failed` appends user, its batches and `Turn stopped: <error>.`
#[test]
fn ac5_a_failed_turn_closes_with_the_stopped_sentence() {
    let mut app = App::default();
    let event = AgentEvent::Failed("transport error: 503".into(), one_batch("a"));
    run_turn(&mut app, "draw", event);
    let expected = record(
        "draw",
        one_batch("a"),
        "Turn stopped: transport error: 503.",
    );
    assert_eq!(app.agent.memory.turns(), [expected]);
}

/// AC 6 — a cancelled turn is its user message and the cancelled text.
#[test]
fn ac6_a_cancelled_turn_is_user_and_cancelled_text() {
    let mut app = App::default();
    let _tx = arm_turn(&mut app, "draw");
    cancel_turn(&mut app);
    assert_eq!(
        app.agent.memory.turns(),
        [record("draw", Vec::new(), CANCELLED_TEXT)]
    );
}

/// AC 5 — both lost-worker exits record the user message and the stopped
/// sentence for `AGENT_LOST_MESSAGE`.
#[test]
fn ac5_a_lost_worker_is_recorded_as_stopped() {
    let lost = format!(
        "Turn stopped: {}.",
        AGENT_LOST_MESSAGE.trim_end_matches('.')
    );
    // Exit 3: the channel disconnects.
    let mut app = App::default();
    drop(arm_turn(&mut app, "draw"));
    poll_agent_rx(&mut app);
    assert_eq!(
        app.agent.memory.turns(),
        [record("draw", Vec::new(), &lost)]
    );
    // Exit 4: nobody reads the answer to an `Act`.
    let mut app = App::default();
    let tx = arm_turn(&mut app, "draw");
    let (reply, answer) = channel::<AgentOutcome>();
    drop(answer);
    let action = AgentAction::QueryEntities;
    tx.send(AgentEvent::Act { action, reply }).unwrap();
    poll_agent_rx(&mut app);
    assert!(!app.agent.busy);
    assert_eq!(
        app.agent.memory.turns(),
        [record("draw", Vec::new(), &lost)]
    );
}

/// AC 7 — after a `Done` turn, the next user message is prefixed only when
/// the document changed: a commit, undo, redo, File > New or Open.
#[test]
fn ac7_a_change_since_the_last_turn_prefixes_the_next_user_message() {
    let dir = std::env::temp_dir().join("lcv153_reopen");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let svg = dir.join("one.svg");
    std::fs::write(
        &svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200" viewBox="0 0 200 200"><line x1="1" y1="1" x2="9" y2="1" stroke="#ff0000" stroke-width="0.1"/></svg>"##,
    )
    .unwrap();
    type Change = fn(&mut App, &std::path::Path);
    let changes: [(&str, Change); 6] = [
        ("nothing", |_, _| {}),
        ("commit", |app, _| human_line(app)),
        ("undo", |app, _| {
            assert!(app.history.undo(&mut app.document))
        }),
        // Applied below, around a second mark.
        ("redo", |_, _| {}),
        ("new", |app, _| app.action_new()),
        ("open", |app, path| app.action_open_path(path.to_owned())),
    ];
    for (name, change) in changes {
        let mut app = App::default();
        human_line(&mut app);
        run_turn(&mut app, "first", AgentEvent::done("ok"));
        if name == "redo" {
            // Undo before the mark, so only the redo is the change.
            assert!(app.history.undo(&mut app.document));
            run_turn(&mut app, "again", AgentEvent::done("ok"));
            assert!(app.history.redo(&mut app.document));
        } else {
            change(&mut app, &svg);
        }
        let _tx = arm_turn(&mut app, "next");
        let expected = match name {
            "nothing" => "next".to_owned(),
            _ => prefixed("next"),
        };
        assert_eq!(app.agent.turn.user, expected, "{name}");
        assert_eq!(app.agent.chat.last().unwrap().1, "next", "{name}: raw row");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// AC 7 — no prefix with empty memory, however much changed; the first turn
/// sends the prompt as typed.
#[test]
fn ac7_empty_memory_is_never_prefixed() {
    let mut app = App::default();
    human_line(&mut app);
    app.action_new();
    let _tx = arm_turn(&mut app, "first");
    assert_eq!(app.agent.turn.user, "first");
}

/// AC 7 / ADR 0007 §D16 — the mark advances only on `Done`: a turn that
/// applied an action and ended `Done` leaves no prefix, while `Failed` or a
/// cancel after an applied action does. A cancel with nothing applied leaves
/// none.
#[test]
fn ac7_only_done_advances_the_mark() {
    let ends: [(&str, bool, bool); 4] = [
        ("done", true, false),
        ("failed", true, true),
        ("cancel", true, true),
        ("cancel, nothing applied", false, false),
    ];
    for (name, apply, want_prefix) in ends {
        let mut app = App::default();
        run_turn(&mut app, "first", AgentEvent::done("ok"));
        let tx = arm_turn(&mut app, "draw");
        if apply {
            act_line(&mut app, &tx);
        }
        match name {
            "done" => tx.send(AgentEvent::done("drew")).unwrap(),
            "failed" => tx.send(AgentEvent::failed("boom")).unwrap(),
            _ => cancel_turn(&mut app),
        }
        poll_agent_rx(&mut app);
        assert!(!app.agent.busy, "{name}");
        let _tx = arm_turn(&mut app, "next");
        let expected = if want_prefix {
            prefixed("next")
        } else {
            "next".to_owned()
        };
        assert_eq!(app.agent.turn.user, expected, "{name}");
    }
}

/// AC 9 — over-cap memory is trimmed when the next turn is armed; the
/// setting is clamped first, so a stored `0` means 8 000.
#[test]
fn ac9_memory_over_the_cap_is_trimmed_at_arm() {
    let result = |bytes: usize| "r".repeat(bytes);
    let turn = |n: usize, bytes: usize| {
        let batch = vec![
            ChatMessage::assistant_with_tool_calls(
                None,
                vec![ToolCall::function("c", "query_entities", "{}")],
            ),
            ChatMessage::tool_result("c", result(bytes)),
        ];
        record(&format!("turn {n}"), batch, "ok")
    };
    // ~5 000 tokens: over the 4 000 cap of the clamped 8 000. Eliding the old
    // result lands under the 2 000 target, so both turns stay.
    let mut app = App::default();
    app.settings.agent_context_tokens = 0;
    app.agent.memory.push_turn(turn(0, 16_000));
    app.agent.memory.push_turn(turn(1, 4_000));
    let _tx = arm_turn(&mut app, "next");
    let turns = app.agent.memory.turns();
    assert_eq!(turns.len(), 2);
    assert_eq!(turns[0][2].text_content(), Some(ELIDED_TOOL_RESULT));
    assert_eq!(turns[1][2].text_content(), Some(result(4_000).as_str()));
    // At or under the cap, nothing moves.
    let mut app = App::default();
    app.settings.agent_context_tokens = 0;
    app.agent.memory.push_turn(turn(0, 4_000));
    app.agent.memory.push_turn(turn(1, 8_000));
    let before = app.agent.memory.clone();
    assert!(estimate_tokens(&before.flatten()) <= 4_000);
    let _tx = arm_turn(&mut app, "next");
    assert_eq!(app.agent.memory, before);
}

/// AC 10 — the turn's config carries exactly the flattened memory, as it
/// stands after the arm.
#[test]
fn ac10_config_for_clones_the_flattened_memory() {
    let mut app = App::default();
    run_turn(
        &mut app,
        "one",
        AgentEvent::Done("1".into(), one_batch("a")),
    );
    run_turn(&mut app, "two", AgentEvent::failed("boom"));
    let _tx = arm_turn(&mut app, "three");
    let config = config_for(&app);
    assert_eq!(config.memory, app.agent.memory.flatten());
    assert_eq!(config.memory.len(), 6);
    assert!(config.memory.iter().all(|m| m.role != "system"));
}
