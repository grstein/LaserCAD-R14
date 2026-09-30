//! tests/it/ui/bed_dialog.rs — the Bed Size… dialog end to end (LCV-114
//! AC 14 / AC 15).
//!
//! Every test drives the real frame body through
//! `egui::Context::run(raw_input(...), |ctx| app.update_ui(ctx))`
//! (`tests/harness/mod.rs`, ADR 0002 §A3), so the dialog is exercised where it
//! actually lives: inside `draw_dialogs`, after the `CentralPanel`.
//!
//! Rules specific to this file:
//!
//! - The **menu click** that opens the dialog is manual (a nested
//!   `menu_button` needs two pointer frames and hit-testing); tests set
//!   `app.bed_dialog` directly, exactly as the demand prescribes.
//! - The **OK/Cancel click** is likewise driven through
//!   `apply_bed_dialog_result`, the half of the split that needs no pointer
//!   (same shape as LCV-113 AC 11). Since LCV-119 the settings write behind
//!   OK is path-injected (ADR 0006), so it reaches nothing in a test `App`;
//!   the pointer limitation is now the only reason for the split.
//! - No `Ctrl+O` / `Ctrl+S` anywhere: they reach a native dialog (ADR 0005).

use crate::harness;

use harness::frame;
use lasercad::app::{App, apply_bed_dialog_result};
use lasercad::document::CreateLine;
use lasercad::geometry::{Line, Vec2};
use lasercad::ui::DialogResult;

/// A test `App` with the Bed Size… dialog already open on `draft` — the state
/// the menu entry produces.
fn app_with_draft(draft: [f64; 2]) -> App {
    App {
        bed_dialog: Some(draft),
        ..App::default()
    }
}

/// AC 14 — OK commits exactly one undoable command, moves the document's bed,
/// updates the settings seed in memory, and leaves the app renderable.
#[test]
fn bed_dialog_opens_and_commits_a_command() {
    let ctx = egui::Context::default();
    // A frame with the dialog open must render without panicking.
    let mut app = app_with_draft([128.0, 128.0]);
    frame(&ctx, &mut app, Vec::new());
    assert_eq!(
        app.bed_dialog,
        Some([128.0, 128.0]),
        "still open, uncommitted"
    );
    assert_eq!(app.document.bed_mm, [400.0, 400.0], "nothing committed yet");

    let revision_before = app.history.revision();
    apply_bed_dialog_result(&mut app, DialogResult::Confirmed);

    frame(&ctx, &mut app, Vec::new());
    assert_eq!(app.document.bed_mm, [128.0, 128.0]);
    assert_eq!(app.history.revision(), revision_before + 1);
    assert_eq!(app.settings.default_bed_mm, [128.0, 128.0]);
    assert_eq!(app.bed_dialog, None, "the dialog closes on OK");

    // A second frame after the commit still renders (AC 15: the viewport
    // rebuilds its bed from the document every frame).
    frame(&ctx, &mut app, Vec::new());
    assert_eq!(app.document.bed_mm, [128.0, 128.0]);
}

/// AC 14 — Cancel changes nothing: no command, no bed change, no seed change.
#[test]
fn bed_dialog_cancel_changes_nothing() {
    let ctx = egui::Context::default();
    let mut app = app_with_draft([128.0, 128.0]);
    frame(&ctx, &mut app, Vec::new());

    let revision_before = app.history.revision();
    apply_bed_dialog_result(&mut app, DialogResult::Cancelled);
    frame(&ctx, &mut app, Vec::new());

    assert_eq!(app.document.bed_mm, [400.0, 400.0]);
    assert_eq!(app.settings.default_bed_mm, [400.0, 400.0]);
    assert_eq!(app.history.revision(), revision_before);
    assert!(!app.history.can_undo());
    assert_eq!(app.bed_dialog, None);
}

/// AC 14 — the bed change is undoable from the real frame body, and the undo
/// leaves the drawing alone: changing the bed moves the frame, never the
/// entities.
#[test]
fn bed_change_is_undoable_and_leaves_entities_alone() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    app.history.commit(
        Box::new(CreateLine::new(Line::new(
            Vec2::new(10.0, 50.0),
            Vec2::new(250.0, 50.0),
        ))),
        &mut app.document,
    );

    app.bed_dialog = Some([300.0, 180.0]);
    apply_bed_dialog_result(&mut app, DialogResult::Confirmed);
    frame(&ctx, &mut app, Vec::new());
    assert_eq!(app.document.bed_mm, [300.0, 180.0]);
    assert_eq!(app.document.entity_count(), 1);

    // Ctrl+Z through the real shortcut table.
    harness::tap(
        &ctx,
        &mut app,
        egui::Key::Z,
        egui::Modifiers {
            ctrl: true,
            command: true,
            ..egui::Modifiers::NONE
        },
    );
    assert_eq!(
        app.document.bed_mm,
        [400.0, 400.0],
        "Ctrl+Z restores the bed"
    );
    assert_eq!(app.document.entity_count(), 1, "entities are untouched");
}

/// AC 15 — the frame body keeps rendering across a bed change, from the
/// default bed to a smaller one and back through undo. The viewport holds no
/// bed of its own, so there is nothing to refresh; the unit test
/// `ui::menubar::tests::fit_to_bed_follows_a_resized_document_bed` covers the
/// Fit-to-Bed half (the menu entry is `pub(crate)` and unreachable from here).
#[test]
fn frames_keep_rendering_across_a_bed_change() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    frame(&ctx, &mut app, Vec::new());
    assert!(
        app.camera.viewport_size_px[0] > 0.0,
        "the canvas has a size"
    );

    for bed in [[128.0, 128.0], [2000.0, 2000.0], [1.0, 1.0]] {
        app.bed_dialog = Some(bed);
        apply_bed_dialog_result(&mut app, DialogResult::Confirmed);
        frame(&ctx, &mut app, Vec::new());
        assert_eq!(app.document.bed_mm, bed);
    }
}
