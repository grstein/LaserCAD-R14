//! Keyboard shortcut dispatch for the main application loop.
//!
//! [`process_shortcuts`] is called once per frame at the very top of
//! [`crate::app::App::update`] before any panel is rendered. It collects
//! key-press events from the egui [`Context`] and delegates to
//! [`dispatch_shortcuts`], which contains the pure, testable logic and may
//! be called directly from unit tests with synthetic values.

use egui::{Key, Modifiers};

use crate::app::App;
use crate::cmdline::ToolKind;
use crate::tools;

/// Called once per frame — the first statement of `App::update` — before any
/// panel is rendered. Reads key-press events (excluding repeats) from `ctx`
/// and calls [`dispatch_shortcuts`] for each one.
///
/// Returns the frame's shortcut flag: `true` when **any** key this frame was
/// consumed by a shortcut. `src/app/input.rs` uses it to drop the matching
/// `Event::Text`, so pressing `l` starts LINE instead of also typing an `l`
/// into the command line (LCV-111 AC 22).
pub fn process_shortcuts(ctx: &egui::Context, app: &mut App) -> bool {
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
    let mut fired = false;
    for (key, mods) in key_events {
        fired |= dispatch_shortcuts(key, mods, wants_kbd, app);
    }
    fired
}

/// Tool-activation dispatch table: `(key, kind)` pairs iterated in a loop
/// inside [`dispatch_shortcuts`]. `kind` is resolved to an instance through
/// [`tools::make`] — the single tool-identity map (ADR 0003 §A3).
///
/// `D` (LCV-104) activates `TextTool` — the AutoCAD R14 `DTEXT` mnemonic.
/// `T`, `X`, `E`, `M`, `R` were already taken (TRIM, EXTEND, ERASE, MOVE,
/// RECT), so `D` is the only free letter; see the demand Notes for the full
/// rejection rationale. This is a settled product decision, not an option.
const TOOL_KEYS: &[(Key, ToolKind)] = &[
    (Key::L, ToolKind::Line),
    (Key::P, ToolKind::Polyline),
    (Key::R, ToolKind::Rect),
    (Key::C, ToolKind::Circle),
    (Key::A, ToolKind::Arc),
    (Key::M, ToolKind::Move),
    (Key::E, ToolKind::Delete),
    (Key::T, ToolKind::Trim),
    (Key::X, ToolKind::Extend),
    (Key::D, ToolKind::Text),
];

/// Inner, testable dispatch. Called from [`process_shortcuts`]; also called
/// directly from unit tests with synthetic `key`, `modifiers`, and
/// `wants_kbd` values.
///
/// - Single-letter tool keys are suppressed when `wants_kbd` is `true` or any
///   modifier is held.
/// - `Ctrl+Z/Y/N/O/S` and F3 / F7 / F8 fire unconditionally.
///
/// Returns `true` when this key was consumed by a shortcut (LCV-111 AC 22).
/// Deliberately **not** `#[must_use]`: the LCV-070 and LCV-104 suites call it
/// as a bare statement and must keep compiling unchanged.
pub fn dispatch_shortcuts(key: Key, modifiers: Modifiers, wants_kbd: bool, app: &mut App) -> bool {
    let bare = modifiers.is_none();
    let ctrl_only = modifiers.command_only();

    // Ctrl+Z / Ctrl+Y — undo / redo (unconditional; fires even with kbd focus).
    // Ctrl+N / Ctrl+O / Ctrl+S — file operations (LCV-062).
    if ctrl_only {
        match key {
            Key::Z => {
                app.history.undo(&mut app.document);
                return true;
            }
            Key::Y => {
                app.history.redo(&mut app.document);
                return true;
            }
            Key::N => {
                app.action_new();
                return true;
            }
            Key::O => {
                app.action_open();
                return true;
            }
            Key::S => {
                app.action_save();
                return true;
            }
            _ => {}
        }
    }

    // Ctrl+Shift+S — Save As (LCV-062).
    if modifiers.command && modifiers.shift && !modifiers.alt && key == Key::S {
        crate::io::action_save_as(app);
        return true;
    }

    // Toggle keys — bare, unconditional (fire even with kbd focus).
    if bare {
        match key {
            Key::F8 => {
                app.ortho_enabled = !app.ortho_enabled;
                return true;
            }
            Key::F3 => {
                app.snap_enabled = !app.snap_enabled;
                return true;
            }
            Key::F7 => {
                app.grid_enabled = !app.grid_enabled;
                return true;
            }
            _ => {}
        }
    }

    // Tool activation — bare key only, suppressed when a text widget has focus.
    if bare && !wants_kbd {
        for &(tool_key, kind) in TOOL_KEYS {
            if key == tool_key {
                app.tool_manager.set_tool(tools::make(kind));
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmdline::{parse, CommandInput};
    use crate::ui::toolbar::TOOLS;
    use std::collections::HashSet;

    /// LCV-104 AC#11 — `TOOL_KEYS` contains exactly `{L, P, R, C, A, M, E, T,
    /// X, D}`, no duplicate key, and every entry's constructed tool matches
    /// the toolbar entry advertising the same shortcut letter.
    #[test]
    fn tool_keys_are_unique_and_match_toolbar_shortcuts() {
        let mut seen_keys = HashSet::new();
        for &(key, kind) in TOOL_KEYS {
            let letter = format!("{key:?}");
            assert!(
                seen_keys.insert(letter.clone()),
                "duplicate TOOL_KEYS key: {letter}"
            );

            let entry = TOOLS
                .iter()
                .find(|e| e.shortcut == Some(letter.as_str()))
                .unwrap_or_else(|| panic!("no toolbar entry advertises shortcut {letter}"));
            assert_eq!(
                tools::make(kind).name(),
                entry.tool_name,
                "TOOL_KEYS[{letter}] constructs the wrong tool"
            );
        }

        let expected: HashSet<String> = ["L", "P", "R", "C", "A", "M", "E", "T", "X", "D"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        assert_eq!(seen_keys, expected);
    }

    /// LCV-110 AC 13 — every `TOOL_KEYS` letter that also has a command-line
    /// alias resolves through `cmdline::parse` to the **same** `ToolKind` as
    /// the keyboard table. `X` and `D` have no command-line alias in this
    /// demand (§Out of scope) and are skipped. This is the test that pins
    /// product decision 2: making `e` mean `Extend` again fails the build.
    #[test]
    fn alias_table_agrees_with_tool_keys() {
        for &(key, kind) in TOOL_KEYS {
            let letter = format!("{key:?}");
            if letter == "X" || letter == "D" {
                continue;
            }
            let lower = letter.to_lowercase();
            match parse(&lower) {
                CommandInput::Tool(parsed) => assert_eq!(
                    parsed, kind,
                    "alias \"{lower}\" must resolve to the same ToolKind as TOOL_KEYS[{letter}]"
                ),
                other => panic!("expected \"{lower}\" to parse as Tool(_), got {other:?}"),
            }
        }
    }
}
