//! LCV-140 — compact R14 chrome and useful action hints.
//!
//! Per-AC coverage (see the demand body for the acceptance criteria text):
//!
//! - AC 1-3: real painted hover-text on a representative sample of toolbar
//!   buttons, the three status-bar mode indicators and the agent toggle
//!   (the *table-derived inventory* half — every `TOOLS` entry, not just a
//!   sample — lives in `src/ui/toolbar.rs::tests::tool_hover_text_traces_to_its_own_entry`,
//!   which has access to the crate-private `TOOLS`/`ToolEntry` this binary
//!   does not); the rail's outer-width cap and full-label rendering, proven
//!   by a real `PanelState` read plus a painted-text line table; a real
//!   click per mode plus the agent toggle.
//! - AC 4-6: the `mm` unit and the preset tooltip, painted; layout-bounds
//!   measurements at 800x600 / 1024x600 / 1280x800, agent panel both closed
//!   and open, plus large signed-coordinate / four-digit-count fixtures.
//! - AC 7: hover-only frames touch no document/history/dirty state.
//!
//! LCV-115's export byte-identity regression
//! (`tests/lcv115_preset_roundtrip.rs`) is kernel-only and untouched by this
//! demand — nothing here re-proves it, since no line in `src/io/svg/`
//! changed.

mod harness;

use harness::paint::{self, group_into_lines, painted_runs_at, scoped_runs, texts, Run};
use harness::{raw_input_at, SCREEN};
use lasercad::app::App;
use lasercad::document::Entity;
use lasercad::geometry::{Line, Vec2};

// ---------------------------------------------------------------------------
// Shared plumbing
// ---------------------------------------------------------------------------

/// A headless context at `pixels_per_point == 1.0` and a default `App`
/// (ADR 0002 §A3 rule 4).
fn ctx_and_app() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    (ctx, App::default())
}

/// The one run reading `label`, and a point inside its glyph rect — the same
/// idiom `tests/lcv141_agent_panel_width_and_settings.rs::locate` uses.
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

/// A complete primary-button click at `pos`, preceded by its own warm-up
/// frame (ADR 0002 §A4 rule 3).
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

/// Click `label` on a settled frame: a warm-up `PointerMoved`, then the
/// press/release pair.
fn click(ctx: &egui::Context, app: &mut App, screen: [f32; 2], runs: &[Run], label: &str) {
    let pos = locate(runs, label);
    let _ = ctx.run(
        raw_input_at(screen, vec![egui::Event::PointerMoved(pos)]),
        |c| app.update_ui(c),
    );
    let _ = ctx.run(raw_input_at(screen, click_events(pos)), |c| {
        app.update_ui(c)
    });
}

/// A stationary hover at `pos`: one frame carrying the move, then one idle
/// frame that reads the tooltip egui paints once the pointer has settled.
///
/// Two frames are enough here — unlike the 45-idle-frame variant
/// `tests/lcv138_document_title_and_file_feedback.rs` needs after several
/// prior pointer moves — because every caller below drives this on a context
/// whose pointer has never moved before, so `last_move_time` never advances
/// (egui-0.29.1 `input_state/mod.rs::PointerState::begin_pass`) and the
/// tooltip is not held back by egui's own movement smoothing.
fn hover_runs_at(
    ctx: &egui::Context,
    app: &mut App,
    screen: [f32; 2],
    pos: egui::Pos2,
) -> Vec<Run> {
    let _ = ctx.run(
        raw_input_at(screen, vec![egui::Event::PointerMoved(pos)]),
        |c| app.update_ui(c),
    );
    let out = ctx.run(raw_input_at(screen, Vec::new()), |c| app.update_ui(c));
    paint::runs_in(&out.shapes)
}

/// Every run in `runs` sits fully inside its own clip rect, vertically and at
/// its horizontal start — LCV-133 / LCV-141's containment shape — and no two
/// runs on genuinely different rows overlap vertically.
fn assert_contained_and_non_overlapping(runs: &[&Run]) {
    const SAME_ROW_SLOP: f32 = 10.0;
    for run in runs {
        assert!(
            run.pos.y >= run.clip.top() && run.pos.y + run.height <= run.clip.bottom(),
            "`{}` is not fully inside its clip rect vertically: pos.y={}, height={}, clip={:?}",
            run.text,
            run.pos.y,
            run.height,
            run.clip
        );
        assert!(
            run.pos.x >= run.clip.left() && run.pos.x <= run.clip.right(),
            "`{}` starts outside its clip rect horizontally: pos.x={}, clip={:?}",
            run.text,
            run.pos.x,
            run.clip
        );
    }
    for a in 0..runs.len() {
        for b in (a + 1)..runs.len() {
            let (ra, rb) = (runs[a], runs[b]);
            if (ra.pos.y - rb.pos.y).abs() <= SAME_ROW_SLOP {
                continue;
            }
            let overlap = ra.pos.y < rb.pos.y + rb.height && rb.pos.y < ra.pos.y + ra.height;
            assert!(
                !overlap,
                "`{}` (y={}, h={}) and `{}` (y={}, h={}) overlap vertically",
                ra.text, ra.pos.y, ra.height, rb.text, rb.pos.y, rb.height
            );
        }
    }
}

/// The eleven `TOOLS` labels, in the acceptance-criteria order — the same
/// fixed v0.1.0 tool set `src/ui/toolbar.rs::tests::toolbar_table_is_the_v010_tool_set`
/// pins from the other side of the crate boundary.
const TOOL_LABELS: [&str; 11] = [
    "Select", "Line", "Polyline", "Rect", "Circle", "Arc", "Text", "Move", "Trim", "Extend",
    "Delete",
];

// ---------------------------------------------------------------------------
// AC 1-3 — hover hints, painted (a representative sample; the full table is
// the lib-level inventory test)
// ---------------------------------------------------------------------------

/// AC 1 / AC 3 — `Select` (no shortcut) and `Line` (shortcut `L`) each paint
/// their own, distinct hover tooltip when actually hovered — proof by
/// rendering, not by reading `src/ui/toolbar.rs`.
#[test]
fn ac1_ac3_toolbar_hover_text_paints_for_a_tool_with_and_without_a_shortcut() {
    for (label, expect_contains) in [("Select", "no keyboard shortcut"), ("Line", "shortcut: L")] {
        let (ctx, mut app) = ctx_and_app();
        let runs = painted_runs_at(&ctx, &mut app, SCREEN, Vec::new());
        let pos = locate(&runs, label);
        let hover = hover_runs_at(&ctx, &mut app, SCREEN, pos);
        assert!(
            hover
                .iter()
                .any(|r| r.text.starts_with(label) && r.text.contains(expect_contains)),
            "hovering {label:?} must paint a tooltip containing {expect_contains:?}: {:?}",
            hover.iter().map(|r| &r.text).collect::<Vec<_>>()
        );
    }
}

/// AC 3 — the three status-bar mode indicators each paint a tooltip naming
/// their own existing key.
#[test]
fn ac3_mode_indicator_hover_text_names_its_own_key() {
    for (label, key) in [("SNAP", "F3"), ("GRID", "F7"), ("ORTHO", "F8")] {
        let (ctx, mut app) = ctx_and_app();
        let runs = painted_runs_at(&ctx, &mut app, SCREEN, Vec::new());
        let pos = locate(&runs, label);
        let hover = hover_runs_at(&ctx, &mut app, SCREEN, pos);
        assert!(
            hover
                .iter()
                .any(|r| r.text.contains(label) && r.text.contains(key)),
            "hovering {label:?} must paint a tooltip naming {key:?}: {:?}",
            hover.iter().map(|r| &r.text).collect::<Vec<_>>()
        );
    }
}

/// AC 3 — the agent toggle paints the short visible text label `"Agent"` —
/// never the old icon-only `"🤖"` — and keeps its pre-existing `"AI
/// Assistant"` hover text.
#[test]
fn ac3_agent_toggle_shows_a_visible_label_and_keeps_its_hover_text() {
    let (ctx, mut app) = ctx_and_app();
    let runs = painted_runs_at(&ctx, &mut app, SCREEN, Vec::new());
    assert!(
        !runs.iter().any(|r| r.text.contains('\u{1F916}')),
        "the old icon-only toggle must be gone"
    );
    let pos = locate(&runs, "Agent");
    let hover = hover_runs_at(&ctx, &mut app, SCREEN, pos);
    assert!(
        hover.iter().any(|r| r.text.trim() == "AI Assistant"),
        "the agent toggle's hover text must stay \"AI Assistant\": {:?}",
        hover.iter().map(|r| &r.text).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// AC 1-2 — the rail: reachability, order, width cap, full-label rendering
// ---------------------------------------------------------------------------

/// AC 1 / AC 2 — the toolbar `SidePanel`'s own persisted outer rect never
/// exceeds 120pt, and every one of the eleven `TOOLS` labels plus the agent
/// toggle paints in full, in order, one per visual line — proof that nothing
/// wrapped, elided or fell back to an icon.
#[test]
fn ac1_ac2_toolbar_width_is_capped_and_every_label_renders_whole_in_order() {
    let (ctx, mut app) = ctx_and_app();
    let runs = painted_runs_at(&ctx, &mut app, SCREEN, Vec::new());

    let rect = egui::containers::panel::PanelState::load(&ctx, egui::Id::new("toolbar"))
        .expect("the toolbar panel must have stored its state by now")
        .rect;
    assert!(
        rect.width() <= 120.0 + 0.5,
        "the tool rail must be at most 120pt wide, got {}",
        rect.width()
    );

    let (scoped, _) = scoped_runs(&runs, "Select");
    let lines = texts(&group_into_lines(&scoped));
    let mut expected: Vec<Vec<String>> = TOOL_LABELS.iter().map(|l| vec![l.to_string()]).collect();
    expected.push(vec!["Agent".to_owned()]);
    assert_eq!(
        lines, expected,
        "the rail must show all eleven tools, in order, then the agent toggle, none truncated"
    );
}

/// AC 2 — a normal-height window shows every entry with nothing scrolled out
/// of view; a cramped one scrolls instead of squeezing or clipping content
/// away — the first row stays visible at the default (top) scroll offset,
/// and strictly fewer than twelve rows paint.
#[test]
fn ac2_scrolling_activates_only_when_the_rail_has_no_room() {
    let (ctx, mut app) = ctx_and_app();
    let runs = painted_runs_at(&ctx, &mut app, [1280.0, 800.0], Vec::new());
    let (scoped, _) = scoped_runs(&runs, "Select");
    let ample = texts(&group_into_lines(&scoped));
    assert_eq!(
        ample.len(),
        12,
        "at ample height all eleven tools plus the agent toggle must be visible: {ample:?}"
    );

    let (ctx2, mut app2) = ctx_and_app();
    let runs2 = painted_runs_at(&ctx2, &mut app2, [1280.0, 220.0], Vec::new());
    let (scoped2, _) = scoped_runs(&runs2, "Select");
    let cramped = texts(&group_into_lines(&scoped2));
    assert!(
        cramped.len() < 12,
        "a cramped window must scroll some rows out of view rather than squeeze them all in: {cramped:?}"
    );
    assert_eq!(
        cramped.first(),
        Some(&vec!["Select".to_owned()]),
        "the first row must still be visible at the default (top) scroll offset"
    );
}

/// AC 2, mutation-testing follow-up — deleting the `ScrollArea` wrapper still
/// passes the row-count check above (it only counts what is *visible*), so
/// this proves the hidden rows are still *reachable*: a real `MouseWheel`
/// scroll over the cramped rail — hovered, not just present — brings the
/// agent toggle (the last, and here hidden, row) into view.
#[test]
fn ac2_a_real_wheel_scroll_reaches_a_row_hidden_by_the_cramped_rail() {
    let (ctx, mut app) = ctx_and_app();
    let screen = [1280.0, 220.0];
    let runs = painted_runs_at(&ctx, &mut app, screen, Vec::new());
    assert!(
        !runs.iter().any(|r| r.text.trim() == "Agent"),
        "control: the agent toggle must start out of view in this cramped fixture"
    );
    let hover_point = locate(&runs, "Select");

    // A real hover over the rail, then many small downward-scroll ticks —
    // each under egui's smoothing threshold, so the whole delta lands within
    // this one input pass (the same idiom
    // `tests/lcv141_agent_panel_width_and_settings.rs::ac7_the_real_settings_dialog_scrolls_to_reach_done`
    // uses). Negative `y`: per `egui::Event::MouseWheel`'s own doc comment, a
    // positive `y` reveals content *above*, the opposite of reaching a row
    // below the fold.
    let mut events = vec![egui::Event::PointerMoved(hover_point)];
    events.extend((0..200).map(|_| egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -7.0),
        modifiers: egui::Modifiers::NONE,
    }));
    let _ = painted_runs_at(&ctx, &mut app, screen, events);
    // One more idle frame settles the offset the wheel just requested
    // (`ScrollArea::show` lays out from the *previous* frame's stored offset).
    let after = painted_runs_at(&ctx, &mut app, screen, Vec::new());

    assert!(
        after.iter().any(|r| r.text.trim() == "Agent"),
        "a real wheel scroll over the rail must reveal the row hidden before \
         it: {:?}",
        after.iter().map(|r| &r.text).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// AC 1 / AC 7 — clicks still fire their one action, exactly once
// ---------------------------------------------------------------------------

/// AC 1 / AC 7 — clicking a toolbar button activates its tool and commits no
/// history entry.
#[test]
fn ac1_ac7_clicking_a_toolbar_button_activates_its_tool_and_commits_nothing() {
    let (ctx, mut app) = ctx_and_app();
    let runs = painted_runs_at(&ctx, &mut app, SCREEN, Vec::new());
    let before = app.history.revision();
    click(&ctx, &mut app, SCREEN, &runs, "Circle");
    assert_eq!(app.tool_manager.active_tool_name(), "CIRCLE");
    assert_eq!(
        app.history.revision(),
        before,
        "activating a tool commits nothing"
    );
}

/// AC 3 / AC 7 — clicking a mode indicator flips exactly its own flag, once.
#[test]
fn ac3_ac7_clicking_a_mode_indicator_flips_only_its_own_flag_once() {
    let (ctx, mut app) = ctx_and_app();
    assert!(app.snap_enabled);
    assert!(app.grid_enabled);
    assert!(!app.ortho_enabled);
    let runs = painted_runs_at(&ctx, &mut app, SCREEN, Vec::new());
    click(&ctx, &mut app, SCREEN, &runs, "SNAP");
    assert!(!app.snap_enabled, "one click must flip snap off");
    assert!(app.grid_enabled, "grid must not move");
    assert!(!app.ortho_enabled, "ortho must not move");
}

/// AC 3 / AC 7 — clicking the agent toggle opens the panel exactly once.
#[test]
fn ac3_ac7_clicking_the_agent_toggle_opens_the_panel_once() {
    let (ctx, mut app) = ctx_and_app();
    assert!(!app.agent.panel_open);
    let runs = painted_runs_at(&ctx, &mut app, SCREEN, Vec::new());
    click(&ctx, &mut app, SCREEN, &runs, "Agent");
    assert!(app.agent.panel_open, "one click must open the panel");
}

// ---------------------------------------------------------------------------
// AC 4 — the mm unit and the preset tooltip
// ---------------------------------------------------------------------------

/// AC 4 — the coordinate readout carries an explicit `mm` unit on both axes.
#[test]
fn ac4_coordinate_readout_carries_an_explicit_mm_unit() {
    let (ctx, mut app) = ctx_and_app();
    app.last_cursor_world = Some(Vec2::new(123.45, 67.89));
    let runs = painted_runs_at(&ctx, &mut app, SCREEN, Vec::new());
    assert!(
        runs.iter()
            .any(|r| r.text.trim() == "X: 123.45mm  Y:  67.89mm"),
        "the coordinate readout must show an explicit mm unit: {:?}",
        runs.iter().map(|r| &r.text).collect::<Vec<_>>()
    );
}

/// AC 4 — hovering the preset badge paints its explanation: the shown preset
/// receives every exported entity, the other two are written empty.
#[test]
fn ac4_preset_badge_hover_text_explains_the_whole_document_choice() {
    let (ctx, mut app) = ctx_and_app();
    assert_eq!(
        app.export_preset,
        lasercad::io::Preset::Cut,
        "positive control: default preset"
    );
    let runs = painted_runs_at(&ctx, &mut app, SCREEN, Vec::new());
    let pos = locate(&runs, "CUT");
    let hover = hover_runs_at(&ctx, &mut app, SCREEN, pos);
    assert!(
        hover.iter().any(|r| {
            let t = r.text.trim();
            t.contains("Cut") && t.contains("Mark") && t.contains("Engrave") && t.contains("empty")
        }),
        "the preset badge's tooltip must name the receiving group and the two empty ones: {:?}",
        hover.iter().map(|r| &r.text).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// AC 5 — every status segment fits, at three sizes
// ---------------------------------------------------------------------------

/// Every expected status-bar label is painted, contained within its own clip
/// rect, and the status bar's own outer height is at most 56pt — all at
/// `screen`.
fn assert_status_bar_fits(screen: [f32; 2]) {
    let (ctx, mut app) = ctx_and_app();
    let runs = painted_runs_at(&ctx, &mut app, screen, Vec::new());

    let rect = egui::containers::panel::PanelState::load(&ctx, egui::Id::new("statusbar"))
        .expect("the status bar must have stored its state by now")
        .rect;
    assert!(
        rect.height() <= 56.0,
        "at {screen:?}: the status bar must be at most 56pt tall, got {}",
        rect.height()
    );

    let expected = ["SELECT", "Entities: 0", "CUT", "SNAP", "GRID", "ORTHO"];
    let found: Vec<&Run> = expected
        .iter()
        .map(|label| {
            let matches: Vec<&Run> = runs.iter().filter(|r| r.text.trim() == *label).collect();
            assert_eq!(
                matches.len(),
                1,
                "at {screen:?}: `{label}` must be painted exactly once, saw {} times",
                matches.len()
            );
            matches[0]
        })
        .collect();
    assert_contained_and_non_overlapping(&found);
    assert!(
        runs.iter().any(|r| r.text.contains("autosave")),
        "at {screen:?}: the autosave indicator must be painted"
    );
}

/// AC 5 — at 800x600, 1024x600 and 1280x800, every status segment paints
/// fully inside its row, and the status bar stays at most 56pt tall.
#[test]
fn ac5_status_segments_fit_at_three_sizes() {
    for screen in [[800.0, 600.0], [1024.0, 600.0], [1280.0, 800.0]] {
        assert_status_bar_fits(screen);
    }
}

// ---------------------------------------------------------------------------
// AC 6 — the agent panel open at its own ceiling, plus large fixtures
// ---------------------------------------------------------------------------

/// AC 6 — with the agent panel open at an 800pt-wide application, the
/// toolbar, status bar, command dock and agent panel together still leave at
/// least 320x300pt of canvas, and a six-digit signed coordinate pair plus a
/// four-digit entity count neither clip nor hide any mode indicator, tool
/// button or the preset badge.
#[test]
fn ac6_canvas_stays_usable_with_the_agent_panel_open_and_large_fixtures() {
    let (ctx, mut app) = ctx_and_app();
    app.agent.panel_open = true;
    app.last_cursor_world = Some(Vec2::new(-1234.56, -1234.56));
    for _ in 0..1234 {
        app.document.entities.push(Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
        )));
    }

    let screen = [800.0, 700.0];
    let runs = painted_runs_at(&ctx, &mut app, screen, Vec::new());

    assert!(
        app.camera.viewport_size_px[0] >= 320.0 && app.camera.viewport_size_px[1] >= 300.0,
        "the canvas must stay at least 320x300pt, got {:?}",
        app.camera.viewport_size_px
    );

    assert!(
        runs.iter()
            .any(|r| r.text.trim() == "X: -1234.56mm  Y: -1234.56mm"),
        "the six-digit signed coordinate pair must paint intact: {:?}",
        runs.iter().map(|r| &r.text).collect::<Vec<_>>()
    );
    let expected = ["SELECT", "Entities: 1234", "CUT", "SNAP", "GRID", "ORTHO"];
    let found: Vec<&Run> = expected
        .iter()
        .map(|label| {
            let matches: Vec<&Run> = runs.iter().filter(|r| r.text.trim() == *label).collect();
            assert_eq!(
                matches.len(),
                1,
                "`{label}` must still be painted exactly once with the agent panel open: saw {} times",
                matches.len()
            );
            matches[0]
        })
        .collect();
    assert_contained_and_non_overlapping(&found);
}

// ---------------------------------------------------------------------------
// AC 7 — hover-only frames touch no document/history/dirty state
// ---------------------------------------------------------------------------

/// AC 7 — hovering every new hint (toolbar buttons, mode indicators, the
/// agent toggle, the preset badge) — with no click at all — leaves the
/// document, the history and the dirty signal exactly as they were.
#[test]
fn ac7_hover_only_frames_leave_document_history_and_dirty_state_untouched() {
    let (ctx, mut app) = ctx_and_app();
    let runs = painted_runs_at(&ctx, &mut app, SCREEN, Vec::new());

    let entities_before = app.document.entity_count();
    let revision_before = app.history.revision();
    let dirty_before = app.dirty_since;
    let panel_open_before = app.agent.panel_open;
    let snap_before = app.snap_enabled;

    for label in [
        "Select", "Line", "Delete", "Agent", "SNAP", "GRID", "ORTHO", "CUT",
    ] {
        let pos = locate(&runs, label);
        let _ = hover_runs_at(&ctx, &mut app, SCREEN, pos);
    }

    assert_eq!(app.document.entity_count(), entities_before);
    assert_eq!(app.history.revision(), revision_before);
    assert_eq!(app.dirty_since, dirty_before);
    assert_eq!(app.agent.panel_open, panel_open_before);
    assert_eq!(app.snap_enabled, snap_before);
}
