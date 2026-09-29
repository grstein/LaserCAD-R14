//! tests/it/app/confirm_discard.rs — confirm discard on New / Open / Exit (LCV-113).
//!
//! Every test here drives the real frame body through
//! `egui::Context::run(raw_input(...), |ctx| app.update_ui(ctx))`
//! (`tests/harness/mod.rs`, ADR 0002 §A3).
//!
//! Two hard rules specific to this demand (see the demand body):
//!
//! - **Never call `request_open` / `request_open_path` on a clean document,
//!   and never confirm a parked `Open` / `OpenPath`.** Both reach
//!   `src/io/file_actions.rs`, which opens a blocking native `rfd` dialog and
//!   hangs the run (ADR 0002 §A4 rule 1). Every test below that touches
//!   `Open` keeps the document dirty and stops at the parked state.
//! - `Ctrl+N` is safe in both directions and is the end-to-end vehicle.

use crate::harness;

use harness::tap;
use lasercad::app::{App, PendingAction};
use lasercad::document::CreateLine;
use lasercad::geometry::{Line, Vec2};

/// `Ctrl` as egui reports it on Linux: `ctrl` and `command` both set, matching
/// `Modifiers::command_only()` in `dispatch_shortcuts`.
fn ctrl() -> egui::Modifiers {
    egui::Modifiers {
        ctrl: true,
        command: true,
        ..egui::Modifiers::NONE
    }
}

/// Put `n` lines directly on the history stack — test-only setup, mirroring
/// `tests/it/app/keyboard_routing.rs`'s `with_lines`.
fn with_lines(app: &mut App, n: usize) {
    for i in 0..n {
        let y = i as f64;
        app.history.commit(
            Box::new(CreateLine::new(Line::new(
                Vec2::new(0.0, y),
                Vec2::new(10.0, y),
            ))),
            &mut app.document,
        );
    }
    assert_eq!(app.document.entity_count(), n);
}

/// One frame of raw input carrying a root-viewport `Close` event — the
/// window's X button. `tests/harness/mod.rs`'s `raw_input` cannot express
/// viewport events (ADR 0002 §A3's five-item list stays untouched), so this
/// is built inline, as the demand's Expected Tests section prescribes.
fn close_request_input() -> egui::RawInput {
    let mut viewports = egui::ViewportIdMap::default();
    viewports.insert(
        egui::ViewportId::ROOT,
        egui::ViewportInfo {
            events: vec![egui::ViewportEvent::Close],
            ..Default::default()
        },
    );
    egui::RawInput {
        viewport_id: egui::ViewportId::ROOT,
        viewports,
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(harness::SCREEN[0], harness::SCREEN[1]),
        )),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// AC 7, 8 — Ctrl+N through the real frame body
// ---------------------------------------------------------------------------

/// AC 8 — a clean document resets immediately; nothing is parked.
#[test]
fn ctrl_n_on_clean_document_resets_immediately() {
    let ctx = egui::Context::default();
    let mut app = App::default();

    tap(&ctx, &mut app, egui::Key::N, ctrl());

    assert!(app.guard.pending_action.is_none());
    assert_eq!(app.document.entity_count(), 0);
}

/// AC 8 — the defect this demand fixes: `Ctrl+N` on a dirty document used to
/// discard the drawing (and the undo stack with it) with no confirmation at
/// all. It must now park `PendingAction::New` and leave the drawing exactly
/// as it was.
#[test]
fn ctrl_n_on_dirty_document_opens_the_dialog_and_keeps_the_drawing() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    with_lines(&mut app, 2);

    tap(&ctx, &mut app, egui::Key::N, ctrl());

    assert_eq!(app.guard.pending_action, Some(PendingAction::New));
    assert_eq!(
        app.document.entity_count(),
        2,
        "the drawing must survive until Discard is clicked"
    );
}

// ---------------------------------------------------------------------------
// AC 7 — Ctrl+O parks without ever reaching the native dialog
// ---------------------------------------------------------------------------

/// AC 7 — `Ctrl+O` on a dirty document parks `PendingAction::Open` and stops
/// there. The test never confirms: confirming would run `action_open`, which
/// opens a blocking native `rfd` dialog and would hang this run.
#[test]
fn ctrl_o_on_dirty_document_parks_without_opening_a_native_dialog() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    with_lines(&mut app, 1);

    tap(&ctx, &mut app, egui::Key::O, ctrl());

    assert_eq!(app.guard.pending_action, Some(PendingAction::Open));
    assert_eq!(app.document.entity_count(), 1);
}

// ---------------------------------------------------------------------------
// AC 9 — the window close button
// ---------------------------------------------------------------------------

/// AC 9 — a close request on a dirty document is cancelled: the returned
/// `FullOutput` carries `ViewportCommand::CancelClose` for the root viewport,
/// and `PendingAction::Exit` is parked so the discard dialog appears next
/// frame.
#[test]
fn close_request_on_a_dirty_document_is_cancelled() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    with_lines(&mut app, 1);

    let out = ctx.run(close_request_input(), |ctx| app.update_ui(ctx));

    assert!(
        out.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .contains(&egui::ViewportCommand::CancelClose),
        "a dirty document's close request must be cancelled"
    );
    assert_eq!(app.guard.pending_action, Some(PendingAction::Exit));
}

/// AC 9 — a close request on a clean document is not cancelled: no
/// `CancelClose` command is emitted and nothing is parked, so the window
/// closes normally.
#[test]
fn close_request_on_a_clean_document_is_not_cancelled() {
    let ctx = egui::Context::default();
    let mut app = App::default();

    let out = ctx.run(close_request_input(), |ctx| app.update_ui(ctx));

    assert!(
        !out.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .contains(&egui::ViewportCommand::CancelClose),
        "a clean document's close request must not be cancelled"
    );
    assert!(app.guard.pending_action.is_none());
}
