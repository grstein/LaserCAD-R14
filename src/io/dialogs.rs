//! Thin blocking wrappers around [`rfd::FileDialog`] for the three file-system
//! operations LaserCAD needs: open a file, save a file, and pick a directory.
//!
//! These functions are the sole consumers of `rfd` in the codebase.  All file
//! I/O (reading / writing SVG) is handled by [`crate::io::svg`]; these
//! wrappers only present the OS dialog and return the path the user chose.
//!
//! **Purity**: this module imports only `rfd` and `std`.  It must not import
//! any UI framework or kernel module (`geometry`, `document`, `io::svg`,
//! `agent`, `text`).  See `AGENTS.md` §Purity rule.

use std::path::PathBuf;

/// Opens a native file-open dialog filtered to SVG files and all files.
///
/// Returns `Some(path)` when the user confirms a selection, `None` when the
/// user cancels or closes the dialog without choosing a file.
pub fn open_file_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("SVG files", &["svg"])
        .add_filter("All files", &["*"])
        .pick_file()
}

/// Opens a native file-save dialog with the filename input pre-filled to
/// `default_name`.
///
/// The same two filters as [`open_file_dialog`] are present.  Returns
/// `Some(path)` on confirm, `None` on cancel.
pub fn save_file_dialog(default_name: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("SVG files", &["svg"])
        .add_filter("All files", &["*"])
        .set_file_name(default_name)
        .save_file()
}

/// Opens a native folder-picker dialog (no file-extension filter).
///
/// Reserved for future batch-export workflows.  Returns `Some(path)` on
/// confirm, `None` on cancel.
pub fn pick_folder_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new().pick_folder()
}

#[cfg(test)]
mod tests {
    use super::{open_file_dialog, pick_folder_dialog, save_file_dialog};

    /// LCV-061 AC#1–3 — compile-time assertion that the three wrappers carry
    /// exactly the declared signatures.  No dialog is opened; the test passes
    /// as long as the function pointer types match.
    #[test]
    fn dialog_fn_signatures() {
        let _: fn() -> Option<std::path::PathBuf> = open_file_dialog;
        let _: fn(&str) -> Option<std::path::PathBuf> = save_file_dialog;
        let _: fn() -> Option<std::path::PathBuf> = pick_folder_dialog;
    }
}
