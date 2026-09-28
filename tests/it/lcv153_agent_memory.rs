//! LCV-153 — the agent remembers earlier turns of the conversation (ADR 0007
//! §D16). The policy is unit-tested in `src/agent/memory.rs` and the worker's
//! request in `src/app/agent_worker.rs`; this file drives the UI side: the
//! Context tokens setting, and memory through `arm_turn` and real events.

use crate::harness;
use harness::paint::{self, Run};
use harness::raw_input_at;
use lasercad::app::App;

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
