//! LCV-167 — UI wording, one AI name and status bar polish.
//!
//! Every wording claim is read off painted runs (`tests/harness/paint.rs`):
//! menus are opened by real pointer clicks, windows by the state their menu
//! item sets, and the text checked is the text egui laid out.

use crate::harness;

use harness::paint::{Run, painted_runs};
use harness::raw_input;
use lasercad::app::App;

/// Words Title Case leaves lowercase (DESIGN.md §9).
const MINOR: [&str; 10] = ["to", "as", "a", "an", "the", "of", "in", "on", "and", "or"];

fn ctx_and_app() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    (ctx, App::default())
}

/// A few idle frames: popups fade in and windows are placed a frame late.
fn settle(ctx: &egui::Context, app: &mut App) -> Vec<Run> {
    for _ in 0..5 {
        let _ = painted_runs(ctx, app);
    }
    painted_runs(ctx, app)
}

/// A point inside the one run reading exactly `label`.
fn locate(runs: &[Run], label: &str) -> egui::Pos2 {
    let hits: Vec<&Run> = runs.iter().filter(|r| r.text.trim() == label).collect();
    assert_eq!(hits.len(), 1, "`{label}` must be painted exactly once");
    egui::pos2(hits[0].pos.x + 2.0, hits[0].pos.y + hits[0].height / 2.0)
}

/// A real pointer click at `pos`, after its own hover frame.
fn click(ctx: &egui::Context, app: &mut App, pos: egui::Pos2) {
    let _ = ctx.run(raw_input(vec![egui::Event::PointerMoved(pos)]), |c| {
        app.update_ui(c)
    });
    let press = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let _ = ctx.run(raw_input(vec![press(true), press(false)]), |c| {
        app.update_ui(c)
    });
}

/// The runs `after` paints whose text `before` did not paint at all.
fn new_texts(before: &[Run], after: &[Run]) -> Vec<String> {
    after
        .iter()
        .map(|r| r.text.trim().to_owned())
        .filter(|t| !before.iter().any(|b| b.text.trim() == t))
        .collect()
}

/// Whether `label` is Title Case: the part before a `\t` shortcut, split on
/// spaces, has no word outside [`MINOR`] starting lowercase.
fn is_title_case(label: &str) -> bool {
    let text = label.split('\t').next().unwrap_or_default();
    text.split_whitespace()
        .all(|word| MINOR.contains(&word) || !word.chars().next().is_some_and(char::is_lowercase))
}

/// Open top-level menu `menu` on a fresh app; return its item texts.
fn menu_items(menu: &str) -> Vec<String> {
    let (ctx, mut app) = ctx_and_app();
    let closed = settle(&ctx, &mut app);
    click(&ctx, &mut app, locate(&closed, menu));
    let open = settle(&ctx, &mut app);
    let items = new_texts(&closed, &open);
    assert!(!items.is_empty(), "`{menu}` must open");
    items
}

// ── AC 1 / AC 2 — Title Case labels, AI Settings ───────────────────────

/// AC 1 — the checker itself: controls in both directions.
#[test]
fn ac1_title_case_check_controls() {
    assert!(is_title_case("Save As…\tCtrl+Shift+S"));
    assert!(is_title_case("Fit to Bed"));
    assert!(is_title_case("Keyboard Shortcuts…\tF1"));
    assert!(!is_title_case("Bed size…"));
    assert!(!is_title_case("Export layers"));
}

/// AC 1 / AC 2 — every item of every top-level menu is Title Case, and the
/// renamed rows are there.
#[test]
fn ac1_every_menu_item_is_title_case() {
    let mut all = Vec::new();
    for menu in ["File", "Edit", "View", "Format", "Tools", "Help"] {
        let items = menu_items(menu);
        for item in &items {
            assert!(is_title_case(item), "{menu} > `{item}` is not Title Case");
        }
        all.extend(items);
    }
    let first_parts: Vec<&str> = all
        .iter()
        .map(|t| t.split('\t').next().unwrap_or_default())
        .collect();
    for row in [
        "Export Layers",
        "Bed Size…",
        "Keyboard Shortcuts…",
        "AI Settings…",
        "Object Snap",
    ] {
        assert!(first_parts.contains(&row), "missing row `{row}`: {all:?}");
    }
}

/// AC 1 — View > Object Snap's own items are Title Case too.
#[test]
fn ac1_object_snap_submenu_is_title_case() {
    let (ctx, mut app) = ctx_and_app();
    let closed = settle(&ctx, &mut app);
    click(&ctx, &mut app, locate(&closed, "View"));
    let view = settle(&ctx, &mut app);
    let row = locate(&view, "Object Snap");
    let _ = ctx.run(raw_input(vec![egui::Event::PointerMoved(row)]), |c| {
        app.update_ui(c)
    });
    let sub = settle(&ctx, &mut app);
    let items = new_texts(&view, &sub);
    assert!(items.contains(&"Endpoint".to_owned()), "{items:?}");
    for item in &items {
        assert!(is_title_case(item), "Object Snap > `{item}`");
    }
}

/// AC 1 / AC 2 — the Bed Size, Keyboard Shortcuts (F1) and AI Settings
/// windows paint their Title Case titles; AI Settings paints
/// `Restore Default`.
#[test]
fn ac1_window_titles_and_restore_default() {
    let (ctx, mut app) = ctx_and_app();
    app.bed_dialog = Some(app.document.bed_mm);
    let runs = settle(&ctx, &mut app);
    let _ = locate(&runs, "Bed Size");

    let (ctx, mut app) = ctx_and_app();
    harness::tap(&ctx, &mut app, egui::Key::F1, egui::Modifiers::NONE);
    let runs = settle(&ctx, &mut app);
    let _ = locate(&runs, "Keyboard Shortcuts");

    let (ctx, mut app) = ctx_and_app();
    app.agent_settings_open = true;
    let runs = settle(&ctx, &mut app);
    let _ = locate(&runs, "AI Settings");
    let _ = locate(&runs, "Restore Default");
}

// ── AC 3 — one AI name ──────────────────────────────────────────────────

/// AC 3 — the rail toggle paints `AI` with tooltip `AI Assistant`, the panel
/// it opens is headed `AI Assistant`, and a `:draw` line routes to `AI`.
#[test]
fn ac3_rail_panel_and_dock_say_ai() {
    let (ctx, mut app) = ctx_and_app();
    let runs = settle(&ctx, &mut app);
    let toggle = locate(&runs, "AI");
    let _ = ctx.run(raw_input(vec![egui::Event::PointerMoved(toggle)]), |c| {
        app.update_ui(c)
    });
    let hover = settle(&ctx, &mut app);
    let _ = locate(&hover, "AI Assistant");

    click(&ctx, &mut app, toggle);
    assert!(app.agent.panel_open, "the toggle opens the panel");
    let away = egui::pos2(600.0, 300.0);
    let _ = ctx.run(raw_input(vec![egui::Event::PointerMoved(away)]), |c| {
        app.update_ui(c)
    });
    let open = settle(&ctx, &mut app);
    let _ = locate(&open, "AI Assistant");

    let (ctx, mut app) = ctx_and_app();
    app.settings.agent_api_key = "sk-test-key".to_owned();
    app.command_line_input = ":draw a line".to_owned();
    let runs = settle(&ctx, &mut app);
    let lines = harness::paint::lines_on_surface_of(&runs, "Destination:");
    let row = lines
        .into_iter()
        .map(|(_, texts)| texts)
        .find(|texts| texts.iter().any(|t| t == "Destination:"))
        .expect("the dock paints its destination row");
    assert_eq!(row.last().map(String::as_str), Some("AI"), "{row:?}");
}
