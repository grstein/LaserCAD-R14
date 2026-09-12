//! LCV-103 — the single keyboard/text gate (ADR 0002 §A6).
//!
//! [`process_input`] owns every key or text route that is **not** a
//! [`dispatch_shortcuts`](crate::ui::shortcuts::dispatch_shortcuts) entry:
//! `Escape` / `Enter` / `Delete` / `Backspace` to the active tool, the
//! `F` / `Ctrl+0` zoom-extents action, and the `Event::Text` forward into
//! [`ToolManager::on_text_input`](crate::tools::ToolManager::on_text_input).
//!
//! Together with `src/ui/shortcuts.rs` these are the only two readers of key
//! presses in the app. Nothing inside the `CentralPanel` closure, and nothing
//! inside a widget, may read a key event: a second reader is a double
//! dispatch (defect D4), and a reader without the focus check below is defect
//! D8. The gate table is the contract:
//!
//! | class | keys | fires while a text widget has focus |
//! |---|---|---|
//! | global commands | `Ctrl+Z/Y/N/O/S`, `Ctrl+Shift+S` | yes (`shortcuts.rs`) |
//! | view toggles | `F3`, `F7`, `F8` | yes (`shortcuts.rs`) |
//! | cancel | `Escape` | yes (here) |
//! | view actions | `F`, `Ctrl+0` | no (here) |
//! | tool activation | `L P R C A M E T X` | no (`shortcuts.rs`) |
//! | tool key routing | `Enter`, `Delete`, `Backspace` | no (here) |
//! | typed characters | `Event::Text` | no (here) |
//!
//! MUST NOT import `eframe` or `rfd`.

use crate::app::{handle_zoom_extents, App};

/// Keys forwarded to the active tool once the gate lets them through.
///
/// `Enter` is gated on purpose: `src/ui/command_line.rs` submits the command
/// line on Enter, so an ungated route would submit the command line *and*
/// commit the active tool on a single press.
const TOOL_ROUTED_KEYS: [egui::Key; 3] =
    [egui::Key::Enter, egui::Key::Delete, egui::Key::Backspace];

/// Route the frame's keyboard and text input behind one focus check.
///
/// Called once per frame from [`App::update_ui`], immediately after
/// [`crate::ui::process_shortcuts`] and before any panel is rendered. Running
/// before the panels is what lets `Escape` cancel the active tool in the same
/// frame the command-line widget clears its own buffer.
///
/// `ctx.wants_keyboard_input()` read here reports the focus established during
/// the **previous** frame — that one-frame lag is egui's documented behaviour,
/// not a bug (ADR 0002).
pub fn process_input(ctx: &egui::Context, app: &mut App) {
    let wants_kbd = ctx.wants_keyboard_input();

    // Ungated: Escape cancels the active tool even while the operator is
    // typing into a text field.
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        route_to_tool(app, egui::Key::Escape);
    }

    if wants_kbd {
        return;
    }

    for key in TOOL_ROUTED_KEYS {
        if ctx.input(|i| i.key_pressed(key)) {
            route_to_tool(app, key);
        }
    }

    // Zoom extents. Reads the viewport size synced by the previous frame's
    // `CentralPanel`; `handle_zoom_extents` no-ops on the zero-area sentinel
    // a `Camera::default()` carries before the first frame is painted.
    let zoom_extents = ctx.input(|i| {
        i.key_pressed(egui::Key::F) || (i.modifiers.ctrl && i.key_pressed(egui::Key::Num0))
    });
    if zoom_extents {
        let viewport_size = app.camera.viewport_size_px;
        handle_zoom_extents(&mut app.camera, &app.document, viewport_size);
    }

    // Typed characters → active tool (LCV-048 TextTool).
    for ch in typed_chars(ctx) {
        app.tool_manager.on_text_input(ch);
    }
}

/// Hand `key` to the active tool.
///
/// `Tool::on_key` takes `&mut App`, so the manager is moved out of `app` for
/// the duration of the call and moved back afterwards.
fn route_to_tool(app: &mut App, key: egui::Key) {
    let mut tm = std::mem::take(&mut app.tool_manager);
    tm.handle_key(key, app);
    app.tool_manager = tm;
}

/// Collect the characters carried by this frame's `Event::Text` events.
fn typed_chars(ctx: &egui::Context) -> Vec<char> {
    ctx.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                egui::Event::Text(t) => Some(t.chars().collect::<Vec<char>>()),
                _ => None,
            })
            .flatten()
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::TextTool;

    /// LCV-103 — `route_to_tool` reaches the active tool and leaves the
    /// manager back in place afterwards (the `mem::take` round-trip).
    #[test]
    fn route_to_tool_restores_the_manager() {
        let mut app = App::default();
        app.tool_manager.set_tool(Box::new(TextTool::default()));
        route_to_tool(&mut app, egui::Key::Escape);
        assert_eq!(app.tool_manager.active_tool_name(), "TEXT");
    }

    /// LCV-103 — the gate routes exactly three keys to the active tool on top
    /// of the ungated `Escape`.
    #[test]
    fn tool_routed_keys_are_enter_delete_backspace() {
        assert_eq!(
            TOOL_ROUTED_KEYS,
            [egui::Key::Enter, egui::Key::Delete, egui::Key::Backspace]
        );
    }
}
