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
//! Introduced by demand LCV-062.

use std::fs;
use std::path::PathBuf;

use crate::app::App;
use crate::document::{Document, History};
use crate::io::svg::{export_svg, import_svg};
use crate::io::{clear_autosave, open_file_dialog, save_file_dialog};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Reset the session to a blank document.
///
/// Clears all entities, resets the undo/redo history, clears the current-file
/// path, clears the dirty-since timer, and removes any pending autosave file.
/// Unsaved changes are discarded without confirmation (a confirm-discard dialog
/// is deferred to a later demand).
pub fn action_new(app: &mut App) {
    app.document = Document::default();
    app.history = History::default();
    app.current_file = None;
    app.dirty_since = None;
    clear_autosave();
}

/// Open a document from disk via a native file-open dialog.
///
/// If the user cancels the dialog this function returns immediately without
/// modifying any `App` field.  On I/O or parse failure `app.error_message` is
/// set to a descriptive string and the existing document is left unchanged.
/// On success the document, history, current-file path, and recent-files list
/// are all updated and the autosave file is cleared.
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

    let entities = match import_svg(&content) {
        Ok(v) => v,
        Err(e) => {
            app.error_message = Some(format!("SVG import failed: {e}"));
            return;
        }
    };

    // All steps below are only reached on full success.
    app.document = Document {
        entities,
        ..Document::default()
    };
    app.history = History::default();
    app.current_file = Some(path.clone());
    app.dirty_since = None;
    app.settings
        .push_recent_file(path.to_string_lossy().into_owned());
    let _ = app.settings.save();
    clear_autosave();
}

/// Save the document to the current file path.
///
/// If no file path is known (`app.current_file` is `None`) this function
/// delegates to [`action_save_as`].  On success `app.dirty_since` is cleared
/// and the autosave file is removed.  On I/O failure `app.error_message` is
/// set; `app.current_file` is never modified by this function.
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

    app.dirty_since = None;
    clear_autosave();
}

/// Present a save dialog and write the document to the chosen path.
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
    app.dirty_since = None;
    app.settings
        .push_recent_file(path.to_string_lossy().into_owned());
    let _ = app.settings.save();
    clear_autosave();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{CreateLine, Entity};
    use crate::geometry::{Line, Vec2};
    use std::time::Instant;

    // --- AC 1 — compile-time function-signature check -----------------------

    /// AC 1 — all four public functions carry the expected `fn(&mut App)` type.
    #[test]
    fn file_actions_fn_signatures() {
        let _: fn(&mut App) = action_new;
        let _: fn(&mut App) = action_open;
        let _: fn(&mut App) = action_save;
        let _: fn(&mut App) = action_save_as;
    }

    // --- AC 3 — action_new --------------------------------------------------

    /// AC 3 — action_new resets document and history.
    #[test]
    fn action_new_clears_document_and_history() {
        let mut app = App::default();
        // Push an entity and commit a command so undo stack is non-empty.
        app.document.entities.push(Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
        )));
        app.commit(Box::new(CreateLine::new(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 1.0),
        ))));

        action_new(&mut app);

        assert_eq!(app.document.entity_count(), 0);
        assert!(!app.history.can_undo());
    }

    /// AC 3 — action_new clears current_file.
    #[test]
    fn action_new_resets_current_file() {
        let mut app = App {
            current_file: Some(PathBuf::from("foo.svg")),
            ..App::default()
        };
        action_new(&mut app);
        assert!(app.current_file.is_none());
    }

    /// AC 3 — action_new clears dirty_since.
    #[test]
    fn action_new_clears_dirty_since() {
        let mut app = App {
            dirty_since: Some(Instant::now()),
            ..App::default()
        };
        action_new(&mut app);
        assert!(app.dirty_since.is_none());
    }

    // --- AC 9 — action_save with current_file set ---------------------------

    /// AC 9 — action_save writes SVG to current_file path.
    #[test]
    fn action_save_writes_svg_to_current_path() {
        let tmp = std::env::temp_dir().join("lcv062_save_test.svg");
        let _ = std::fs::remove_file(&tmp);

        let mut app = App {
            current_file: Some(tmp.clone()),
            ..App::default()
        };
        app.document.entities.push(Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(5.0, 5.0),
        )));

        action_save(&mut app);

        assert!(tmp.exists(), "file should have been written");
        let content = std::fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("<svg"), "should contain SVG root");
        let _ = std::fs::remove_file(&tmp);
    }

    /// AC 9 — action_save clears dirty_since on success.
    #[test]
    fn action_save_clears_dirty_since() {
        let tmp = std::env::temp_dir().join("lcv062_save_dirty.svg");
        let _ = std::fs::remove_file(&tmp);

        let mut app = App {
            current_file: Some(tmp.clone()),
            dirty_since: Some(Instant::now()),
            ..App::default()
        };

        action_save(&mut app);

        assert!(app.dirty_since.is_none());
        let _ = std::fs::remove_file(&tmp);
    }

    /// AC 9 — action_save sets error_message on I/O failure.
    #[test]
    fn action_save_io_error_sets_error_message() {
        let mut app = App {
            current_file: Some(PathBuf::from("/nonexistent_dir/canary.svg")),
            ..App::default()
        };
        action_save(&mut app);
        assert!(app.error_message.is_some());
    }
}
