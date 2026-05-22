//! Keyboard shortcut dispatch for the main application loop.
//!
//! [`process_shortcuts`] is called once per frame at the very top of
//! [`crate::app::App::update`] before any panel is rendered. It collects
//! key-press events from the egui [`Context`] and delegates to
//! [`dispatch_shortcuts`], which contains the pure, testable logic and may
//! be called directly from unit tests with synthetic values.

use egui::{Key, Modifiers};

use crate::app::App;
use crate::tools::{
    ArcTool, CircleTool, DeleteTool, ExtendTool, LineTool, MoveTool, PolylineTool, RectTool, Tool,
    TrimTool,
};

/// Called once per frame — the first statement of `App::update` — before any
/// panel is rendered. Reads key-press events (excluding repeats) from `ctx`
/// and calls [`dispatch_shortcuts`] for each one.
pub fn process_shortcuts(ctx: &egui::Context, app: &mut App) {
    let wants_kbd = ctx.wants_keyboard_input();
    let key_events: Vec<(Key, Modifiers)> = ctx.input(|i| {
        i.events
            .iter()
            .filter_map(|e| {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } = e
                {
                    Some((*key, *modifiers))
                } else {
                    None
                }
            })
            .collect()
    });
    for (key, mods) in key_events {
        dispatch_shortcuts(key, mods, wants_kbd, app);
    }
}

/// Tool-activation dispatch table: `(key, constructor)` pairs iterated in a
/// loop inside [`dispatch_shortcuts`].
type ToolCtor = fn() -> Box<dyn Tool>;
const TOOL_KEYS: &[(Key, ToolCtor)] = &[
    (Key::L, || Box::new(LineTool::default())),
    (Key::P, || Box::new(PolylineTool::default())),
    (Key::R, || Box::new(RectTool::default())),
    (Key::C, || Box::new(CircleTool::default())),
    (Key::A, || Box::new(ArcTool::default())),
    (Key::M, || Box::new(MoveTool::default())),
    (Key::E, || Box::new(DeleteTool)),
    (Key::T, || Box::new(TrimTool)),
    (Key::X, || Box::new(ExtendTool::default())),
];

/// Inner, testable dispatch. Called from [`process_shortcuts`]; also called
/// directly from unit tests with synthetic `key`, `modifiers`, and
/// `wants_kbd` values.
///
/// - Single-letter tool keys are suppressed when `wants_kbd` is `true` or any
///   modifier is held.
/// - `Ctrl+Z/Y/N/O/S` and F3 / F7 / F8 fire unconditionally.
pub fn dispatch_shortcuts(key: Key, modifiers: Modifiers, wants_kbd: bool, app: &mut App) {
    let bare = modifiers.is_none();
    let ctrl_only = modifiers.command_only();

    // Ctrl+Z / Ctrl+Y — undo / redo (unconditional; fires even with kbd focus).
    // Ctrl+N / Ctrl+O / Ctrl+S — file operations (LCV-062).
    if ctrl_only {
        match key {
            Key::Z => {
                app.history.undo(&mut app.document);
                return;
            }
            Key::Y => {
                app.history.redo(&mut app.document);
                return;
            }
            Key::N => {
                app.action_new();
                return;
            }
            Key::O => {
                app.action_open();
                return;
            }
            Key::S => {
                app.action_save();
                return;
            }
            _ => {}
        }
    }

    // Ctrl+Shift+S — Save As (LCV-062).
    if modifiers.command && modifiers.shift && !modifiers.alt && key == Key::S {
        crate::io::action_save_as(app);
        return;
    }

    // Toggle keys — bare, unconditional (fire even with kbd focus).
    if bare {
        match key {
            Key::F8 => {
                app.ortho_enabled = !app.ortho_enabled;
                return;
            }
            Key::F3 => {
                app.snap_enabled = !app.snap_enabled;
                return;
            }
            Key::F7 => {
                app.grid_enabled = !app.grid_enabled;
                return;
            }
            _ => {}
        }
    }

    // Tool activation — bare key only, suppressed when a text widget has focus.
    if bare && !wants_kbd {
        for &(tool_key, ctor) in TOOL_KEYS {
            if key == tool_key {
                app.tool_manager.set_tool(ctor());
                return;
            }
        }
    }
}
