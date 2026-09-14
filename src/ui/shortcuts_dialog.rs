//! The keyboard-shortcuts dialog (`F1`): the binding table it renders from,
//! the rule that splits that table into two columns, and the `Window` that
//! paints them.
//!
//! Split out of `src/ui/dialogs.rs` by LCV-134, executing the seam ADR 0004
//! Amended (2) pre-decided: that file stood at 299 of its 300 implementation
//! lines, and this is the only dialog in the app whose body is rendered from a
//! table, so it is the only one that can grow without anybody editing a
//! layout. `src/ui/dialogs.rs` keeps the three generic modal helpers;
//! `src/ui/mod.rs` re-exports every name that moved here, so no caller outside
//! `src/ui/` changed.
//!
//! Like every dialog in this repo, this one is **read-only with respect to the
//! keyboard**: it reads no key. `src/ui/shortcuts.rs` — one word away in the
//! file listing, and the file that really does dispatch `F1` — plus
//! `src/app/input.rs` are the only two key readers in the app (ADR 0002 §A6).
//! `the_shortcuts_dialog_reads_no_key` below is what keeps that true across
//! this move, and it scans **this** file.

use egui::{Align2, Context, Window};

use crate::ui::toolbar::TOOLS;

/// One group of bindings in the keyboard-shortcuts dialog. `pub` (not
/// `pub(crate)`) so `tests/lcv133_shortcuts_dialog_fits.rs` can derive counts.
pub struct ShortcutGroup {
    /// Heading rendered above the group.
    pub heading: &'static str,
    /// `(binding, description)` rows, in display order.
    pub rows: &'static [(&'static str, &'static str)],
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
/// with no edit here (AC 14). `pub` (not `pub(crate)`) so `tests/lcv133_shortcuts_dialog_fits.rs` can derive this table's counts too.
pub const SHORTCUT_GROUPS: &[ShortcutGroup] = &[
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
/// tool change. `pub` so `tests/lcv133_shortcuts_dialog_fits.rs` can derive these rows too.
pub fn tool_rows() -> Vec<(&'static str, &'static str)> {
    TOOLS
        .iter()
        .filter_map(|entry| entry.shortcut.map(|key| (key, entry.label)))
        .collect()
}

/// A heading plus its rows: the atomic, never-reordered unit that moves.
struct Section {
    heading: &'static str,
    rows: Vec<(&'static str, &'static str)>,
}

/// One item per heading, plus one per row: what the split balances on.
fn item_count(section: &Section) -> usize {
    1 + section.rows.len()
}

/// `Tools` plus every [`SHORTCUT_GROUPS`] entry, each as one [`Section`].
fn sections() -> Vec<Section> {
    let mut all = vec![Section {
        heading: "Tools",
        rows: tool_rows(),
    }];
    all.extend(SHORTCUT_GROUPS.iter().map(|group| Section {
        heading: group.heading,
        rows: group.rows.to_vec(),
    }));
    all
}

/// Fill the left column section by section until its item count reaches half
/// the total, then put the rest right (LCV-133 AC 5, derived, not hand-typed).
fn split_into_columns(sections: Vec<Section>) -> (Vec<Section>, Vec<Section>) {
    let half = sections.iter().map(item_count).sum::<usize>() / 2;
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut filled = 0;
    for section in sections {
        if right.is_empty() && filled < half {
            filled += item_count(&section);
            left.push(section);
        } else {
            right.push(section);
        }
    }
    (left, right)
}

/// Render one column: a heading then its rows, with breathing room before
/// every heading after the column's first.
fn render_column(ui: &mut egui::Ui, sections: &[Section]) {
    for (i, section) in sections.iter().enumerate() {
        if i > 0 {
            ui.add_space(6.0);
        }
        ui.heading(section.heading);
        for (binding, description) in &section.rows {
            shortcut_row(ui, binding, description);
        }
    }
}

/// Render the Keyboard shortcuts dialog (LCV-116 AC 11).
///
/// Follows [`about_dialog`]: opened/closed through `open`; centre-anchored,
/// not collapsible. Laid out in **two columns** ([`split_into_columns`])
/// because `Window::new`'s baked 420pt `default_size` caps a `ScrollArea` at
/// that height regardless of screen size (egui 0.29.1) and one column of
/// this content needs ~836pt (LCV-133); the `ScrollArea` stays as the safety
/// net for a screen shorter than the two columns' own height.
///
/// Read-only: no widget changes application state, and it reads no key. `F1`
/// is dispatched in `src/ui/shortcuts.rs` like `F3` / `F7` / `F8`, not here.
pub fn shortcuts_dialog(ctx: &Context, open: &mut bool) {
    Window::new("Keyboard shortcuts")
        .open(open)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let (left, right) = split_into_columns(sections());
                ui.columns(2, |columns| {
                    render_column(&mut columns[0], &left);
                    render_column(&mut columns[1], &right);
                });
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

    /// LCV-133 AC 5 — the column split is computed from `sections()`, not
    /// hand-typed: this pins today's derived boundary (`Tools`/`File`/`Edit`
    /// left, the rest right) as a regression, while the function under test
    /// takes no group name as input at all. `tests/lcv133_shortcuts_dialog_fits.rs`
    /// carries the corresponding rendered-frame proof at three screen sizes;
    /// this is the cheaper, non-rendering half.
    #[test]
    fn split_into_columns_balances_by_item_count_not_by_name() {
        let (left, right) = split_into_columns(sections());

        let headings = |s: &[Section]| -> Vec<&str> { s.iter().map(|s| s.heading).collect() };
        assert_eq!(headings(&left), vec!["Tools", "File", "Edit"]);
        assert_eq!(
            headings(&right),
            vec!["View", "Modes", "Drawing", "Command line", "Help"]
        );

        // Every section is placed exactly once, in its original order, split
        // between whole sections only (no row crosses the boundary).
        let mut rejoined = headings(&left);
        rejoined.extend(headings(&right));
        assert_eq!(
            rejoined,
            headings(&sections()),
            "the split must partition sections(), never reorder or drop one"
        );
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

        let found = hits(include_str!("shortcuts_dialog.rs"), &needles);
        assert!(
            found.is_empty(),
            "the shortcuts-dialog module must read no key (ADR 0002 §A6); found {found:?}"
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
        let src = include_str!("shortcuts_dialog.rs");
        let cfg_test_at = src
            .find("\n#[cfg(test)]")
            .expect("shortcuts_dialog.rs must have a bare #[cfg(test)] anchor");
        &src[..cfg_test_at]
    }
}
