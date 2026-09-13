//! In-app modal dialogs: confirm, error, about, and keyboard shortcuts.
//!
//! All dialogs are pure egui floating windows — no OS dialog, no extra crate.
//! They are stateless helpers; the caller owns any open/visible `bool` flag.
//!
//! Every dialog here is **read-only with respect to the keyboard**: none of
//! them reads a key event. `src/ui/shortcuts.rs` and `src/app/input.rs` are
//! the only two key readers in the app (ADR 0002 §A6), and the × button is
//! egui's own [`Window::open`] flag, not a key binding of ours.

use egui::{Align2, Context, Window};

use crate::ui::toolbar::TOOLS;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// The definitive answer returned by a confirmation dialog.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DialogResult {
    /// The user clicked the confirm button (`confirm_label`, e.g. `"Yes"` or
    /// `"Discard"` — the label is caller-supplied, LCV-113).
    Confirmed,
    /// The user clicked the cancel button (`cancel_label`, e.g. `"No"` or
    /// `"Cancel"` — the label is caller-supplied, LCV-113).
    Cancelled,
}

// ---------------------------------------------------------------------------
// Dialog functions
// ---------------------------------------------------------------------------

/// Render a modal-style confirmation window centered in the viewport.
///
/// The window is not resizable, not collapsible, and has no × close button.
/// The body shows `message` and two buttons, `confirm_label` then
/// `cancel_label`, in that order in a horizontal row (LCV-113: the labels
/// were hard-coded `"Yes"` / `"No"` until the discard-confirmation dialog
/// needed `"Discard"` / `"Cancel"`).
///
/// Returns:
/// - `Some(DialogResult::Confirmed)` on the frame `confirm_label` is clicked.
/// - `Some(DialogResult::Cancelled)` on the frame `cancel_label` is clicked.
/// - `None` every other frame.
///
/// The caller is responsible for holding a `bool` flag and stopping the
/// call once a `Some` result is received.
pub fn confirm_dialog(
    ctx: &Context,
    title: &str,
    message: &str,
    confirm_label: &str,
    cancel_label: &str,
) -> Option<DialogResult> {
    let mut result = None;

    Window::new(title)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(message);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(confirm_label).clicked() {
                    result = Some(DialogResult::Confirmed);
                }
                if ui.button(cancel_label).clicked() {
                    result = Some(DialogResult::Cancelled);
                }
            });
        });

    result
}

/// Render a modal-style error window centered in the viewport.
///
/// The window is not resizable, not collapsible, and has no × close button.
/// The body shows `message` and a single **OK** button.
///
/// Returns `true` on the frame **OK** is clicked, `false` every other frame.
///
/// The caller is responsible for holding a `bool` flag and stopping the
/// call once `true` is returned.
pub fn error_dialog(ctx: &Context, title: &str, message: &str) -> bool {
    let mut clicked = false;

    Window::new(title)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(message);
            ui.add_space(8.0);
            if ui.button("OK").clicked() {
                clicked = true;
            }
        });

    clicked
}

/// Render the About dialog.
///
/// The window is opened/closed via `open`; egui's built-in × button sets
/// `*open = false`. The body shows the application name, the crate version
/// from `Cargo.toml`, and the license identifier.
///
/// Pass `open: &mut bool` from `App`; the Help → About menu item sets it to
/// `true` (wired by LCV-065).
pub fn about_dialog(ctx: &Context, open: &mut bool) {
    Window::new("About LaserCAD")
        .open(open)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label("LaserCAD v2");
            ui.label(env!("CARGO_PKG_VERSION"));
            ui.label("MIT OR Apache-2.0");
        });
}

/// One group of bindings in the keyboard-shortcuts dialog.
pub(crate) struct ShortcutGroup {
    /// Heading rendered above the group.
    pub(crate) heading: &'static str,
    /// `(binding, description)` rows, in display order.
    pub(crate) rows: &'static [(&'static str, &'static str)],
}

/// The bindings that are **not** tool activations (LCV-116 AC 15).
///
/// Every row here is a real binding, verified against the only two files that
/// read keys: `src/ui/shortcuts.rs` (the Ctrl combinations and `F3` / `F7` /
/// `F8` / `F1`) and `src/app/input.rs` (`Esc`, `Enter`, `Delete`,
/// `Backspace`, `F`, `Ctrl+0`, `ArrowUp` / `ArrowDown` command recall, and the
/// typed-character seed). Nothing aspirational goes in this table — the
/// dialog documents what exists, it does not propose.
///
/// The tool letters are deliberately absent: they are generated from
/// [`TOOLS`] by [`tool_rows`], so adding or removing a tool changes the dialog
/// with no edit here (AC 14).
pub(crate) const SHORTCUT_GROUPS: &[ShortcutGroup] = &[
    ShortcutGroup {
        heading: "File",
        rows: &[
            ("Ctrl+N", "New"),
            ("Ctrl+O", "Open"),
            ("Ctrl+S", "Save"),
            ("Ctrl+Shift+S", "Save As"),
        ],
    },
    ShortcutGroup {
        heading: "Edit",
        rows: &[("Ctrl+Z", "Undo"), ("Ctrl+Y", "Redo")],
    },
    ShortcutGroup {
        heading: "View",
        rows: &[("F", "Zoom extents"), ("Ctrl+0", "Zoom extents")],
    },
    ShortcutGroup {
        heading: "Modes",
        rows: &[("F3", "Snap"), ("F7", "Grid"), ("F8", "Ortho")],
    },
    ShortcutGroup {
        heading: "Drawing",
        rows: &[
            ("Enter", "Accept / finish the current tool step"),
            ("Esc", "Cancel the current operation"),
            ("Delete / Backspace", "Sent to the active tool"),
        ],
    },
    ShortcutGroup {
        heading: "Command line",
        rows: &[
            ("Any other character", "Start a command in the command line"),
            ("ArrowUp", "Previous command (command line focused)"),
            ("ArrowDown", "Next command (command line focused)"),
        ],
    },
    ShortcutGroup {
        heading: "Help",
        rows: &[("F1", "This dialog")],
    },
];

/// The tool-activation rows, derived from [`TOOLS`] (LCV-116 AC 14).
///
/// One row per entry that has a `shortcut`; `Select` has no binding and is
/// skipped. `TOOLS` is already the single source of truth for the toolbar and
/// the Tools menu (LCV-104) — a hand-typed third copy would drift on the first
/// tool change.
pub(crate) fn tool_rows() -> Vec<(&'static str, &'static str)> {
    TOOLS
        .iter()
        .filter_map(|entry| entry.shortcut.map(|key| (key, entry.label)))
        .collect()
}

/// Render the Keyboard shortcuts dialog (LCV-116 AC 11).
///
/// Follows [`about_dialog`]: opened and closed through `open`, so egui's own ×
/// sets `*open = false`; centre-anchored, not collapsible. The body sits in a
/// vertical [`egui::ScrollArea`] so every row is reachable on a short window.
///
/// Read-only: no widget in it changes application state, and it reads no key.
/// `F1` is dispatched in `src/ui/shortcuts.rs` like `F3` / `F7` / `F8`, not
/// here.
pub fn shortcuts_dialog(ctx: &Context, open: &mut bool) {
    Window::new("Keyboard shortcuts")
        .open(open)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Tools");
                for (binding, description) in tool_rows() {
                    shortcut_row(ui, binding, description);
                }
                for group in SHORTCUT_GROUPS {
                    ui.add_space(6.0);
                    ui.heading(group.heading);
                    for (binding, description) in group.rows {
                        shortcut_row(ui, binding, description);
                    }
                }
            });
        });
}

/// One `binding`/`description` row, drawn as a plain two-column line.
fn shortcut_row(ui: &mut egui::Ui, binding: &str, description: &str) {
    ui.horizontal(|ui| {
        ui.monospace(format!("{binding:<20}"));
        ui.label(description);
    });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// §1 — confirm_dialog returns None when no button is clicked.
    #[test]
    fn confirm_dialog_returns_none_without_click() {
        let ctx = egui::Context::default();
        let mut captured = None;
        let _out = ctx.run(egui::RawInput::default(), |ctx| {
            captured = confirm_dialog(ctx, "T", "M", "Yes", "No");
        });
        assert_eq!(captured, None);
    }

    /// LCV-113 AC 10 — the button labels are caller-supplied, not hard-coded
    /// `"Yes"` / `"No"`. Renders with the discard dialog's own labels and
    /// asserts only that nothing panics and no click means no result — the
    /// same shape as the test above, with different labels.
    #[test]
    fn confirm_dialog_renders_custom_labels() {
        let ctx = egui::Context::default();
        let mut captured = None;
        let _out = ctx.run(egui::RawInput::default(), |ctx| {
            captured = confirm_dialog(
                ctx,
                "Discard unsaved changes?",
                "The current drawing has unsaved changes.",
                "Discard",
                "Cancel",
            );
        });
        assert_eq!(captured, None);
    }

    /// §2 — error_dialog returns false when no button is clicked.
    #[test]
    fn error_dialog_returns_false_without_click() {
        let ctx = egui::Context::default();
        let mut captured = false;
        let _out = ctx.run(egui::RawInput::default(), |ctx| {
            captured = error_dialog(ctx, "E", "Msg");
        });
        assert!(!captured);
    }

    /// §3 — about_dialog leaves `open` true when no close event is simulated.
    #[test]
    fn about_dialog_leaves_open_true_without_close() {
        let ctx = egui::Context::default();
        let mut open = true;
        let _out = ctx.run(egui::RawInput::default(), |ctx| {
            about_dialog(ctx, &mut open);
        });
        assert!(open);
    }

    // ── LCV-116 (c) — the keyboard-shortcuts dialog ───────────────────────

    /// LCV-116 AC 11 — renders like `about_dialog`: `open` survives a frame in
    /// which nothing is clicked, and a closed dialog renders nothing and
    /// stays closed.
    #[test]
    fn shortcuts_dialog_leaves_open_true_without_close() {
        let ctx = egui::Context::default();
        let mut open = true;
        let _out = ctx.run(egui::RawInput::default(), |ctx| {
            shortcuts_dialog(ctx, &mut open);
        });
        assert!(open);

        let mut closed = false;
        let _out = ctx.run(egui::RawInput::default(), |ctx| {
            shortcuts_dialog(ctx, &mut closed);
        });
        assert!(!closed);
    }

    /// LCV-116 AC 14 — the tool rows are **derived** from `toolbar::TOOLS`,
    /// not typed: one row per entry with a `shortcut`, in table order, with
    /// the entry's own shortcut and label. The two length assertions are what
    /// make adding or removing a `ToolEntry` fail here.
    #[test]
    fn tool_rows_match_the_toolbar_table() {
        let rows = tool_rows();
        let with_shortcut: Vec<_> = TOOLS.iter().filter(|t| t.shortcut.is_some()).collect();

        assert_eq!(rows.len(), with_shortcut.len());
        assert_eq!(
            TOOLS.len(),
            11,
            "the tool table changed — re-check the shortcuts dialog"
        );
        assert_eq!(rows.len(), 10, "ten tools carry a keyboard shortcut");

        for (row, entry) in rows.iter().zip(with_shortcut.iter()) {
            assert_eq!(row.0, entry.shortcut.expect("filtered on is_some"));
            assert_eq!(row.1, entry.label);
        }
        assert!(
            !rows.iter().any(|r| r.1 == "Select"),
            "Select has no binding and must be skipped"
        );
    }

    /// LCV-116 AC 14 — the dialog owns no hand-typed tool list. Bounded to the
    /// implementation half of this file, with a positive control proving the
    /// scan is looking at a real slice.
    #[test]
    fn the_dialog_holds_no_second_copy_of_the_tool_table() {
        let implementation = implementation_source();
        assert!(
            implementation.contains("TOOLS\n        .iter()"),
            "positive control: the rows are derived from TOOLS"
        );
        for entry in TOOLS {
            if let Some(key) = entry.shortcut {
                let typed = format!("(\"{key}\", \"{}\")", entry.label);
                assert!(
                    !implementation.contains(&typed),
                    "{typed} is hand-typed; it must come from TOOLS"
                );
            }
        }
    }

    /// LCV-116 AC 15 — the static rows, by exact string, in the documented
    /// order. Deleting, renaming or reordering one fails here.
    #[test]
    fn static_rows_cover_every_documented_binding() {
        let expected: &[(&str, &str, &str)] = &[
            ("File", "Ctrl+N", "New"),
            ("File", "Ctrl+O", "Open"),
            ("File", "Ctrl+S", "Save"),
            ("File", "Ctrl+Shift+S", "Save As"),
            ("Edit", "Ctrl+Z", "Undo"),
            ("Edit", "Ctrl+Y", "Redo"),
            ("View", "F", "Zoom extents"),
            ("View", "Ctrl+0", "Zoom extents"),
            ("Modes", "F3", "Snap"),
            ("Modes", "F7", "Grid"),
            ("Modes", "F8", "Ortho"),
            ("Drawing", "Enter", "Accept / finish the current tool step"),
            ("Drawing", "Esc", "Cancel the current operation"),
            ("Drawing", "Delete / Backspace", "Sent to the active tool"),
            (
                "Command line",
                "Any other character",
                "Start a command in the command line",
            ),
            (
                "Command line",
                "ArrowUp",
                "Previous command (command line focused)",
            ),
            (
                "Command line",
                "ArrowDown",
                "Next command (command line focused)",
            ),
            ("Help", "F1", "This dialog"),
        ];
        let actual: Vec<(&str, &str, &str)> = SHORTCUT_GROUPS
            .iter()
            .flat_map(|g| g.rows.iter().map(move |r| (g.heading, r.0, r.1)))
            .collect();
        assert_eq!(actual, expected);
    }

    /// LCV-116 AC 15 — **no binding may appear that `grep` cannot find** in
    /// the two key-reading files. Every static row is mapped to the `Key::`
    /// identifier that implements it and looked up in
    /// `src/ui/shortcuts.rs` + `src/app/input.rs`; the negative control proves
    /// a fabricated binding would be rejected.
    #[test]
    fn every_documented_binding_exists_in_the_two_key_readers() {
        let readers = format!(
            "{}{}",
            include_str!("shortcuts.rs"),
            include_str!("../app/input.rs")
        );

        // Negative control: an invented binding must not be findable.
        assert!(
            !readers.contains("Key::F12"),
            "negative control: F12 is not bound, so the scan can reject rows"
        );

        let bindings: &[(&str, &[&str])] = &[
            ("Ctrl+N", &["Key::N"]),
            ("Ctrl+O", &["Key::O"]),
            ("Ctrl+S", &["Key::S"]),
            ("Ctrl+Shift+S", &["Key::S", "modifiers.shift"]),
            ("Ctrl+Z", &["Key::Z"]),
            ("Ctrl+Y", &["Key::Y"]),
            ("F", &["Key::F)"]),
            ("Ctrl+0", &["Key::Num0"]),
            ("F3", &["Key::F3"]),
            ("F7", &["Key::F7"]),
            ("F8", &["Key::F8"]),
            ("Enter", &["Key::Enter"]),
            ("Esc", &["Key::Escape"]),
            ("Delete / Backspace", &["Key::Delete", "Key::Backspace"]),
            (
                "Any other character",
                &["typed_chars(", "focus_command_line"],
            ),
            ("ArrowUp", &["Key::ArrowUp"]),
            ("ArrowDown", &["Key::ArrowDown"]),
            ("F1", &["Key::F1"]),
        ];

        let documented: Vec<&str> = SHORTCUT_GROUPS
            .iter()
            .flat_map(|g| g.rows.iter().map(|r| r.0))
            .collect();
        let mapped: Vec<&str> = bindings.iter().map(|b| b.0).collect();
        assert_eq!(
            documented, mapped,
            "every documented binding must be checked against the readers"
        );

        for (binding, tokens) in bindings {
            for token in *tokens {
                assert!(
                    readers.contains(token),
                    "{binding} claims a binding whose {token} is in neither key reader"
                );
            }
        }
    }

    /// LCV-116 AC 16 — the dialog is read-only. It reads no key: the whole
    /// file is scanned (matching the criterion's `grep`), and every needle is
    /// proved findable twice over — against a synthetic haystack, and against
    /// `src/app/input.rs`, a file that really does read keys. An absence
    /// assertion over a haystack that could never match proves nothing.
    #[test]
    fn the_shortcuts_dialog_reads_no_key() {
        let needles = key_reader_needles();

        for needle in needles {
            assert_eq!(
                hits(&format!("prefix {needle} suffix"), &needles),
                vec![needle],
                "the scan must be able to find {needle}"
            );
        }

        let control = hits(include_str!("../app/input.rs"), &needles);
        assert!(
            control.len() >= 3,
            "positive control: src/app/input.rs really does read keys, found {control:?}"
        );

        let found = hits(include_str!("dialogs.rs"), &needles);
        assert!(
            found.is_empty(),
            "the dialogs module must read no key (ADR 0002 §A6); found {found:?}"
        );
    }

    /// LCV-116 AC 16 — no widget in the dialog changes application state: the
    /// function never sees an `App`, so there is nothing for it to mutate but
    /// the `open` flag egui's × owns.
    #[test]
    fn the_shortcuts_dialog_changes_no_app_state() {
        let implementation = implementation_source();
        let start = implementation
            .find("pub fn shortcuts_dialog(")
            .expect("shortcuts_dialog must exist");
        let end = implementation[start..]
            .find("\n/// One `binding`/`description` row")
            .expect("shortcuts_dialog is followed by shortcut_row")
            + start;
        let body = &implementation[start..end];
        assert!(
            body.contains("ScrollArea::vertical()"),
            "positive control: the body is vertically scrollable (AC 11)"
        );
        assert!(
            body.contains(".open(open)"),
            "positive control: egui's × owns the open flag"
        );
        for forbidden in ["App", "app.", ".clicked()", "ui.button("] {
            assert!(
                !body.contains(forbidden),
                "the shortcuts dialog is read-only; it must not contain {forbidden}"
            );
        }
    }

    /// The key-reading identifiers AC 16 forbids, each assembled from
    /// fragments so this file never contains the literal it scans for.
    fn key_reader_needles() -> [&'static str; 3] {
        [
            concat!("key_", "pressed"),
            concat!("ev", "ents"),
            concat!("inp", "ut("),
        ]
    }

    /// Which of `needles` occur in `hay`.
    fn hits<'a>(hay: &str, needles: &[&'a str]) -> Vec<&'a str> {
        needles
            .iter()
            .copied()
            .filter(|needle| hay.contains(needle))
            .collect()
    }

    /// The implementation half of this file — everything before the bare
    /// `#[cfg(test)]` anchor.
    fn implementation_source() -> &'static str {
        let src = include_str!("dialogs.rs");
        let cfg_test_at = src
            .find("\n#[cfg(test)]")
            .expect("dialogs.rs must have a bare #[cfg(test)] anchor");
        &src[..cfg_test_at]
    }
}
