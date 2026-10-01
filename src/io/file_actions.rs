//! File-level actions: New, Open, Save, Save As.
//!
//! Four pure `fn(app: &mut App)` functions connect keyboard shortcuts and
//! future menu items to the SVG pipeline. Each function is self-contained:
//! it reads/writes only through `app`, the dialog wrappers, and `std::fs`.
//!
//! Error reporting: on I/O or parse failure the functions set
//! `app.error_message = Some(...)`. The UI renders the message via
//! [`crate::ui::error_dialog`] on the next frame; the caller does not need
//! to inspect a return value.
//!
//! **Purity rule**: this module MUST NOT contain top-level `use egui`,
//! `use eframe`, or `use rfd` statements. Dialog access goes through the
//! wrappers in `crate::io`.
//!
//! `File > Export layers` writes per-layer files without saving the mother
//! and lives in [`crate::io::export_layers`] (LCV-156).
//!
//! Introduced by demand LCV-062.

use std::fs;
use std::path::PathBuf;

use crate::app::App;
use crate::document::{Document, History};
use crate::io::svg::{export_svg, import_svg_with};
use crate::io::{open_file_dialog, save_file_dialog};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Reset the session to a blank document.
///
/// Clears all entities, resets the undo/redo history, clears the current-file
/// path, marks the fresh document safe to discard, and removes any pending
/// autosave file. The confirm-discard dialog that guards this action against
/// unsaved work lives one layer up, in `App::request_new`
/// (`src/app/file_ops.rs`, LCV-113) — by the time this function runs, the
/// discard has already been confirmed (or the document was already clean).
///
/// The blank document is seeded with the operator's configured default bed
/// (LCV-114 AC 11) so `File > New` lands on their machine, not on 400 × 400.
pub fn action_new(app: &mut App) {
    app.document = Document::with_bed(app.settings.clamped_default_bed_mm());
    app.history = History::default();
    app.current_file = None;
    app.mark_saved();
    app.clear_autosave();
}

/// Open a document from disk via a native file-open dialog.
///
/// If the user cancels the dialog this function returns immediately without
/// modifying any `App` field.  On I/O or parse failure `app.error_message` is
/// set to a descriptive string and the existing document is left unchanged.
/// On success the document, history, current-file path and recent-files
/// list are all updated, the autosave file is cleared, and the command-line
/// feedback says what the import ignored (LCV-171).
pub fn action_open(app: &mut App) {
    let path = match open_file_dialog() {
        Some(p) => p,
        None => return,
    };

    let content = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            app.error_message = Some(format!("Could not read '{}': {e}", path.display()));
            return;
        }
    };
    open_content(app, path, &content);
}

/// Save the document to the current file path as the mother SVG, one group
/// per layer (LCV-156 AC 9).
///
/// If no file path is known (`app.current_file` is `None`) this function
/// delegates to [`action_save_as`].  On success `app.mark_saved()` marks the
/// document safe to discard and clears the autosave debounce; the autosave
/// file is removed.  On I/O failure `app.error_message` is set;
/// `app.current_file` is never modified by this function.
pub fn action_save(app: &mut App) {
    let path = match app.current_file.clone() {
        Some(p) => p,
        None => {
            action_save_as(app);
            return;
        }
    };

    let svg = export_svg(&app.document);
    if let Err(e) = fs::write(&path, svg.as_bytes()) {
        app.error_message = Some(format!("Could not write '{}': {e}", path.display()));
        return;
    }

    app.mark_saved();
    app.clear_autosave();
}

/// Load a document from a known file path (no dialog).
///
/// Called by the File → Open Recent menu to load a path that was already
/// chosen by the operator. On success the document, history, current-file
/// path and recent-files list are updated, the autosave file is cleared, and
/// the command-line feedback says what the import ignored (LCV-171).
/// On I/O or parse failure `app.error_message` is set; the existing document
/// is left unchanged.
pub fn action_open_path(app: &mut App, path: PathBuf) {
    let content = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            app.error_message = Some(format!("Could not read '{}': {e}", path.display()));
            return;
        }
    };
    open_content(app, path, &content);
}

/// Import `content` read from `path` and, only on full success, install it
/// and say what the import ignored (LCV-171 AC 9). On failure
/// `app.error_message` is set and nothing else changes.
fn open_content(app: &mut App, path: PathBuf, content: &str) {
    let opened = import_svg_with(content, &app.fonts).and_then(|imported| {
        let feedback = ignored_feedback(&imported.report);
        imported
            .into_document()
            .map(|document| (document, feedback))
    });
    let (document, feedback) = match opened {
        Ok(v) => v,
        Err(e) => {
            app.error_message = Some(format!("SVG import failed: {e}"));
            return;
        }
    };

    // The document adopts the *file's* bed (LCV-114 AC 10): re-saving it
    // must not re-mirror every Y around a different height. The settings
    // seed is deliberately left alone — opening a file does not re-home the
    // operator's machine. Its layers, membership and current layer come
    // with it (LCV-156 AC 9).
    app.document = document;
    app.history = History::default();
    app.current_file = Some(path.clone());
    app.mark_saved();
    app.settings
        .push_recent_file(path.to_string_lossy().into_owned());
    app.persist_settings();
    app.clear_autosave();
    app.command_feedback = feedback;
}

/// `Ignored: <count> <label>, …` in report order; empty for an empty report.
fn ignored_feedback(report: &[(String, usize)]) -> String {
    if report.is_empty() {
        return String::new();
    }
    let entries: Vec<String> = report
        .iter()
        .map(|(label, count)| format!("{count} {label}"))
        .collect();
    format!("Ignored: {}", entries.join(", "))
}

/// Present a save dialog and write the document to the chosen path as the
/// mother SVG (LCV-156 AC 9).
///
/// The default filename is the current file's name component, or
/// `"untitled.svg"` when no file is open.  A `.svg` extension is appended if
/// absent or different (case-sensitive).  If the user cancels the dialog this
/// function returns immediately without modifying any `App` field.  On success
/// `app.current_file` is updated and the recent-files list is refreshed.
pub fn action_save_as(app: &mut App) {
    let default_name = app
        .current_file
        .as_deref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("untitled.svg")
        .to_owned();

    let mut path: PathBuf = match save_file_dialog(&default_name) {
        Some(p) => p,
        None => return,
    };

    // Ensure .svg extension.
    if path.extension().and_then(|e| e.to_str()) != Some("svg") {
        path.set_extension("svg");
    }

    let svg = export_svg(&app.document);
    if let Err(e) = fs::write(&path, svg.as_bytes()) {
        app.error_message = Some(format!("Could not write '{}': {e}", path.display()));
        return;
    }

    app.current_file = Some(path.clone());
    app.mark_saved();
    app.settings
        .push_recent_file(path.to_string_lossy().into_owned());
    app.persist_settings();
    app.clear_autosave();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
