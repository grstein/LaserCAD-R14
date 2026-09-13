//! LCV-120 — the canvas asks for a follow-up frame only while something is live.
//!
//! `src/app/viewport.rs` used to call `ctx.request_repaint()` unconditionally on
//! every frame, so the application never idled by its own choice: it asked the
//! windowing system to wake it again immediately even when nothing at all was
//! happening. What that costs is platform-dependent — a compositor that
//! throttles a surface which is not visibly changing declines the request — but
//! nothing in this repo decided that, and the blanket request also masked the
//! two conditional wake-ups underneath it, `schedule_flush_repaint` above all.
//!
//! These tests drive the real [`App::update_ui`] through `egui::Context::run`
//! and assert on `FullOutput`'s `repaint_delay`: `Duration::MAX` means "the app
//! asked for nothing and egui may sleep", `Duration::ZERO` means "wake me
//! immediately", anything between is a scheduled wake-up.
//!
//! # Two traps, both of which have already produced a test that cannot fail
//!
//! 1. **Frame 0 of a fresh `egui::Context` always reports 0 ns**, whatever the
//!    frame body asked for — it is still settling its layout. Every assertion
//!    about the *absence* of a repaint request must therefore be made from
//!    frame 1 on. [`boot`] spends frame 0 for that reason (the same rule is
//!    documented on `tests/lcv116_autosave_repaint.rs::settle`).
//! 2. **An asserting frame must carry no input event at all.** If a frame
//!    carries any event — including a repeated `PointerMoved` at the position
//!    the pointer is already at — egui reports `repaint_delay == 0` regardless
//!    of what the frame body requested. A version of
//!    [`a_hovered_canvas_keeps_asking_for_frames`] whose asserting frames each
//!    carried a `PointerMoved` was measured *passing* against a mutant with
//!    `response.hovered()` deleted from the predicate. The shape that actually
//!    bites is: one frame that makes the pointer resident, one settle frame,
//!    then assertions on empty frames only. Keep it.
//!
//! ADR 0002 §A4 is respected throughout: `App::default()`, never `App::new()`;
//! no `Ctrl+O` / `Ctrl+S`; both injected paths stay `None`, so nothing here can
//! write to the filesystem even if a debounce did elapse (it does not — every
//! `dirty_since` is freshly stamped and no test sleeps).

mod harness;

use std::time::{Duration, Instant};

use harness::{frame, raw_input, tap};
use lasercad::app::App;

/// The autosave debounce, mirrored. `AUTOSAVE_DEBOUNCE` is private to
/// `src/app/autosave.rs`, where a unit test pins its value, so a drift fails
/// loudly on that side rather than silently here.
const DEBOUNCE: Duration = Duration::from_millis(800);

/// Run one real `App::update_ui` frame and report the repaint egui was asked
/// for. The only difference from `harness::frame` is that this keeps the
/// `FullOutput` instead of dropping it.
fn frame_delay(ctx: &egui::Context, app: &mut App, events: Vec<egui::Event>) -> Duration {
    let out = ctx.run(raw_input(events), |ctx| app.update_ui(ctx));
    out.viewport_output
        .get(&egui::ViewportId::ROOT)
        .expect("the root viewport is always present")
        .repaint_delay
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

/// Spend frame 0 — which always reads 0 ns (trap 1) — and report the canvas
/// rect egui left for the `CentralPanel` once the chrome has claimed its share.
fn boot(app: App) -> (egui::Context, App, egui::Rect) {
    let ctx = egui::Context::default();
    let mut app = app;
    let mut canvas = egui::Rect::NOTHING;
    let _ = ctx.run(raw_input(vec![]), |c| {
        app.update_ui(c);
        canvas = c.available_rect();
    });
    (ctx, app, canvas)
}

/// AC 4 — an app nobody is touching asks for nothing and is allowed to sleep.
///
/// No frame in this test carries a single input event. On HEAD before LCV-120
/// this read 0 ns on every frame; the defect is exactly that.
#[test]
fn an_idle_app_asks_for_no_repaint() {
    let (ctx, mut app, _canvas) = boot(App::default());
    assert!(
        app.last_cursor_world.is_none() && app.dirty_since.is_none() && !app.agent_busy,
        "precondition: nothing about this app is live"
    );

    for frame in 1..=2 {
        let delay = frame_delay(&ctx, &mut app, vec![]);
        assert_eq!(
            delay,
            Duration::MAX,
            "frame {frame}: an idle app must let egui sleep, got {delay:?}"
        );
    }
}

/// AC 5 — while the pointer sits over the canvas, the canvas keeps asking for
/// frames, so cursor coordinates and previews stay live.
///
/// The positive control is `last_cursor_world`: only `handle_hover` writes it
/// and only under `response.hovered()`, so it proves the chosen point really is
/// over the canvas rather than the assertion passing for some other reason.
/// Its negative control lives in
/// [`a_pointer_outside_the_canvas_lets_the_app_idle`] — without that test, this
/// one cannot fail.
#[test]
fn a_hovered_canvas_keeps_asking_for_frames() {
    let (ctx, mut app, canvas) = boot(App::default());
    let p = canvas.center();

    // Frame A: the pointer moves into the canvas and stays there.
    let _ = frame_delay(&ctx, &mut app, vec![egui::Event::PointerMoved(p)]);
    assert!(
        app.last_cursor_world.is_some(),
        "positive control: {p:?} must be over the canvas {canvas:?}"
    );

    // Frame B: settle. Frames C and D carry no events at all (trap 2).
    let _ = frame_delay(&ctx, &mut app, vec![]);
    for frame in ["C", "D"] {
        let delay = frame_delay(&ctx, &mut app, vec![]);
        assert_eq!(
            delay,
            Duration::ZERO,
            "frame {frame}: a hovered canvas must keep asking, got {delay:?}"
        );
    }
}

/// AC 5's negative control — the same shape with the pointer parked on the
/// menubar instead. The app idles and no cursor coordinate is ever written.
#[test]
fn a_pointer_outside_the_canvas_lets_the_app_idle() {
    let (ctx, mut app, canvas) = boot(App::default());
    let p = egui::pos2(canvas.center().x, 2.0);
    assert!(
        !canvas.contains(p),
        "precondition: {p:?} must be outside the canvas {canvas:?}"
    );

    let _ = frame_delay(&ctx, &mut app, vec![egui::Event::PointerMoved(p)]);
    let _ = frame_delay(&ctx, &mut app, vec![]);
    for frame in ["C", "D"] {
        let delay = frame_delay(&ctx, &mut app, vec![]);
        assert_eq!(
            delay,
            Duration::MAX,
            "frame {frame}: a pointer off the canvas must not keep it awake, got {delay:?}"
        );
    }
    assert!(
        app.last_cursor_world.is_none(),
        "control: the pointer never entered the canvas, so nothing wrote a coordinate"
    );
}

/// AC 6 — `autosave::schedule_flush_repaint` is what keeps a pending write
/// alive now that the canvas idles, and this is its witness.
///
/// Same idle shape as [`an_idle_app_asks_for_no_repaint`] — no input event in
/// any frame — but with the document dirty. Delete `schedule_flush_repaint`'s
/// call from `update_ui` and this reads `Duration::MAX`: an app that sleeps
/// forever and never writes the operator's work (LCV-116 AC 10).
///
/// The `dirty_since` stamp is fresh and nothing here sleeps, so the debounce
/// never elapses — a matter of focus, not of safety: since LCV-119 / ADR 0006
/// this `App` carries `autosave_path: None`, so even a fired flush would write
/// nothing anywhere (ADR 0002 §A4 rule 2 as amended).
#[test]
fn a_pending_autosave_still_wakes_an_idle_app() {
    let (ctx, mut app, _canvas) = boot(App {
        dirty_since: Some(Instant::now()),
        ..App::default()
    });

    for frame in 1..=2 {
        let delay = frame_delay(&ctx, &mut app, vec![]);
        assert!(
            delay > Duration::ZERO && delay < DEBOUNCE,
            "frame {frame}: a pending autosave must schedule its own wake-up \
             inside the {DEBOUNCE:?} window, got {delay:?}"
        );
    }
    assert!(
        app.dirty_since.is_some(),
        "control: the debounce must not have elapsed during the test"
    );
}

/// The third term of the predicate, isolated — a tool preview on screen keeps
/// the canvas asking for frames even once the pointer has left it.
///
/// Not one of the four tests the demand named, added so the mutation check the
/// demand *does* require — delete `!app.preview_entities.is_empty()` and a
/// preview-in-progress case must fail — has something to fail. Its pair is
/// [`a_pointer_outside_the_canvas_lets_the_app_idle`]: the identical
/// pointer-parked-on-the-menubar shape, differing only in the live preview, and
/// that one asserts `Duration::MAX`. So this cannot pass for want of a negative
/// control, and it cannot pass through `hovered()` either.
#[test]
fn a_live_preview_keeps_asking_for_frames_with_the_pointer_away() {
    let (ctx, mut app, canvas) = boot(App::default());
    let anchor = canvas.center();
    let elsewhere = anchor + egui::vec2(40.0, 30.0);
    let off_canvas = egui::pos2(canvas.center().x, 2.0);

    tap(&ctx, &mut app, egui::Key::L, egui::Modifiers::NONE);
    assert_eq!(app.tool_manager.active_tool_name(), "LINE");
    frame(&ctx, &mut app, vec![egui::Event::PointerMoved(anchor)]); // warm-up
    frame(&ctx, &mut app, click_events(anchor));
    frame(&ctx, &mut app, vec![egui::Event::PointerMoved(elsewhere)]);

    // Park the pointer off the canvas, settle, then assert on empty frames.
    // `paint` runs before `handle_hover`, so the rubber band a frame paints is
    // the one the previous frame's cursor produced (LCV-040); the tool's state
    // is frozen from here on, so it is on screen for every frame below.
    frame(&ctx, &mut app, vec![egui::Event::PointerMoved(off_canvas)]);
    assert!(
        !app.preview_entities.is_empty(),
        "positive control: the rubber band must be on screen"
    );
    let _ = frame_delay(&ctx, &mut app, vec![]);
    for frame in ["C", "D"] {
        let delay = frame_delay(&ctx, &mut app, vec![]);
        assert_eq!(
            delay,
            Duration::ZERO,
            "frame {frame}: a live preview must keep the canvas awake, got {delay:?}"
        );
    }
    assert!(
        !app.preview_entities.is_empty() && app.dirty_since.is_none(),
        "controls: the preview is still up, and the first click committed nothing, \
         so the delay above cannot be an autosave wake-up"
    );
}
