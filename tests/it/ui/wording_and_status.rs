//! LCV-167 — UI wording, one AI name and status bar polish.
//!
//! Every wording claim is read off painted runs (`tests/harness/paint.rs`):
//! menus are opened by real pointer clicks, windows by the state their menu
//! item sets, and the text checked is the text egui laid out.

use crate::harness;

use harness::paint::{Run, painted_runs};
use harness::raw_input;
use harness::scan::{is_test_file, occurrences, rs_files};
use lasercad::app::App;

/// `status.error` (DESIGN.md §3; pinned to `ui/theme.rs` by its own test).
const STATUS_ERROR: egui::Color32 = egui::Color32::from_rgb(0xff, 0x6b, 0x6b);
/// `status.warning` (DESIGN.md §3).
const STATUS_WARNING: egui::Color32 = egui::Color32::from_rgb(0xff, 0x8f, 0x00);

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

// ── AC 7 — one readable error red ───────────────────────────────────────

/// Every text shape of one settled frame, `Shape::Vec` flattened.
fn text_shapes(ctx: &egui::Context, app: &mut App) -> Vec<egui::epaint::TextShape> {
    let _ = settle(ctx, app);
    let out = ctx.run(raw_input(Vec::new()), |c| app.update_ui(c));
    let mut stack: Vec<egui::Shape> = out.shapes.into_iter().map(|c| c.shape).collect();
    let mut found = Vec::new();
    while let Some(shape) = stack.pop() {
        match shape {
            egui::Shape::Vec(inner) => stack.extend(inner),
            egui::Shape::Text(t) => found.push(t),
            _ => {}
        }
    }
    found
}

/// The colours of the one text shape whose text starts with `prefix`
/// (a `PLACEHOLDER` section paints in the shape's `fallback_color`).
fn colours_of(shapes: &[egui::epaint::TextShape], prefix: &str) -> Vec<egui::Color32> {
    let hits: Vec<&egui::epaint::TextShape> = shapes
        .iter()
        .filter(|t| t.galley.text().starts_with(prefix))
        .collect();
    assert_eq!(hits.len(), 1, "`{prefix}…` must paint once");
    let t = hits[0];
    if let Some(c) = t.override_text_color {
        return vec![c];
    }
    t.galley
        .job
        .sections
        .iter()
        .map(|s| match s.format.color {
            egui::Color32::PLACEHOLDER => t.fallback_color,
            c => c,
        })
        .collect()
}

/// AC 7 — an AI panel `error` row paints in `status.error`.
#[test]
fn ac7_panel_error_row_paints_status_error() {
    let (ctx, mut app) = ctx_and_app();
    app.agent.panel_open = true;
    app.agent
        .chat
        .push(("error".to_owned(), "LCV167ERR the turn failed".to_owned()));
    let shapes = text_shapes(&ctx, &mut app);
    assert_eq!(colours_of(&shapes, "LCV167ERR"), vec![STATUS_ERROR]);
}

/// AC 7 — the `! AI unavailable` dock line paints in `status.error`; an
/// ordinary feedback line stays `status.warning` (control).
#[test]
fn ac7_dock_error_line_is_status_error_and_feedback_stays_warning() {
    let (ctx, mut app) = ctx_and_app();
    harness::submit_command(&ctx, &mut app, ":draw a line");
    assert!(app.command_feedback.starts_with("! AI unavailable"));
    let shapes = text_shapes(&ctx, &mut app);
    assert_eq!(colours_of(&shapes, "! AI unavailable"), vec![STATUS_ERROR]);

    let (ctx, mut app) = ctx_and_app();
    harness::submit_command(&ctx, &mut app, "lien");
    let shapes = text_shapes(&ctx, &mut app);
    assert_eq!(colours_of(&shapes, "Unknown command"), vec![STATUS_WARNING]);
}

/// AC 7 — no implementation code under `src/` names egui's pure red.
#[test]
fn ac7_no_pure_red_in_src_outside_tests() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rs_files(&root, &mut files);
    files.retain(|p| !is_test_file(p));
    let sections: Vec<(String, String)> = files
        .iter()
        .map(|p| {
            let src = std::fs::read_to_string(p).expect("readable source");
            let body = src.split("\n#[cfg(test)]").next().unwrap_or_default();
            (p.display().to_string(), body.to_owned())
        })
        .collect();
    let needle = concat!("Color32::", "RED");
    let control = occurrences(&sections, concat!("Color32::", "from_rgb("));
    assert!(!control.is_empty(), "positive control: the scan reads code");
    let hits = occurrences(&sections, needle);
    assert!(hits.is_empty(), "{hits:?}");
}

// ── AC 8 — egui's selection fill, accent foreground-only ────────────────

/// AC 8 — after `apply_theme` the selection fill is egui's `#005c80` and no
/// widget fill is the `#4fa3e0` accent, which stays a foreground (the
/// selection stroke is the positive control).
#[test]
fn ac8_selection_fill_is_default_and_accent_is_foreground_only() {
    const FILL_SELECTED: egui::Color32 = egui::Color32::from_rgb(0x00, 0x5c, 0x80);
    const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x4f, 0xa3, 0xe0);
    let ctx = egui::Context::default();
    lasercad::ui::apply_theme(&ctx);
    let v = ctx.style().visuals.clone();
    assert_eq!(v.selection.bg_fill, FILL_SELECTED);
    assert_eq!(v.selection.stroke.color, ACCENT, "control: accent is used");
    let w = &v.widgets;
    for state in [
        &w.noninteractive,
        &w.inactive,
        &w.hovered,
        &w.active,
        &w.open,
    ] {
        assert_ne!(state.bg_fill, ACCENT, "{state:?}");
        assert_ne!(state.weak_bg_fill, ACCENT, "{state:?}");
    }
}

// ── AC 9 — the failed autosave badge ────────────────────────────────────

/// AC 9 — an autosave path that is a directory fails the write: the bar
/// shows `× autosave failed` in `status.error`. Pointed at a writable file
/// and dirtied again, the next write restores `○ autosaved`.
#[test]
fn ac9_a_failed_autosave_shows_until_the_next_write_succeeds() {
    let dir = std::env::temp_dir().join("lcv167_autosave_failed");
    let _ = std::fs::remove_dir_all(&dir);
    let blocked = dir.join("autosave.json");
    std::fs::create_dir_all(&blocked).expect("temp dir is writable");
    let (ctx, mut app) = ctx_and_app();
    app.autosave_path = Some(blocked);
    let due = || std::time::Instant::now().checked_sub(std::time::Duration::from_secs(5));

    app.dirty_since = due();
    let shapes = text_shapes(&ctx, &mut app);
    assert!(app.autosave_failed, "the write into a directory fails");
    assert_eq!(
        colours_of(&shapes, "\u{d7} autosave failed"),
        vec![STATUS_ERROR]
    );

    app.autosave_path = Some(dir.join("ok.json"));
    app.dirty_since = due();
    let runs = settle(&ctx, &mut app);
    let _ = locate(&runs, "\u{25cb} autosaved");
    assert!(!runs.iter().any(|r| r.text.contains("autosave failed")));
    let _ = std::fs::remove_dir_all(&dir);
}

/// AC 9 — with no autosave path a due flush never shows the failed badge.
#[test]
fn ac9_a_pathless_flush_never_shows_failed() {
    let (ctx, mut app) = ctx_and_app();
    app.dirty_since = std::time::Instant::now().checked_sub(std::time::Duration::from_secs(5));
    let runs = settle(&ctx, &mut app);
    assert!(app.dirty_since.is_none(), "control: the flush ran");
    let _ = locate(&runs, "\u{25cb} no autosave yet");
}
