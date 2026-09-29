//! LCV-142 AC 4 / AC 11 — the thinking row reports progress as painted text.
//!
//! The count is kept UI-side in `app.agent.turn` (ADR 0007 §D13): one step per
//! `Act` the frame loop receives, with no progress event. So these tests push
//! `Act`s by hand onto a turn armed with `arm_turn` and read the panel's paint
//! list — no thread, no socket, no endpoint.
//!
//! ADR 0002 §A2: `App::default()` only. ADR 0005: no dialog is armed and no
//! test sends `Ctrl+O` / `Ctrl+S`. ADR 0006: no per-user path is injected.

use crate::harness;

use harness::paint::{lines_on_surface_of, painted_runs, texts};
use lasercad::agent::{AgentAction, AgentEvent, AgentOutcome};
use lasercad::app::{App, arm_turn};
use std::sync::mpsc::{Receiver, Sender, channel};

fn ctx_and_app() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let app = App::default();
    assert!(app.settings_path.is_none() && app.autosave_path.is_none());
    (ctx, app)
}

fn push_act(tx: &Sender<AgentEvent>, action: AgentAction) -> Receiver<AgentOutcome> {
    let (reply, answer) = channel::<AgentOutcome>();
    tx.send(AgentEvent::Act { action, reply })
        .expect("the armed Receiver must still be on App");
    answer
}

/// The panel's painted lines, after a settling frame.
fn panel_lines(ctx: &egui::Context, app: &mut App) -> Vec<Vec<String>> {
    app.agent.panel_open = true;
    let _ = painted_runs(ctx, app);
    texts(&lines_on_surface_of(
        &painted_runs(ctx, app),
        "AI Assistant",
    ))
}

fn has(lines: &[Vec<String>], text: &str) -> bool {
    lines.iter().flatten().any(|t| t == text)
}

/// AC 4 — three step `Act`s (a mutation, a query and a malformed call) read
/// `Thinking… 3 of 256 steps` on a default turn; before any, `0 of 256`.
#[test]
fn ac4_the_thinking_row_counts_every_act_against_the_limit() {
    let (ctx, mut app) = ctx_and_app();
    let tx = arm_turn(&mut app, "draw a few things");

    let lines = panel_lines(&ctx, &mut app);
    assert!(has(&lines, "Thinking… 0 of 256 steps"), "{lines:?}");

    let _answers = [
        push_act(
            &tx,
            AgentAction::CreateLine {
                layer: None,
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 0.0,
            },
        ),
        push_act(&tx, AgentAction::QueryEntities),
        push_act(
            &tx,
            AgentAction::Malformed {
                tool: "nope".into(),
                reason: "unknown tool: `nope`".into(),
            },
        ),
    ];
    let lines = panel_lines(&ctx, &mut app);
    assert!(app.agent.busy, "Acts are not verdicts");
    assert_eq!(app.agent.turn.steps, 3);
    assert!(has(&lines, "Thinking… 3 of 256 steps"), "{lines:?}");
    assert!(
        !has(&lines, "Thinking… 0 of 256 steps"),
        "control: the old count is gone: {lines:?}"
    );
}

/// AC 11 — the limit is the turn-start snapshot: armed at 7, then the setting
/// changes to 9 mid-turn, and the row still reads `of 7 steps`.
#[test]
fn ac11_the_row_shows_the_snapshotted_limit() {
    let (ctx, mut app) = ctx_and_app();
    app.settings.agent_step_budget = 7;
    let _tx = arm_turn(&mut app, "small turn");
    app.settings.agent_step_budget = 9;

    let lines = panel_lines(&ctx, &mut app);
    assert!(has(&lines, "Thinking… 0 of 7 steps"), "{lines:?}");
    assert!(!has(&lines, "Thinking… 0 of 9 steps"), "{lines:?}");
}

/// AC 2 / AC 4 — the row shows the *effective* limit: a stored 5000 is clamped
/// to 4096 where the turn reads it, and the stored value itself is untouched.
#[test]
fn the_row_shows_the_clamped_limit_not_the_stored_one() {
    let (ctx, mut app) = ctx_and_app();
    app.settings.agent_step_budget = 5000;
    let _tx = arm_turn(&mut app, "big turn");
    let lines = panel_lines(&ctx, &mut app);
    assert!(has(&lines, "Thinking… 0 of 4096 steps"), "{lines:?}");
    assert_eq!(app.settings.agent_step_budget, 5000, "kept verbatim");
}
