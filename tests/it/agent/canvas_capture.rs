//! LCV-145 — opt-in canvas observations, driven through real frames.
//!
//! Like `tests/it/agent/turn.rs`, every test pushes `Act`s by hand onto a turn
//! armed with `arm_turn` and reads the answers off its own reply `Receiver`:
//! no thread, no socket, no endpoint (AC 12). The PNG is decoded with `png`
//! and compared as bytes.
//!
//! ADR 0002 §A2: `App::default()` only. ADR 0005: no dialog is armed and no
//! test sends `Ctrl+O` / `Ctrl+S`. ADR 0006: no per-user path is injected.

use crate::harness;

use harness::frame;
use lasercad::agent::{AgentAction, AgentEvent, AgentOutcome};
use lasercad::app::{AGENT_FENCE_REFUSAL, App, arm_turn};
use lasercad::document::{CreateCircle, CreateLine};
use lasercad::geometry::{Circle, Line, Vec2};
use std::sync::mpsc::{Receiver, Sender, channel};

const DISABLED: &str = "canvas capture is disabled in Agent settings";

fn ctx_and_app() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let app = App::default();
    assert!(app.settings_path.is_none() && app.autosave_path.is_none());
    (ctx, app)
}

fn idle(ctx: &egui::Context, app: &mut App) {
    frame(ctx, app, Vec::new());
}

fn push_act(tx: &Sender<AgentEvent>, action: AgentAction) -> Receiver<AgentOutcome> {
    let (reply, answer) = channel::<AgentOutcome>();
    tx.send(AgentEvent::Act { action, reply })
        .expect("the armed Receiver must still be on App");
    answer
}

fn allow(app: &mut App, allow: bool, supports: bool) {
    app.settings.agent_allow_canvas_capture = allow;
    app.settings.agent_model_supports_vision = supports;
}

/// A fixed camera: 800×600 px at 0.5 mm/px, centred on the default bed.
fn pin_camera(app: &mut App) {
    app.camera.center_world = Vec2::new(200.0, 200.0);
    app.camera.mm_per_px = 0.5;
    app.camera.viewport_size_px = [800.0, 600.0];
}

/// A small drawing, committed before any turn is armed.
fn draw_something(app: &mut App) {
    app.commit(Box::new(CreateLine::new(Line::new(
        Vec2::new(10.0, 10.0),
        Vec2::new(390.0, 390.0),
    ))));
    app.commit(Box::new(CreateCircle::new(Circle::new(
        Vec2::new(200.0, 200.0),
        60.0,
    ))));
}

/// Answer one `CaptureCanvas` on an armed turn in exactly one `update_ui`.
fn capture_in_one_frame(
    ctx: &egui::Context,
    app: &mut App,
    tx: &Sender<AgentEvent>,
) -> AgentOutcome {
    let answer = push_act(tx, AgentAction::CaptureCanvas);
    idle(ctx, app);
    answer
        .try_recv()
        .expect("AC 6: the Act is answered within the same update_ui call")
}

/// AC 2 — of the four live combinations only (on, on) observes; the other
/// three get the pinned refusal, transcribed as `refused`.
#[test]
fn only_both_live_opt_ins_observe() {
    for (on_allow, on_supports) in [(false, false), (true, false), (false, true), (true, true)] {
        let (ctx, mut app) = ctx_and_app();
        idle(&ctx, &mut app);
        let tx = arm_turn(&mut app, "look");
        allow(&mut app, on_allow, on_supports);
        pin_camera(&mut app);
        let outcome = capture_in_one_frame(&ctx, &mut app, &tx);
        let (role, row) = app.agent.chat.last().cloned().expect("a row");
        if on_allow && on_supports {
            let AgentOutcome::Observed { text, png } = outcome else {
                panic!("(on, on) must observe");
            };
            assert!(
                text.starts_with("Canvas 800×600 px of X 0.000..400.000 mm"),
                "{text}"
            );
            assert_eq!(&png[1..4], b"PNG");
            assert_eq!((role.as_str(), row.as_str()), ("tool", text.as_str()));
        } else {
            assert_eq!(
                outcome,
                AgentOutcome::Refused(DISABLED.into()),
                "({on_allow}, {on_supports})"
            );
            assert_eq!((role.as_str(), row.as_str()), ("refused", DISABLED));
        }
        assert_eq!(app.agent.turn.steps, 1, "a capture is one step");
    }
}

/// AC 2 — a capture behind a tripped fence is `Fenced` like every action.
#[test]
fn a_fenced_capture_is_fenced() {
    let (ctx, mut app) = ctx_and_app();
    idle(&ctx, &mut app);
    let tx = arm_turn(&mut app, "look");
    allow(&mut app, true, true);
    app.commit(Box::new(CreateCircle::new(Circle::new(
        Vec2::new(5.0, 5.0),
        1.0,
    ))));
    pin_camera(&mut app);
    assert_eq!(
        capture_in_one_frame(&ctx, &mut app, &tx),
        AgentOutcome::Fenced(AGENT_FENCE_REFUSAL.to_owned())
    );
}

/// The PNG a capture produces with `setup` applied to the UI first. The
/// camera is pinned after the settling frame, so every variant frames the
/// same world rectangle: what may differ is UI state only.
fn png_with(setup: impl FnOnce(&egui::Context, &mut App)) -> Vec<u8> {
    let (ctx, mut app) = ctx_and_app();
    draw_something(&mut app);
    allow(&mut app, true, true);
    setup(&ctx, &mut app);
    idle(&ctx, &mut app);
    let tx = arm_turn(&mut app, "look");
    pin_camera(&mut app);
    match capture_in_one_frame(&ctx, &mut app, &tx) {
        AgentOutcome::Observed { png, .. } => png,
        other => panic!("expected Observed, got {other:?}"),
    }
}

/// A UI state applied before the settle frame.
type Setup = dyn FnOnce(&egui::Context, &mut App);

/// AC 6 — the image is a function of the document and the camera only: the
/// agent panel, the settings window, a pointer resting on the toolbar, a
/// selection, a hover snap and a tool preview change no byte of it.
#[test]
fn ui_state_never_reaches_the_png() {
    let plain = png_with(|_, _| {});
    let decoded = decode(&plain);
    assert!(decoded.contains(&0), "positive control: entities are inked");
    assert!(decoded.contains(&128), "positive control: the bed is drawn");

    let variants: [(&str, Box<Setup>); 6] = [
        (
            "agent panel",
            Box::new(|_, app| app.agent.panel_open = true),
        ),
        (
            "settings window",
            Box::new(|_, app| app.agent_settings_open = true),
        ),
        (
            "pointer on the toolbar",
            Box::new(|ctx, app| {
                for _ in 0..3 {
                    frame(
                        ctx,
                        app,
                        vec![egui::Event::PointerMoved(egui::pos2(12.0, 30.0))],
                    );
                }
            }),
        ),
        (
            "selection",
            Box::new(|_, app| app.document.selection.add(1)),
        ),
        (
            "hover preview",
            Box::new(|_, app| {
                app.last_cursor_world = Some(Vec2::new(100.0, 100.0));
                app.preview_entities = vec![lasercad::document::Entity::Circle(Circle::new(
                    Vec2::new(100.0, 100.0),
                    40.0,
                ))];
            }),
        ),
        (
            "grid off",
            Box::new(|_, app| app.grid_enabled = !app.grid_enabled),
        ),
    ];
    for (name, setup) in variants {
        assert_eq!(png_with(setup), plain, "{name} leaked into the image");
    }
}

fn decode(png: &[u8]) -> Vec<u8> {
    let decoder = png::Decoder::new(std::io::Cursor::new(png));
    let mut reader = decoder.read_info().expect("png header");
    let mut buf = vec![0; reader.output_buffer_size().expect("size")];
    let info = reader.next_frame(&mut buf).expect("png frame");
    assert_eq!((info.width, info.height), (800, 600));
    buf.truncate(info.buffer_size());
    buf
}

/// Answer one `AuthorizeUpload` in one frame.
fn authorize_in_one_frame(
    ctx: &egui::Context,
    app: &mut App,
    tx: &Sender<AgentEvent>,
) -> AgentOutcome {
    let action = AgentAction::AuthorizeUpload {
        endpoint: app.settings.agent_endpoint.clone(),
        model: app.settings.agent_model.clone(),
    };
    let answer = push_act(tx, action);
    idle(ctx, app);
    answer
        .try_recv()
        .expect("answered within the same update_ui call")
}

/// AC 11 — `AuthorizeUpload` is not a step and is not transcribed except for
/// the disclosure note; a yes writes the pinned `note` row.
#[test]
fn authorize_upload_is_not_a_step_and_a_yes_discloses() {
    let (ctx, mut app) = ctx_and_app();
    idle(&ctx, &mut app);
    let tx = arm_turn(&mut app, "look");
    allow(&mut app, true, true);
    let rows = app.agent.chat.len();
    let model = app.settings.agent_model.clone();
    let yes = authorize_in_one_frame(&ctx, &mut app, &tx);
    assert!(matches!(yes, AgentOutcome::Ok(_)), "{yes:?}");
    assert_eq!(app.agent.turn.steps, 0, "not a step");
    assert_eq!(app.agent.chat.len(), rows + 1);
    assert_eq!(
        app.agent.chat.last().cloned(),
        Some((
            "note".to_owned(),
            format!("Sending a canvas image to {model}.")
        ))
    );
}

/// AC 11 — it bypasses the fence: answered (yes) after the fence tripped.
#[test]
fn authorize_upload_is_answered_after_the_fence_tripped() {
    let (ctx, mut app) = ctx_and_app();
    idle(&ctx, &mut app);
    let tx = arm_turn(&mut app, "look");
    allow(&mut app, true, true);
    app.commit(Box::new(CreateCircle::new(Circle::new(
        Vec2::new(5.0, 5.0),
        1.0,
    ))));
    let fenced = push_act(&tx, AgentAction::QueryEntities);
    idle(&ctx, &mut app);
    assert!(
        fenced.try_recv().unwrap().is_fenced(),
        "positive control: fence tripped"
    );
    let steps = app.agent.turn.steps;
    let yes = authorize_in_one_frame(&ctx, &mut app, &tx);
    assert!(matches!(yes, AgentOutcome::Ok(_)), "{yes:?}");
    assert_eq!(
        app.agent.turn.steps, steps,
        "not counted behind the fence either"
    );
}

/// AC 11 — changing `agent_model` live after the capture turns the answer
/// into a no, and a no leaves no row.
#[test]
fn a_live_model_change_refuses_the_upload() {
    let (ctx, mut app) = ctx_and_app();
    idle(&ctx, &mut app);
    let tx = arm_turn(&mut app, "look");
    allow(&mut app, true, true);
    pin_camera(&mut app);
    assert!(matches!(
        capture_in_one_frame(&ctx, &mut app, &tx),
        AgentOutcome::Observed { .. }
    ));
    let action = AgentAction::AuthorizeUpload {
        endpoint: app.settings.agent_endpoint.clone(),
        model: app.settings.agent_model.clone(),
    };
    app.settings.agent_model = "someone/else".to_owned();
    let rows = app.agent.chat.len();
    let answer = push_act(&tx, action);
    idle(&ctx, &mut app);
    assert!(answer.try_recv().unwrap().is_refused());
    assert_eq!(app.agent.chat.len(), rows, "a no leaves no row");
}

/// AC 2 — at 800×600 the AI Settings dialog paints both opt-in checkboxes
/// and the exact disclosure sentence.
#[test]
fn agent_settings_paints_both_opt_ins_and_the_disclosure() {
    let (ctx, mut app) = ctx_and_app();
    app.agent_settings_open = true;
    let small = [800.0, 600.0];
    let _ = harness::paint::painted_runs_at(&ctx, &mut app, small, Vec::new());
    let runs = harness::paint::painted_runs_at(&ctx, &mut app, small, Vec::new());
    let painted = |text: &str| runs.iter().any(|r| r.text.trim() == text);
    assert!(
        painted("Endpoint URL"),
        "positive control: the dialog is open"
    );
    for text in [
        "Allow canvas capture",
        "Model supports images",
        "When both are on, the agent may send a picture of the drawing (not the window) \
         to the configured provider and model.",
    ] {
        assert!(painted(text), "{text:?} is not painted");
    }
}

/// Every `src/` file as `(relative path, body)`, the path rebuilt from
/// `components()` joined with `/`. `bounded` cuts each body at its column-0
/// `#[cfg(test)]` (scan rule 1).
fn src_sections(bounded: bool) -> Vec<(String, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    harness::scan::rs_files(&root.join("src"), &mut files);
    if bounded {
        files.retain(|path| !harness::scan::is_test_file(path));
    }
    let mut sections: Vec<(String, String)> = files
        .iter()
        .map(|path| {
            let relative = path
                .strip_prefix(root)
                .expect("under the manifest dir")
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            let body = std::fs::read_to_string(path).expect("readable source");
            let end = match (bounded, body.find("\n#[cfg(test)]")) {
                (true, Some(end)) => end,
                _ => body.len(),
            };
            (relative, body[..end].to_owned())
        })
        .collect();
    sections.sort();
    assert!(sections.len() > 50, "control: the walk found the tree");
    sections
}

/// AC 7 — `render/raster.rs` is kernel-pure: the whole file, inline tests
/// included (as LCV-128 AC 1 scans the kernel), names no GUI crate.
#[test]
fn the_raster_imports_no_gui_crate() {
    let sections = src_sections(false);
    let needles = [
        concat!("eg", "ui"),
        concat!("efr", "ame"),
        concat!("rf", "d::"),
    ];
    let hits = harness::scan::files_containing(&sections, needles[0]);
    assert!(
        hits.iter().any(|f| f == "src/render/camera.rs"),
        "positive control: the matcher finds egui in render/camera.rs"
    );
    let raster: Vec<_> = sections
        .into_iter()
        .filter(|(path, _)| path == "src/render/raster.rs")
        .collect();
    assert_eq!(raster.len(), 1, "control: raster.rs was read");
    for needle in needles {
        assert_eq!(
            harness::scan::files_containing(&raster, needle),
            Vec::<String>::new()
        );
    }
}

/// AC 1 — no framebuffer read exists: no source names egui's screenshot
/// command or event.
#[test]
fn nothing_requests_a_screenshot() {
    let needles = [
        concat!("ViewportCommand::", "Screenshot"),
        concat!("Event::", "Screenshot"),
    ];
    let control = vec![(
        "control.rs".to_owned(),
        format!("ctx.send_viewport_cmd({}(Default::default()));", needles[0]),
    )];
    assert_eq!(
        harness::scan::files_containing(&control, needles[0]),
        ["control.rs"]
    );
    let sections = src_sections(false);
    for needle in needles {
        assert_eq!(
            harness::scan::files_containing(&sections, needle),
            Vec::<String>::new()
        );
    }
}

/// AC 7 — base64 is the wire's business alone.
#[test]
fn only_the_wire_encodes_base64() {
    let needle = concat!("base", "64");
    let cargo = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("Cargo.toml");
    assert!(
        cargo.contains(&format!("{needle} = ")),
        "control: the dependency is declared"
    );
    assert_eq!(
        harness::scan::files_containing(&src_sections(true), needle),
        ["src/agent/wire.rs"],
        "positive control and claim at once: wire.rs, and nothing else"
    );
}

/// AC 8 — the transcript carries the outcome text only: no PNG signature and
/// no data URL reaches `agent.chat`, even after an authorised upload.
#[test]
fn no_image_bytes_reach_the_transcript() {
    let (ctx, mut app) = ctx_and_app();
    draw_something(&mut app);
    allow(&mut app, true, true);
    idle(&ctx, &mut app);
    let tx = arm_turn(&mut app, "look");
    pin_camera(&mut app);
    let AgentOutcome::Observed { png, .. } = capture_in_one_frame(&ctx, &mut app, &tx) else {
        panic!("expected Observed");
    };
    assert!(matches!(
        authorize_in_one_frame(&ctx, &mut app, &tx),
        AgentOutcome::Ok(_)
    ));

    let needles = [concat!("iVB", "OR"), concat!("data:", "image")];
    let part = lasercad::agent::wire::ContentPart::png(&png);
    let wire = serde_json::to_string(&lasercad::agent::ChatMessage::user_parts(vec![part]))
        .expect("serialisable");
    for needle in needles {
        assert!(
            wire.contains(needle),
            "positive control: {needle} is on the wire"
        );
        for (role, text) in &app.agent.chat {
            assert!(!text.contains(needle), "{role} row carries {needle}");
        }
    }
    assert!(app.agent.chat.iter().any(|(_, t)| t.starts_with("Canvas ")));
}

/// AC 12 — no logging or printing call anywhere in `src/` carries the PNG.
#[test]
fn no_log_call_carries_the_png() {
    let macros = [
        concat!("print", "!("),
        concat!("println", "!("),
        concat!("eprint", "!("),
        concat!("eprintln", "!("),
        concat!("dbg", "!("),
        concat!("log", "::"),
        concat!("trac", "ing"),
    ];
    let png = concat!("pn", "g");
    let logs_png = |body: &str| {
        body.lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .any(|l| macros.iter().any(|m| l.contains(m)) && l.contains(png))
    };
    assert!(
        logs_png(&format!("    {}\"{{:?}}\", {png});", macros[3])),
        "positive control: the matcher fires on a png print"
    );
    for (path, body) in src_sections(true) {
        assert!(!logs_png(&body), "{path} logs the png");
    }
}
