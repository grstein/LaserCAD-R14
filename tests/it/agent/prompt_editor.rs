//! LCV-143 AC 3, AC 4 and AC 8 — the System prompt editor in AI Settings,
//! driven headless through the real `App` and the real
//! `src/app/panels.rs::agent_settings_dialog`, persisting to a test-owned
//! `settings_path` (ADR 0006), never the real one.

use crate::harness;

use harness::key_events;
use harness::paint::{Run, painted_runs_at};
use lasercad::agent::DEFAULT_PROMPT;
use lasercad::app::App;
use lasercad::io::settings::Settings;
use std::path::PathBuf;

const SCREEN: [f32; 2] = [1280.0, 800.0];

/// The first line of the built-in prompt, as the editor paints it.
const DEFAULT_FIRST_LINE: &str = "You are the CAD assistant";

/// A private, empty directory under the system temp dir.
fn tempdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lcv143_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the system temp dir must be writable");
    dir
}

/// A pixels-per-point 1.0 context and an `App` persisting to `dir`.
fn ctx_and_app(dir: &std::path::Path, prompt: Option<&str>) -> (egui::Context, App) {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let mut app = App {
        settings_path: Some(dir.join("settings.json")),
        ..App::default()
    };
    app.settings.agent_system_prompt = prompt.map(str::to_owned);
    (ctx, app)
}

fn run(ctx: &egui::Context, app: &mut App, events: Vec<egui::Event>) -> Vec<Run> {
    painted_runs_at(ctx, app, SCREEN, events)
}

/// Open AI Settings and settle it (trap 7: the first frame is unplaced).
fn open(ctx: &egui::Context, app: &mut App) -> Vec<Run> {
    app.agent_settings_open = true;
    let _ = run(ctx, app, Vec::new());
    run(ctx, app, Vec::new())
}

/// A point on the first line of the one run whose trimmed text starts with
/// `prefix`. The editor paints its whole text as one multi-line galley, so the
/// point sits a few points below the run's top, not at its vertical middle.
fn locate(runs: &[Run], prefix: &str) -> egui::Pos2 {
    let hits: Vec<&Run> = runs
        .iter()
        .filter(|r| r.text.trim().starts_with(prefix))
        .collect();
    assert_eq!(hits.len(), 1, "`{prefix}` must be painted exactly once");
    egui::pos2(hits[0].pos.x + 2.0, hits[0].pos.y + 6.0)
}

/// A real pointer click at `pos`, preceded by its own hover frame (ADR 0002
/// §A4 rule 3). Returns the click frame's runs.
fn click(ctx: &egui::Context, app: &mut App, pos: egui::Pos2) -> Vec<Run> {
    let _ = run(ctx, app, vec![egui::Event::PointerMoved(pos)]);
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    run(
        ctx,
        app,
        vec![egui::Event::PointerMoved(pos), button(true), button(false)],
    )
}

/// Click Done and return what the test-owned file persisted.
fn done(ctx: &egui::Context, app: &mut App) -> Settings {
    let runs = run(ctx, app, Vec::new());
    let _ = click(ctx, app, locate(&runs, "Done"));
    assert!(!app.agent_settings_open, "Done closes the dialog");
    let path = app.settings_path.clone().expect("a test-owned path");
    let json = std::fs::read_to_string(path).expect("closing persists the settings");
    serde_json::from_str(&json).expect("the persisted settings parse")
}

/// AC 3 — opening and closing AI Settings without touching the field
/// creates no override, and keeps an existing one exactly.
#[test]
fn ac3_an_untouched_editor_creates_no_override() {
    assert!(DEFAULT_PROMPT.starts_with(DEFAULT_FIRST_LINE), "control");
    for (name, prompt) in [("none", None), ("some", Some("  mine\n\n "))] {
        let dir = tempdir(&format!("untouched_{name}"));
        let (ctx, mut app) = ctx_and_app(&dir, prompt);
        let runs = open(&ctx, &mut app);
        let expected_text = prompt.map_or(DEFAULT_FIRST_LINE, |_| "mine");
        assert!(
            runs.iter()
                .any(|r| r.text.trim().starts_with(expected_text)),
            "control: the editor paints the effective prompt"
        );
        let persisted = done(&ctx, &mut app);
        assert_eq!(app.settings.agent_system_prompt.as_deref(), prompt);
        assert_eq!(persisted.agent_system_prompt.as_deref(), prompt);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// AC 3 + AC 4 — text typed across several frames lands verbatim (no trim,
/// newline kept), persists through Done, and is what a reopened dialog shows.
#[test]
fn ac3_typed_text_persists_verbatim_through_done() {
    let dir = tempdir("typed");
    let (ctx, mut app) = ctx_and_app(&dir, None);
    let runs = open(&ctx, &mut app);
    let _ = click(&ctx, &mut app, locate(&runs, DEFAULT_FIRST_LINE));
    let _ = run(
        &ctx,
        &mut app,
        key_events(egui::Key::A, egui::Modifiers::COMMAND),
    );
    for events in [
        vec![egui::Event::Text("Draw ".into())],
        vec![egui::Event::Text("in mm".into())],
        key_events(egui::Key::Enter, egui::Modifiers::NONE),
        vec![egui::Event::Text("  keep  ".into())],
    ] {
        let _ = run(&ctx, &mut app, events);
    }
    let typed = "Draw in mm\n  keep  ";
    assert_eq!(app.settings.agent_system_prompt.as_deref(), Some(typed));

    let persisted = done(&ctx, &mut app);
    assert_eq!(persisted.agent_system_prompt.as_deref(), Some(typed));

    let runs = open(&ctx, &mut app);
    assert!(runs.iter().any(|r| r.text == typed), "the editor shows it");
    assert!(
        !runs
            .iter()
            .any(|r| r.text.trim().starts_with(DEFAULT_FIRST_LINE))
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// AC 4 — Restore Default clears the override and the editor shows the
/// built-in text on the very frame of the click; Done persists `None`.
#[test]
fn ac4_restore_default_clears_the_override_on_the_same_frame() {
    let dir = tempdir("restore");
    let (ctx, mut app) = ctx_and_app(&dir, Some("CUSTOM-PROMPT"));
    let runs = open(&ctx, &mut app);
    assert!(
        runs.iter().any(|r| r.text.trim() == "CUSTOM-PROMPT"),
        "control"
    );

    let runs = click(&ctx, &mut app, locate(&runs, "Restore Default"));
    assert_eq!(app.settings.agent_system_prompt, None);
    assert!(
        runs.iter()
            .any(|r| r.text.trim().starts_with(DEFAULT_FIRST_LINE)),
        "the built-in text is painted on the click frame"
    );
    assert!(!runs.iter().any(|r| r.text.contains("CUSTOM-PROMPT")));

    let persisted = done(&ctx, &mut app);
    assert_eq!(persisted.agent_system_prompt, None);
    let json = std::fs::read_to_string(dir.join("settings.json")).expect("persisted");
    assert!(json.contains(r#""agent_system_prompt": null"#), "{json}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// AC 8 — at 800x600 with a long multi-paragraph prompt, every control of
/// the dialog is painted inside its clip at the top of the body or after
/// scrolling the body; none is clipped outright by the window bound.
#[test]
fn ac8_every_control_is_reachable_at_800x600_with_a_long_prompt() {
    let long = (0..40)
        .map(|i| format!("Paragraph {i}: draw carefully in millimeters.\n"))
        .collect::<String>();
    let dir = tempdir("ac8");
    let (ctx, mut app) = ctx_and_app(&dir, Some(&long));
    app.agent_settings_open = true;
    let small = [800.0, 600.0];
    let _ = painted_runs_at(&ctx, &mut app, small, Vec::new());
    let top = painted_runs_at(&ctx, &mut app, small, Vec::new());

    // Scroll the dialog body, hovering a label outside the prompt's own
    // scroll area so the wheel moves the body.
    let anchor = locate(&top, "Endpoint URL");
    let mut events = vec![egui::Event::PointerMoved(anchor)];
    events.extend((0..200).map(|_| egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -7.0),
        modifiers: egui::Modifiers::NONE,
    }));
    let _ = painted_runs_at(&ctx, &mut app, small, events);
    let bottom = painted_runs_at(&ctx, &mut app, small, Vec::new());

    let contained = |runs: &[Run], label: &str| {
        runs.iter().any(|r| {
            r.text.trim().starts_with(label)
                && r.pos.y >= r.clip.top()
                && r.pos.y + r.height <= r.clip.bottom()
        })
    };
    for label in [
        "Endpoint URL",
        "Model",
        "API Key",
        "The API key is stored in plain text",
        "Steps per turn",
        "System prompt",
        "Restore Default",
        "Done",
    ] {
        assert!(
            contained(&top, label) || contained(&bottom, label),
            "`{label}` must be painted inside its clip at 800x600"
        );
    }
    assert!(
        top.iter()
            .any(|r| r.text.trim().starts_with("Paragraph 0:")),
        "control: the long prompt is loaded into the editor"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
