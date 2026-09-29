//! LCV-139 — the two-row command dock: bounded context row, a real editor
//! row, and a destination label that previews routing with no side effects.
//!
//! Every geometry claim here is read from egui's own persisted state —
//! `PanelState::load` for the dock's outer rect (AC 1), `ctx.read_response`
//! for the editor's own rect (AC 1, AC 7) — never inferred from painted
//! content, exactly the pattern `tests/it/agent/panel_width_and_settings.rs`
//! established for the agent panel. Painted-text assertions go through
//! `tests/harness/paint.rs` (AGENTS.md: a rendering acceptance criterion is
//! not satisfied by a source scan alone).
//!
//! `src/ui/command_destination.rs`'s own inline tests already cover the pure
//! precedence table (AC 3-5) with no `App` and no UI context at all; this
//! file's job is the part that table cannot prove — that the real widget
//! paints the label the table predicts, that computing it has no side
//! effects (AC 6), and that the two rows are real, bounded and separate on
//! screen (AC 1, AC 2, AC 7).

use crate::harness;

use harness::paint::{painted_runs_at, runs_in};
use harness::{frame, raw_input_at, submit_command, tap, type_command};
use lasercad::app::{App, arm_turn};
use lasercad::document::Entity;
use lasercad::ui::{
    LABEL_AI, LABEL_AI_BUSY, LABEL_AI_PROMPT_EMPTY, LABEL_AI_UNAVAILABLE, LABEL_CAD,
    LABEL_TOOL_INPUT,
};

/// The id `src/app/panels.rs::draw_chrome` registers the command-line
/// `TopBottomPanel` under.
fn command_line_panel_id() -> egui::Id {
    egui::Id::new("command_line")
}

/// The id `src/ui/command_line.rs::editor_id` pins on the single-line editor.
fn editor_id() -> egui::Id {
    egui::Id::new("command_line_editor")
}

/// A headless context at `pixels_per_point == 1.0` and a default `App`,
/// already settled at 800x600.
///
/// The command-line dock's very first frame computes its panel height from
/// egui's own single-row default (`PanelState::load` finds nothing yet —
/// `containers/panel.rs::TopBottomPanel::show_inside_dyn`), which clips the
/// second (editor) row out of the visible clip rect until a frame's actual
/// two-row content is adopted as the persisted height. One warm-up frame
/// here settles it once; every later frame in this file reuses that height
/// regardless of what text it renders, because the row count and font never
/// change — only the strings do.
fn ctx_and_app() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let mut app = App::default();
    let _ = ctx.run(raw_input_at([800.0, 600.0], Vec::new()), |c| {
        app.update_ui(c)
    });
    (ctx, app)
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

/// Click the viewport's centre at `screen`, so a tool that only wants raw
/// input after an anchor click (`TextTool`) really is anchored: a warm-up
/// `PointerMoved` frame (ADR 0002 §A4 rule 3), then the frame carrying the
/// click itself.
fn click_viewport_centre(ctx: &egui::Context, app: &mut App, screen: [f32; 2]) {
    let mut viewport = egui::Rect::NOTHING;
    let _ = ctx.run(raw_input_at(screen, Vec::new()), |c| {
        app.update_ui(c);
        viewport = c.available_rect();
    });
    let centre = viewport.center();
    let _ = ctx.run(
        raw_input_at(screen, vec![egui::Event::PointerMoved(centre)]),
        |c| app.update_ui(c),
    );
    let _ = ctx.run(raw_input_at(screen, click_events(centre)), |c| {
        app.update_ui(c)
    });
}

/// Drive one settled frame at `screen` (a warm-up, then the measured one —
/// the first frame does not yet place every `Area`/panel), returning the
/// dock's outer rect and the editor's own rect.
fn dock_and_editor_rects(
    ctx: &egui::Context,
    app: &mut App,
    screen: [f32; 2],
) -> (egui::Rect, egui::Rect) {
    let _ = ctx.run(raw_input_at(screen, Vec::new()), |c| app.update_ui(c));
    let _ = ctx.run(raw_input_at(screen, Vec::new()), |c| app.update_ui(c));
    let dock = egui::containers::panel::PanelState::load(ctx, command_line_panel_id())
        .expect("the command-line dock must have stored its panel state by now")
        .rect;
    let editor = ctx
        .read_response(editor_id())
        .expect("the editor must have painted a response by now")
        .rect;
    (dock, editor)
}

// ---------------------------------------------------------------------------
// AC 1 — bounded height, a wide-enough editor, at 800x600
// ---------------------------------------------------------------------------

/// AC 1 — at 800x600 logical points, the whole dock is at most 64pt tall and
/// the editor alone is at least 240pt wide.
///
/// Mutation this catches: collapsing the two rows back into one (the editor
/// sharing a row with the prompt/feedback/label) would still likely fit
/// under 64pt on a single row, but would no longer leave the editor its own
/// width independent of how long the prompt or feedback happens to be — the
/// row-separation test below is what catches that regression directly.
#[test]
fn ac1_dock_is_bounded_and_the_editor_is_wide_enough_at_800x600() {
    let (ctx, mut app) = ctx_and_app();
    let (dock, editor) = dock_and_editor_rects(&ctx, &mut app, [800.0, 600.0]);

    assert!(
        dock.height() <= 64.0,
        "the two-row dock must be at most 64pt tall, got {}",
        dock.height()
    );
    assert!(
        editor.width() >= 240.0,
        "the single-line editor must be at least 240pt wide, got {}",
        editor.width()
    );
}

// ---------------------------------------------------------------------------
// AC 1 / AC 7 — the two rows are visually separate, and the label never
// overlaps or truncates the editor
// ---------------------------------------------------------------------------

/// AC 1 / AC 7 — the context row's own text and the editor's typed text sit
/// on genuinely different visual lines (more than `SAME_LINE` apart), and the
/// editor's rect never overlaps the destination label's own line.
///
/// `command_line_input` is set directly to a short string so the `TextEdit`
/// paints a real galley on its row (an empty field paints nothing — the
/// shared collector drops empty runs, `tests/harness/paint.rs` trap 6).
#[test]
fn ac1_ac7_context_row_and_editor_row_are_on_separate_visual_lines() {
    let (ctx, mut app) = ctx_and_app();
    app.command_line_input = "abc".to_owned();

    let runs = painted_runs_at(&ctx, &mut app, [800.0, 600.0], Vec::new());
    let context_y = runs
        .iter()
        .find(|r| r.text.trim() == "Destination:")
        .expect("the destination caption must paint on the context row")
        .y();
    let editor_y = runs
        .iter()
        .find(|r| r.text.trim() == "abc")
        .expect("the typed text must paint on the editor row")
        .y();

    assert!(
        (editor_y - context_y).abs() > harness::paint::SAME_LINE,
        "context row (y={context_y}) and editor row (y={editor_y}) must be \
         genuinely separate lines"
    );

    // AC 7 — the destination label's own rect (read via the dock/editor
    // geometry) never reaches down into the editor's rect: the editor's top
    // must sit at or below the context row's own text.
    let (_, editor_rect) = dock_and_editor_rects(&ctx, &mut app, [800.0, 600.0]);
    assert!(
        editor_rect.top() as i32 >= context_y,
        "the editor must sit at or below the context row, never overlapping \
         it: editor top {}, context row y {context_y}",
        editor_rect.top()
    );
}

// ---------------------------------------------------------------------------
// AC 2 — bounded width, byte-exact storage, elided-text tooltip
// ---------------------------------------------------------------------------

/// AC 2 — a feedback message far longer than [`CONTEXT_SEGMENT_MAX_WIDTH`]
/// (private to `src/ui/command_line.rs`; 300 `x` characters is long enough
/// at any reasonable font) does not push the destination caption off an
/// 800pt-wide screen, the stored `command_feedback` stays byte-exact, and
/// hovering the elided run paints a second copy of it — egui's own
/// full-text tooltip, attached automatically once `Label::truncate` elides
/// (`src/ui/command_line.rs::bounded_label`, egui-0.29.1
/// `widgets/label.rs`).
///
/// Mutation this catches: dropping `.set_max_width` (or `.truncate()`) from
/// `bounded_label` — the destination caption would then be pushed far to the
/// right by the 300-character run, and no second copy of the message would
/// ever appear on hover.
#[test]
fn ac2_a_long_feedback_message_is_bounded_stored_verbatim_and_tooltips_on_hover() {
    let (ctx, mut app) = ctx_and_app();
    let long = "x".repeat(300);
    app.command_feedback = long.clone();

    let runs = painted_runs_at(&ctx, &mut app, [800.0, 600.0], Vec::new());

    // Bounded: the destination caption, painted after the feedback segment in
    // the same row, stays on screen — far short of where an unbounded
    // 300-character run would push it.
    let destination_x = runs
        .iter()
        .find(|r| r.text.trim() == "Destination:")
        .expect("the destination caption must still paint")
        .x();
    assert!(
        (destination_x as f32) < 800.0,
        "the destination caption must stay on an 800pt-wide screen, got x={destination_x}"
    );

    // Byte-exact storage: only the display elides.
    assert_eq!(
        app.command_feedback, long,
        "the stored feedback must be untouched"
    );

    // Exactly one copy of the message paints before any hover.
    let before = runs.iter().filter(|r| r.text == long).count();
    assert_eq!(
        before, 1,
        "before a hover, only the widget's own (elided) run may carry the full text"
    );

    // Locate the elided run and hover it: a warm-up PointerMoved frame, then
    // the settled frame the tooltip actually paints on.
    let target = runs
        .iter()
        .find(|r| r.text == long)
        .expect("the feedback run must be findable by its full text");
    let pos = egui::pos2(target.pos.x + 2.0, target.pos.y + target.height / 2.0);

    let _ = ctx.run(
        harness::raw_input_at([800.0, 600.0], vec![egui::Event::PointerMoved(pos)]),
        |c| app.update_ui(c),
    );
    let out = ctx.run(harness::raw_input_at([800.0, 600.0], Vec::new()), |c| {
        app.update_ui(c)
    });
    let hover_runs = runs_in(&out.shapes);
    let after = hover_runs.iter().filter(|r| r.text == long).count();
    assert_eq!(
        after, 2,
        "hovering an elided label must add its own tooltip copy of the full \
         text, saw {after} copies"
    );
}

/// AC 2 — a short feedback message never elides, so hovering it must not
/// add a tooltip copy at all: the positive control for the test above, so a
/// hover that "always" duplicates the text (whether elided or not) cannot
/// pass silently.
#[test]
fn ac2_a_short_feedback_message_never_tooltips_on_hover() {
    let (ctx, mut app) = ctx_and_app();
    app.command_feedback = "SNAP on".to_owned();

    let runs = painted_runs_at(&ctx, &mut app, [800.0, 600.0], Vec::new());
    let target = runs
        .iter()
        .find(|r| r.text == "SNAP on")
        .expect("the short feedback must paint");
    let pos = egui::pos2(target.pos.x + 2.0, target.pos.y + target.height / 2.0);

    let _ = ctx.run(
        harness::raw_input_at([800.0, 600.0], vec![egui::Event::PointerMoved(pos)]),
        |c| app.update_ui(c),
    );
    let out = ctx.run(harness::raw_input_at([800.0, 600.0], Vec::new()), |c| {
        app.update_ui(c)
    });
    let hover_runs = runs_in(&out.shapes);
    let after = hover_runs.iter().filter(|r| r.text == "SNAP on").count();
    assert_eq!(
        after, 1,
        "a message that fits must not gain a tooltip copy on hover, saw {after}"
    );
}

// ---------------------------------------------------------------------------
// AC 3-6 — the destination label matches real submission, and computing it
// has no side effects
// ---------------------------------------------------------------------------

/// The literal text `src/ui/command_line.rs::draw_context_row` paints for the
/// destination on the settled frame — the same row as `"Destination:"`.
fn painted_destination(ctx: &egui::Context, app: &mut App, screen: [f32; 2]) -> String {
    let runs = painted_runs_at(ctx, app, screen, Vec::new());
    let lines = harness::paint::lines_on_surface_of(&runs, "Destination:");
    let line = lines
        .into_iter()
        .find(|(_, texts)| texts.iter().any(|t| t == "Destination:"))
        .expect("the context row must contain its own destination caption");
    line.1
        .last()
        .expect("the destination row must carry a value after its caption")
        .clone()
}

/// AC 3-6 — a table of scenarios: what the dock paints as the destination,
/// and what actually happens on a real Enter. A key is configured through
/// `DUMMY_KEY`-style plumbing is unnecessary here — every row below either
/// needs no key, needs one refused synchronously, or is proven busy through
/// `arm_turn` (LCV-124's own pattern), so no test in this file starts a real
/// background thread.
#[test]
fn ac3_ac4_ac5_destination_matches_a_real_submit_for_cad_lines() {
    let (ctx, mut app) = ctx_and_app();

    // A blank line: always CAD, and Enter's existing empty-input behaviour
    // is unchanged (a fresh `SelectTool` answers nothing to an empty submit).
    assert_eq!(
        painted_destination(&ctx, &mut app, [800.0, 600.0]),
        LABEL_CAD
    );

    // A recognised CAD alias. Checked, then cleared before the real gesture
    // types it fresh — `submit_command` types into whatever is already
    // there, and this string must not be typed twice.
    app.command_line_input = "l".to_owned();
    assert_eq!(
        painted_destination(&ctx, &mut app, [800.0, 600.0]),
        LABEL_CAD
    );
    app.command_line_input.clear();
    submit_command(&ctx, &mut app, "l");
    assert_eq!(app.tool_manager.active_tool_name(), "LINE");

    // An unrecognised word never reaches the agent, key or no key, and the
    // grammar's own refusal is what actually happens on Enter.
    app.command_line_input = "lien".to_owned();
    assert_eq!(
        painted_destination(&ctx, &mut app, [800.0, 600.0]),
        LABEL_CAD
    );
    app.command_line_input.clear();
    submit_command(&ctx, &mut app, "lien");
    assert_eq!(app.command_feedback, "Unknown command: \"lien\"");
    assert!(!app.agent.busy);
}

/// AC 3 — raw-input mode's `tool input` destination, painted while `TextTool`
/// is mid-flow and holding the field, even for text that looks exactly like
/// an agent prefix.
#[test]
fn ac3_raw_input_mode_paints_tool_input() {
    let (ctx, mut app) = ctx_and_app();
    app.tool_manager
        .set_tool(Box::new(lasercad::tools::TextTool::default()));
    click_viewport_centre(&ctx, &mut app, [800.0, 600.0]);
    assert!(
        app.tool_manager.wants_raw_input(),
        "control: the click must have anchored TEXT into WaitingText"
    );
    app.command_line_input = ":not an agent prompt".to_owned();

    assert_eq!(
        painted_destination(&ctx, &mut app, [800.0, 600.0]),
        LABEL_TOOL_INPUT
    );
}

/// AC 4 — the four remaining labels, each proven with a real `App`: an
/// unavailable agent, an empty prefix, a busy turn, and (as the control) a
/// reachable one. `agent.busy` never relabels a CAD line — proven by
/// re-checking the blank-line and `lien` cases from the table above while a
/// turn (armed with no thread, `arm_turn`) is in flight.
#[test]
fn ac4_the_remaining_labels_and_busy_never_touches_a_cad_line() {
    let (ctx, mut app) = ctx_and_app();

    // No key configured at all: a non-empty prefixed prompt is unavailable.
    app.command_line_input = ":draw a line".to_owned();
    assert_eq!(
        painted_destination(&ctx, &mut app, [800.0, 600.0]),
        LABEL_AI_UNAVAILABLE
    );

    // A whitespace-only key is the same as none (AC 4, `agent_available`'s
    // own definition).
    app.settings.agent_api_key = "   ".to_owned();
    assert_eq!(
        painted_destination(&ctx, &mut app, [800.0, 600.0]),
        LABEL_AI_UNAVAILABLE
    );

    // A configured key: a bare prefix is refused for emptiness, not sent.
    app.settings.agent_api_key = "sk-test-key".to_owned();
    app.command_line_input = ":".to_owned();
    assert_eq!(
        painted_destination(&ctx, &mut app, [800.0, 600.0]),
        LABEL_AI_PROMPT_EMPTY
    );
    app.command_line_input.clear();
    submit_command(&ctx, &mut app, ":");
    assert_eq!(app.command_feedback, "Agent prompt is empty.");
    assert!(!app.agent.busy);

    // A real prompt with the key configured: reachable.
    app.command_line_input = ":draw a line".to_owned();
    assert_eq!(
        painted_destination(&ctx, &mut app, [800.0, 600.0]),
        LABEL_AI
    );

    // Arm a turn with no thread (LCV-124's `arm_turn` pattern) and re-check:
    // the same prefixed line is now `AI busy`, but the blank line and `lien`
    // from the CAD table are still `CAD`, untouched by the busy flag.
    let _tx = arm_turn(&mut app, "an in-flight prompt");
    assert!(app.agent.busy);
    assert_eq!(
        painted_destination(&ctx, &mut app, [800.0, 600.0]),
        LABEL_AI_BUSY
    );

    app.command_line_input.clear();
    assert_eq!(
        painted_destination(&ctx, &mut app, [800.0, 600.0]),
        LABEL_CAD
    );
    app.command_line_input = "lien".to_owned();
    assert_eq!(
        painted_destination(&ctx, &mut app, [800.0, 600.0]),
        LABEL_CAD
    );
}

// ---------------------------------------------------------------------------
// AC 6 — computing/painting the destination has no side effects
// ---------------------------------------------------------------------------

/// AC 6 — typing into the field and re-rendering the label, frame after
/// frame, with no Enter, never sends anything, arms a turn, opens the panel,
/// writes to the recall ring, or mutates the document/history. A key is
/// configured throughout, and every typed line is `:`-prefixed (the one
/// shape that *would* reach the agent on a real Enter), which is what makes
/// "nothing happened" a real claim rather than a vacuous one.
#[test]
fn ac6_editing_and_rendering_never_sends_or_mutates_with_no_enter() {
    let (ctx, mut app) = ctx_and_app();
    app.settings.agent_api_key = "sk-test-key".to_owned();

    let history_before = app.history.revision();
    let ring_before = app.command_history.len();
    let entities_before = app.document.entity_count();

    for ch in ":draw a 20 mm line at 10,10".chars() {
        app.command_line_input.push(ch);
        // Re-render the whole frame — including the destination label —
        // without ever sending the Enter key.
        frame(&ctx, &mut app, Vec::new());
        assert!(!app.agent.busy, "no turn may be armed by rendering alone");
        assert!(app.agent.rx.is_none());
        assert!(app.agent.chat.is_empty(), "nothing reached the transcript");
        assert!(!app.agent.panel_open, "the panel must not open itself");
    }

    assert_eq!(
        app.history.revision(),
        history_before,
        "the document/history must be untouched"
    );
    assert_eq!(
        app.command_history.len(),
        ring_before,
        "the recall ring must be untouched with no Enter"
    );
    assert_eq!(app.document.entity_count(), entities_before);
    assert_eq!(app.command_line_input, ":draw a 20 mm line at 10,10");
}

// ---------------------------------------------------------------------------
// AC 7 — a real Enter/Escape frame on the two-row layout
// ---------------------------------------------------------------------------

/// AC 7 — the existing gestures still work, driven through the real,
/// now-two-row frame body: Enter submits exactly once and commits a line,
/// and Escape clears the field. Recall (`ArrowUp`/`ArrowDown`) is not
/// re-driven here — `tests/it/cmdline/drives_tools.rs` already covers it end to end — this
/// file's job is only the two-row layout these gestures now run on top of.
#[test]
fn ac7_enter_and_escape_still_work_on_the_two_row_layout() {
    let (ctx, mut app) = ctx_and_app();

    submit_command(&ctx, &mut app, "l");
    submit_command(&ctx, &mut app, "0,0");
    submit_command(&ctx, &mut app, "10,0");

    let lines: Vec<&Entity> = app
        .document
        .entities
        .iter()
        .filter(|e| matches!(e, Entity::Line(_)))
        .collect();
    assert_eq!(lines.len(), 1, "the line must have committed");

    app.command_feedback = "stale".to_owned();
    type_command(&ctx, &mut app, "99");
    assert_eq!(app.command_line_input, "99");

    tap(&ctx, &mut app, egui::Key::Escape, egui::Modifiers::NONE);

    assert_eq!(app.command_line_input, "", "Escape must clear the field");
    assert_eq!(app.command_feedback, "", "and the stale feedback");
}
