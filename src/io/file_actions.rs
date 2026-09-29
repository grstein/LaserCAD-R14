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
///
/// `app.export_preset` is deliberately **not** reset (LCV-115 AC 5): the
/// export profile is a property of the session's job, not of the document,
/// and an operator doing three mark jobs in a row picks it once.
pub fn action_new(app: &mut App) {
    app.document = Document {
        bed_mm: app.settings.clamped_default_bed_mm(),
        ..Document::default()
    };
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
/// On success the document, history, current-file path, export preset, and
/// recent-files list are all updated and the autosave file is cleared.
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

    let imported = match import_svg(&content) {
        Ok(v) => v,
        Err(e) => {
            app.error_message = Some(format!("SVG import failed: {e}"));
            return;
        }
    };

    // All steps below are only reached on full success. The document adopts
    // the *file's* bed (LCV-114 AC 10): re-saving it must not re-mirror every
    // Y around a different height. The settings seed is deliberately left
    // alone — opening a file does not re-home the operator's machine.
    //
    // The session adopts the *file's* preset too (LCV-115 AC 9), so Ctrl+S on
    // a marking file returns its geometry to the `mark` group.
    app.export_preset = imported.preset;
    app.document = Document {
        entities: imported.entities,
        bed_mm: imported.bed_mm,
        ..Document::default()
    };
    app.history = History::default();
    app.current_file = Some(path.clone());
    app.mark_saved();
    app.settings
        .push_recent_file(path.to_string_lossy().into_owned());
    app.persist_settings();
    app.clear_autosave();
}

/// Save the document to the current file path, in `app.export_preset`'s
/// colour group (LCV-115 AC 5).
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

    let svg = export_svg(&app.document, app.export_preset);
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
/// path, export preset, and recent-files list are updated and the autosave
/// file is cleared.
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

    let imported = match import_svg(&content) {
        Ok(v) => v,
        Err(e) => {
            app.error_message = Some(format!("SVG import failed: {e}"));
            return;
        }
    };

    // Adopts the file's bed and preset, same as `action_open` (LCV-114 AC 10,
    // LCV-115 AC 9).
    app.export_preset = imported.preset;
    app.document = Document {
        entities: imported.entities,
        bed_mm: imported.bed_mm,
        ..Document::default()
    };
    app.history = History::default();
    app.current_file = Some(path.clone());
    app.mark_saved();
    app.settings
        .push_recent_file(path.to_string_lossy().into_owned());
    app.persist_settings();
    app.clear_autosave();
}

/// Present a save dialog and write the document to the chosen path, in
/// `app.export_preset`'s colour group (LCV-115 AC 5).
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

    let svg = export_svg(&app.document, app.export_preset);
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
mod tests {
    use super::*;
    use crate::document::{CreateLine, Entity};
    use crate::geometry::{Line, Vec2};
    use crate::io::Preset;
    use std::path::Path;
    use std::time::Instant;

    // --- tempdir fixtures (LCV-119 / ADR 0006) ------------------------------

    /// A private, empty directory under the system temp dir, named after the
    /// test that owns it so parallel tests never share one. Leftovers from a
    /// previous run are removed first, which is what makes the `read_dir`
    /// counting assertions meaningful.
    fn tempdir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lcv119_fa_{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// An `App` whose two persistence paths point inside `dir` — the ADR 0006
    /// way to drive a file action from a test. Nothing it writes escapes the
    /// temporary directory.
    fn app_with_tempdir(dir: &Path) -> App {
        App {
            settings_path: Some(dir.join("settings.json")),
            autosave_path: Some(dir.join("autosave.json")),
            ..App::default()
        }
    }

    /// Write a real SVG into `dir` carrying `bed_mm` in its header and one
    /// line in `preset`'s colour group, through the production exporter, so
    /// the test reads back exactly what the app would have written.
    fn svg_file(dir: &Path, name: &str, bed_mm: [f64; 2], preset: Preset) -> PathBuf {
        let mut doc = Document {
            bed_mm,
            ..Document::default()
        };
        doc.entities.push(Entity::Line(Line::new(
            Vec2::new(10.0, 10.0),
            Vec2::new(40.0, 25.0),
        )));
        let path = dir.join(name);
        fs::write(&path, export_svg(&doc, preset)).unwrap();
        path
    }

    /// A stand-in crash-recovery file at `dir/autosave.json`.
    fn seeded_autosave(dir: &Path) -> PathBuf {
        let path = dir.join("autosave.json");
        fs::write(&path, br#"{"probe":"lcv119"}"#).unwrap();
        path
    }

    /// The source text of `fn action_open`, bounded to the implementation
    /// section and then to the function, so a test body can never satisfy a
    /// scan over it.
    fn action_open_body() -> &'static str {
        let src = include_str!("file_actions.rs");
        let cfg_test_at = src
            .find("\n#[cfg(test)]")
            .expect("file_actions.rs must have a bare #[cfg(test)] anchor");
        let implementation = &src[..cfg_test_at];
        let start = implementation
            .find("pub fn action_open(app: &mut App)")
            .expect("action_open must exist");
        let end = implementation[start..]
            .find("\n/// Save the document")
            .expect("action_open is followed by action_save")
            + start;
        &implementation[start..end]
    }

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

    /// LCV-114 AC 11 — `File > New` starts on the operator's configured
    /// machine, not on the 400 mm constant, and each axis goes through the
    /// clamp because the settings file is hand-editable.
    #[test]
    fn new_document_seeds_bed_from_settings() {
        let mut app = App::default();
        app.settings.default_bed_mm = [300.0, 180.0];
        app.document.bed_mm = [128.0, 128.0];
        action_new(&mut app);
        assert_eq!(app.document.bed_mm, [300.0, 180.0]);

        app.settings.default_bed_mm = [0.0, 5000.0];
        action_new(&mut app);
        assert_eq!(app.document.bed_mm, [1.0, 2000.0]);
    }

    /// LCV-114 AC 10, repaid behaviourally by LCV-119 AC 12 — `action_open_path`
    /// installs the *file's* bed, leaves the operator's settings seed alone,
    /// persists the recent-files list, and drops the stale recovery file.
    ///
    /// Every path this test touches is inside a temporary directory it owns:
    /// `settings_path` and `autosave_path` are injected (ADR 0006), so the
    /// developer's real `~/.config/lasercad` and `~/.local/share/lasercad`
    /// are not involved. Before LCV-119 this test could not be written at all
    /// and the criterion was paid for with a source scan.
    ///
    /// The `action_open` half stays a scan, in
    /// [`open_via_the_dialog_adopts_the_file_bed_and_leaves_the_seed_alone`].
    #[test]
    fn both_open_paths_adopt_the_file_bed_and_leave_the_seed_alone() {
        let dir = tempdir("open_path_bed");
        // A bed that is neither the 400 mm default nor the settings seed, so
        // neither fallback can accidentally satisfy the assertion.
        let file_bed = [265.0, 185.0];
        let svg = svg_file(&dir, "bed.svg", file_bed, Preset::Cut);
        let autosave = seeded_autosave(&dir);

        let mut app = app_with_tempdir(&dir);
        app.settings.default_bed_mm = [333.0, 222.0];
        action_open_path(&mut app, svg.clone());

        assert_eq!(app.error_message, None, "the open must succeed");
        assert_eq!(
            app.document.bed_mm, file_bed,
            "the document adopts the file's bed (LCV-114 AC 10)"
        );
        assert_eq!(
            app.settings.default_bed_mm,
            [333.0, 222.0],
            "opening a file must not re-home the operator's default"
        );

        let persisted = crate::io::settings::load_from(&dir.join("settings.json"));
        assert_eq!(
            persisted.recent_files.first().map(String::as_str),
            Some(svg.to_string_lossy().as_ref()),
            "the opened path is persisted at the front of the recent list"
        );
        assert_eq!(
            persisted.default_bed_mm,
            [333.0, 222.0],
            "and the persisted seed is the operator's, not the file's"
        );
        assert!(
            !autosave.exists(),
            "the stale recovery file is dropped once a document is opened"
        );
    }

    /// LCV-114 AC 10, dialog half — `action_open` opens a native `rfd` dialog,
    /// which [ADR 0005](../../docs/adr/0005-native-dialogs-disarmed-by-default.md)
    /// deliberately keeps unreachable from any test: it panics outside
    /// `crate::run`. That is the only reason this half is a source scan, and
    /// it does not expire — LCV-119 repaid the `action_open_path` half above
    /// and ADR 0006 explicitly rules out injecting `rfd`.
    ///
    /// The haystack is bounded to the function's body, so this test's own
    /// source cannot satisfy it, and every claim has a positive control: the
    /// absence assertion sits next to presence assertions over the same slice.
    #[test]
    fn open_via_the_dialog_adopts_the_file_bed_and_leaves_the_seed_alone() {
        let body = action_open_body();
        assert!(
            body.contains("import_svg(&content)"),
            "positive control: action_open must import the file"
        );
        assert!(
            body.contains("entities: imported.entities,"),
            "positive control: the entities come from the import"
        );
        assert!(
            body.contains("bed_mm: imported.bed_mm,"),
            "action_open must adopt the file's bed (AC 10)"
        );
        assert!(
            !body.contains("default_bed_mm"),
            "opening a file must not re-home the operator's default (AC 10)"
        );
    }

    /// LCV-115 AC 9, repaid behaviourally by LCV-119 AC 13 — `action_open_path`
    /// adopts the *file's* preset, so Ctrl+S on a marking file returns its
    /// geometry to the `mark` group instead of cutting through the workpiece.
    /// The session starts on `Preset::Cut`, so a no-op implementation cannot
    /// pass. Tempdir-injected paths (ADR 0006); the dialog half is
    /// [`open_via_the_dialog_adopts_the_file_preset`].
    #[test]
    fn both_open_paths_adopt_the_file_preset() {
        let dir = tempdir("open_path_preset");
        let svg = svg_file(&dir, "mark.svg", [400.0, 400.0], Preset::Mark);

        let mut app = app_with_tempdir(&dir);
        assert_eq!(
            app.export_preset,
            Preset::Cut,
            "positive control: the session starts on Cut"
        );

        action_open_path(&mut app, svg);

        assert_eq!(app.error_message, None, "the open must succeed");
        assert_eq!(
            app.export_preset,
            Preset::Mark,
            "the session adopts the file's preset (LCV-115 AC 9)"
        );
    }

    /// LCV-115 AC 9, dialog half — a source scan for the ADR 0005 reason and
    /// that reason alone: `action_open` opens a native dialog no test may
    /// reach.
    #[test]
    fn open_via_the_dialog_adopts_the_file_preset() {
        let body = action_open_body();
        assert!(
            body.contains("import_svg(&content)"),
            "positive control: action_open must import the file"
        );
        assert!(
            body.contains("app.export_preset = imported.preset;"),
            "action_open must adopt the file's preset (AC 9)"
        );
        for literal in ["Preset::Cut", "Preset::Mark", "Preset::Engrave"] {
            assert!(
                !body.contains(literal),
                "action_open must not hard-code {literal}"
            );
        }
    }

    /// LCV-119 AC 11 — `action_new` removes the autosave file it was *given*
    /// and, with no path injected, removes nothing at all. The second half is
    /// the regression this whole demand exists for: `cargo test` used to
    /// delete the developer's real `~/.local/share/lasercad/autosave.json`
    /// through exactly this call.
    #[test]
    fn action_new_clears_only_the_injected_autosave_file() {
        let dir = tempdir("new_clears_autosave");
        let autosave = seeded_autosave(&dir);

        let mut app = App {
            autosave_path: Some(autosave.clone()),
            ..App::default()
        };
        action_new(&mut app);
        assert!(!autosave.exists(), "the injected autosave file is removed");

        let dir = tempdir("new_clears_nothing");
        let bystander = seeded_autosave(&dir);
        let mut app = App::default();
        action_new(&mut app);
        assert!(
            bystander.exists(),
            "a pathless App must not delete a file it was never given"
        );
        assert_eq!(
            std::fs::read_dir(&dir).unwrap().count(),
            1,
            "and must not create anything either"
        );
    }

    /// LCV-115 AC 5 — both save paths export in the *session's* preset. Same
    /// reason as above for the scan: `action_save_as` opens a native dialog.
    #[test]
    fn both_save_paths_export_in_the_session_preset() {
        let src = include_str!("file_actions.rs");
        for (start_marker, end_marker) in [
            (
                "pub fn action_save(app: &mut App)",
                "\n/// Load a document from a known file path",
            ),
            ("pub fn action_save_as(app: &mut App)", "\n#[cfg(test)]"),
        ] {
            let start = src
                .find(start_marker)
                .unwrap_or_else(|| panic!("{start_marker} must exist"));
            let end = src[start..]
                .find(end_marker)
                .unwrap_or_else(|| panic!("{start_marker} must be followed by {end_marker}"))
                + start;
            let body = &src[start..end];
            assert!(
                body.contains("fs::write(&path, svg.as_bytes())"),
                "positive control: {start_marker} must write the file"
            );
            assert!(
                body.contains("export_svg(&app.document, app.export_preset)"),
                "{start_marker} must export in the session preset (AC 5)"
            );
        }
    }

    // `action_new` keeping the preset (LCV-115 AC 5) is covered in
    // `tests/it/app/preset_ui.rs`, where the demand places it.

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

    /// LCV-113 AC 4 — action_new marks the fresh document safe to discard
    /// via `App::mark_saved`, which supersedes the bare autosave-clean call
    /// this action used to make.
    #[test]
    fn action_new_marks_the_fresh_document_saved() {
        let mut app = App::default();
        app.history.commit(
            Box::new(CreateLine::new(Line::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 1.0),
            ))),
            &mut app.document,
        );
        assert!(app.has_unsaved_changes());

        action_new(&mut app);

        assert!(!app.has_unsaved_changes());
        assert_eq!(app.guard.saved_revision, Some(app.history.revision()));
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

    /// LCV-119 AC 9 — `action_save` removes the autosave file it was given.
    ///
    /// The site is `action_save`'s `app.clear_autosave()`, which nothing
    /// pinned until this test: every other `action_save` test builds a bare
    /// `App::default()` with only `current_file` set, so deleting the call
    /// left the whole suite green. The regression it guards is user-visible
    /// and silent — a stale recovery file that outlives a save means a crash
    /// straight after `File > Save` recovers the *wrong* document, the one
    /// from before the save.
    ///
    /// Both halves, as everywhere in this demand: an injected path is
    /// cleared, and a pathless `App` deletes nothing it was never given.
    #[test]
    fn action_save_clears_the_injected_autosave_file() {
        let dir = tempdir("save_clears_autosave");
        let autosave = seeded_autosave(&dir);

        let mut app = App {
            current_file: Some(dir.join("saved.svg")),
            ..app_with_tempdir(&dir)
        };
        action_save(&mut app);

        assert_eq!(app.error_message, None, "the save must succeed");
        assert!(
            dir.join("saved.svg").is_file(),
            "positive control: the document really was written"
        );
        assert!(
            !autosave.exists(),
            "a saved document must not leave a stale recovery file behind"
        );

        // The pathless half: same action, nothing to clear, nothing cleared.
        let dir = tempdir("save_clears_nothing");
        let bystander = seeded_autosave(&dir);
        let mut app = App {
            current_file: Some(dir.join("saved.svg")),
            ..App::default()
        };
        action_save(&mut app);
        assert!(
            bystander.exists(),
            "a pathless App must not delete a file it was never given"
        );
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

    /// LCV-113 AC 4 — action_save marks the document safe to discard.
    #[test]
    fn action_save_marks_the_document_saved() {
        let tmp = std::env::temp_dir().join("lcv113_save_marks_saved.svg");
        let _ = std::fs::remove_file(&tmp);

        let mut app = App {
            current_file: Some(tmp.clone()),
            ..App::default()
        };
        app.history.commit(
            Box::new(CreateLine::new(Line::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 1.0),
            ))),
            &mut app.document,
        );
        assert!(app.has_unsaved_changes());

        action_save(&mut app);

        assert!(!app.has_unsaved_changes());
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
