//! LCV-199 — the agent's reference image: one test per acceptance criterion.
//!
//! The native picker is never opened (ADR 0005): a file is attached through
//! `lasercad::app::attach_image`, the seam the picker's result feeds, and
//! the `Attach image…` click is driven on a frame that draws the agent panel
//! alone, so the frame wiring that would open the picker does not run.
//! Turns that send run against a `mockito` endpoint.

use crate::harness::paint::{Run, runs_in};
use crate::harness::raw_input_at;
use lasercad::agent::draw_agent_panel;
use lasercad::agent::wire::{ChatMessage, ContentPart};
use lasercad::app::{App, Severity, attach_image, config_for, start_turn};
use std::path::PathBuf;

const SCREEN: [f32; 2] = [1280.0, 800.0];
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";
const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0];

/// A fresh file `name` holding `bytes` in this process's scratch directory.
fn scratch(name: &str, bytes: &[u8]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lcv199_it_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("a scratch file");
    path
}

/// A headless context at 1 pt/px and an `App` whose model sees images.
fn ctx_and_app() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let mut app = App::default();
    app.agent.panel_open = true;
    app.settings.agent_model_supports_vision = true;
    (ctx, app)
}

/// One frame drawing only the agent panel — never `update_ui`, whose
/// frame wiring would answer a press by opening the native picker.
fn panel_frame(ctx: &egui::Context, app: &mut App, events: Vec<egui::Event>) -> Vec<Run> {
    let out = ctx.run(raw_input_at(SCREEN, events), |c| {
        egui::SidePanel::right("agent_panel").show(c, |ui| draw_agent_panel(ui, app));
    });
    runs_in(&out.shapes)
}

/// The one run reading `label` exactly.
fn find<'a>(runs: &'a [Run], label: &str) -> &'a Run {
    let found: Vec<&Run> = runs.iter().filter(|r| r.text.trim() == label).collect();
    assert_eq!(found.len(), 1, "`{label}` must be painted once");
    found[0]
}

fn centre(run: &Run) -> egui::Pos2 {
    egui::pos2(run.pos.x + 2.0, run.pos.y + run.height / 2.0)
}

/// Hover `pos` for a frame, then click it.
fn click(ctx: &egui::Context, app: &mut App, pos: egui::Pos2) {
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    panel_frame(ctx, app, vec![egui::Event::PointerMoved(pos)]);
    panel_frame(ctx, app, vec![button(true), button(false)]);
}

/// Run `update_ui` frames until the turn ends.
fn run_until_idle(ctx: &egui::Context, app: &mut App) {
    for _ in 0..400 {
        let _ = ctx.run(raw_input_at(SCREEN, Vec::new()), |c| app.update_ui(c));
        if !app.agent.busy {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("still busy after ~2 s: {:?}", app.agent.chat);
}

/// A server answering every request with the text `reply`, matched only
/// when the body matches `body`.
fn endpoint(app: &mut App, body: &str) -> (mockito::ServerGuard, mockito::Mock) {
    let mut server = mockito::Server::new();
    let mock = server
        .mock("POST", "/chat/completions")
        .match_body(mockito::Matcher::Regex(body.to_owned()))
        .with_status(200)
        .with_body(r#"{"choices":[{"message":{"role":"assistant","content":"Drawn."}}]}"#)
        .expect(1)
        .create();
    app.settings.agent_endpoint = server.url();
    app.settings.agent_api_key = "k".to_owned();
    (server, mock)
}

/// AC 1 — pressing `Attach image…` raises the picker request; the chosen
/// file shows as a chip with its name, and the chip's `×` removes it.
#[test]
fn ac1_the_button_requests_the_picker_and_the_chip_is_removable() {
    let (ctx, mut app) = ctx_and_app();
    panel_frame(&ctx, &mut app, Vec::new());
    let runs = panel_frame(&ctx, &mut app, Vec::new());
    click(&ctx, &mut app, centre(find(&runs, "Attach image…")));
    assert!(app.agent.attach_requested, "the press raises the request");

    attach_image(&mut app, &scratch("ac1_sketch.png", PNG));
    panel_frame(&ctx, &mut app, Vec::new());
    let runs = panel_frame(&ctx, &mut app, Vec::new());
    let chip = find(&runs, "ac1_sketch.png");
    let attach = find(&runs, "Attach image…");
    assert!(
        (chip.pos.y - attach.pos.y).abs() < 6.0,
        "the chip sits in the attach row"
    );
    let prompt_row = find(&runs, "Ask the AI…");
    assert!(
        chip.pos.y < prompt_row.pos.y,
        "the chip is above the prompt"
    );
    let remove = runs
        .iter()
        .find(|r| {
            r.text.trim() == "×" && r.pos.x > chip.pos.x && (r.pos.y - chip.pos.y).abs() < 6.0
        })
        .expect("the chip paints its ×");
    click(&ctx, &mut app, centre(remove));
    assert!(app.agent.attachment.is_none(), "× removes the attachment");
}

/// AC 2 — the sent prompt carries the image as `image/jpeg` in its user
/// message, the transcript gets `Image: <name>`, and the attachment clears.
#[test]
fn ac2_the_image_rides_the_user_message_and_is_cleared() {
    let (ctx, mut app) = ctx_and_app();
    let body = concat!(
        r#"\{"role":"user","content":\[\{"type":"text","text":"draw the bracket"\},"#,
        r#"\{"type":"image_url","image_url":\{"url":"data:image/jpeg;base64,/9j/4A=="\}\}\]\}"#
    );
    let (_server, mock) = endpoint(&mut app, body);
    attach_image(&mut app, &scratch("ac2_part.jpg", JPEG));
    start_turn(&mut app, "draw the bracket");
    run_until_idle(&ctx, &mut app);
    mock.assert();
    assert!(app.agent.attachment.is_none());
    let rows: Vec<(&str, &str)> = app
        .agent
        .chat
        .iter()
        .map(|(r, c)| (r.as_str(), c.as_str()))
        .collect();
    assert_eq!(
        rows[..2],
        [
            ("user", "draw the bracket"),
            ("user", "Image: ac2_part.jpg")
        ]
    );
    assert!(rows.contains(&("assistant", "Drawn.")), "{rows:?}");
}

/// AC 3 — with `Model supports images` off the button is disabled and its
/// hover paints the tooltip naming the setting.
#[test]
fn ac3_the_button_is_disabled_with_its_tooltip_while_vision_is_off() {
    let (ctx, mut app) = ctx_and_app();
    app.settings.agent_model_supports_vision = false;
    panel_frame(&ctx, &mut app, Vec::new());
    let runs = panel_frame(&ctx, &mut app, Vec::new());
    let pos = centre(find(&runs, "Attach image…"));
    panel_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(pos)]);
    let hover = panel_frame(&ctx, &mut app, Vec::new());
    assert!(
        hover
            .iter()
            .any(|r| r.text.trim() == "Turn on \"Model supports images\" in agent settings."),
        "{:?}",
        hover.iter().map(|r| &r.text).collect::<Vec<_>>()
    );
    click(&ctx, &mut app, pos);
    assert!(
        !app.agent.attach_requested,
        "a disabled button raises nothing"
    );
}

/// AC 4 — a GIF and a PNG one byte over 2 MB are refused with the reason
/// in the status line; nothing is attached. 2 MB exactly is accepted.
#[test]
fn ac4_a_gif_or_an_oversized_file_is_refused_with_its_reason() {
    let (_, mut app) = ctx_and_app();
    attach_image(&mut app, &scratch("ac4_cat.gif", b"GIF89a\x01\x00"));
    assert!(app.agent.attachment.is_none());
    assert_eq!(
        app.command_feedback,
        "Image not attached: ac4_cat.gif is not a PNG or JPEG."
    );
    assert_eq!(app.command_feedback_severity, Severity::Warning);

    let mut big = PNG.to_vec();
    big.resize(2 * 1024 * 1024 + 1, 0);
    attach_image(&mut app, &scratch("ac4_big.png", &big));
    assert!(app.agent.attachment.is_none());
    assert_eq!(
        app.command_feedback,
        "Image not attached: ac4_big.png is over 2 MB."
    );

    big.pop();
    attach_image(&mut app, &scratch("ac4_edge.png", &big));
    assert_eq!(
        app.agent.attachment.map(|a| a.name),
        Some("ac4_edge.png".to_owned())
    );
}

/// AC 5 — a file gone by send time sends nothing: an `error` row says so,
/// the prompt is back in the draft, and no turn is armed to send one.
#[test]
fn ac5_an_unreadable_file_sends_nothing_and_keeps_the_prompt() {
    let (_, mut app) = ctx_and_app();
    let path = scratch("ac5_gone.png", PNG);
    attach_image(&mut app, &path);
    std::fs::remove_file(&path).expect("the file can be deleted");
    start_turn(&mut app, "draw from the photo");
    assert!(!app.agent.busy && app.agent.rx.is_none(), "no turn armed");
    assert_eq!(app.agent.input_draft, "draw from the photo");
    let (role, row) = app.agent.chat.last().expect("a transcript row");
    assert_eq!(role, "error");
    assert!(
        row.starts_with("Image ac5_gone.png could not be read: "),
        "{row}"
    );
    assert!(row.ends_with("Nothing was sent."), "{row}");
}

/// AC 6 — the next turn's request holds `image elided` where the image was.
#[test]
fn ac6_the_next_request_carries_image_elided() {
    let (ctx, mut app) = ctx_and_app();
    let (_server, _mock) = endpoint(&mut app, "image_url");
    attach_image(&mut app, &scratch("ac6_sketch.png", PNG));
    start_turn(&mut app, "trace the sketch");
    run_until_idle(&ctx, &mut app);
    let memory = config_for(&app).memory;
    assert_eq!(
        memory[0],
        ChatMessage::user_parts(vec![
            ContentPart::text("trace the sketch"),
            ContentPart::text("image elided"),
        ])
    );
    let sent = serde_json::to_string(&memory).expect("memory serialises");
    assert!(!sent.contains("image_url"), "{sent}");
}

/// AC 7 — with nothing attached the user message is today's plain string,
/// and no image part is sent.
#[test]
fn ac7_without_an_image_the_request_is_unchanged() {
    let (ctx, mut app) = ctx_and_app();
    let body = r#"\{"role":"user","content":"plain prompt"\}\]"#;
    let (_server, mock) = endpoint(&mut app, body);
    start_turn(&mut app, "plain prompt");
    run_until_idle(&ctx, &mut app);
    mock.assert();
    assert!(config_for(&app).image.is_none());
    assert!(app.agent.chat.iter().all(|(_, c)| !c.starts_with("Image:")));
}
