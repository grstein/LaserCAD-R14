//! LCV-116 AC 11 / AC 13 / AC 16 — the F1 shortcuts dialog.
//!
//! Product decision 5: `F1` sits in the `F3`/`F7`/`F8` class — a bare key
//! dispatched by `src/ui/shortcuts.rs::dispatch_shortcuts` that fires *even
//! while a text widget holds keyboard focus*, because the operator who is lost
//! mid-text is exactly the one reaching for help. ADR 0002 §A6 still holds:
//! `dispatch_shortcuts` and `src/app/input.rs` remain the only two key readers,
//! and the dialog itself reads nothing.
//!
//! ADR 0002 §A4: every tap is press+release (`harness::tap`), no `Ctrl+O` /
//! `Ctrl+S` / `Ctrl+Shift+S` is sent, and no test lets the autosave debounce
//! elapse.

use crate::harness;

use harness::{frame, tap, type_command};
use lasercad::app::App;
use lasercad::ui::shortcuts::dispatch_shortcuts;

/// AC 11 / AC 13 — `F1` through the real frame body opens the dialog, and the
/// following frame renders it without panicking.
#[test]
fn f1_opens_the_shortcuts_dialog() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    assert!(!app.shortcuts_open, "a fresh app starts with it closed");

    tap(&ctx, &mut app, egui::Key::F1, egui::Modifiers::NONE);
    assert!(app.shortcuts_open, "F1 must open the dialog");

    // The dialog body now runs for real, scroll area and all.
    frame(&ctx, &mut app, vec![]);
    frame(&ctx, &mut app, vec![]);
    assert!(app.shortcuts_open, "rendering must not close it");
}

/// AC 13 — the F3/F7/F8 class contract, asserted at the dispatcher: `F1` fires
/// with `wants_kbd == true`. Calling `dispatch_shortcuts` directly is the only
/// way to pin this half; through a frame, `wants_keyboard_input` lags a frame
/// behind focus (ADR 0002 §A4) and the assertion would be about egui's timing
/// rather than about the gate.
#[test]
fn f1_opens_the_dialog_even_with_keyboard_focus() {
    let mut app = App::default();

    let fired = dispatch_shortcuts(
        egui::Key::F1,
        egui::Modifiers::NONE,
        /* wants_kbd */ true,
        &mut app,
    );

    assert!(
        fired,
        "F1 must be consumed even while a text widget has focus"
    );
    assert!(app.shortcuts_open);

    // Negative control: a gated key in the same call shape does *not* fire, so
    // the assertion above is about F1's class and not about the argument being
    // ignored.
    let mut other = App::default();
    let tool_before = other.tool_manager.active_tool_name().to_owned();
    let gated = dispatch_shortcuts(
        egui::Key::L,
        egui::Modifiers::NONE,
        /* wants_kbd */ true,
        &mut other,
    );
    assert!(
        !gated,
        "a tool letter must stay gated behind keyboard focus"
    );
    assert_eq!(other.tool_manager.active_tool_name(), tool_before);
}

/// AC 13 / the manual smoke's last line — `F1` while the operator is typing in
/// the command line opens the dialog and steals no character. Two frames of
/// typing first, so the field genuinely holds focus by the time F1 arrives.
#[test]
fn f1_opens_while_typing_without_eating_the_text() {
    let ctx = egui::Context::default();
    let mut app = App::default();

    type_command(&ctx, &mut app, "lin");
    let typed = app.command_line_input.clone();
    assert_eq!(typed, "lin", "positive control: the text really was typed");

    tap(&ctx, &mut app, egui::Key::F1, egui::Modifiers::NONE);

    assert!(app.shortcuts_open, "F1 must open the dialog mid-text");
    assert_eq!(
        app.command_line_input, typed,
        "F1 must steal no character from the command line"
    );
}

/// AC 11 — egui's × is the only close path, and it works through the `open`
/// flag: clearing `shortcuts_open` and rendering leaves nothing behind.
#[test]
fn closing_the_dialog_leaves_no_state_behind() {
    let ctx = egui::Context::default();
    let mut app = App::default();

    tap(&ctx, &mut app, egui::Key::F1, egui::Modifiers::NONE);
    frame(&ctx, &mut app, vec![]);
    let tool = app.tool_manager.active_tool_name().to_owned();
    let entities = app.document.entity_count();
    let revision = app.history.revision();

    app.shortcuts_open = false;
    frame(&ctx, &mut app, vec![]);
    frame(&ctx, &mut app, vec![]);

    assert!(!app.shortcuts_open, "it must stay closed");
    assert!(!app.about_open, "and must not have opened its neighbour");
    assert_eq!(app.tool_manager.active_tool_name(), tool);
    assert_eq!(app.document.entity_count(), entities);
    assert_eq!(app.history.revision(), revision);
    assert!(app.dirty_since.is_none(), "opening help dirties nothing");

    // And it reopens — the flag is not one-shot.
    tap(&ctx, &mut app, egui::Key::F1, egui::Modifiers::NONE);
    assert!(app.shortcuts_open);
}

/// AC 13 — "No other code reads `F1`." `Key::F1` may appear in exactly one
/// implementation file: `src/ui/shortcuts.rs`, the dispatcher.
///
/// The scan is bounded to implementation text (everything before a bare
/// `#[cfg(test)]` at column zero, per ADR 0004) so that test modules naming the
/// key — including this demand's own — cannot satisfy or break it.
#[test]
fn f1_has_exactly_one_reader() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let needle = concat!("Key::", "F1");
    let mut readers = Vec::new();
    let mut scanned = 0usize;

    for path in walk(&root.join("src")) {
        if crate::harness::scan::is_test_file(&path) {
            continue;
        }
        let src = std::fs::read_to_string(&path).expect("source file");
        scanned += 1;
        if implementation(&src).contains(needle) {
            // Forward slashes on every platform: `Path::display` emits `\` on
            // Windows, which would make this comparison a CI-only failure.
            readers.push(
                path.strip_prefix(root)
                    .unwrap_or(&path)
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/"),
            );
        }
    }

    assert!(
        scanned >= 40,
        "only {scanned} files scanned — walk is broken"
    );
    assert_eq!(
        readers,
        vec!["src/ui/shortcuts.rs".to_string()],
        "F1 must have exactly one reader (ADR 0002 §A6)"
    );

    // Positive control over the same slice: the sibling keys of the same class
    // are found by this scan too, so a zero-hit result cannot be silent.
    let dispatcher =
        std::fs::read_to_string(root.join("src/ui/shortcuts.rs")).expect("the dispatcher");
    let dispatcher = implementation(&dispatcher);
    for sibling in [concat!("Key::", "F3"), concat!("Key::", "F8")] {
        assert!(
            dispatcher.contains(sibling),
            "positive control: {sibling} must be in the same file"
        );
    }
}

/// AC 17 — `src/app/input.rs`'s gate table, the living record of which keys are
/// read where, carries the `F1` row and marks it ungated like its class-mates.
#[test]
fn the_gate_table_documents_f1() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = std::fs::read_to_string(root.join("src/app/input.rs")).expect("src/app/input.rs");
    let table: String = src
        .lines()
        .filter(|l| l.starts_with("//! |"))
        .map(|l| format!("{l}\n"))
        .collect();

    assert!(
        table.contains("| tool activation |"),
        "positive control: the gate table must still list the gated tool keys"
    );
    let row = table
        .lines()
        .find(|l| l.contains("`F1`"))
        .expect("AC 17 — the gate table must carry an F1 row");
    assert!(
        row.contains("shortcuts.rs"),
        "the F1 row must name its reader, got: {row}"
    );
    assert!(
        row.contains("yes"),
        "the F1 row must record that it is ungated, got: {row}"
    );
}

/// Implementation text only — everything before a bare `#[cfg(test)]` at
/// column zero (ADR 0004).
fn implementation(src: &str) -> &str {
    src.split_once("\n#[cfg(test)]")
        .map_or(src, |(head, _)| head)
}

/// Every `.rs` under `dir`, recursively.
fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).expect("readable directory") {
        let path = entry.expect("readable entry").path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out.sort();
    out
}
