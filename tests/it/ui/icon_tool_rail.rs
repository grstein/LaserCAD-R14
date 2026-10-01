//! LCV-183 — icon tool rail in two columns.
//!
//! Per-AC coverage, all on painted output:
//!
//! - AC 1: the rail paints no text but `AI`; the selected fill under a tool
//!   button is a 32 pt square (the icon shapes themselves are pinned by
//!   `src/ui/icons.rs::tests`).
//! - AC 3 / AC 6: hovering each computed button centre paints exactly that
//!   tool's tooltip — the draw group top-down on the left, the modify group
//!   top-down on the right.
//! - AC 4 / AC 5: clicking each centre activates its tool, and the selected
//!   fill sits under the active button only.
//! - AC 7: the `AI` toggle — 32 pt square, `AI Assistant` tooltip, selected
//!   fill while open, click toggles.
//! - AC 8: rail width, and every button reachable without scrolling at
//!   800x600, 1024x600 and 1280x800.
//! - AC 9: a wheel scroll over a 220 pt-tall rail reveals `AI`.
//!
//! The rail geometry below mirrors `src/ui/toolbar.rs`'s layout constants;
//! `ac8_rail_is_at_most_80pt_wide_and_nothing_scrolls` pins them through the
//! measured panel width, so a drifted constant fails there first.

use crate::harness;

use harness::paint::{Run, painted_runs_at, runs_in};
use harness::{SCREEN, raw_input_at};
use lasercad::app::App;

/// Frame inner margin of the rail, in points.
const MARGIN: f32 = 4.0;
/// Side of one rail button, in points.
const BUTTON: f32 = 32.0;
/// Gap between buttons, both across and down, in points.
const GAP: f32 = 4.0;

/// Expected tooltip per rail button, `[column][row]` (AC 3, AC 6).
const DRAW_COLUMN: [&str; 7] = [
    "Select — SELECT",
    "Line — L · LINE",
    "Polyline — P · PLINE",
    "Rect — R · RECT",
    "Circle — C · CIRCLE",
    "Arc — A · ARC",
    "Text — D · TEXT",
];
const MODIFY_COLUMN: [&str; 9] = [
    "Move — M · MOVE",
    "Copy — COPY",
    "Rotate — ROTATE",
    "Mirror — MIRROR",
    "Scale — SCALE",
    "Trim — T · TRIM",
    "Extend — X · EXTEND",
    "Delete — E · ERASE",
    "Dist — DIST",
];

/// Every button as `(column, row, tooltip)`, draw column first.
fn buttons() -> Vec<(usize, usize, &'static str)> {
    let left = DRAW_COLUMN.iter().enumerate().map(|(r, t)| (0, r, *t));
    let right = MODIFY_COLUMN.iter().enumerate().map(|(r, t)| (1, r, *t));
    left.chain(right).collect()
}

/// The command word a tooltip ends with — the active tool's name, uppercased.
fn word(tooltip: &str) -> &str {
    tooltip.rsplit([' ', '·']).next().unwrap_or_default()
}

fn ctx_and_app() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    (ctx, App::default())
}

/// The toolbar panel's outer rect, as egui stored it last frame.
fn rail_rect(ctx: &egui::Context) -> egui::Rect {
    egui::containers::panel::PanelState::load(ctx, egui::Id::new("toolbar"))
        .expect("the toolbar panel must have stored its state")
        .rect
}

/// Centre of the button at `(column, row)` in a rail at `rail`, unscrolled.
fn centre(rail: egui::Rect, column: usize, row: usize) -> egui::Pos2 {
    let step = BUTTON + GAP;
    rail.min
        + egui::vec2(MARGIN + column as f32 * step, MARGIN + row as f32 * step)
        + egui::Vec2::splat(BUTTON / 2.0)
}

/// One frame; every leaf shape painted, with its clip rect.
fn frame_shapes(
    ctx: &egui::Context,
    app: &mut App,
    screen: [f32; 2],
    events: Vec<egui::Event>,
) -> (Vec<Run>, Vec<egui::Shape>) {
    let out = ctx.run(raw_input_at(screen, events), |c| app.update_ui(c));
    let runs = runs_in(&out.shapes);
    let mut leaves = Vec::new();
    let mut stack: Vec<egui::Shape> = out.shapes.into_iter().map(|c| c.shape).collect();
    while let Some(shape) = stack.pop() {
        match shape {
            egui::Shape::Vec(inner) => stack.extend(inner),
            other => leaves.push(other),
        }
    }
    (runs, leaves)
}

/// Rects painted in the selected fill inside the rail.
fn selected_fills(ctx: &egui::Context, shapes: &[egui::Shape]) -> Vec<egui::Rect> {
    let fill = ctx.style().visuals.selection.bg_fill;
    let rail = rail_rect(ctx);
    shapes
        .iter()
        .filter_map(|s| match s {
            egui::Shape::Rect(r) if r.fill == fill && rail.contains(r.rect.center()) => {
                Some(r.rect)
            }
            _ => None,
        })
        .collect()
}

/// The text runs painted inside the rail.
fn rail_runs<'a>(ctx: &egui::Context, runs: &'a [Run]) -> Vec<&'a Run> {
    let rail = rail_rect(ctx);
    runs.iter().filter(|r| rail.contains(r.pos)).collect()
}

/// The point at the middle of the rail's `AI` text.
fn ai_point(ctx: &egui::Context, runs: &[Run]) -> egui::Pos2 {
    let ai: Vec<&Run> = rail_runs(ctx, runs)
        .into_iter()
        .filter(|r| r.text.trim() == "AI")
        .collect();
    assert_eq!(ai.len(), 1, "the rail must paint `AI` exactly once");
    egui::pos2(ai[0].pos.x + 4.0, ai[0].pos.y + ai[0].height / 2.0)
}

/// Hover `pos` on a fresh context and return the painted texts.
fn tooltip_at(screen: [f32; 2], pos: egui::Pos2) -> Vec<String> {
    let (ctx, mut app) = ctx_and_app();
    let _ = painted_runs_at(&ctx, &mut app, screen, Vec::new());
    let _ = painted_runs_at(&ctx, &mut app, screen, vec![egui::Event::PointerMoved(pos)]);
    let runs = painted_runs_at(&ctx, &mut app, screen, Vec::new());
    runs.iter().map(|r| r.text.trim().to_owned()).collect()
}

/// A settled primary click at `pos`.
fn click(ctx: &egui::Context, app: &mut App, screen: [f32; 2], pos: egui::Pos2) {
    let _ = frame_shapes(ctx, app, screen, vec![egui::Event::PointerMoved(pos)]);
    let press = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let _ = frame_shapes(ctx, app, screen, vec![press(true), press(false)]);
}

/// AC 1 — the rail paints no text except the `AI` toggle.
#[test]
fn ac1_tool_buttons_paint_no_text() {
    let (ctx, mut app) = ctx_and_app();
    let runs = painted_runs_at(&ctx, &mut app, SCREEN, Vec::new());
    let texts: Vec<&str> = rail_runs(&ctx, &runs)
        .iter()
        .map(|r| r.text.trim())
        .collect();
    assert_eq!(texts, vec!["AI"], "tool buttons carry icons, not text");
}

/// AC 3 / AC 6 — each computed centre shows its own tooltip, proving the
/// two-column order and the tooltip format.
#[test]
fn ac3_ac6_each_button_shows_its_tooltip_in_column_order() {
    let (ctx, mut app) = ctx_and_app();
    let _ = painted_runs_at(&ctx, &mut app, SCREEN, Vec::new());
    let rail = rail_rect(&ctx);
    for (column, row, tooltip) in buttons() {
        let texts = tooltip_at(SCREEN, centre(rail, column, row));
        assert!(
            texts.iter().any(|t| t == tooltip),
            "column {column} row {row} must show {tooltip:?}: {texts:?}"
        );
    }
}

/// AC 4 / AC 5 — clicking a button activates its tool, and the selected fill
/// (a 32 pt square) sits under that button only.
#[test]
fn ac4_ac5_click_activates_and_only_the_active_button_is_filled() {
    let (ctx, mut app) = ctx_and_app();
    let _ = frame_shapes(&ctx, &mut app, SCREEN, Vec::new());
    let rail = rail_rect(&ctx);
    for (column, row, tooltip) in buttons() {
        let pos = centre(rail, column, row);
        click(&ctx, &mut app, SCREEN, pos);
        assert_eq!(
            app.tool_manager.active_tool_name().to_uppercase(),
            word(tooltip),
            "clicking {tooltip:?} must activate it"
        );
        // Park the pointer off the rail so no hover fill is painted.
        let away = vec![egui::Event::PointerMoved(egui::pos2(600.0, 300.0))];
        let _ = frame_shapes(&ctx, &mut app, SCREEN, away);
        let (_, shapes) = frame_shapes(&ctx, &mut app, SCREEN, Vec::new());
        let fills = selected_fills(&ctx, &shapes);
        assert_eq!(
            fills.len(),
            1,
            "{tooltip:?}: exactly one filled button: {fills:?}"
        );
        assert!(
            (fills[0].center() - pos).length() < 1.0,
            "{tooltip:?}: the fill must sit under the active button"
        );
        assert!(
            (fills[0].size() - egui::Vec2::splat(BUTTON)).length() < 1.0,
            "{tooltip:?}: a button is a 32 pt square, got {:?}",
            fills[0].size()
        );
    }
}

/// AC 7 — the `AI` toggle: its tooltip, a click opens the panel and fills it
/// as a 32 pt square, a second click closes it.
#[test]
fn ac7_ai_toggle_text_tooltip_fill_and_click() {
    let (ctx, mut app) = ctx_and_app();
    let runs = painted_runs_at(&ctx, &mut app, SCREEN, Vec::new());
    let pos = ai_point(&ctx, &runs);
    let texts = tooltip_at(SCREEN, pos);
    assert!(
        texts.iter().any(|t| t == "AI Assistant"),
        "the AI toggle's tooltip: {texts:?}"
    );

    click(&ctx, &mut app, SCREEN, pos);
    assert!(app.agent.panel_open, "one click opens the panel");
    let away = vec![egui::Event::PointerMoved(egui::pos2(600.0, 300.0))];
    let _ = frame_shapes(&ctx, &mut app, SCREEN, away);
    let (_, shapes) = frame_shapes(&ctx, &mut app, SCREEN, Vec::new());
    let fills = selected_fills(&ctx, &shapes);
    let ai_fill = fills.iter().find(|r| r.contains(pos));
    let ai_fill = ai_fill.unwrap_or_else(|| panic!("the open toggle must be filled: {fills:?}"));
    assert!(
        (ai_fill.size() - egui::Vec2::splat(BUTTON)).length() < 1.0,
        "the AI toggle is a 32 pt square, got {:?}",
        ai_fill.size()
    );

    click(&ctx, &mut app, SCREEN, pos);
    assert!(!app.agent.panel_open, "a second click closes the panel");
}

/// AC 8 — the rail is at most 80 pt wide (76 pt: two buttons, a gap and the
/// margins) and, at the three window sizes, the last tool of each column and
/// the `AI` toggle are all on screen with no scroll.
#[test]
fn ac8_rail_is_at_most_80pt_wide_and_nothing_scrolls() {
    for screen in [[800.0, 600.0], [1024.0, 600.0], [1280.0, 800.0]] {
        let (ctx, mut app) = ctx_and_app();
        let runs = painted_runs_at(&ctx, &mut app, screen, Vec::new());
        let rail = rail_rect(&ctx);
        assert!(
            rail.width() <= 80.0,
            "{screen:?}: rail width {}",
            rail.width()
        );
        assert!(
            (rail.width() - (2.0 * BUTTON + GAP + 2.0 * MARGIN)).abs() < 0.5,
            "{screen:?}: the test geometry must match the rail, width {}",
            rail.width()
        );

        let ai = ai_point(&ctx, &runs);
        let ai_run = rail_runs(&ctx, &runs)
            .into_iter()
            .find(|r| r.text.trim() == "AI")
            .expect("AI is painted");
        assert!(
            ai_run.pos.y + ai_run.height <= ai_run.clip.bottom(),
            "{screen:?}: the AI toggle must be fully visible without scrolling"
        );
        for (column, row) in [(0, DRAW_COLUMN.len() - 1), (1, MODIFY_COLUMN.len() - 1)] {
            let pos = centre(rail, column, row);
            assert!(
                pos.y + BUTTON / 2.0 < ai.y,
                "{screen:?}: AI sits below both columns"
            );
        }
        let texts = tooltip_at(screen, centre(rail, 1, MODIFY_COLUMN.len() - 1));
        assert!(
            texts.iter().any(|t| t == "Dist — DIST"),
            "{screen:?}: the last button must be reachable unscrolled: {texts:?}"
        );
    }
}

/// AC 9 — on a 220 pt-tall window the `AI` toggle starts out of view and a
/// real wheel scroll over the rail brings it in.
#[test]
fn ac9_a_wheel_scroll_reveals_the_ai_toggle_on_a_short_window() {
    let (ctx, mut app) = ctx_and_app();
    let screen = [1280.0, 220.0];
    let runs = painted_runs_at(&ctx, &mut app, screen, Vec::new());
    let visible = |ctx: &egui::Context, runs: &[Run]| {
        rail_runs(ctx, runs)
            .iter()
            .any(|r| r.text.trim() == "AI" && r.pos.y + r.height <= r.clip.bottom())
    };
    assert!(!visible(&ctx, &runs), "control: AI starts out of view");

    let rail = rail_rect(&ctx);
    let mut events = vec![egui::Event::PointerMoved(centre(rail, 0, 0))];
    events.extend((0..200).map(|_| egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -7.0),
        modifiers: egui::Modifiers::NONE,
    }));
    let _ = painted_runs_at(&ctx, &mut app, screen, events);
    let after = painted_runs_at(&ctx, &mut app, screen, Vec::new());
    assert!(
        visible(&ctx, &after),
        "a wheel scroll must reveal the AI toggle"
    );
}

/// Centre of the CHECK button: the `AI` row, the right column (LCV-190).
fn check_point(ctx: &egui::Context, runs: &[Run]) -> egui::Pos2 {
    let ai = ai_point(ctx, runs);
    egui::pos2(centre(rail_rect(ctx), 1, 0).x, ai.y)
}

/// LCV-190 AC 1 — the bottom row carries a CHECK icon button beside `AI`,
/// with the tooltip `Check — CHECK`, and paints no text of its own.
#[test]
fn lcv190_check_button_sits_beside_ai_with_its_tooltip() {
    let (ctx, mut app) = ctx_and_app();
    let runs = painted_runs_at(&ctx, &mut app, SCREEN, Vec::new());
    let pos = check_point(&ctx, &runs);
    let ai = ai_point(&ctx, &runs);
    assert!(pos.x > ai.x + BUTTON / 2.0, "CHECK is right of AI");
    let texts = tooltip_at(SCREEN, pos);
    assert!(
        texts.iter().any(|t| t == "Check — CHECK"),
        "the CHECK button's tooltip: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t == "AI Assistant"),
        "the CHECK point is not the AI toggle: {texts:?}"
    );
}

/// LCV-190 AC 1 — clicking CHECK runs the check: findings fill the report
/// and the dock, a clean drawing says so; the active tool does not change.
#[test]
fn lcv190_clicking_check_runs_the_check() {
    use lasercad::document::Entity;
    use lasercad::geometry::{Line, Vec2};

    let (ctx, mut app) = ctx_and_app();
    let runs = painted_runs_at(&ctx, &mut app, SCREEN, Vec::new());
    let pos = check_point(&ctx, &runs);
    click(&ctx, &mut app, SCREEN, pos);
    assert_eq!(app.command_feedback, "CHECK: no problems found.");
    assert_eq!(app.check_report, None);

    let open = Line::new(Vec2::new(10.0, 10.0), Vec2::new(50.0, 10.0));
    app.document.push_current(Entity::Line(open));
    click(&ctx, &mut app, SCREEN, pos);
    assert_eq!(app.command_feedback, "CHECK: 2 open ends");
    assert_eq!(app.check_report.as_ref().map(Vec::len), Some(3));
    assert_eq!(app.tool_manager.active_tool_name(), "Select");
    assert!(!app.agent.panel_open, "CHECK is not the AI toggle");
}
