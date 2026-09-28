//! LCV-150 — a New Conversation button in the agent panel's header empties
//! the transcript and the agent's memory (LCV-153). The `AgentState` method is
//! unit-tested in `src/app/agent_state.rs`, and the empty memory reaching the
//! next `TurnConfig` in `src/app/agent_turn.rs`; this file drives the painted
//! button with real clicks (ADR 0002 §A4 rule 3).

use crate::harness;
use harness::paint::{painted_runs_at, Run};
use harness::raw_input_at;
use lasercad::agent::{AgentEvent, ChatMessage};
use lasercad::app::{arm_turn, App};
use lasercad::document::CreateLine;
use lasercad::geometry::{Line, Vec2};

const SCREEN: [f32; 2] = [1280.0, 800.0];
const SMALL: [f32; 2] = [800.0, 600.0];
const LABEL: &str = "New Conversation";

/// A headless context at `pixels_per_point == 1.0` and an `App` with the
/// agent panel open.
fn ctx_and_app() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let mut app = App::default();
    app.agent.panel_open = true;
    (ctx, app)
}

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

fn frame(ctx: &egui::Context, app: &mut App, screen: [f32; 2], events: Vec<egui::Event>) {
    let _ = ctx.run(raw_input_at(screen, events), |c| app.update_ui(c));
}

/// Hover `pos` for a frame, then click it.
fn click(ctx: &egui::Context, app: &mut App, screen: [f32; 2], pos: egui::Pos2) {
    frame(ctx, app, screen, vec![egui::Event::PointerMoved(pos)]);
    frame(ctx, app, screen, click_events(pos));
}

/// The agent panel's persisted outer rect.
fn panel_rect(ctx: &egui::Context) -> egui::Rect {
    egui::containers::panel::PanelState::load(ctx, egui::Id::new("agent_panel"))
        .expect("the agent panel must have stored its state by now")
        .rect
}

/// The full painted rect (position and galley size) of every text run reading
/// `label` — `Run` carries no width, and AC 7 needs the right edge.
fn label_rects(shapes: &[egui::epaint::ClippedShape], label: &str) -> Vec<egui::Rect> {
    fn walk(shape: &egui::Shape, label: &str, out: &mut Vec<egui::Rect>) {
        match shape {
            egui::Shape::Text(t) if t.galley.text().trim() == label => {
                out.push(egui::Rect::from_min_size(t.pos, t.galley.size()));
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, label, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    shapes.iter().for_each(|c| walk(&c.shape, label, &mut out));
    out
}

/// Three transcript rows, two remembered turns and one committed line.
fn talked(app: &mut App) {
    for (role, text) in [
        ("user", "LCV150-A"),
        ("tool", "LCV150-B"),
        ("assistant", "LCV150-C"),
    ] {
        app.agent.chat.push((role.to_owned(), text.to_owned()));
    }
    for prompt in ["one", "two"] {
        app.agent.memory.push_turn(vec![
            ChatMessage::user(prompt),
            ChatMessage::assistant("ok"),
        ]);
    }
    let line = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
    app.commit(Box::new(CreateLine::new(line)));
}

fn fill_short_transcript(app: &mut App, n: usize) {
    for i in 0..n {
        app.agent
            .chat
            .push(("assistant".to_owned(), format!("LCV150-ROW-{i}")));
    }
}

/// AC 1 — the label is painted once, inside the agent panel.
#[test]
fn ac1_the_button_is_painted_inside_the_agent_panel() {
    let (ctx, mut app) = ctx_and_app();
    frame(&ctx, &mut app, SCREEN, Vec::new());
    let out = ctx.run(raw_input_at(SCREEN, Vec::new()), |c| app.update_ui(c));
    let rects = label_rects(&out.shapes, LABEL);
    assert_eq!(rects.len(), 1, "`{LABEL}` must be painted once");
    let panel = panel_rect(&ctx);
    assert!(
        panel.contains_rect(rects[0]),
        "{:?} outside {panel:?}",
        rects[0]
    );
}

/// AC 2 / AC 4 / AC 5 — an idle click empties the transcript and the memory,
/// paints no row afterwards, and starts nothing and changes no drawing.
#[test]
fn ac2_ac4_ac5_an_idle_click_clears_the_conversation_and_nothing_else() {
    let (ctx, mut app) = ctx_and_app();
    talked(&mut app);
    let (id, revision, depth) = (app.history.id(), app.history.revision(), app.history.len());
    let entities = app.document.entities.len();

    let runs = painted_runs_at(&ctx, &mut app, SCREEN, Vec::new());
    assert!(runs.iter().any(|r| r.text.contains("LCV150-C")), "control");
    click(&ctx, &mut app, SCREEN, locate(&runs, LABEL));

    assert!(app.agent.chat.is_empty());
    assert!(app.agent.memory.is_empty());
    let runs = painted_runs_at(&ctx, &mut app, SCREEN, Vec::new());
    assert!(
        !runs.iter().any(|r| r.text.contains("LCV150-")),
        "empty transcript"
    );
    assert_eq!(app.history.id(), id);
    assert_eq!(app.history.revision(), revision);
    assert_eq!(app.history.len(), depth);
    assert_eq!(app.document.entities.len(), entities);
    assert!(!app.agent.busy);
    assert!(app.agent.rx.is_none(), "no turn started");
}

/// The colours the one text run reading `label` was painted with —
/// `(fallback_color, override_text_color)`; a disabled widget is tinted.
fn label_look(
    shapes: &[egui::epaint::ClippedShape],
    label: &str,
) -> (egui::Color32, Option<egui::Color32>) {
    fn walk(
        shape: &egui::Shape,
        label: &str,
        out: &mut Vec<(egui::Color32, Option<egui::Color32>)>,
    ) {
        match shape {
            egui::Shape::Text(t) if t.galley.text().trim() == label => {
                out.push((t.fallback_color, t.override_text_color));
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, label, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    shapes.iter().for_each(|c| walk(&c.shape, label, &mut out));
    assert_eq!(out.len(), 1, "`{label}` must be painted once");
    out[0]
}

/// AC 3 / AC 6 — while a turn runs the button is painted greyed and a click
/// is inert; the very frame whose poll ends the turn paints it enabled, and
/// a click then clears the transcript and the memory the turn recorded.
#[test]
fn ac3_ac6_a_busy_click_is_inert_and_the_ending_frame_reenables() {
    let (ctx, mut app) = ctx_and_app();
    talked(&mut app);
    let runs = painted_runs_at(&ctx, &mut app, SCREEN, Vec::new());
    let pos = locate(&runs, LABEL);
    let idle = ctx.run(raw_input_at(SCREEN, Vec::new()), |c| app.update_ui(c));
    let idle = label_look(&idle.shapes, LABEL);

    let tx = arm_turn(&mut app, "draw");
    let busy = ctx.run(raw_input_at(SCREEN, Vec::new()), |c| app.update_ui(c));
    assert_ne!(
        label_look(&busy.shapes, LABEL),
        idle,
        "greyed the frame busy starts"
    );
    click(&ctx, &mut app, SCREEN, pos);
    assert!(app.agent.busy);
    assert_eq!(app.agent.chat.len(), 4, "three rows and the prompt");
    assert_eq!(app.agent.memory.turns().len(), 2);

    // The pointer leaves while still busy, so the idle look is not the
    // hovered one.
    frame(&ctx, &mut app, SCREEN, vec![egui::Event::PointerGone]);
    tx.send(AgentEvent::done("drawn")).expect("receiver armed");
    let ended = ctx.run(raw_input_at(SCREEN, Vec::new()), |c| app.update_ui(c));
    assert!(!app.agent.busy, "the turn ended this frame");
    assert_eq!(
        label_look(&ended.shapes, LABEL),
        idle,
        "enabled the same frame"
    );
    assert_eq!(app.agent.memory.turns().len(), 3, "the turn was recorded");

    click(&ctx, &mut app, SCREEN, pos);
    assert!(app.agent.chat.is_empty());
    assert!(app.agent.memory.is_empty());
}

/// AC 7 — at 800×600, with 200 rows, idle and busy, the button lies wholly
/// inside the panel and the screen, beside the heading and `×` without
/// overlapping either; an idle click on it clears.
#[test]
fn ac7_the_button_is_reachable_at_800x600() {
    for busy in [false, true] {
        let (ctx, mut app) = ctx_and_app();
        fill_short_transcript(&mut app, 200);
        app.agent.busy = busy;
        frame(&ctx, &mut app, SMALL, Vec::new());
        let out = ctx.run(raw_input_at(SMALL, Vec::new()), |c| app.update_ui(c));
        let button = label_rects(&out.shapes, LABEL);
        assert_eq!(button.len(), 1, "busy={busy}: painted once");
        let button = button[0];
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, SMALL.into());
        assert!(
            panel_rect(&ctx).contains_rect(button),
            "busy={busy}: {button:?}"
        );
        assert!(screen.contains_rect(button), "busy={busy}: {button:?}");
        for other in ["AI Assistant", "×"] {
            let rect = label_rects(&out.shapes, other)[0];
            assert!(!rect.intersects(button), "busy={busy}: overlaps `{other}`");
        }
        if !busy {
            click(&ctx, &mut app, SMALL, button.center());
            assert!(app.agent.chat.is_empty());
        }
    }
}
