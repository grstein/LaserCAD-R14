//! tests/lcv103.rs — keyboard routing regressions (ADR 0002 decision A).
//!
//! Every test here drives the **real** frame body through
//! `egui::Context::run(raw_input(...), |ctx| app.update_ui(ctx))`, which is
//! the whole point: `tests/lcv070.rs` proves the dispatch table is correct,
//! these prove it is reachable, fires exactly once, and stops at the keyboard
//! focus gate.
//!
//! Three traps, all documented in ADR 0002 and all stepped around here:
//!
//! - key presses always come as a press **and** a release (`harness::tap`),
//!   otherwise egui rewrites the next press on the same key as a repeat and
//!   `src/ui/shortcuts.rs` drops it;
//! - pointer interaction needs a warm-up frame carrying `PointerMoved` alone
//!   before the frame carrying the button;
//! - `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S` would open a blocking `rfd` dialog
//!   and are never sent.

mod harness;

use harness::{frame, key_events, raw_input, submit_command, tap, type_command};
use lasercad::app::App;
use lasercad::document::CreateLine;
use lasercad::geometry::{Line, Vec2};
use lasercad::tools::{SelectTool, TextTool};

// ---------------------------------------------------------------------------
// Local helpers (the harness exposes exactly five items; these are test-local)
// ---------------------------------------------------------------------------

fn none() -> egui::Modifiers {
    egui::Modifiers::NONE
}

/// `Ctrl` as egui reports it on Linux: `ctrl` and `command` both set, which is
/// what `Modifiers::command_only()` in `dispatch_shortcuts` tests for.
fn ctrl() -> egui::Modifiers {
    egui::Modifiers {
        ctrl: true,
        command: true,
        ..egui::Modifiers::NONE
    }
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

/// Boot an app and run one frame, returning the context, the app, and the
/// central-panel (viewport) rect the frame laid out.
///
/// The first frame is what registers every widget rect for hit-testing and
/// what syncs `camera.viewport_size_px` away from its zero-area sentinel.
fn boot() -> (egui::Context, App, egui::Rect) {
    let ctx = egui::Context::default();
    let mut app = App::default();
    let mut viewport = egui::Rect::NOTHING;
    let _ = ctx.run(raw_input(vec![]), |c| {
        app.update_ui(c);
        viewport = c.available_rect();
    });
    (ctx, app, viewport)
}

/// Click the command-line `TextEdit`, which sits just below the viewport, and
/// assert it took keyboard focus.
///
/// The assert is load-bearing: a gated test that "passes" because the click
/// missed the widget would prove nothing.
fn focus_command_line(ctx: &egui::Context, app: &mut App, viewport: egui::Rect) {
    let pos = egui::pos2(viewport.center().x, viewport.max.y + 12.0);
    frame(ctx, app, vec![egui::Event::PointerMoved(pos)]); // warm-up
    frame(ctx, app, click_events(pos));
    assert!(
        ctx.wants_keyboard_input(),
        "the command-line TextEdit must hold keyboard focus for a gate test"
    );
}

/// Activate `TextTool`, click an anchor in the viewport, and drive the real
/// raw-input flow (LCV-112) up to the height prompt with `"HI"` pending.
///
/// LCV-111 repurposed the gate's typed-character row to seed and focus the
/// command line, which is exactly what makes this reachable: the anchor
/// click puts the tool in `WaitingText`, `submit_command` types `HI` and
/// taps Enter, and the tool advances to `WaitingHeight` with nothing
/// committed yet. This is the real path — the old per-character trait method
/// no longer exists (LCV-112 deleted it along with `TextTool`'s own
/// per-character handling).
fn text_tool_at_height_prompt(ctx: &egui::Context, app: &mut App, viewport: egui::Rect) {
    app.tool_manager.set_tool(Box::new(TextTool::default()));
    let p = viewport.center();
    frame(ctx, app, vec![egui::Event::PointerMoved(p)]); // warm-up
    frame(ctx, app, click_events(p)); // anchor -> WaitingText
    submit_command(ctx, app, "HI"); // WaitingText -> WaitingHeight
    assert_eq!(
        app.tool_manager.active_status_text(),
        "TEXT Specify height <5>:"
    );
    assert!(
        !app.tool_manager.preview().is_empty(),
        "WaitingHeight must preview the pending string at the default height"
    );
}

/// Put `n` lines in the document without leaving anything on the undo stack.
fn with_lines(app: &mut App, n: usize) {
    for i in 0..n {
        let y = i as f64;
        app.commit(Box::new(CreateLine::new(Line::new(
            Vec2::new(0.0, y),
            Vec2::new(10.0, y),
        ))));
    }
    assert_eq!(app.document.entity_count(), n);
}

// ---------------------------------------------------------------------------
// AC#1, AC#2, AC#3 — the frame body is reachable from a test
// ---------------------------------------------------------------------------

/// AC#1 — `App::update_ui` is public, carries the frame body, and can be
/// driven headlessly. Proof that the `CentralPanel` body really ran: the
/// camera viewport size is synced away from the `[0.0, 0.0]` sentinel.
#[test]
fn update_ui_is_the_frame_body() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    assert_eq!(app.camera.viewport_size_px, [0.0, 0.0]);
    frame(&ctx, &mut app, vec![]);
    assert!(
        app.camera.viewport_size_px[0] > 0.0 && app.camera.viewport_size_px[1] > 0.0,
        "the CentralPanel body must run inside update_ui"
    );
}

// ---------------------------------------------------------------------------
// AC#5 — D4: F8 toggles ortho exactly once per press
// ---------------------------------------------------------------------------

/// AC#5 — the ADR 0002 §A5 reference test.
#[test]
fn f8_toggles_ortho_exactly_once_per_press() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    assert!(!app.ortho_enabled, "ortho starts off");

    let _ = ctx.run(
        raw_input(key_events(egui::Key::F8, egui::Modifiers::NONE)),
        |ctx| app.update_ui(ctx),
    );
    assert!(
        app.ortho_enabled,
        "one F8 press must toggle ortho on (D4: double dispatch made this a no-op)"
    );

    tap(&ctx, &mut app, egui::Key::F8, egui::Modifiers::NONE);
    assert!(
        !app.ortho_enabled,
        "a second F8 press must toggle it back off"
    );

    tap(&ctx, &mut app, egui::Key::F8, egui::Modifiers::NONE);
    assert!(app.ortho_enabled, "and a third turns it on again");
}

// ---------------------------------------------------------------------------
// AC#6 — D4: Ctrl+Z / Ctrl+Y move the history by exactly one step
// ---------------------------------------------------------------------------

/// AC#6 — one Ctrl+Z tap undoes one command, not two.
#[test]
fn ctrl_z_undoes_exactly_one_command() {
    let (ctx, mut app, _viewport) = boot();
    with_lines(&mut app, 3);

    tap(&ctx, &mut app, egui::Key::Z, ctrl());
    assert_eq!(
        app.document.entity_count(),
        2,
        "one Ctrl+Z must undo exactly one command (D4 undid two)"
    );
}

/// AC#6 — and one Ctrl+Y tap redoes exactly one.
#[test]
fn ctrl_y_redoes_exactly_one_command() {
    let (ctx, mut app, _viewport) = boot();
    with_lines(&mut app, 3);

    tap(&ctx, &mut app, egui::Key::Z, ctrl());
    assert_eq!(app.document.entity_count(), 2);
    tap(&ctx, &mut app, egui::Key::Y, ctrl());
    assert_eq!(
        app.document.entity_count(),
        3,
        "one Ctrl+Y must redo exactly one command"
    );
}

// ---------------------------------------------------------------------------
// AC#7, AC#8 — D5: Enter reaches the tool, but not through a focused widget
// ---------------------------------------------------------------------------

/// AC#7, rebuilt on LCV-112's raw-input path — the accepted height commits
/// the TEXT geometry through `crate::app::submit`, the only route a tool can
/// commit from now that the old per-character trait method and `TextTool`'s
/// own Enter handling are gone. `submit_command`'s Enter tap only fires when
/// the field held focus in the previous frame (ADR 0003 §F3 trap 5), so this
/// still proves Enter is reaching the application at all (D5's original
/// concern).
#[test]
fn enter_commits_text_tool() {
    let (ctx, mut app, viewport) = boot();
    text_tool_at_height_prompt(&ctx, &mut app, viewport);
    assert!(
        !app.history.can_undo(),
        "nothing committed before the height is accepted"
    );

    submit_command(&ctx, &mut app, "10");

    assert!(
        app.history.can_undo(),
        "the accepted height must commit the TEXT geometry"
    );
    assert!(app.document.entity_count() > 0, "Hershey strokes committed");
    assert!(app.tool_manager.preview().is_empty(), "tool back to idle");
}

/// AC#8, amended by LCV-111 AC 14 and rebuilt by LCV-112 — the operator can
/// also reach the command line by **clicking** it directly (not only by
/// typing, LCV-111's seed path), and one Enter press still reaches the tool
/// **exactly once**. The regression this guards: the gate's `wants_kbd`
/// early-out must keep `TOOL_ROUTED_KEYS` from firing a *second* Enter on top
/// of the widget's own `lost_focus() && key_pressed(Enter)` submit.
#[test]
fn enter_reaches_the_tool_exactly_once_while_focused() {
    let (ctx, mut app, viewport) = boot();
    app.tool_manager.set_tool(Box::new(TextTool::default()));
    let p = viewport.center();
    frame(&ctx, &mut app, vec![egui::Event::PointerMoved(p)]); // warm-up
    frame(&ctx, &mut app, click_events(p)); // anchor -> WaitingText

    focus_command_line(&ctx, &mut app, viewport);
    type_command(&ctx, &mut app, "HI");
    tap(&ctx, &mut app, egui::Key::Enter, none());
    assert_eq!(
        app.tool_manager.active_status_text(),
        "TEXT Specify height <5>:"
    );

    type_command(&ctx, &mut app, "10");
    tap(&ctx, &mut app, egui::Key::Enter, none());

    assert_eq!(
        app.history.len(),
        1,
        "Enter must commit exactly once — not twice (double dispatch), not zero"
    );
    assert!(app.document.entity_count() > 0, "Hershey strokes committed");
    assert!(app.tool_manager.preview().is_empty(), "tool back to idle");
}

// ---------------------------------------------------------------------------
// AC#9 — D8: F / zoom-extents is gated
// ---------------------------------------------------------------------------

/// AC#9 — typing `f` into a focused text field must not zoom the drawing.
/// A real keyboard delivers both the key event and the character, so both go
/// into the frame.
#[test]
fn f_does_not_zoom_while_text_widget_focused() {
    let (ctx, mut app, viewport) = boot();
    with_lines(&mut app, 2);
    focus_command_line(&ctx, &mut app, viewport);
    let mm_per_px = app.camera.mm_per_px;
    let center = app.camera.center_world;

    let mut events = key_events(egui::Key::F, none());
    events.push(egui::Event::Text("f".to_owned()));
    frame(&ctx, &mut app, events);

    assert_eq!(app.camera.mm_per_px, mm_per_px, "zoom must not change");
    assert_eq!(app.camera.center_world, center, "pan must not change");
}

/// AC#9 — with nothing focused, `F` still zooms to extents.
#[test]
fn f_zooms_when_nothing_focused() {
    let (ctx, mut app, _viewport) = boot();
    app.commit(Box::new(CreateLine::new(Line::new(
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 150.0),
    ))));
    let mm_per_px = app.camera.mm_per_px;

    tap(&ctx, &mut app, egui::Key::F, none());

    assert_ne!(app.camera.mm_per_px, mm_per_px, "F must zoom to extents");
    assert_eq!(app.camera.center_world, Vec2::new(150.0, 125.0));
}

// ---------------------------------------------------------------------------
// AC#10 — D8: Delete / Backspace are gated
// ---------------------------------------------------------------------------

/// AC#10 — Backspace correcting a typo in a text field must not delete the
/// selected entities.
#[test]
fn backspace_does_not_delete_while_focused() {
    let (ctx, mut app, viewport) = boot();
    with_lines(&mut app, 2);
    app.history = lasercad::document::History::default(); // start from a clean undo stack
    app.document.selection.set([0, 1]);
    app.tool_manager.set_tool(Box::new(SelectTool::default()));
    focus_command_line(&ctx, &mut app, viewport);

    tap(&ctx, &mut app, egui::Key::Backspace, none());

    assert_eq!(
        app.document.entity_count(),
        2,
        "Backspace in a text field must not delete entities"
    );
    assert!(!app.history.can_undo(), "nothing was committed");
}

/// AC#10 — with nothing focused, Delete removes the selection.
#[test]
fn delete_removes_selection_when_unfocused() {
    let (ctx, mut app, _viewport) = boot();
    with_lines(&mut app, 2);
    app.history = lasercad::document::History::default();
    app.document.selection.set([0, 1]);
    app.tool_manager.set_tool(Box::new(SelectTool::default()));

    tap(&ctx, &mut app, egui::Key::Delete, none());

    assert_eq!(
        app.document.entity_count(),
        0,
        "Delete must reach SelectTool"
    );
    assert!(app.history.can_undo());
}

// ---------------------------------------------------------------------------
// AC#11 — D8: typed characters are gated
// ---------------------------------------------------------------------------

/// AC#11, amended by LCV-111 AC 21 — the gate reads `Event::Text` only when
/// no text widget has focus, and what it does with it is now "seed and focus
/// the command line" instead of "forward to the active tool".
///
/// The focused half is the double-insert guard: while the field has focus the
/// `TextEdit` itself consumes the character, so the gate must add nothing —
/// one keystroke, one character in the buffer.
#[test]
fn text_event_gated_by_focus() {
    let (ctx, mut app, _viewport) = boot();
    assert!(app.command_line_input.is_empty());

    // Unfocused: the gate seeds the field with exactly one copy.
    frame(&ctx, &mut app, vec![egui::Event::Text("A".to_owned())]);
    assert_eq!(
        app.command_line_input, "A",
        "an unfocused typed character must seed the command line exactly once"
    );

    // The seed also asked for focus, granted at the end of that frame.
    assert!(
        ctx.wants_keyboard_input(),
        "typing must focus the command line (LCV-111 AC 21)"
    );

    // Focused: the widget consumes the character and the gate adds nothing.
    frame(&ctx, &mut app, vec![egui::Event::Text("b".to_owned())]);
    assert_eq!(
        app.command_line_input.len(),
        2,
        "a typed character must land exactly once while the field has focus, \
         got {:?}",
        app.command_line_input
    );
}

// ---------------------------------------------------------------------------
// AC#12 — Escape is ungated and handled exactly once
// ---------------------------------------------------------------------------

/// AC#12 — one Escape tap clears the command-line buffer *and* cancels the
/// tool, in a single frame, with the tool cancelled exactly once. Reaches
/// `WaitingHeight` through the real raw-input path, then types a partial
/// height (`"40"`, never submitted) before cancelling.
#[test]
fn escape_cancels_tool_and_clears_command_line_once() {
    let (ctx, mut app, viewport) = boot();
    text_tool_at_height_prompt(&ctx, &mut app, viewport);
    type_command(&ctx, &mut app, "40");

    tap(&ctx, &mut app, egui::Key::Escape, none());

    assert!(
        app.command_line_input.is_empty(),
        "Escape must clear the command-line buffer"
    );
    assert!(
        app.tool_manager.preview().is_empty(),
        "Escape must cancel the active tool even while a text widget has focus"
    );
    assert!(!app.history.can_undo(), "cancelling commits nothing");
}

// ---------------------------------------------------------------------------
// AC#13 — the "yes" rows of the gate table
// ---------------------------------------------------------------------------

/// AC#13 — global commands and view toggles fire even with a focused widget.
#[test]
fn global_commands_fire_while_focused() {
    let (ctx, mut app, viewport) = boot();
    with_lines(&mut app, 2);
    focus_command_line(&ctx, &mut app, viewport);

    tap(&ctx, &mut app, egui::Key::Z, ctrl());
    assert_eq!(
        app.document.entity_count(),
        1,
        "Ctrl+Z is ungated and undoes exactly one command"
    );

    assert!(!app.ortho_enabled);
    tap(&ctx, &mut app, egui::Key::F8, none());
    assert!(app.ortho_enabled, "F8 is ungated");
}
