//! Integration tests for LCV-070 — keyboard shortcuts (tool activation hotkeys).
//!
//! These tests call `dispatch_shortcuts` directly with synthetic values so
//! they do not require a running egui frame (no `egui::Context` needed).

use egui::{Key, Modifiers};
use lasercad::app::{App, suppress_snap_if_disabled};
use lasercad::document::CreateLine;
use lasercad::geometry::{Line, SnapKind, SnapResult, Vec2};
use lasercad::ui::shortcuts::dispatch_shortcuts;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn no_mod() -> Modifiers {
    Modifiers::NONE
}

fn ctrl_mod() -> Modifiers {
    Modifiers {
        ctrl: true,
        command: true,
        ..Modifiers::NONE
    }
}

fn shift_mod() -> Modifiers {
    Modifiers {
        shift: true,
        ..Modifiers::NONE
    }
}

fn make_line_app() -> App {
    let mut app = App::default();
    let line = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
    app.commit(Box::new(CreateLine::new(line)));
    app
}

// ---------------------------------------------------------------------------
// AC#1 / AC#2 — module compiles and re-export resolves
// ---------------------------------------------------------------------------

/// AC#1, AC#2 — `use lasercad::ui::process_shortcuts` resolves at compile time.
#[test]
fn shortcuts_module_compiles() {
    use lasercad::ui::process_shortcuts as _ps;
    let _ = _ps as *const () as usize; // force monomorphisation check
}

// ---------------------------------------------------------------------------
// AC#4 — App::default() fields
// ---------------------------------------------------------------------------

/// AC#4 — `App::default()` has `snap_enabled == true` and `grid_enabled == true`.
#[test]
fn app_default_snap_and_grid_enabled() {
    let app = App::default();
    assert!(app.snap_enabled, "snap_enabled must default to true");
    assert!(app.grid_enabled, "grid_enabled must default to true");
}

// ---------------------------------------------------------------------------
// AC#5 — tool activation keys
// ---------------------------------------------------------------------------

#[test]
fn tool_key_l_activates_line_tool() {
    let mut app = App::default();
    dispatch_shortcuts(Key::L, no_mod(), false, &mut app);
    assert_eq!(app.tool_manager.active_tool_name(), "LINE");
}

#[test]
fn tool_key_p_activates_polyline_tool() {
    let mut app = App::default();
    dispatch_shortcuts(Key::P, no_mod(), false, &mut app);
    assert_eq!(app.tool_manager.active_tool_name(), "PLINE");
}

#[test]
fn tool_key_r_activates_rect_tool() {
    let mut app = App::default();
    dispatch_shortcuts(Key::R, no_mod(), false, &mut app);
    assert_eq!(app.tool_manager.active_tool_name(), "RECT");
}

#[test]
fn tool_key_c_activates_circle_tool() {
    let mut app = App::default();
    dispatch_shortcuts(Key::C, no_mod(), false, &mut app);
    assert_eq!(app.tool_manager.active_tool_name(), "CIRCLE");
}

#[test]
fn tool_key_a_activates_arc_tool() {
    let mut app = App::default();
    dispatch_shortcuts(Key::A, no_mod(), false, &mut app);
    assert_eq!(app.tool_manager.active_tool_name(), "ARC");
}

#[test]
fn tool_key_m_activates_move_tool() {
    let mut app = App::default();
    dispatch_shortcuts(Key::M, no_mod(), false, &mut app);
    assert_eq!(app.tool_manager.active_tool_name(), "MOVE");
}

#[test]
fn tool_key_e_activates_delete_tool() {
    let mut app = App::default();
    dispatch_shortcuts(Key::E, no_mod(), false, &mut app);
    assert_eq!(app.tool_manager.active_tool_name(), "ERASE");
}

#[test]
fn tool_key_t_activates_trim_tool() {
    let mut app = App::default();
    dispatch_shortcuts(Key::T, no_mod(), false, &mut app);
    assert_eq!(app.tool_manager.active_tool_name(), "TRIM");
}

#[test]
fn tool_key_x_activates_extend_tool() {
    let mut app = App::default();
    dispatch_shortcuts(Key::X, no_mod(), false, &mut app);
    assert_eq!(app.tool_manager.active_tool_name(), "EXTEND");
}

// ---------------------------------------------------------------------------
// AC#6 — blocked when wants_keyboard_input == true
// ---------------------------------------------------------------------------

/// AC#6 — tool key must NOT activate when a text widget has keyboard focus.
#[test]
fn tool_key_blocked_when_wants_keyboard_input() {
    let mut app = App::default();
    dispatch_shortcuts(Key::L, no_mod(), true /* wants_kbd */, &mut app);
    assert_eq!(
        app.tool_manager.active_tool_name(),
        "Select",
        "tool must not change when wants_keyboard_input is true"
    );
}

// ---------------------------------------------------------------------------
// AC#7 — blocked with any modifier held
// ---------------------------------------------------------------------------

/// AC#7 — `Ctrl+L` must NOT activate the line tool.
#[test]
fn tool_key_blocked_with_ctrl_modifier() {
    let mut app = App::default();
    dispatch_shortcuts(Key::L, ctrl_mod(), false, &mut app);
    assert_eq!(
        app.tool_manager.active_tool_name(),
        "Select",
        "Ctrl+L must not activate LineTool"
    );
}

/// AC#7 — `Shift+L` must NOT activate the line tool.
#[test]
fn tool_key_blocked_with_shift_modifier() {
    let mut app = App::default();
    dispatch_shortcuts(Key::L, shift_mod(), false, &mut app);
    assert_eq!(
        app.tool_manager.active_tool_name(),
        "Select",
        "Shift+L must not activate LineTool"
    );
}

// ---------------------------------------------------------------------------
// AC#8 — Ctrl+Z undo
// ---------------------------------------------------------------------------

/// AC#8 — `Ctrl+Z` undoes the last commit.
#[test]
fn ctrl_z_calls_undo() {
    let mut app = make_line_app();
    assert_eq!(app.document.entity_count(), 1);
    dispatch_shortcuts(Key::Z, ctrl_mod(), false, &mut app);
    assert_eq!(
        app.document.entity_count(),
        0,
        "entity should be removed after undo"
    );
    assert!(
        app.history.can_redo(),
        "redo should be available after undo"
    );
}

/// AC#8 — `Ctrl+Z` on an empty history is a no-op.
#[test]
fn ctrl_z_noop_when_empty_history() {
    let mut app = App::default();
    assert!(!app.history.can_undo());
    dispatch_shortcuts(Key::Z, ctrl_mod(), false, &mut app);
    assert_eq!(app.document.entity_count(), 0);
    assert!(!app.history.can_undo());
}

// ---------------------------------------------------------------------------
// AC#9 — Ctrl+Y redo
// ---------------------------------------------------------------------------

/// AC#9 — `Ctrl+Y` redoes after an undo.
#[test]
fn ctrl_y_calls_redo() {
    let mut app = make_line_app();
    app.history.undo(&mut app.document);
    assert_eq!(app.document.entity_count(), 0);
    dispatch_shortcuts(Key::Y, ctrl_mod(), false, &mut app);
    assert_eq!(
        app.document.entity_count(),
        1,
        "entity should be restored after redo"
    );
}

// ---------------------------------------------------------------------------
// AC#10 — Ctrl+Z fires even when wants_keyboard_input == true
// ---------------------------------------------------------------------------

/// AC#10 — `Ctrl+Z` fires unconditionally regardless of `wants_keyboard_input`.
#[test]
fn ctrl_z_fires_when_wants_keyboard_input() {
    let mut app = make_line_app();
    assert_eq!(app.document.entity_count(), 1);
    dispatch_shortcuts(Key::Z, ctrl_mod(), true /* wants_kbd */, &mut app);
    assert_eq!(
        app.document.entity_count(),
        0,
        "undo must fire even when a text widget has keyboard focus"
    );
}

// ---------------------------------------------------------------------------
// AC#12 — F8 toggles ortho_enabled
// ---------------------------------------------------------------------------

/// AC#12 — `F8` toggles `ortho_enabled`.
#[test]
fn f8_toggles_ortho_enabled() {
    let mut app = App::default();
    assert!(!app.ortho_enabled);
    dispatch_shortcuts(Key::F8, no_mod(), false, &mut app);
    assert!(app.ortho_enabled, "F8 should enable ortho");
    dispatch_shortcuts(Key::F8, no_mod(), false, &mut app);
    assert!(!app.ortho_enabled, "F8 again should disable ortho");
}

// ---------------------------------------------------------------------------
// AC#13 — F3 toggles snap_enabled
// ---------------------------------------------------------------------------

/// AC#13 — `F3` toggles `snap_enabled`.
#[test]
fn f3_toggles_snap_enabled() {
    let mut app = App::default();
    assert!(app.snap_enabled);
    dispatch_shortcuts(Key::F3, no_mod(), false, &mut app);
    assert!(!app.snap_enabled, "F3 should disable snap");
    dispatch_shortcuts(Key::F3, no_mod(), false, &mut app);
    assert!(app.snap_enabled, "F3 again should re-enable snap");
}

// ---------------------------------------------------------------------------
// AC#14 — F7 toggles grid_enabled
// ---------------------------------------------------------------------------

/// AC#14 — `F7` toggles `grid_enabled`.
#[test]
fn f7_toggles_grid_enabled() {
    let mut app = App::default();
    assert!(app.grid_enabled);
    dispatch_shortcuts(Key::F7, no_mod(), false, &mut app);
    assert!(!app.grid_enabled, "F7 should disable grid");
    dispatch_shortcuts(Key::F7, no_mod(), false, &mut app);
    assert!(app.grid_enabled, "F7 again should re-enable grid");
}

// ---------------------------------------------------------------------------
// AC#15 — toggle keys fire unconditionally (even when wants_keyboard_input)
// ---------------------------------------------------------------------------

/// AC#15 — `F8` fires even when a text widget has keyboard focus.
#[test]
fn f8_fires_when_wants_keyboard_input() {
    let mut app = App::default();
    dispatch_shortcuts(Key::F8, no_mod(), true /* wants_kbd */, &mut app);
    assert!(
        app.ortho_enabled,
        "F8 must fire regardless of wants_keyboard_input"
    );
}

// ---------------------------------------------------------------------------
// AC#16 — suppress_snap_if_disabled helper
// ---------------------------------------------------------------------------

/// AC#16 — `suppress_snap_if_disabled` clears `active_snap` when disabled and
/// leaves it intact when enabled.
#[test]
fn snap_disabled_clears_active_snap() {
    let dummy_snap = Some(SnapResult {
        point: Vec2::new(5.0, 5.0),
        kind: SnapKind::Endpoint,
        primary_idx: 0,
        secondary_idx: None,
    });

    let mut active = dummy_snap;
    suppress_snap_if_disabled(false, &mut active);
    assert!(
        active.is_none(),
        "active_snap must be None when snap_enabled=false"
    );

    let mut active2 = dummy_snap;
    suppress_snap_if_disabled(true, &mut active2);
    assert!(
        active2.is_some(),
        "active_snap must be preserved when snap_enabled=true"
    );
}
