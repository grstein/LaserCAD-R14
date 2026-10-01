//! LCV-103 — the single keyboard/text gate (ADR 0002 §A6).
//!
//! [`process_input`] owns every key or text route that is **not** a
//! [`dispatch_shortcuts`](crate::ui::shortcuts::dispatch_shortcuts) entry:
//! `Escape` / `Enter` / `Delete` / `Backspace` to the active tool, the
//! `F` / `Ctrl+0` zoom-extents action, `ArrowUp` / `ArrowDown` command recall,
//! and the `Event::Text` seed that focuses the command line (LCV-111).
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
//! | help | `F1` | yes (`shortcuts.rs`) |
//! | cancel | `Escape` | yes (here) |
//! | view actions | `F`, `Ctrl+0` | no (here) |
//! | tool activation | `L P R C A M E T X D` | no (`shortcuts.rs`) |
//! | tool key routing | `Enter`, `Delete`, `Backspace` | no (here) |
//! | command recall | `ArrowUp`, `ArrowDown` | only while the command line has focus (here) |
//! | typed characters → seed + focus the command line | `Event::Text` | no (here) |
//!
//! MUST NOT import `eframe` or `rfd`.

use crate::app::{App, handle_zoom_extents};

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
///
/// `shortcut_fired` is the frame flag folded by
/// [`crate::ui::process_shortcuts`]: `true` means a shortcut already consumed
/// a key this frame. A real keyboard emits `Event::Key` **and** `Event::Text`
/// for one keystroke, so without this flag `l` would start LINE *and* type an
/// `l` into the command line (LCV-111 AC 22).
pub fn process_input(ctx: &egui::Context, app: &mut App, shortcut_fired: bool) {
    let wants_kbd = ctx.wants_keyboard_input();

    // Ungated: Escape cancels the active tool even while the operator is
    // typing into a text field.
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        route_to_tool(app, egui::Key::Escape);
    }

    // Command recall, read *before* the focus early-out because it only acts
    // while the command line has focus (LCV-111 AC 23). egui 0.29.1's
    // single-line `TextEdit` leaves its buffer alone on arrow keys, so the
    // field can be replaced from here without taking the event away from it.
    if app.command_line_focused {
        recall(ctx, app);
    }

    if wants_kbd {
        return;
    }

    for key in TOOL_ROUTED_KEYS {
        if ctx.input(|i| i.key_pressed(key)) {
            // An unfocused Enter is Enter on an empty line: the same body as
            // the field's empty submit, so it repeats at rest (LCV-165 AC 4).
            match key {
                egui::Key::Enter => super::cmdline::empty_enter(app),
                _ => route_to_tool(app, key),
            }
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

    // Typed characters → seed and focus the command line (LCV-111 AC 21).
    // Reached only when no text widget had focus and no shortcut fired, so
    // the character lands exactly once: the widget could not have consumed
    // this frame's text, having had no focus while it ran.
    if !shortcut_fired {
        let typed: String = typed_chars(ctx).into_iter().collect();
        if !typed.is_empty() {
            app.command_line_input.push_str(&typed);
            app.focus_command_line = true;
        }
    }
}

/// Walk the recall ring with `ArrowUp` / `ArrowDown` (LCV-111 AC 23).
///
/// Up replaces the field with the previous entry when the ring has one. Down
/// replaces it with the next entry, and clears the field when there is no
/// newer one — `CommandHistory::newer` returns `None` both at the newest entry
/// and when no recall is in progress, and clearing an already-empty field in
/// the second case is indistinguishable to the operator.
fn recall(ctx: &egui::Context, app: &mut App) {
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp))
        && let Some(entry) = app.command_history.older()
    {
        app.command_line_input = entry;
    }
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
        app.command_line_input = app.command_history.newer().unwrap_or_default();
    }
}

/// Hand `key` to the active tool.
///
/// `Tool::on_key` takes `&mut App`, so the manager is moved out of `app` for
/// the duration of the call and moved back afterwards.
pub(super) fn route_to_tool(app: &mut App, key: egui::Key) {
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
