//! LCV-116 AC 9 / AC 10 — the autosave flush schedules its own wake-up.
//!
//! `flush_if_due` only runs inside a frame. Without a scheduled repaint an
//! operator who draws a line and then lets go of the mouse gets no further
//! frames, so the 800 ms debounce never elapses and the autosave never lands
//! (AC 10). `src/app/autosave.rs::schedule_flush_repaint` fixes that by asking
//! egui for exactly one more frame, timed to the remaining debounce.
//!
//! # The shortcut this file exists to block
//!
//! A blanket `ctx.request_repaint()` would also make AC 10 pass, at the cost of
//! never letting the app idle at all. Both halves are therefore asserted:
//!
//! * a **dirty** app schedules a repaint inside the debounce window, and
//! * a **clean** app schedules nothing sub-second.
//!
//! The second assertion is what fails if the guard is replaced by a blanket
//! repaint. Verified by temporarily doing exactly that.
//!
//! # Why this file still drives `schedule_flush_repaint` directly
//!
//! When LCV-116 landed, `src/app/viewport.rs` called `ctx.request_repaint()`
//! unconditionally on every frame, which pinned `repaint_delay` at zero on
//! every full frame, clean or dirty: the clean half of the contract was not
//! observable through `update_ui` at all, so it was asserted here instead, at
//! the granularity where it was real. **LCV-120 closed that**: the canvas now
//! asks for a follow-up frame only while it is hovered, dragged or previewing,
//! and `tests/it/app/idle_repaint.rs` asserts both halves through the real
//! `App::update_ui` — an idle app sleeps, and a dirty one is woken by
//! `schedule_flush_repaint` and by nothing else, which is what makes this
//! function load-bearing rather than redundant.
//!
//! This file keeps the narrower, unit-granularity assertions on
//! `schedule_flush_repaint` itself, driven through a genuine
//! `egui::Context::run`: a failure here names the scheduler rather than the
//! whole frame, and the shrinking-deadline and going-clean cases below are not
//! visible from `update_ui`. `the_agent_repaint_is_untouched_by_the_autosave_schedule`
//! below is a bounded source scan over `src/app/mod.rs` pinning that
//! `update_ui` still calls it, immediately after the flush.
//!
//! No test here lets the debounce actually elapse: every `dirty_since` is
//! freshly stamped and `flush_if_due` is never called, because what is under
//! test is the *scheduling* decision, not the write. Since LCV-119 that is a
//! matter of focus rather than of safety — these `App`s carry
//! `autosave_path: None`, so a fired autosave would write nothing anywhere
//! (ADR 0006).

use crate::harness;

use std::time::{Duration, Instant};

use harness::raw_input;
use lasercad::app::{schedule_flush_repaint, AgentState, App};

/// The debounce, mirrored. `AUTOSAVE_DEBOUNCE` is private to
/// `src/app/autosave.rs`; the value is pinned there by a unit test, so a drift
/// between the two fails loudly on that side rather than silently here.
const DEBOUNCE: Duration = Duration::from_millis(800);

/// Run one frame whose entire body is `schedule_flush_repaint` and report what
/// egui was asked to wake up after.
fn scheduled_delay(ctx: &egui::Context, app: &App) -> Duration {
    let out = ctx.run(raw_input(vec![]), |ctx| schedule_flush_repaint(ctx, app));
    out.viewport_output
        .get(&egui::ViewportId::ROOT)
        .expect("the root viewport is always present")
        .repaint_delay
}

/// egui reports a zero delay for the very first frame of a fresh `Context`
/// regardless of what the body asked for — it is still settling its layout.
/// Every assertion about *absence* of a repaint request therefore has to be
/// made from the second frame on.
fn settle(ctx: &egui::Context, app: &App) {
    let _ = scheduled_delay(ctx, app);
}

/// AC 9 — a pending autosave schedules its own flush frame, no later than the
/// end of the debounce window.
#[test]
fn a_dirty_app_schedules_the_flush_within_the_debounce() {
    let ctx = egui::Context::default();
    let app = App {
        dirty_since: Some(Instant::now()),
        ..App::default()
    };

    settle(&ctx, &app);
    let delay = scheduled_delay(&ctx, &app);

    assert!(
        delay <= DEBOUNCE,
        "a pending autosave must wake up inside the {DEBOUNCE:?} window, got {delay:?}"
    );
    assert!(
        delay > Duration::ZERO,
        "and it must be a *scheduled* wake-up, not a blanket per-frame repaint: got {delay:?}"
    );
}

/// AC 9 — the schedule tracks the remaining debounce rather than restarting it.
/// An app that has been dirty for a while asks for correspondingly less.
#[test]
fn the_schedule_shrinks_as_the_debounce_runs_down() {
    let ctx = egui::Context::default();
    let app = App {
        dirty_since: Some(Instant::now()),
        ..App::default()
    };

    settle(&ctx, &app);
    let first = scheduled_delay(&ctx, &app);
    std::thread::sleep(Duration::from_millis(60));
    let second = scheduled_delay(&ctx, &app);

    assert!(
        second < first,
        "the wake-up must chase a fixed deadline, not reset it: {first:?} then {second:?}"
    );
    // Still safely short of the debounce — ADR 0002 §A4 rule 2 is not tested.
    assert!(second > Duration::ZERO, "got {second:?}");
}

/// AC 9, the half that matters — an idle, clean app asks for **nothing**.
///
/// This is the assertion that fails if `schedule_flush_repaint` is ever
/// "simplified" into an unconditional `ctx.request_repaint()`.
#[test]
fn a_clean_app_requests_no_sub_second_repaint() {
    let ctx = egui::Context::default();
    let app = App::default();
    assert!(
        app.dirty_since.is_none(),
        "a fresh app is clean — precondition for this test"
    );

    settle(&ctx, &app);
    for frame in 0..3 {
        let delay = scheduled_delay(&ctx, &app);
        assert!(
            delay >= Duration::from_secs(1),
            "frame {frame}: a clean app must not schedule work; got {delay:?}"
        );
    }
}

/// AC 9 — the transition, in one context: dirty schedules, going clean stops.
/// Interleaved on the same `Context` so the result cannot be an artefact of a
/// freshly constructed one.
#[test]
fn the_schedule_stops_when_the_document_goes_clean() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    settle(&ctx, &app);

    app.dirty_since = Some(Instant::now());
    let dirty = scheduled_delay(&ctx, &app);
    assert!(
        dirty > Duration::ZERO && dirty <= DEBOUNCE,
        "dirty must schedule, got {dirty:?}"
    );

    app.dirty_since = None;
    // One frame to clear the request egui is still holding from the dirty one.
    settle(&ctx, &app);
    let clean = scheduled_delay(&ctx, &app);
    assert!(
        clean >= Duration::from_secs(1),
        "clean must stop scheduling, got {clean:?}"
    );
}

/// AC 9 — the pre-existing agent repaint is independent of this one and still
/// works. `schedule_flush_repaint` reads only `dirty_since`; the `agent.busy`
/// repaint in `update_ui` is a separate, unconditional request that LCV-116
/// must not have disturbed.
#[test]
fn the_agent_repaint_is_untouched_by_the_autosave_schedule() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = std::fs::read_to_string(root.join("src/app/mod.rs")).expect("src/app/mod.rs");
    let body = src
        .split_once("pub fn update_ui")
        .expect("update_ui must exist")
        .1;
    let body = body
        .split_once("\n    }\n")
        .expect("update_ui must close")
        .0;

    assert!(
        body.contains("if self.agent.busy {"),
        "the agent-busy repaint guard must survive LCV-116"
    );
    assert!(
        body.contains("schedule_flush_repaint(ctx, self)"),
        "positive control: update_ui must also schedule the autosave flush"
    );

    // And the autosave scheduler itself must not be the one repainting for the
    // agent: a clean app with `agent.busy` set still schedules nothing.
    let ctx = egui::Context::default();
    let app = App {
        agent: AgentState {
            busy: true,
            ..Default::default()
        },
        ..App::default()
    };
    settle(&ctx, &app);
    let delay = scheduled_delay(&ctx, &app);
    assert!(
        delay >= Duration::from_secs(1),
        "schedule_flush_repaint must key off dirty_since alone, got {delay:?}"
    );
}
