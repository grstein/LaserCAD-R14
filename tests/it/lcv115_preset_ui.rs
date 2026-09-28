//! LCV-115 AC 5 / AC 6 / AC 7 / AC 10 — the export preset as session state.
//!
//! The frame tests drive the real body through
//! `egui::Context::run(raw_input(...), |ctx| app.update_ui(ctx))`
//! (`tests/harness/mod.rs`, ADR 0002 §A3), so the status-bar and menubar code
//! paths are exercised where they actually live.
//!
//! Two deliberate deviations from the demand's *Expected tests* list, both
//! recorded here rather than silently:
//!
//! 1. The demand asks this file to assert `format_preset(Preset::Engrave) ==
//!    "ENGRAVE"`. AC 7 also specifies `format_preset` as `pub(crate)`, which
//!    an integration test cannot reach. The normative AC wins: the text
//!    assertion for all three variants lives in
//!    `src/ui/statusbar.rs::tests::format_preset_covers_all_three_variants`,
//!    and what this file proves is that the render path runs with each preset
//!    without panicking and leaves the field alone.
//! 2. No `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S` and no call to `action_open`,
//!    `action_open_path`, `action_save` or `action_save_as`. `action_open`
//!    and `action_save_as` reach native dialogs, disarmed outside
//!    `crate::run` by ADR 0005. The other two are drivable from a test since
//!    LCV-119 injected their persistence paths (ADR 0006), but they belong to
//!    the file actions' own unit tests, not to this file, whose subject is
//!    the preset as *session state* across a frame.
//!
//! The menu *click* is likewise not simulated — a nested `menu_button` needs
//! two pointer frames and hit-testing (same limitation LCV-114's bed-dialog
//! tests hit). `preset_submenu`'s binding is covered by the bounded source
//! scans in `src/ui/menubar.rs`; what this file covers is the state it writes.

use crate::harness;

use harness::frame;
use lasercad::app::App;
use lasercad::document::CreateLine;
use lasercad::geometry::{Line, Vec2};
use lasercad::io::{action_new, Preset};

/// AC 7 — the status bar renders with every preset, on a document with content
/// and on an empty one, without panicking, and the frame does not disturb the
/// field it is displaying.
#[test]
fn status_bar_shows_the_active_preset() {
    for preset in Preset::ALL {
        let ctx = egui::Context::default();
        let mut app = App {
            export_preset: preset,
            ..App::default()
        };
        app.history.commit(
            Box::new(CreateLine::new(Line::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 10.0),
            ))),
            &mut app.document,
        );

        frame(&ctx, &mut app, vec![]);
        frame(&ctx, &mut app, vec![]);

        assert_eq!(
            app.export_preset, preset,
            "rendering the bar must not change the preset"
        );
    }
}

/// AC 5 — a fresh session starts on `Cut`. Together with the persistence scan
/// below, this is what "restarting returns the selector to Cut" means: there is
/// no stored value for a restart to restore.
#[test]
fn a_fresh_app_starts_on_cut() {
    assert_eq!(App::default().export_preset, Preset::Cut);
    assert_eq!(Preset::default(), Preset::Cut);
}

/// AC 5 — `File > New` keeps the preset. The operator doing three mark jobs in
/// a row picks it once; the preset is a property of the session's job, not of
/// the document.
#[test]
fn action_new_preserves_the_export_preset() {
    let mut app = App {
        export_preset: Preset::Mark,
        ..App::default()
    };
    app.history.commit(
        Box::new(CreateLine::new(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 10.0),
        ))),
        &mut app.document,
    );
    assert_eq!(app.document.entity_count(), 1);

    action_new(&mut app);

    assert_eq!(app.export_preset, Preset::Mark);
    assert_eq!(app.document.entity_count(), 0);
}

/// AC 10 — the preset is nowhere in the persistence layer: not in `Settings`,
/// not in the autosave envelope, not in the document. A sticky preset across
/// restarts is a physical failure mode (yesterday's mark job cutting today's
/// workpiece), so its absence is asserted, not assumed.
///
/// Each file scanned is checked for a token it must contain, so a renamed or
/// moved file fails loudly instead of passing vacuously.
#[test]
fn the_preset_is_not_persisted_anywhere() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut scanned = 0usize;

    for (rel, control) in [
        ("src/io/settings.rs", "pub struct Settings {"),
        ("src/io/autosave.rs", "struct DocumentEnvelope {"),
    ] {
        let src = std::fs::read_to_string(root.join(rel)).expect(rel);
        assert!(src.contains(control), "positive control missing in {rel}");
        assert!(!src.contains("export_preset"), "{rel} must not persist it");
        assert!(!src.contains("Preset"), "{rel} must not know the type");
        scanned += 1;
    }

    for entry in walk(&root.join("src/document")) {
        let src = std::fs::read_to_string(&entry).expect("document source");
        assert!(
            !src.contains("export_preset"),
            "{} must not persist it",
            entry.display()
        );
        scanned += 1;
    }

    assert!(
        scanned >= 8,
        "only {scanned} files scanned — walk is broken"
    );
    let state = std::fs::read_to_string(root.join("src/document/state.rs")).unwrap();
    assert!(
        state.contains("pub bed_mm"),
        "positive control: the document owns the bed but not the preset"
    );
}

/// Every `.rs` under `dir`, recursively.
fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).expect("readable directory") {
        let path = entry.expect("readable entry").path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out
}
