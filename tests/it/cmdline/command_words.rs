//! tests/it/cmdline/command_words.rs — the full command name reaches the tool,
//! not only the parser (LCV-131 AC 5).
//!
//! `src/cmdline/parse.rs::tests::parses_tool_aliases` proves `parse(word)`
//! resolves to the right `ToolKind`; that is a claim about the grammar alone.
//! This drives the real frame body through
//! `egui::Context::run(raw_input(...), |ctx| app.update_ui(ctx))`
//! (`harness::submit_command`) and asserts the tool that actually became
//! active plus the absence of `Unknown command` feedback, so a parser-only
//! change — `parse` answers `Tool(Line)` for `"line"` while `submit`'s
//! dispatch never reaches `tools::make` — fails here where it would still
//! pass the unit table (the demand's mutation (f)).

use crate::harness;

use harness::{frame, submit_command};
use lasercad::app::App;

/// Boot an app and run the first frame, which registers every widget rect and
/// syncs the camera away from its zero-area sentinel (harness rule 3 does not
/// bind here — no pointer event is ever sent in this file).
fn boot() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    let mut app = App::default();
    frame(&ctx, &mut app, vec![]);
    (ctx, app)
}

/// AC 5 — every one of the fifteen approved words activates exactly the tool
/// its name is expected to, and leaves no `Unknown command` feedback behind.
///
/// A fresh `App` per word: which tool a word activates must not depend on
/// whichever tool a previous word in the list left active. `LINE` (upper) and
/// `line` (lower) are both included per AC 2's case-insensitivity claim.
#[test]
fn every_command_word_activates_its_tool() {
    let words: &[(&str, &str)] = &[
        ("LINE", "LINE"),
        ("line", "LINE"),
        ("polyline", "PLINE"),
        ("pline", "PLINE"),
        ("rect", "RECT"),
        ("rectangle", "RECT"),
        ("circle", "CIRCLE"),
        ("arc", "ARC"),
        ("select", "Select"),
        ("trim", "TRIM"),
        ("extend", "EXTEND"),
        ("move", "MOVE"),
        ("text", "TEXT"),
        ("delete", "ERASE"),
        ("del", "ERASE"),
        ("erase", "ERASE"),
    ];

    for &(word, expected_tool) in words {
        let (ctx, mut app) = boot();
        submit_command(&ctx, &mut app, word);
        assert_eq!(
            app.tool_manager.active_tool_name(),
            expected_tool,
            "{word:?} must activate the {expected_tool} tool"
        );
        assert!(
            !app.command_feedback.contains("Unknown command"),
            "{word:?} must not answer Unknown command, got {:?}",
            app.command_feedback
        );
    }
}

/// LCV-156 AC 4 — `LAYER` and its R14 alias `LA` (any case) open the
/// Layers… dialog on the current layer, leave the active tool alone and
/// answer no `Unknown command`.
#[test]
fn layer_and_la_open_the_layers_dialog() {
    for word in ["LAYER", "layer", "LA", "la"] {
        let (ctx, mut app) = boot();
        assert!(app.layers_dialog.is_none(), "positive control");
        let tool = app.tool_manager.active_tool_name();
        submit_command(&ctx, &mut app, word);
        let dialog = app.layers_dialog.as_ref();
        assert_eq!(
            dialog.map(|d| d.selected),
            Some(app.document.current_layer()),
            "{word:?} must open the Layers dialog on the current layer"
        );
        assert_eq!(app.tool_manager.active_tool_name(), tool, "{word:?}");
        assert!(
            !app.command_feedback.contains("Unknown command"),
            "{word:?} answered {:?}",
            app.command_feedback
        );
    }
}
