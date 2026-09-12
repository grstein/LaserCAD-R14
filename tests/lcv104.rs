//! tests/lcv104.rs — LCV-104: TextTool reachability via the `D` shortcut.
//!
//! AC#8 drives the real frame body (`harness::tap`), proving `D` is
//! reachable end-to-end, not just through the dispatch table. AC#9 and
//! AC#10 call `dispatch_shortcuts` directly with synthetic values, mirroring
//! `tests/lcv070.rs`, since they exercise the gate table's focus and
//! bare-key-only rules that `dispatch_shortcuts` alone owns.

mod harness;

use egui::{Key, Modifiers};
use harness::tap;
use lasercad::app::App;
use lasercad::ui::shortcuts::dispatch_shortcuts;

/// AC#8 — one `D` tap through `App::update_ui` activates TEXT.
#[test]
fn d_activates_text_tool() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    assert_ne!(app.tool_manager.active_tool_name(), "TEXT");

    tap(&ctx, &mut app, Key::D, Modifiers::NONE);

    assert_eq!(
        app.tool_manager.active_tool_name(),
        "TEXT",
        "a bare D tap must activate TextTool"
    );
}

/// AC#9 — bare `D` while a text widget has focus must not activate TEXT
/// (ADR 0002 "tool activation" gate row: bare key, suppressed under focus).
#[test]
fn d_suppressed_while_text_widget_focused() {
    let mut app = App::default();
    dispatch_shortcuts(Key::D, Modifiers::NONE, true /* wants_kbd */, &mut app);
    assert_eq!(
        app.tool_manager.active_tool_name(),
        "Select",
        "D must not activate TextTool while a text widget has focus"
    );
}

/// AC#10 — `Ctrl+D` must not activate TEXT (bare-key-only rule).
#[test]
fn ctrl_d_does_not_activate_text() {
    let mut app = App::default();
    let ctrl = Modifiers {
        ctrl: true,
        command: true,
        ..Modifiers::NONE
    };
    dispatch_shortcuts(Key::D, ctrl, false, &mut app);
    assert_eq!(
        app.tool_manager.active_tool_name(),
        "Select",
        "Ctrl+D must not activate TextTool"
    );
}

/// AC#10 — `Shift+D` must not activate TEXT (bare-key-only rule).
#[test]
fn shift_d_does_not_activate_text() {
    let mut app = App::default();
    let shift = Modifiers {
        shift: true,
        ..Modifiers::NONE
    };
    dispatch_shortcuts(Key::D, shift, false, &mut app);
    assert_eq!(
        app.tool_manager.active_tool_name(),
        "Select",
        "Shift+D must not activate TextTool"
    );
}
