//! tests/lcv111.rs — the command line drives the tools (ADR 0003 §F).
//!
//! Every test here drives the **real** frame body through
//! `egui::Context::run(raw_input(...), |ctx| app.update_ui(ctx))`. Unit tests
//! that call `submit` or `on_command_input` directly prove those functions are
//! correct; only these prove they are *reachable* from a keystroke, which is
//! the blind spot ADR 0002 was written to close.
//!
//! Trap 5 (ADR 0003 §F3) governs the whole file: the command line submits on
//! `lost_focus() && key_pressed(Enter)`, so the field must have held focus
//! during the **previous** frame. `harness::submit_command` types one
//! character per frame and taps Enter in a frame of its own; a one-frame Enter
//! against an unfocused field is a silent false negative.
//!
//! `harness::type_command` sends `Event::Text` only. That is exactly what a
//! real keyboard delivers to a **focused** field (the bare tool letters are
//! suppressed by `wants_kbd` while typing). The unfocused case — where a real
//! keystroke carries `Event::Key` *and* `Event::Text` — is covered explicitly
//! by the double-dispatch pair below.

use crate::harness;

use harness::{frame, key_events, submit_command, tap, text_events, type_command};
use lasercad::app::App;
use lasercad::document::{CreateLine, Entity};
use lasercad::geometry::{Line, Vec2};
use lasercad::tools::{MoveTool, PolylineTool};

const EPS: f64 = 1e-9;

fn none() -> egui::Modifiers {
    egui::Modifiers::NONE
}

/// Boot an app and run the first frame, which registers every widget rect and
/// syncs the camera away from its zero-area sentinel.
fn boot() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    let mut app = App::default();
    frame(&ctx, &mut app, vec![]);
    (ctx, app)
}

fn lines(app: &App) -> Vec<Line> {
    app.document
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Line(l) => Some(*l),
            _ => None,
        })
        .collect()
}

fn assert_near(actual: Vec2, expected: Vec2) {
    assert!(
        (actual - expected).length() <= EPS,
        "expected {expected:?}, got {actual:?}"
    );
}

// ---------------------------------------------------------------------------
// AC 32, 33, 34 — the milestone checks
// ---------------------------------------------------------------------------

/// AC 32 — the roadmap sequence. `l` ⏎ `0,0` ⏎ `@100,0` ⏎ `50` ⏎ with the
/// cursor resting to the right draws two connected, exact segments.
///
/// This is the single test that says v2 is a CAD program: absolute, relative
/// and direct-distance entry, one after the other, through the real frame.
#[test]
fn roadmap_sequence_draws_two_lines() {
    let (ctx, mut app) = boot();
    // A hover to the right of the second point — the direction the third
    // point will travel in. The mouse is never moved again.
    app.last_cursor_world = Some(Vec2::new(200.0, 0.0));

    // A real keyboard would also activate LINE through the bare `L`
    // shortcut; typed into the field it goes through the alias table instead,
    // and both land on the same `ToolKind` (LCV-110 AC 13).
    submit_command(&ctx, &mut app, "l");
    assert_eq!(app.tool_manager.active_tool_name(), "LINE");

    submit_command(&ctx, &mut app, "0,0");
    submit_command(&ctx, &mut app, "@100,0");
    submit_command(&ctx, &mut app, "50");

    let drawn = lines(&app);
    assert_eq!(drawn.len(), 2, "expected exactly two lines, got {drawn:?}");
    assert_near(drawn[0].p1, Vec2::new(0.0, 0.0));
    assert_near(drawn[0].p2, Vec2::new(100.0, 0.0));
    assert_near(drawn[1].p1, Vec2::new(100.0, 0.0));
    assert_near(drawn[1].p2, Vec2::new(150.0, 0.0));
    assert_eq!(app.history.len(), 2);
    assert!(app.command_feedback.is_empty());
}

/// AC 33 — the same sequence with no cursor refuses the bare distance
/// instead of inventing a direction (product decision 2).
#[test]
fn no_direction_is_a_clean_refusal() {
    let (ctx, mut app) = boot();
    assert_eq!(app.last_cursor_world, None);

    submit_command(&ctx, &mut app, "l");
    submit_command(&ctx, &mut app, "0,0");
    submit_command(&ctx, &mut app, "@100,0");
    submit_command(&ctx, &mut app, "50");

    assert_eq!(
        lines(&app).len(),
        1,
        "the bare distance must commit nothing"
    );
    assert_eq!(app.history.len(), 1);
    assert_eq!(
        app.tool_manager.active_status_text(),
        "LINE Specify next point (Enter to finish):",
        "the tool's phase must be untouched by a refusal"
    );
    assert_eq!(
        app.command_feedback,
        "No direction for distance input — move the cursor or type X,Y."
    );
}

/// AC 34 — the mouse-free circle: `c` ⏎ `50,50` ⏎ `25` ⏎ with no cursor at
/// all. This is why `ToolInput::Distance` carries the magnitude as well as
/// the resolved point.
#[test]
fn mouse_free_circle_commits() {
    let (ctx, mut app) = boot();
    submit_command(&ctx, &mut app, "c");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "CIRCLE Specify center point:"
    );
    submit_command(&ctx, &mut app, "50,50");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "CIRCLE Specify radius:"
    );
    submit_command(&ctx, &mut app, "25");

    assert_eq!(app.last_cursor_world, None, "no mouse was involved");
    assert_eq!(app.document.entity_count(), 1);
    match app.document.entities[0] {
        Entity::Circle(c) => {
            assert_near(c.center, Vec2::new(50.0, 50.0));
            assert!((c.r - 25.0).abs() <= EPS);
        }
        ref other => panic!("expected a circle, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// AC 3, 4 — one test per wired tool, asserting the committed coordinates
// ---------------------------------------------------------------------------

/// PLINE: three typed points, then a blank Enter to finish (AC 14).
#[test]
fn typed_polyline_commits_exact_coordinates() {
    let (ctx, mut app) = boot();
    submit_command(&ctx, &mut app, "p");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "PLINE Specify start point:"
    );
    submit_command(&ctx, &mut app, "0,0");
    submit_command(&ctx, &mut app, "100,0");
    submit_command(&ctx, &mut app, "100,50");

    let drawn = lines(&app);
    assert_eq!(drawn.len(), 2);
    assert_near(drawn[0].p2, Vec2::new(100.0, 0.0));
    assert_near(drawn[1].p2, Vec2::new(100.0, 50.0));
}

/// RECT takes absolute coordinates only (product decision 4) and commits four
/// exact sides.
#[test]
fn typed_rect_commits_exact_coordinates() {
    let (ctx, mut app) = boot();
    submit_command(&ctx, &mut app, "r");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "RECT Specify first corner:"
    );
    submit_command(&ctx, &mut app, "0,0");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "RECT Specify opposite corner:"
    );
    submit_command(&ctx, &mut app, "100,50");

    let drawn = lines(&app);
    assert_eq!(drawn.len(), 4, "a rectangle is four sides");
    let (min, max) = app.document.bounds().expect("the rect has bounds");
    assert_near(min, Vec2::new(0.0, 0.0));
    assert_near(max, Vec2::new(100.0, 50.0));
    assert_eq!(app.history.len(), 1, "four sides, one undo step");
}

/// ARC: start, end, then the point on the arc.
#[test]
fn typed_arc_commits_exact_geometry() {
    let (ctx, mut app) = boot();
    submit_command(&ctx, &mut app, "a");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "ARC Specify start point:"
    );
    submit_command(&ctx, &mut app, "0,0");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "ARC Specify end point:"
    );
    submit_command(&ctx, &mut app, "100,0");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "ARC Specify point on arc:"
    );
    submit_command(&ctx, &mut app, "50,50");

    assert_eq!(app.document.entity_count(), 1);
    match app.document.entities[0] {
        Entity::Arc(a) => {
            assert_near(a.center, Vec2::new(50.0, 0.0));
            assert!((a.r - 50.0).abs() <= EPS);
        }
        ref other => panic!("expected an arc, got {other:?}"),
    }
}

/// MOVE with two absolute points moves the selection by their difference.
#[test]
fn typed_move_relocates_the_selection() {
    let (ctx, mut app) = boot();
    app.commit(Box::new(CreateLine::new(Line::new(
        Vec2::new(0.0, 0.0),
        Vec2::new(10.0, 0.0),
    ))));
    app.document.selection.set([0]);
    app.tool_manager.set_tool(Box::new(MoveTool::default()));

    submit_command(&ctx, &mut app, "0,0");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "MOVE Specify destination point:"
    );
    submit_command(&ctx, &mut app, "0,25");

    assert_near(lines(&app)[0].p1, Vec2::new(0.0, 25.0));
    assert_near(lines(&app)[0].p2, Vec2::new(10.0, 25.0));
}

/// AC 6 — after a typed commit the app polls succession exactly as the
/// pointer path does, so MOVE hands back to SELECT.
#[test]
fn relative_move_hands_back_to_select() {
    let (ctx, mut app) = boot();
    app.commit(Box::new(CreateLine::new(Line::new(
        Vec2::new(0.0, 0.0),
        Vec2::new(10.0, 0.0),
    ))));
    app.document.selection.set([0]);

    submit_command(&ctx, &mut app, "m");
    assert_eq!(app.tool_manager.active_tool_name(), "MOVE");
    submit_command(&ctx, &mut app, "0,0");
    submit_command(&ctx, &mut app, "@10,0");

    assert_near(lines(&app)[0].p1, Vec2::new(10.0, 0.0));
    assert_eq!(
        app.tool_manager.active_tool_name(),
        "Select",
        "MOVE must hand control back after a typed commit, as it does after a click"
    );
    assert_eq!(app.tool_manager.active_status_text(), "Command:");
}

// ---------------------------------------------------------------------------
// AC 21, 22 — focus-on-typing and the double-dispatch pair
// ---------------------------------------------------------------------------

/// AC 22 — a real `l` keystroke while the field is unfocused: the shortcut
/// wins, and the character must **not** also land in the command line.
#[test]
fn typing_l_while_unfocused_starts_line_and_types_nothing() {
    let (ctx, mut app) = boot();
    let mut events = key_events(egui::Key::L, none());
    events.push(egui::Event::Text("l".to_owned()));
    frame(&ctx, &mut app, events);

    assert_eq!(app.tool_manager.active_tool_name(), "LINE");
    assert_eq!(
        app.command_line_input, "",
        "the tool shortcut consumed the keystroke; the field must stay empty"
    );
}

/// AC 22 — the same keystroke while the field has focus: the character is
/// typed and no tool is activated (ADR 0003 §E1's `ze` example).
#[test]
fn typing_l_while_focused_types_an_l_and_does_not_start_line() {
    let (ctx, mut app) = boot();
    // Seed focus with a character that is not a bound shortcut.
    type_command(&ctx, &mut app, "z");
    assert!(ctx.wants_keyboard_input(), "the field must hold focus");

    let mut events = key_events(egui::Key::L, none());
    events.push(egui::Event::Text("l".to_owned()));
    frame(&ctx, &mut app, events);

    assert_eq!(
        app.tool_manager.active_tool_name(),
        "Select",
        "a bare letter must not activate a tool while the field has focus"
    );
    assert_eq!(app.command_line_input, "zl");
}

/// AC 21 — one typed digit focuses the field and seeds it exactly once.
#[test]
fn typing_a_digit_focuses_and_seeds_the_field() {
    let (ctx, mut app) = boot();
    frame(&ctx, &mut app, text_events("5"));

    assert_eq!(
        app.command_line_input, "5",
        "not \"55\" — one character, one copy"
    );
    assert!(
        ctx.wants_keyboard_input(),
        "the widget must have consumed the one-shot focus request"
    );
    assert!(
        !app.focus_command_line,
        "the focus flag is one-shot and must be consumed"
    );
}

// ---------------------------------------------------------------------------
// AC 14 — Enter on an empty field
// ---------------------------------------------------------------------------

/// AC 14 — a blank Enter reaches the active tool exactly once, so PLINE
/// finishes instead of hanging mid-chain.
#[test]
fn enter_on_an_empty_field_finishes_the_polyline_exactly_once() {
    let (ctx, mut app) = boot();
    app.tool_manager.set_tool(Box::new(PolylineTool::default()));
    submit_command(&ctx, &mut app, "0,0");
    submit_command(&ctx, &mut app, "100,0");
    assert_eq!(app.history.len(), 1);

    // The field is unfocused after a submit, so focus it the way an operator
    // would before pressing Enter on an empty line.
    type_command(&ctx, &mut app, "9");
    app.command_line_input.clear();
    tap(&ctx, &mut app, egui::Key::Enter, none());

    assert_eq!(
        app.tool_manager.active_status_text(),
        "PLINE Specify start point:",
        "Enter must finish the polyline"
    );
    assert_eq!(
        app.history.len(),
        1,
        "finishing commits nothing extra — and Enter must not fire twice"
    );
    assert_eq!(lines(&app).len(), 1);
}

// ---------------------------------------------------------------------------
// AC 23 — recall
// ---------------------------------------------------------------------------

/// AC 23 — ArrowUp walks back through the ring; ArrowDown walks forward and
/// clears the field past the newest entry.
#[test]
fn arrow_up_recalls_and_arrow_down_clears() {
    let (ctx, mut app) = boot();
    submit_command(&ctx, &mut app, "snap");
    submit_command(&ctx, &mut app, "grid");
    assert_eq!(app.command_history.len(), 2);

    // Focus the field, then let one frame pass: `command_line_focused` is a
    // mirror of the *previous* frame's `has_focus()`, the same one-frame lag
    // egui's own `wants_keyboard_input` has.
    type_command(&ctx, &mut app, "9");
    frame(&ctx, &mut app, vec![]);
    assert!(app.command_line_focused);

    tap(&ctx, &mut app, egui::Key::ArrowUp, none());
    assert_eq!(app.command_line_input, "grid");
    tap(&ctx, &mut app, egui::Key::ArrowUp, none());
    assert_eq!(app.command_line_input, "snap");
    tap(&ctx, &mut app, egui::Key::ArrowDown, none());
    assert_eq!(app.command_line_input, "grid");
    tap(&ctx, &mut app, egui::Key::ArrowDown, none());
    assert_eq!(
        app.command_line_input, "",
        "past the newest entry the field clears"
    );
}

/// AC 23 — the arrows are inert while the field is unfocused: the ring must
/// not hijack a key the drawing area may want later.
#[test]
fn arrows_do_nothing_while_the_field_is_unfocused() {
    let (ctx, mut app) = boot();
    submit_command(&ctx, &mut app, "snap");
    assert!(!app.command_line_focused, "a submit drops focus");

    tap(&ctx, &mut app, egui::Key::ArrowUp, none());
    assert_eq!(app.command_line_input, "");
    tap(&ctx, &mut app, egui::Key::ArrowDown, none());
    assert_eq!(app.command_line_input, "");
}

// ---------------------------------------------------------------------------
// AC 15, 24, 25 — feedback and Escape
// ---------------------------------------------------------------------------

/// AC 15 — a tool that refuses says so by name, through the real frame.
#[test]
fn select_refuses_a_typed_point_by_name() {
    let (ctx, mut app) = boot();
    submit_command(&ctx, &mut app, "50,25");
    assert_eq!(app.command_feedback, "Select does not accept that input.");
    assert_eq!(app.document.entity_count(), 0);
    assert_eq!(app.history.len(), 0);
}

/// AC 24, AC 25 — one Escape clears the field *and* the feedback *and*
/// cancels the active tool, in a single frame.
#[test]
fn escape_clears_the_field_and_the_feedback_and_cancels_the_tool() {
    let (ctx, mut app) = boot();
    submit_command(&ctx, &mut app, "l");
    submit_command(&ctx, &mut app, "0,0");
    submit_command(&ctx, &mut app, "bogus");
    assert_eq!(app.command_feedback, "Unknown command: \"bogus\"");

    type_command(&ctx, &mut app, "99");
    assert_eq!(app.command_line_input, "99");

    tap(&ctx, &mut app, egui::Key::Escape, none());

    assert_eq!(app.command_line_input, "", "Escape clears the field");
    assert_eq!(app.command_feedback, "", "and the stale message");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "LINE Specify first point:",
        "and cancels the tool, even while the field has focus"
    );
    assert_eq!(app.history.len(), 0, "cancelling commits nothing");
}

// ---------------------------------------------------------------------------
// AC 7 — the dirty signal
// ---------------------------------------------------------------------------

/// AC 7 — a typed commit goes through `history.commit` like any other, so
/// ADR 0002 §B's revision-based dirty signal picks it up with no new code.
/// (Asserted on `dirty_since`, never on disk: trap 2.)
#[test]
fn typed_commit_marks_the_document_dirty() {
    let (ctx, mut app) = boot();
    assert!(app.dirty_since.is_none(), "a fresh document is clean");

    submit_command(&ctx, &mut app, "l");
    submit_command(&ctx, &mut app, "0,0");
    assert!(app.dirty_since.is_none(), "a phase advance commits nothing");

    submit_command(&ctx, &mut app, "100,0");
    assert!(
        app.dirty_since.is_some(),
        "a typed commit must arm the autosave debounce"
    );
}

// ---------------------------------------------------------------------------
// AC 17 — the prompt follows the phase
// ---------------------------------------------------------------------------

/// AC 17 — the operator can always see what the program is waiting for. This
/// walks LINE and ARC through every phase by typed input alone.
#[test]
fn prompt_follows_the_active_phase() {
    let (ctx, mut app) = boot();
    assert_eq!(app.tool_manager.active_status_text(), "Command:");

    submit_command(&ctx, &mut app, "l");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "LINE Specify first point:"
    );
    submit_command(&ctx, &mut app, "0,0");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "LINE Specify next point (Enter to finish):"
    );
    submit_command(&ctx, &mut app, "100,0");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "LINE Specify next point (Enter to finish):",
        "LINE chains, so the prompt stays on the next point"
    );

    submit_command(&ctx, &mut app, "a");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "ARC Specify start point:"
    );
    submit_command(&ctx, &mut app, "0,0");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "ARC Specify end point:"
    );
    submit_command(&ctx, &mut app, "100,0");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "ARC Specify point on arc:"
    );
    submit_command(&ctx, &mut app, "50,50");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "ARC Specify start point:",
        "a committed arc returns the tool to its idle prompt"
    );
}
