//! LCV-112 — TEXT: raw-input mode for the string, then the height,
//! in exactly one undo step (ADR 0003 §D, §F4).
//!
//! Every test here drives the **real** frame body through
//! `egui::Context::run(raw_input(...), |ctx| app.update_ui(ctx))` —
//! `tests/it/app/keyboard_routing.rs`'s and `tests/it/cmdline/drives_tools.rs`'s pattern, and the whole reason
//! this file exists rather than trusting `src/tools/text.rs`'s inline unit
//! tests alone: those prove the state machine is correct, this proves it is
//! *reachable* from a keystroke and a click.
//!
//! Two traps matter here specifically (ADR 0003 §F3): trap 3 — the anchor
//! click needs a warm-up frame carrying `PointerMoved` alone, or the
//! viewport's widget rect is never registered and the click is never seen —
//! and trap 5 — Enter only submits if the command line held focus in the
//! *previous* frame, so every submit is `harness::submit_command`, never a
//! bare `tap(Enter)`.

use crate::harness;

use harness::{frame, key_events, raw_input, submit_command, tap, type_command};
use lasercad::app::App;
use lasercad::document::Entity;
use lasercad::geometry::Vec2;
use lasercad::text::layout_text;
use lasercad::tools::TextTool;

fn none() -> egui::Modifiers {
    egui::Modifiers::NONE
}

/// `Ctrl` as egui reports it on Linux — both `ctrl` and `command` set, which
/// is what `Modifiers::command_only()` in `dispatch_shortcuts` tests for.
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

/// Boot an app and run the first frame, which registers every widget rect
/// and syncs the camera away from its zero-area sentinel.
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

/// Click the anchor at the viewport's center: a warm-up `PointerMoved` frame
/// (ADR 0003 §F3 trap 3), then the frame carrying the click itself.
fn click_anchor(ctx: &egui::Context, app: &mut App, viewport: egui::Rect) {
    let p = viewport.center();
    frame(ctx, app, vec![egui::Event::PointerMoved(p)]);
    frame(ctx, app, click_events(p));
}

fn lines_of(entities: &[Entity]) -> Vec<(Vec2, Vec2)> {
    entities
        .iter()
        .filter_map(|e| match e {
            Entity::Line(l) => Some((l.p1, l.p2)),
            _ => None,
        })
        .collect()
}

fn y_extent(entities: &[Entity]) -> (f64, f64) {
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for e in entities {
        if let Entity::Line(l) = e {
            min_y = min_y.min(l.p1.y).min(l.p2.y);
            max_y = max_y.max(l.p1.y).max(l.p2.y);
        }
    }
    (min_y, max_y)
}

// ---------------------------------------------------------------------------
// AC 16 — the milestone check
// ---------------------------------------------------------------------------

/// AC 16 — `D` activates TEXT, a click sets the anchor, `HELLO` ⏎ `10` ⏎
/// commits ten-millimetre text as a single undo step, and one Ctrl+Z removes
/// all of it.
#[test]
fn text_flow_commits_ten_millimetre_text_in_one_undo_step() {
    let (ctx, mut app, viewport) = boot();
    tap(&ctx, &mut app, egui::Key::D, none());
    assert_eq!(app.tool_manager.active_tool_name(), "TEXT");

    click_anchor(&ctx, &mut app, viewport);
    let anchor_y = app
        .last_cursor_world
        .expect("the click must have moved the cursor into the viewport")
        .y;
    submit_command(&ctx, &mut app, "HELLO");
    submit_command(&ctx, &mut app, "10");

    // The cap-height check, not the AC 8 exact-span oracle: `HELLO` contains
    // 'O', which (per the Hershey table and the demand's own §Risks note)
    // descends *below* the baseline by a full cap height — using it for a
    // `min_y == baseline` assertion is exactly the trap AC 8 names `'H'` to
    // avoid. The cap top is unaffected: every capital, 'O' included, reaches
    // exactly ten millimetres above the anchor.
    let (min_y, max_y) = y_extent(&app.document.entities);
    assert!(
        (max_y - (anchor_y + 10.0)).abs() < 1e-9,
        "cap top ten millimetres above the anchor, got {max_y}"
    );
    assert!(
        min_y <= anchor_y,
        "the baseline row must not be above the anchor"
    );
    assert_eq!(app.history.len(), 1, "the whole word is one undo step");

    tap(&ctx, &mut app, egui::Key::Z, ctrl());
    assert_eq!(
        app.document.entity_count(),
        0,
        "one Ctrl+Z must remove the whole word"
    );
}

/// The final commit must release focus in the same frame it completes.
///
/// `src/ui/command_line.rs` reads `wants_raw_input()` *after* the Enter
/// branch runs, so an accepted height that returns `TextTool` to `Idle`
/// stops requesting focus right then — it must not take an extra frame, and
/// it must not linger. Proved behaviourally, not by reading the focus flag:
/// on the very next frame a bare `L` activates LINE instead of typing an `l`
/// into a command line that is still (wrongly) holding focus. A reverted fix
/// — checking `wants_raw_input()` *before* `submit()` runs instead of after
/// — passes every other test in this suite and still fails this one, which
/// is why it exists.
#[test]
fn the_final_commit_releases_focus_so_the_next_bare_letter_activates_a_tool() {
    let (ctx, mut app, viewport) = boot();
    app.tool_manager.set_tool(Box::new(TextTool::default()));
    click_anchor(&ctx, &mut app, viewport);
    submit_command(&ctx, &mut app, "HELLO");
    submit_command(&ctx, &mut app, "10");
    assert_eq!(
        app.tool_manager.active_tool_name(),
        "TEXT",
        "still TEXT, just idle"
    );
    assert_eq!(app.history.len(), 1, "the commit already happened");

    let mut events = key_events(egui::Key::L, none());
    events.push(egui::Event::Text("l".to_owned()));
    frame(&ctx, &mut app, events);

    assert_eq!(
        app.tool_manager.active_tool_name(),
        "LINE",
        "the command line must not still be holding focus after the commit"
    );
    assert_eq!(
        app.command_line_input, "",
        "the bare L must not have landed in the field as text"
    );
}

// ---------------------------------------------------------------------------
// AC 17 — the default height
// ---------------------------------------------------------------------------

/// AC 17 — Enter alone at the height prompt commits at the 5 mm default,
/// still in one undo step.
#[test]
fn enter_at_the_height_prompt_uses_five_millimetres() {
    let (ctx, mut app, viewport) = boot();
    app.tool_manager.set_tool(Box::new(TextTool::default()));
    click_anchor(&ctx, &mut app, viewport);
    let anchor_y = app
        .last_cursor_world
        .expect("the click must have moved the cursor into the viewport")
        .y;
    submit_command(&ctx, &mut app, "H");

    submit_command(&ctx, &mut app, "");

    let (min_y, max_y) = y_extent(&app.document.entities);
    assert!((min_y - anchor_y).abs() < 1e-9);
    assert!(
        (max_y - (anchor_y + 5.0)).abs() < 1e-9,
        "the default is 5 mm, got {max_y}"
    );
    assert_eq!(app.history.len(), 1);
}

// ---------------------------------------------------------------------------
// AC 18 — the refusal
// ---------------------------------------------------------------------------

/// AC 18 — an out-of-range or unparseable height commits nothing, keeps the
/// text, and re-prompts with the AC 6 retry literal; a following valid
/// height still commits normally.
#[test]
fn invalid_height_keeps_the_text_and_re_prompts() {
    let (ctx, mut app, viewport) = boot();
    app.tool_manager.set_tool(Box::new(TextTool::default()));
    click_anchor(&ctx, &mut app, viewport);
    submit_command(&ctx, &mut app, "HELLO");

    for bad in ["abc", "0", "5000", "10,5"] {
        submit_command(&ctx, &mut app, bad);
        assert_eq!(app.document.entity_count(), 0, "{bad} must not commit");
        assert_eq!(app.history.len(), 0);
        assert_eq!(
            app.tool_manager.active_status_text(),
            "TEXT Height must be between 0.1 and 2000 mm. Specify height <5>:",
            "{bad} must show the retry prompt"
        );
    }

    submit_command(&ctx, &mut app, "10");
    assert!(
        app.document.entity_count() > 0,
        "the operator never lost the string"
    );
    assert_eq!(app.history.len(), 1);
}

// ---------------------------------------------------------------------------
// AC 3 — raw mode holds focus
// ---------------------------------------------------------------------------

/// AC 3 — after the anchor click the command line claims focus on its own,
/// with no keystroke required, and a bare `l` types into the field instead
/// of activating LINE (the gate reuse this demand depends on).
#[test]
fn raw_mode_holds_focus() {
    let (ctx, mut app, viewport) = boot();
    app.tool_manager.set_tool(Box::new(TextTool::default()));
    click_anchor(&ctx, &mut app, viewport);

    // No typing at all: raw mode must still request focus every frame.
    frame(&ctx, &mut app, vec![]);
    frame(&ctx, &mut app, vec![]);
    assert!(
        app.command_line_focused,
        "the field must hold focus with no typing at all"
    );

    let mut events = key_events(egui::Key::L, none());
    events.push(egui::Event::Text("l".to_owned()));
    frame(&ctx, &mut app, events);

    assert_eq!(
        app.command_line_input, "l",
        "the letter must land in the field"
    );
    assert_eq!(
        app.tool_manager.active_tool_name(),
        "TEXT",
        "a bare letter must not activate a tool while raw mode holds focus"
    );
}

// ---------------------------------------------------------------------------
// Decision 1 — raw means raw
// ---------------------------------------------------------------------------

/// Decision 1 — `submit_command("line")` at the text prompt is text, not a
/// tool switch: TEXT stays active and the committed geometry is the word
/// `"line"`, laid out at the click's own world anchor.
#[test]
fn raw_input_is_not_parsed() {
    let (ctx, mut app, viewport) = boot();
    app.tool_manager.set_tool(Box::new(TextTool::default()));
    click_anchor(&ctx, &mut app, viewport);
    let anchor = app
        .last_cursor_world
        .expect("the click must have moved the cursor into the viewport");

    submit_command(&ctx, &mut app, "line");
    assert_eq!(app.tool_manager.active_tool_name(), "TEXT");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "TEXT Specify height <5>:"
    );

    submit_command(&ctx, &mut app, "10");

    let expected = layout_text("line", anchor, 10.0, 1.0);
    assert_eq!(lines_of(&app.document.entities), lines_of(&expected));
}

/// Decision 1 — raw-mode content never enters the recall ring: `HELLO` and
/// `10` do not grow `command_history`, and there is nothing for ArrowUp to
/// bring back.
#[test]
fn raw_input_is_not_recalled() {
    let (ctx, mut app, viewport) = boot();
    app.tool_manager.set_tool(Box::new(TextTool::default()));
    click_anchor(&ctx, &mut app, viewport);
    let before = app.command_history.len();

    submit_command(&ctx, &mut app, "HELLO");
    submit_command(&ctx, &mut app, "10");

    assert_eq!(
        app.command_history.len(),
        before,
        "raw-mode text must never enter the recall ring"
    );

    // Focus the (now Idle) command line the ordinary way and confirm there
    // is nothing to recall.
    type_command(&ctx, &mut app, "9");
    app.command_line_input.clear();
    frame(&ctx, &mut app, vec![]);
    assert!(app.command_line_focused);

    tap(&ctx, &mut app, egui::Key::ArrowUp, none());
    assert_eq!(
        app.command_line_input, "",
        "ArrowUp must not bring back HELLO — it was never pushed"
    );
}

// ---------------------------------------------------------------------------
// Escape mid-flow
// ---------------------------------------------------------------------------

/// Escape while typing the height commits nothing and clears the field, even
/// though the string ("HELLO") was already accepted at the text prompt.
#[test]
fn escape_mid_flow_commits_nothing_and_clears_the_field() {
    let (ctx, mut app, viewport) = boot();
    app.tool_manager.set_tool(Box::new(TextTool::default()));
    click_anchor(&ctx, &mut app, viewport);
    submit_command(&ctx, &mut app, "HELLO");
    type_command(&ctx, &mut app, "40");

    tap(&ctx, &mut app, egui::Key::Escape, none());

    assert_eq!(app.command_line_input, "", "Escape clears the field");
    assert_eq!(app.document.entity_count(), 0);
    assert_eq!(app.history.len(), 0, "cancelling commits nothing");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "TEXT Specify start point:"
    );
}

// ---------------------------------------------------------------------------
// The dirty signal
// ---------------------------------------------------------------------------

/// A committed TEXT object goes through `history.commit` like any other
/// tool, so ADR 0002 §B's revision-based dirty signal picks it up with no
/// new code. Asserted on `dirty_since`, never on disk (trap 2).
#[test]
fn text_commit_marks_the_document_dirty() {
    let (ctx, mut app, viewport) = boot();
    assert!(app.dirty_since.is_none(), "a fresh document is clean");

    app.tool_manager.set_tool(Box::new(TextTool::default()));
    click_anchor(&ctx, &mut app, viewport);
    submit_command(&ctx, &mut app, "HELLO");
    assert!(app.dirty_since.is_none(), "a phase advance commits nothing");

    submit_command(&ctx, &mut app, "10");
    assert!(
        app.dirty_since.is_some(),
        "the committed text must arm the autosave debounce"
    );
}
