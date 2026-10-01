//! tests/it/ui/discard_dialog_pointer_click.rs — Discard/Cancel through a
//! real pointer click (LCV-136).
//!
//! `tests/it/app/confirm_discard.rs` and `src/app/file_ops.rs`'s inline tests exhaustively
//! cover the *state machine* — `request_new`/`request_open`/`request_exit`
//! parking the right `PendingAction`, and `apply_dialog_result` running it
//! exactly once on `Confirmed` and restoring nothing on `Cancelled` — but
//! every one of those tests calls `apply_dialog_result` directly. Every test
//! in this file instead locates the real "Discard" / "Cancel" button through
//! `tests/harness/paint.rs`'s painted text and drives a real
//! `PointerMoved` + `PointerButton` press/release pair at it (ADR 0002 §A4
//! rule 3) — the operator's actual gesture, and the one path LCV-113's own
//! tests never exercised.
//!
//! **Investigation finding (AC 8), corrected:** New and OpenPath, driven
//! through this real pointer path, reproduce no defect — `confirm_dialog`'s
//! button is reachable, click-through to the canvas behind the modal does
//! not happen, and a stray second click at the button's old position does
//! not re-dispatch. Layer ordering (the `CentralPanel`'s `Order::Background`
//! vs. the confirm `Window`'s `Order::Middle`, always on top irrespective of
//! draw-call order at egui 0.29.1) and the "first frame paints nothing"
//! settling trap (`tests/harness/paint.rs` trap 7) were the two mechanics
//! that looked like plausible culprits, and neither broke a New/OpenPath
//! test below.
//!
//! **Exit did reproduce**, on the real re-issued-Close mechanism verified
//! against the vendored `egui-winit`/`eframe` 0.29.1 sources:
//! `ViewportCommand::Close` does not synchronously destroy the window — it
//! only records a fresh `ViewportEvent::Close` on the viewport
//! (`egui_winit::process_viewport_command`), which reports
//! `close_requested() == true` again on the *next* frame, for the *same*
//! close. `Exit` mutates no document, so `has_unsaved_changes()` was still
//! `true` on that next frame: `poll_close_request` re-ran `request_exit`,
//! re-parked `PendingAction::Exit`, and sent `CancelClose` — cancelling the
//! close the operator just confirmed and reopening the dialog, forever. The
//! fix is `App::guard.exit_confirmed`, a latch set by `apply_dialog_result`'s
//! confirmed-`Exit` arm and checked first by `poll_close_request`
//! (`src/app/file_ops.rs`), which lets every later close request through
//! unconditionally once the operator has answered once. A repeated *native*
//! Close request arriving *before* any confirmation (the window-X pressed
//! twice while the dialog is still up) was already handled correctly and
//! stays covered below.
//!
//! Two rules specific to this demand, both inherited from `tests/it/app/confirm_discard.rs`:
//!
//! - **Never call `request_open` / confirm a parked `Open`.** It reaches
//!   `crate::io::action_open`, which opens a blocking native `rfd` dialog and
//!   hangs the run (ADR 0002 §A4 rule 1). `OpenPath` is used instead, driven
//!   against a test-owned SVG fixture — no dialog is ever opened.
//! - A settled frame is required before a button can be located: a freshly
//!   opened `Window`'s `Area` is not yet placed on the frame it first
//!   appears, so it paints none of its body (paint.rs trap 7). Every test
//!   below runs one idle frame between parking the action and calling
//!   [`locate`].

use crate::harness;

use harness::paint::{self, Run};
use harness::{key_events, raw_input, tap};
use lasercad::app::{App, PendingAction};
use lasercad::document::{CreateLine, Entity, Selection};
use lasercad::geometry::{Line, Vec2};
use std::path::PathBuf;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Shared fixtures
// ---------------------------------------------------------------------------

/// `Ctrl` as egui reports it on Linux: `ctrl` and `command` both set,
/// matching `Modifiers::command_only()` in `dispatch_shortcuts`.
fn ctrl() -> egui::Modifiers {
    egui::Modifiers {
        ctrl: true,
        command: true,
        ..egui::Modifiers::NONE
    }
}

/// Put `n` lines directly on the history stack — test-only setup, mirroring
/// `tests/it/app/confirm_discard.rs`'s `with_lines`.
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

/// A complete primary-button click at `pos`: move, press, release — the
/// second half of the harness's pointer-click convention (ADR 0002 §A4
/// rule 3). Always preceded by its own warm-up frame; see [`click_button`].
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

/// One root-viewport `Close` event — the window's X button. Built inline,
/// same as `tests/it/app/confirm_discard.rs::close_request_input`: `tests/harness/mod.rs`'s
/// `raw_input` cannot express viewport events.
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

/// Boot the app: one idle frame, which registers every widget rect and syncs
/// the camera away from its zero-area sentinel (`tests/it/cmdline/text_command.rs`'s `boot`),
/// and hand back the rect `CentralPanel` left once the chrome claimed its
/// share.
fn boot(ctx: &egui::Context, app: &mut App) -> egui::Rect {
    let mut viewport = egui::Rect::NOTHING;
    let _ = ctx.run(raw_input(vec![]), |c| {
        app.update_ui(c);
        viewport = c.available_rect();
    });
    viewport
}

/// Two idle frames, to let a freshly opened `Window` place its `Area` and
/// paint its body (paint.rs trap 7: the frame a dialog first appears paints
/// none of it). Some callers reach this function right after a state change
/// that rendered no frame at all (e.g. `App::request_open_path`, called
/// directly rather than through a keystroke), so the *first* idle frame here
/// may be the dialog's unsettled one; the second is always settled, whether
/// or not the first already was. Hands back what that second frame painted.
fn settle(ctx: &egui::Context, app: &mut App) -> Vec<Run> {
    let _ = paint::painted_runs(ctx, app);
    paint::painted_runs(ctx, app)
}

/// Locate the one run reading `label` on a settled frame and return a point
/// inside its glyph rect. `button_padding` is `[4.0, 1.0]` at the default
/// egui style (`Style::spacing`), so a point 2pt right of the glyph's own
/// origin and centred on its height sits inside the button's padded rect a
/// fortiori — never on a neighbouring widget.
fn locate(runs: &[Run], label: &str) -> egui::Pos2 {
    let matches: Vec<&Run> = runs.iter().filter(|r| r.text.trim() == label).collect();
    assert_eq!(
        matches.len(),
        1,
        "`{label}` must be painted exactly once, saw {} times",
        matches.len()
    );
    let run = matches[0];
    egui::pos2(run.pos.x + 2.0, run.pos.y + run.height / 2.0)
}

/// Drive a real pointer click at `pos`: a warm-up frame carrying
/// `PointerMoved` alone, then the frame carrying the press/release pair
/// (ADR 0002 §A4 rule 3 / AC 1). Hands back the click frame's `FullOutput` so
/// a caller can inspect e.g. `ViewportCommand::Close`.
fn click_button(ctx: &egui::Context, app: &mut App, pos: egui::Pos2) -> egui::FullOutput {
    let _ = ctx.run(raw_input(vec![egui::Event::PointerMoved(pos)]), |c| {
        app.update_ui(c)
    });
    ctx.run(raw_input(click_events(pos)), |c| app.update_ui(c))
}

/// Open the File menu through a real click on its "File" label in the
/// menubar — the same trap 7 shape as a `Window`: the dropdown is a popup
/// `Area` of its own and paints no items on the frame it first opens, so one
/// more idle frame is spent settling it before its items are handed back.
/// A menu row paints its label and its shortcut as two runs (LCV-166), so a
/// caller locates the bare label run, e.g. `"New"`, never a substring.
fn open_file_menu(ctx: &egui::Context, app: &mut App) -> Vec<Run> {
    let runs = paint::painted_runs(ctx, app);
    let file = locate(&runs, "File");
    click_button(ctx, app, file);
    paint::painted_runs(ctx, app)
}

/// A tempdir this test owns and cleans first — `src/io/file_actions.rs`'s own
/// fixture convention, reproduced here since that helper is private to its
/// module.
fn tempdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lcv136_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A minimal, valid, test-owned SVG fixture: one line, on a 200×200 mm bed.
const VALID_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200" viewBox="0 0 200 200">
<line x1="10" y1="10" x2="150" y2="10" stroke="#ff0000" stroke-width="0.1"/>
</svg>"##;

/// A test-owned fixture that is not valid XML at all, so `import_svg` fails
/// with `SvgImportError::XmlParse` before it ever reaches geometry parsing.
const MALFORMED_SVG: &str = "not an svg file at all";

/// Everything about `app` this file asserts is unaffected by Cancel.
struct Snapshot {
    entities: Vec<Entity>,
    selection: Selection,
    revision: u64,
    current_file: Option<PathBuf>,
    dirty_since: Option<Instant>,
}

fn snapshot(app: &App) -> Snapshot {
    Snapshot {
        entities: app.document.entities.clone(),
        selection: app.document.selection.clone(),
        revision: app.history.revision(),
        current_file: app.current_file.clone(),
        dirty_since: app.autosave.dirty_since,
    }
}

/// AC 4 — every field [`snapshot`] captured is exactly what it was before.
fn assert_unchanged(app: &App, before: &Snapshot) {
    assert_eq!(
        app.document.entities, before.entities,
        "Cancel must not touch the entities"
    );
    assert_eq!(
        app.document.selection, before.selection,
        "Cancel must not touch the selection"
    );
    assert_eq!(
        app.history.revision(),
        before.revision,
        "Cancel must not touch the history revision"
    );
    assert_eq!(
        app.current_file, before.current_file,
        "Cancel must not touch current_file"
    );
    assert_eq!(
        app.autosave.dirty_since, before.dirty_since,
        "Cancel must not touch the dirty signal"
    );
}

// ---------------------------------------------------------------------------
// AC 1, 3 — Discard confirms the parked action exactly once, not replayed
// ---------------------------------------------------------------------------

/// AC 1, 3 — Discard confirmed through a real pointer click runs `action_new`
/// exactly once: the drawing present before the click is gone, `current_file`
/// is cleared, and an idle frame right after release neither reopens the
/// dialog nor discards anything a second time.
#[test]
fn discard_confirms_new_exactly_once_and_is_not_replayed() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    ctx.set_pixels_per_point(1.0);
    boot(&ctx, &mut app);

    with_lines(&mut app, 2);
    app.current_file = Some(PathBuf::from("before.svg"));

    let _ = ctx.run(raw_input(key_events(egui::Key::N, ctrl())), |c| {
        app.update_ui(c)
    });
    assert_eq!(app.guard.pending_action, Some(PendingAction::New));

    let runs = settle(&ctx, &mut app);
    let discard = locate(&runs, "Discard");
    click_button(&ctx, &mut app, discard);

    assert!(
        app.guard.pending_action.is_none(),
        "the dialog must be dismissed"
    );
    assert_eq!(
        app.document.entity_count(),
        0,
        "action_new must have run exactly once"
    );
    assert!(
        app.current_file.is_none(),
        "action_new must have reset current_file"
    );

    // Not replayed: an idle frame after release does not reopen the dialog.
    let runs = paint::painted_runs(&ctx, &mut app);
    assert!(
        !runs.iter().any(|r| r.text.trim() == "Discard"),
        "the dialog must not reopen on an idle frame"
    );
    assert_eq!(app.document.entity_count(), 0);
}

/// AC 1, 3 — same shape, for a parked `OpenPath` against a test-owned SVG
/// fixture: Discard adopts the fixture's one line exactly once.
#[test]
fn discard_confirms_open_path_exactly_once_and_is_not_replayed() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    ctx.set_pixels_per_point(1.0);
    boot(&ctx, &mut app);

    with_lines(&mut app, 1);
    app.current_file = Some(PathBuf::from("before.svg"));

    let dir = tempdir("open_path_confirm");
    let fixture = dir.join("fixture.svg");
    std::fs::write(&fixture, VALID_SVG).unwrap();

    app.request_open_path(fixture.clone());
    assert_eq!(
        app.guard.pending_action,
        Some(PendingAction::OpenPath(fixture.clone()))
    );

    let runs = settle(&ctx, &mut app);
    let discard = locate(&runs, "Discard");
    click_button(&ctx, &mut app, discard);

    assert!(app.guard.pending_action.is_none());
    assert_eq!(
        app.document.entity_count(),
        1,
        "the fixture's one line must be adopted"
    );
    assert_eq!(app.current_file, Some(fixture));

    let runs = paint::painted_runs(&ctx, &mut app);
    assert!(!runs.iter().any(|r| r.text.trim() == "Discard"));
    assert_eq!(
        app.document.entity_count(),
        1,
        "not replayed: the import must not run a second time"
    );
}

/// AC 1, 3, 7 — same shape, for a parked `Exit`: Discard sends
/// `ViewportCommand::Close` exactly once, and an idle frame afterward neither
/// reopens the dialog nor sends a second `Close`.
#[test]
fn discard_confirms_exit_exactly_once_and_does_not_reopen() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    ctx.set_pixels_per_point(1.0);
    boot(&ctx, &mut app);
    with_lines(&mut app, 1);

    let out = ctx.run(close_request_input(), |c| app.update_ui(c));
    assert!(
        out.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .contains(&egui::ViewportCommand::CancelClose)
    );
    assert_eq!(app.guard.pending_action, Some(PendingAction::Exit));

    let runs = settle(&ctx, &mut app);
    let discard = locate(&runs, "Discard");
    let out = click_button(&ctx, &mut app, discard);
    assert!(
        out.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .contains(&egui::ViewportCommand::Close),
        "the confirmed Exit must send Close"
    );
    assert!(app.guard.pending_action.is_none());

    // The real framework's follow-up frame is not an idle one: sending
    // `ViewportCommand::Close` only *records* a fresh `ViewportEvent::Close`
    // (`egui_winit::process_viewport_command`, verified against the vendored
    // 0.29.1 source) rather than destroying the window, so the very next
    // frame's `RawInput` reports `close_requested() == true` again for that
    // same close — modeled here with the identical `close_request_input()`
    // fixture used for the operator's original close, not an empty frame.
    // Without `App::guard.exit_confirmed` this second `close_requested()` would
    // hit `request_exit` again, find the (still dirty — `Exit` performs no
    // save) document unsaved, re-park `PendingAction::Exit` and emit
    // `CancelClose`, cancelling the close the operator just confirmed and
    // reopening the dialog.
    let out = ctx.run(close_request_input(), |c| app.update_ui(c));
    assert!(
        !out.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .contains(&egui::ViewportCommand::CancelClose),
        "the re-delivered close request must not be cancelled"
    );
    assert!(
        app.guard.pending_action.is_none(),
        "the re-delivered close request must not re-park Exit"
    );
    let runs = paint::runs_in(&out.shapes);
    assert!(
        !runs.iter().any(|r| r.text.trim() == "Discard"),
        "a confirmed Exit must not reopen the dialog"
    );

    // A *second* re-delivered Close, not just one: `App::guard.exit_confirmed` must
    // stay latched rather than being read once and cleared. A one-shot guard
    // shaped `if app.guard.exit_confirmed { app.guard.exit_confirmed = false; return; }`
    // passes everything above — the first re-delivery still finds the flag
    // set — and only misbehaves on the request after that, once its own read
    // has cleared it: `poll_close_request` would fall through to
    // `request_exit` again, find the document still dirty, re-park
    // `PendingAction::Exit` and cancel this close too.
    let out = ctx.run(close_request_input(), |c| app.update_ui(c));
    assert!(
        !out.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .contains(&egui::ViewportCommand::CancelClose),
        "a second re-delivered close request must not be cancelled either"
    );
    assert!(
        app.guard.pending_action.is_none(),
        "a second re-delivered close request must not re-park Exit either"
    );
    let runs = paint::runs_in(&out.shapes);
    assert!(
        !runs.iter().any(|r| r.text.trim() == "Discard"),
        "a confirmed Exit must not reopen the dialog on a second re-delivery either"
    );
}

// ---------------------------------------------------------------------------
// The reported entry points themselves: File > New and File > Open, driven
// through a real click on the menubar (not just the keyboard shortcut) — the
// user's report named these two menu paths specifically.
// ---------------------------------------------------------------------------

/// A real click on File > New reaches `request_new` exactly as `Ctrl+N`
/// does: on a dirty document it parks `PendingAction::New` and leaves the
/// drawing untouched.
#[test]
fn file_menu_new_reaches_request_new_through_a_real_pointer_click() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    ctx.set_pixels_per_point(1.0);
    boot(&ctx, &mut app);
    with_lines(&mut app, 2);

    let menu_runs = open_file_menu(&ctx, &mut app);
    let new_item = locate(&menu_runs, "New");
    click_button(&ctx, &mut app, new_item);

    assert_eq!(
        app.guard.pending_action,
        Some(PendingAction::New),
        "a real click on File > New must reach request_new, same as Ctrl+N"
    );
    assert_eq!(
        app.document.entity_count(),
        2,
        "the drawing must survive until Discard is clicked"
    );
}

/// A real click on File > Open… reaches `request_open` exactly as `Ctrl+O`
/// does: on a dirty document it parks `PendingAction::Open` without ever
/// touching the filesystem, so no native `rfd` dialog opens and this test
/// cannot hang (ADR 0002 §A4 rule 1).
#[test]
fn file_menu_open_reaches_request_open_through_a_real_pointer_click() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    ctx.set_pixels_per_point(1.0);
    boot(&ctx, &mut app);
    with_lines(&mut app, 1);

    let menu_runs = open_file_menu(&ctx, &mut app);
    let open_item = locate(&menu_runs, "Open…");
    click_button(&ctx, &mut app, open_item);

    assert_eq!(
        app.guard.pending_action,
        Some(PendingAction::Open),
        "a real click on File > Open… must reach request_open, same as Ctrl+O"
    );
    assert_eq!(
        app.document.entity_count(),
        1,
        "the drawing must be untouched"
    );
}

// ---------------------------------------------------------------------------
// AC 4 — Cancel preserves everything, for each of the three actions
// ---------------------------------------------------------------------------

/// AC 4 — Cancel for a parked `New` leaves entities, selection, history
/// revision, `current_file` and the dirty signal exactly as they were.
#[test]
fn cancel_preserves_state_for_new() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    ctx.set_pixels_per_point(1.0);
    boot(&ctx, &mut app);

    with_lines(&mut app, 2);
    app.document.selection.add(0);
    app.current_file = Some(PathBuf::from("keep.svg"));
    let _ = ctx.run(raw_input(vec![]), |c| app.update_ui(c)); // let sync_dirty settle
    let before = snapshot(&app);

    let _ = ctx.run(raw_input(key_events(egui::Key::N, ctrl())), |c| {
        app.update_ui(c)
    });
    assert_eq!(app.guard.pending_action, Some(PendingAction::New));

    let runs = settle(&ctx, &mut app);
    let cancel = locate(&runs, "Cancel");
    click_button(&ctx, &mut app, cancel);

    assert!(app.guard.pending_action.is_none());
    assert_unchanged(&app, &before);
}

/// AC 4 — same shape, for a parked `OpenPath`.
#[test]
fn cancel_preserves_state_for_open_path() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    ctx.set_pixels_per_point(1.0);
    boot(&ctx, &mut app);

    with_lines(&mut app, 2);
    app.document.selection.add(1);
    app.current_file = Some(PathBuf::from("keep.svg"));
    let _ = ctx.run(raw_input(vec![]), |c| app.update_ui(c));
    let before = snapshot(&app);

    let dir = tempdir("open_path_cancel");
    let fixture = dir.join("fixture.svg");
    std::fs::write(&fixture, VALID_SVG).unwrap();
    app.request_open_path(fixture.clone());
    assert_eq!(
        app.guard.pending_action,
        Some(PendingAction::OpenPath(fixture))
    );

    let runs = settle(&ctx, &mut app);
    let cancel = locate(&runs, "Cancel");
    click_button(&ctx, &mut app, cancel);

    assert!(app.guard.pending_action.is_none());
    assert_unchanged(&app, &before);
}

/// AC 4 — same shape, for a parked `Exit`, plus: Cancel must never send a
/// viewport command.
#[test]
fn cancel_preserves_state_for_exit() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    ctx.set_pixels_per_point(1.0);
    boot(&ctx, &mut app);

    with_lines(&mut app, 2);
    app.document.selection.add(0);
    app.current_file = Some(PathBuf::from("keep.svg"));
    let _ = ctx.run(raw_input(vec![]), |c| app.update_ui(c));
    let before = snapshot(&app);

    let out = ctx.run(close_request_input(), |c| app.update_ui(c));
    assert!(
        out.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .contains(&egui::ViewportCommand::CancelClose)
    );
    assert_eq!(app.guard.pending_action, Some(PendingAction::Exit));

    let runs = settle(&ctx, &mut app);
    let cancel = locate(&runs, "Cancel");
    let out = click_button(&ctx, &mut app, cancel);

    assert!(
        !out.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .contains(&egui::ViewportCommand::Close),
        "Cancel must never send Close"
    );
    assert!(app.guard.pending_action.is_none());
    assert_unchanged(&app, &before);
}

// ---------------------------------------------------------------------------
// AC 5 — a failing OpenPath preserves the drawing and surfaces the error
// ---------------------------------------------------------------------------

/// AC 5 — a parked `OpenPath` whose file is missing, confirmed through the
/// real pointer path, preserves the original drawing, history and filename,
/// and surfaces the error modal; it neither clears the drawing nor retries.
#[test]
fn open_path_missing_file_confirmed_preserves_drawing_and_surfaces_error() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    ctx.set_pixels_per_point(1.0);
    boot(&ctx, &mut app);

    with_lines(&mut app, 1);
    app.current_file = Some(PathBuf::from("original.svg"));
    let before_entities = app.document.entities.clone();
    let before_revision = app.history.revision();

    let dir = tempdir("open_path_missing");
    let missing = dir.join("does_not_exist.svg");
    app.request_open_path(missing.clone());
    assert_eq!(
        app.guard.pending_action,
        Some(PendingAction::OpenPath(missing))
    );

    let runs = settle(&ctx, &mut app);
    let discard = locate(&runs, "Discard");
    click_button(&ctx, &mut app, discard);

    assert!(
        app.guard.pending_action.is_none(),
        "the dialog must be dismissed either way"
    );
    assert_eq!(
        app.document.entities, before_entities,
        "a failed open must not clear the drawing"
    );
    assert_eq!(app.history.revision(), before_revision);
    assert_eq!(app.current_file, Some(PathBuf::from("original.svg")));
    assert!(app.error_message.is_some(), "the failure must surface");

    // The error modal's own Window is freshly created the frame
    // error_message first became Some, and is itself unsettled that frame
    // (trap 7): one more idle frame is what proves it, not a retry.
    let _ = ctx.run(raw_input(vec![]), |c| app.update_ui(c));
    let runs = paint::painted_runs(&ctx, &mut app);
    assert!(
        runs.iter().any(|r| r.text.contains("Could not read")),
        "the error modal must surface the failure"
    );
    assert!(!runs.iter().any(|r| r.text.trim() == "Discard"));
    assert_eq!(
        app.document.entities, before_entities,
        "no automatic retry must have run"
    );
}

/// AC 5 — same shape, for a parked `OpenPath` whose file exists but fails to
/// parse as SVG.
#[test]
fn open_path_malformed_svg_confirmed_preserves_drawing_and_surfaces_error() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    ctx.set_pixels_per_point(1.0);
    boot(&ctx, &mut app);

    with_lines(&mut app, 1);
    app.current_file = Some(PathBuf::from("original.svg"));
    let before_entities = app.document.entities.clone();
    let before_revision = app.history.revision();

    let dir = tempdir("open_path_malformed");
    let fixture = dir.join("malformed.svg");
    std::fs::write(&fixture, MALFORMED_SVG).unwrap();
    app.request_open_path(fixture.clone());
    assert_eq!(
        app.guard.pending_action,
        Some(PendingAction::OpenPath(fixture))
    );

    let runs = settle(&ctx, &mut app);
    let discard = locate(&runs, "Discard");
    click_button(&ctx, &mut app, discard);

    assert!(app.guard.pending_action.is_none());
    assert_eq!(
        app.document.entities, before_entities,
        "a failed parse must not clear the drawing"
    );
    assert_eq!(app.history.revision(), before_revision);
    assert_eq!(app.current_file, Some(PathBuf::from("original.svg")));
    assert!(app.error_message.is_some());

    let _ = ctx.run(raw_input(vec![]), |c| app.update_ui(c));
    let runs = paint::painted_runs(&ctx, &mut app);
    assert!(
        runs.iter().any(|r| r.text.contains("SVG import failed")),
        "the error modal must surface the parse failure"
    );
    assert_eq!(app.document.entities, before_entities);
}

// ---------------------------------------------------------------------------
// AC 6 — the dialog owns the frame while it is up
// ---------------------------------------------------------------------------

/// AC 6 — while the dialog is open, a pointer click at a canvas location
/// behind it does not reach an armed drawing tool, and a click on a toolbar
/// button behind it does not switch tools. The dialog is still parked either
/// way.
#[test]
fn pointer_input_behind_the_dialog_is_inert() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    ctx.set_pixels_per_point(1.0);
    let central_panel = boot(&ctx, &mut app);

    with_lines(&mut app, 1);
    assert_eq!(
        app.tool_manager.active_tool_name(),
        "Select",
        "control: Select is the default tool"
    );

    let _ = ctx.run(raw_input(key_events(egui::Key::N, ctrl())), |c| {
        app.update_ui(c)
    });
    assert_eq!(app.guard.pending_action, Some(PendingAction::New));

    settle(&ctx, &mut app);
    let dialog_id = egui::Id::new("Discard unsaved changes?");
    let dialog_rect = ctx
        .memory(|m| m.area_rect(dialog_id))
        .expect("the confirm dialog's Area must be placed by now");

    // A point genuinely inside the dialog's own rect — its blank top-left
    // corner, away from its label and its two buttons — never a spot merely
    // elsewhere on the canvas, which is unblocked ground and would make this
    // a test of ordinary input, not of the modal (`Order::Middle` always
    // outranks the `CentralPanel`'s `Order::Background` for hit-testing at
    // egui 0.29.1, regardless of paint-call order — verified against the
    // vendored source during this demand's investigation).
    let behind_pos = egui::pos2(dialog_rect.left() + 4.0, dialog_rect.top() + 4.0);

    // A drawing entity placed exactly under that point: "no click-through to
    // the drawing underneath the modal" (LCV-113, restated in this demand's
    // Scope) is witnessed directly against a real entity, not only inferred
    // from an armed tool's anchor.
    let world_pos = app
        .camera
        .screen_to_world(behind_pos - central_panel.min.to_vec2());
    app.history.commit(
        Box::new(CreateLine::new(Line::new(
            world_pos,
            world_pos + Vec2::new(2.0, 0.0),
        ))),
        &mut app.document,
    );
    let entities_before = app.document.entities.clone();

    // Select is active: a click-through would select the entity underneath.
    click_button(&ctx, &mut app, behind_pos);
    assert!(
        app.document.selection.is_empty(),
        "a click behind the dialog must not select the drawing underneath it"
    );
    assert_eq!(
        app.document.entities, entities_before,
        "a click behind the dialog must not draw anything either"
    );

    // Arm a drawing tool and repeat at the same blocked position: a
    // click-through would anchor the first point of a new line.
    tap(&ctx, &mut app, egui::Key::L, egui::Modifiers::NONE);
    assert_eq!(app.tool_manager.active_tool_name(), "LINE");
    click_button(&ctx, &mut app, behind_pos);
    assert!(
        app.tool_manager.anchor().is_none(),
        "a click behind the dialog must not reach the armed drawing tool"
    );
    assert_eq!(app.document.entities, entities_before);

    assert_eq!(
        app.guard.pending_action,
        Some(PendingAction::New),
        "the dialog must still be up throughout"
    );
}

/// AC 6 — a second click at the discard button's old screen position, after
/// the first click already dismissed the dialog, must not dispatch a second
/// destructive action.
#[test]
fn a_second_click_after_discard_is_handled_does_not_double_dispatch() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    ctx.set_pixels_per_point(1.0);
    boot(&ctx, &mut app);

    with_lines(&mut app, 2);
    let _ = ctx.run(raw_input(key_events(egui::Key::N, ctrl())), |c| {
        app.update_ui(c)
    });
    let runs = settle(&ctx, &mut app);
    let discard = locate(&runs, "Discard");

    click_button(&ctx, &mut app, discard);
    assert!(app.guard.pending_action.is_none());
    assert_eq!(app.document.entity_count(), 0);

    // Dirty the document again and click at the exact same screen position,
    // now that the dialog is gone: the canvas is normal, unblocked ground
    // there, so the click may land a harmless Select-tool deselect (which
    // touches `document.selection`, not `document.entities`) — what must
    // never happen is `action_new` running a second time.
    with_lines(&mut app, 3);
    let entities_before_second_click = app.document.entities.clone();
    click_button(&ctx, &mut app, discard);

    assert_eq!(
        app.document.entities, entities_before_second_click,
        "a stray click at the dialog's old position must not run action_new again"
    );
    assert!(app.guard.pending_action.is_none());
}

// ---------------------------------------------------------------------------
// AC 7 — repeated native Close requests
// ---------------------------------------------------------------------------

/// AC 7 — two `ViewportEvent::Close` requests in a row, while the dialog is
/// up, neither replace the parked action nor consume the Discard click that
/// follows.
#[test]
fn repeated_close_requests_do_not_replace_the_parked_action_or_consume_the_next_click() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    ctx.set_pixels_per_point(1.0);
    boot(&ctx, &mut app);
    with_lines(&mut app, 1);

    let out1 = ctx.run(close_request_input(), |c| app.update_ui(c));
    assert!(
        out1.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .contains(&egui::ViewportCommand::CancelClose)
    );
    assert_eq!(app.guard.pending_action, Some(PendingAction::Exit));

    let out2 = ctx.run(close_request_input(), |c| app.update_ui(c));
    assert!(
        out2.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .contains(&egui::ViewportCommand::CancelClose)
    );
    assert_eq!(
        app.guard.pending_action,
        Some(PendingAction::Exit),
        "a repeated close request must not replace the parked action"
    );

    let runs = settle(&ctx, &mut app);
    let discard = locate(&runs, "Discard");
    let out = click_button(&ctx, &mut app, discard);

    assert!(
        out.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .contains(&egui::ViewportCommand::Close),
        "the click after the repeated close requests must still confirm Exit"
    );
    assert!(app.guard.pending_action.is_none());
}

// ---------------------------------------------------------------------------
// AC 2 — painted_runs_at's own precondition
// ---------------------------------------------------------------------------

/// AC 2 — a test that leaves the context at a non-1.0 scale must fail loudly
/// instead of silently reading positions at the wrong scale.
#[test]
#[should_panic(expected = "pixels_per_point")]
fn painted_runs_at_panics_when_pixels_per_point_is_not_one() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    ctx.set_pixels_per_point(2.0);
    let _ = paint::painted_runs_at(&ctx, &mut app, harness::SCREEN, Vec::new());
}
