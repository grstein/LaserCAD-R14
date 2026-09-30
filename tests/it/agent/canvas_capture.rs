//! LCV-145 — opt-in canvas observations, driven through real frames;
//! LCV-187 — the `frame` argument and the post-send notes.
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
use lasercad::agent::{
    AgentAction, AgentEvent, AgentOutcome, CaptureFrame, parse_tool_call, tool_definitions,
};
use lasercad::app::{AGENT_FENCE_REFUSAL, App, arm_turn};
use lasercad::document::{CreateCircle, CreateLine};
use lasercad::geometry::{Circle, Line, Vec2};
use serde_json::{Value, json};
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
    capture_frame_in_one_frame(ctx, app, tx, CaptureFrame::View)
}

/// Answer one `CaptureCanvas(frame)` on an armed turn in one `update_ui`.
fn capture_frame_in_one_frame(
    ctx: &egui::Context,
    app: &mut App,
    tx: &Sender<AgentEvent>,
    frame: CaptureFrame,
) -> AgentOutcome {
    let answer = push_act(tx, AgentAction::CaptureCanvas(frame));
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

/// AC 2 — at 800×600 the Agent Settings dialog paints both opt-in checkboxes
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

// ── LCV-187: the `frame` argument ────────────────────────────────────────

fn parse(args: Value) -> Result<AgentAction, String> {
    parse_tool_call("capture_canvas", &args).map_err(|e| e.to_string())
}

fn region(x0: f64, y0: f64, x1: f64, y1: f64) -> CaptureFrame {
    CaptureFrame::Region { x0, y0, x1, y1 }
}

/// LCV-187 AC 1 — no arguments, `{}`, a null or `"view"` frame, and stray
/// keys all parse to today's viewport capture.
#[test]
fn no_frame_or_view_parses_to_the_viewport() {
    for args in [
        Value::Null,
        json!({}),
        json!({"frame": null}),
        json!({"frame": "view"}),
        json!({"frame": "view", "x0": null}),
        json!({"stray": 1}),
    ] {
        assert_eq!(
            parse(args.clone()),
            Ok(AgentAction::CaptureCanvas(CaptureFrame::View)),
            "{args}"
        );
    }
    assert_eq!(
        parse(json!({"frame": "drawing"})),
        Ok(AgentAction::CaptureCanvas(CaptureFrame::Drawing))
    );
    assert_eq!(
        parse(json!({"frame": "region", "x0": 30, "y0": 40.5, "x1": -10, "y1": 0})),
        Ok(AgentAction::CaptureCanvas(region(30.0, 40.5, -10.0, 0.0))),
        "corners are kept as given; the capture normalises them"
    );
}

/// LCV-187 AC 4 — a frame name outside the three, a missing or non-numeric
/// region corner, and a corner given with another frame are refused, each
/// naming its field.
#[test]
fn a_bad_frame_or_corner_is_refused_naming_the_field() {
    let frame = "tool `capture_canvas` argument `frame` is invalid: \
                 must be \"view\", \"drawing\" or \"region\"";
    for bad in [json!("bed"), json!("VIEW"), json!(3), json!(true)] {
        assert_eq!(parse(json!({"frame": bad})), Err(frame.to_owned()), "{bad}");
    }
    let full = json!({"frame": "region", "x0": 0, "y0": 0, "x1": 10, "y1": 10});
    for field in ["x0", "y0", "x1", "y1"] {
        for bad in [Value::Null, json!("inf"), json!("NaN"), json!([1])] {
            let mut args = full.clone();
            args[field] = bad.clone();
            assert_eq!(
                parse(args),
                Err(format!(
                    "tool `capture_canvas` missing required argument `{field}`"
                )),
                "{field} = {bad}"
            );
        }
        let mut args = full.clone();
        args.as_object_mut().unwrap().remove(field);
        assert!(parse(args).is_err(), "{field} absent");
    }
    for name in ["view", "drawing"] {
        assert_eq!(
            parse(json!({"frame": name, "y1": 5})),
            Err("tool `capture_canvas` argument `y1` is invalid: \
                 only used with frame \"region\""
                .to_owned()),
            "{name}"
        );
    }
    assert!(
        serde_json::from_str::<Value>(r#"{"x0":1e999}"#).is_err(),
        "a non-finite corner cannot reach the parser: JSON has none"
    );
}

/// LCV-187 AC 7 — with the opt-ins off `capture_canvas` stays unadvertised
/// and every frame is refused as today.
#[test]
fn opt_ins_off_leave_every_frame_unadvertised_and_refused() {
    let named = |vision: bool| {
        tool_definitions(vision)
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["function"]["name"] == "capture_canvas")
    };
    assert!(named(true), "positive control: advertised with vision");
    assert!(!named(false));
    for frame in [
        CaptureFrame::View,
        CaptureFrame::Drawing,
        region(0.0, 0.0, 9.0, 9.0),
    ] {
        let (ctx, mut app) = ctx_and_app();
        draw_something(&mut app);
        idle(&ctx, &mut app);
        let tx = arm_turn(&mut app, "look");
        pin_camera(&mut app);
        assert_eq!(
            capture_frame_in_one_frame(&ctx, &mut app, &tx, frame.clone()),
            AgentOutcome::Refused(DISABLED.into()),
            "{frame:?}"
        );
    }
}

/// The PNG's `(width, height)`, and whether any pixel is inked black.
fn dims_and_ink(png: &[u8]) -> ((u32, u32), bool) {
    let decoder = png::Decoder::new(std::io::Cursor::new(png));
    let mut reader = decoder.read_info().expect("png header");
    let mut buf = vec![0; reader.output_buffer_size().expect("size")];
    let info = reader.next_frame(&mut buf).expect("png frame");
    buf.truncate(info.buffer_size());
    ((info.width, info.height), buf.contains(&0))
}

/// Capture `frame` on a fresh app holding `entities` lines, the camera
/// pinned elsewhere; returns the outcome, the last row and the revision.
fn capture_lines(
    lines: &[((f64, f64), (f64, f64))],
    frame: CaptureFrame,
) -> (AgentOutcome, (String, String), u64) {
    let (ctx, mut app) = ctx_and_app();
    for &((ax, ay), (bx, by)) in lines {
        app.commit(Box::new(CreateLine::new(Line::new(
            Vec2::new(ax, ay),
            Vec2::new(bx, by),
        ))));
    }
    allow(&mut app, true, true);
    idle(&ctx, &mut app);
    let tx = arm_turn(&mut app, "look");
    pin_camera(&mut app);
    let outcome = capture_frame_in_one_frame(&ctx, &mut app, &tx, frame);
    let row = app.agent.chat.last().cloned().expect("a row");
    assert_eq!(app.agent.turn.steps, 1, "a framed capture is one step");
    (outcome, row, app.history.revision())
}

fn observed(outcome: AgentOutcome) -> (String, Vec<u8>) {
    match outcome {
        AgentOutcome::Observed { text, png } => (text, png),
        other => panic!("expected Observed, got {other:?}"),
    }
}

fn canvas_text(w: u32, h: u32, x: &str, y: &str, revision: u64) -> String {
    format!(
        "Canvas {w}×{h} px of X {x} mm, Y {y} mm \
         (Y up; bed outline grey, entities black), revision {revision}."
    )
}

/// LCV-187 AC 2 / AC 5 — `"drawing"` frames the extents plus 5 % of the
/// longest extent on each side, whatever the viewport shows, and renders
/// the longest edge at exactly 1024 px, aspect kept, under 2 MiB; the
/// outcome text reports that frame.
#[test]
fn the_drawing_frame_is_the_extents_plus_five_percent_at_1024() {
    // Extents X 10..210, Y 20..120: margin 10 mm → 220 × 120 mm.
    let lines = [
        ((10.0, 20.0), (210.0, 120.0)),
        ((50.0, 100.0), (60.0, 20.0)),
    ];
    let (outcome, (role, row), revision) = capture_lines(&lines, CaptureFrame::Drawing);
    let (text, png) = observed(outcome);
    let expected = canvas_text(1024, 559, "0.000..220.000", "10.000..130.000", revision);
    assert_eq!(text, expected);
    assert_eq!((role.as_str(), row.as_str()), ("tool", expected.as_str()));
    assert_eq!(dims_and_ink(&png), ((1024, 559), true));
    assert!(png.len() < 2 * 1024 * 1024);

    // A lone horizontal line still frames an area: the margin is the
    // longest extent's, on both axes.
    let (outcome, _, revision) =
        capture_lines(&[((0.0, 0.0), (100.0, 0.0))], CaptureFrame::Drawing);
    let (text, png) = observed(outcome);
    assert_eq!(
        text,
        canvas_text(1024, 93, "-5.000..105.000", "-5.000..5.000", revision)
    );
    assert_eq!(dims_and_ink(&png), ((1024, 93), true));
}

/// LCV-187 AC 3 / AC 5 — `"region"` frames exactly that rectangle, corners
/// in any order, longest edge 1024 px even when that upscales a tiny one.
#[test]
fn a_region_is_framed_exactly_at_1024() {
    let lines = [((0.0, 0.0), (20.0, 20.0))];
    let frame = CaptureFrame::Region {
        x0: 30.0,
        y0: 40.5,
        x1: -10.0,
        y1: 0.0,
    };
    let (outcome, _, revision) = capture_lines(&lines, frame);
    let (text, png) = observed(outcome);
    assert_eq!(
        text,
        canvas_text(1011, 1024, "-10.000..30.000", "0.000..40.500", revision)
    );
    assert_eq!(dims_and_ink(&png), ((1011, 1024), true));

    let tiny = CaptureFrame::Region {
        x0: 1.0,
        y0: 1.0,
        x1: 1.5,
        y1: 1.25,
    };
    let (outcome, _, revision) = capture_lines(&lines, tiny);
    let (text, png) = observed(outcome);
    assert_eq!(
        text,
        canvas_text(1024, 512, "1.000..1.500", "1.000..1.250", revision)
    );
    assert_eq!(
        dims_and_ink(&png),
        ((1024, 512), true),
        "the diagonal crosses it"
    );
    assert!(png.len() < 2 * 1024 * 1024, "{} bytes", png.len());
}

/// LCV-187 AC 4 — a frame with no area is refused naming the cause, is
/// transcribed `refused`, and captures nothing.
#[test]
fn a_frame_without_area_is_refused_naming_the_cause() {
    let region = |x0, y0, x1, y1| CaptureFrame::Region { x0, y0, x1, y1 };
    let one = [((1.0, 1.0), (9.0, 9.0))];
    let cases: [(&[((f64, f64), (f64, f64))], CaptureFrame, &str); 8] = [
        (
            &[],
            CaptureFrame::Drawing,
            "the drawing is empty; nothing to capture",
        ),
        (
            &[((5.0, 5.0), (5.0, 5.0))],
            CaptureFrame::Drawing,
            "the drawing has no area; nothing to capture",
        ),
        (
            &one,
            region(5.0, 5.0, 5.0, 20.0),
            "the region has no area; nothing to capture",
        ),
        (
            &one,
            region(5.0, 5.0, 20.0, 5.0),
            "the region has no area; nothing to capture",
        ),
        (
            &one,
            region(f64::NAN, 0.0, 1.0, 1.0),
            "the region is not finite; nothing to capture",
        ),
        (
            &one,
            region(0.0, 0.0, 1.0, f64::INFINITY),
            "the region is not finite; nothing to capture",
        ),
        (
            &one,
            region(0.0, f64::NEG_INFINITY, 1.0, 1.0),
            "the region is not finite; nothing to capture",
        ),
        (
            &one,
            region(-1e308, 0.0, 1e308, 1.0),
            "the region is not finite; nothing to capture",
        ),
    ];
    for (lines, frame, reason) in cases {
        let (outcome, (role, row), _) = capture_lines(lines, frame.clone());
        assert_eq!(
            outcome,
            AgentOutcome::Refused(reason.to_owned()),
            "{frame:?}"
        );
        assert_eq!((role.as_str(), row.as_str()), ("refused", reason));
    }
}
