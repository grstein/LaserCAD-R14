//! tests/lcv138_document_title_and_file_feedback.rs — native window title,
//! recovered-from-autosave feedback, and honest Open Recent behaviour
//! (LCV-138).
//!
//! Four groups, one per acceptance criterion that needs a real frame or a
//! real menu click rather than a plain function call (AC 1's four
//! `display_title()` cases and the bulk of AC 2 already live as unit tests
//! next to the implementation, `src/app/document_title.rs`):
//!
//! - **AC 2** — a real `App::update_ui` frame sends `ViewportCommand::Title`
//!   only on a genuine change, never on an idle frame in between.
//! - **AC 3** — Save and Open Recent, success and failure, each asserting the
//!   title before and after; a cancelled discard dialog leaves it untouched.
//!   Save As… and the interactive Open… both reach a blocking native `rfd`
//!   dialog that [ADR 0005](../docs/adr/0005-native-dialogs-disarmed-by-default.md)
//!   deliberately keeps unreachable outside `crate::run` — the same reason
//!   `src/io/file_actions.rs`'s own tests keep that half a source scan — so
//!   here it is a scan proving both functions return before touching
//!   `current_file` when the dialog is cancelled; title correctness for the
//!   success path is already exhaustively covered by AC 1's four cases plus
//!   the Save / Open Recent cases below, since `display_title()` is a pure
//!   function of `current_file` and `has_unsaved_changes()` alone.
//! - **AC 4** — the recovered-from-autosave label paints, its hover text
//!   paints on a real hover, and rendering it touches neither `dirty_since`
//!   nor `last_autosave_at`.
//! - **AC 5** — Open Recent's disambiguated labels paint through a real
//!   File > Open Recent click sequence, and a failing entry (missing file,
//!   malformed SVG) preserves the drawing, history, `current_file` and the
//!   recent-files list order, surfacing `error_message` instead.
//!
//! Every `App` here is `App::default()`, optionally with `settings_path` /
//! `autosave_path` pointed at a tempdir this file owns (ADR 0006); no test
//! calls `App::new()` (ADR 0002 §A2), and no test sends `Ctrl+O`, `Ctrl+S` or
//! `Ctrl+Shift+S` (ADR 0002 §A4 rule 1).

mod harness;

use harness::paint::{self, Run};
use harness::raw_input;
use lasercad::app::{App, DocumentTitleState, PendingAction};
use lasercad::document::CreateLine;
use lasercad::geometry::{Line, Vec2};
use lasercad::ui::DialogResult;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Shared fixtures
// ---------------------------------------------------------------------------

fn some_line() -> Line {
    Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0))
}

/// A private, empty directory under the system temp dir, named after the
/// test that owns it — `src/io/file_actions.rs`'s own fixture convention.
fn tempdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lcv138_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A minimal, valid, test-owned SVG fixture: one line, on a 200×200 mm bed.
const VALID_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200" viewBox="0 0 200 200">
<line x1="10" y1="10" x2="150" y2="10" stroke="#ff0000" stroke-width="0.1"/>
</svg>"##;

/// Not valid XML at all, so `import_svg` fails before geometry parsing.
const MALFORMED_SVG: &str = "not an svg file at all";

/// A complete primary-button click at `pos`: move, press, release
/// (ADR 0002 §A4 rule 3), always preceded by its own warm-up frame — see
/// [`click_button`].
fn click_events(pos: egui::Pos2) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

/// One idle boot frame: registers every widget rect and syncs the camera away
/// from its zero-area sentinel (mirrors `tests/lcv136_*`'s own `boot`).
fn boot(ctx: &egui::Context, app: &mut App) {
    let _ = ctx.run(raw_input(vec![]), |c| app.update_ui(c));
}

/// Locate the one run reading `label` on a settled frame and return a point
/// inside its glyph rect.
fn locate(runs: &[Run], label: &str) -> egui::Pos2 {
    let matches: Vec<&Run> = runs.iter().filter(|r| r.text.trim() == label).collect();
    assert_eq!(
        matches.len(),
        1,
        "`{label}` must be painted exactly once, saw {} times",
        matches.len()
    );
    let run = matches[0];
    egui::pos2(run.pos.x + 2.0, run.pos.y + run.height / 2.0)
}

/// Drive a real pointer click at `pos`: a warm-up frame carrying
/// `PointerMoved` alone, then the frame carrying the press/release pair
/// (ADR 0002 §A4 rule 3).
fn click_button(ctx: &egui::Context, app: &mut App, pos: egui::Pos2) -> egui::FullOutput {
    let _ = ctx.run(raw_input(vec![egui::Event::PointerMoved(pos)]), |c| {
        app.update_ui(c)
    });
    ctx.run(raw_input(click_events(pos)), |c| app.update_ui(c))
}

/// Open the File menu through a real click on its "File" label (mirrors
/// `tests/lcv136_discard_dialog_pointer_click.rs::open_file_menu`).
fn open_file_menu(ctx: &egui::Context, app: &mut App) -> Vec<Run> {
    let runs = paint::painted_runs(ctx, app);
    let file = locate(&runs, "File");
    click_button(ctx, app, file);
    paint::painted_runs(ctx, app)
}

/// Open File > Open Recent — a **nested** submenu, which egui-0.29.1 opens on
/// hover, never on click (`menu.rs::submenu_button_interaction`,
/// `!open && button.hovered()`). Only a `PointerMoved` is sent, landing on
/// the already-open File menu's "Open Recent ▶" row.
fn open_recent_submenu(ctx: &egui::Context, app: &mut App) -> Vec<Run> {
    let menu_runs = open_file_menu(ctx, app);
    let recent = locate(&menu_runs, "Open Recent \u{25b6}");
    let _ = ctx.run(raw_input(vec![egui::Event::PointerMoved(recent)]), |c| {
        app.update_ui(c)
    });
    paint::painted_runs(ctx, app)
}

// ---------------------------------------------------------------------------
// AC 2 — the Title command fires only on a real change
// ---------------------------------------------------------------------------

/// AC 2 — a real `App::update_ui` frame sequence: the first frame always
/// carries a `Title` command (nothing was cached yet); an otherwise-idle
/// second frame carries none either (nothing changed); a genuine state
/// change (a direct `history.commit`, dirtying the document, ADR 0002 §B)
/// carries exactly one more, proving `update_ui` really reaches
/// `document_title::update_title` and not just that the cache is quiet; the
/// following idle frame carries none again. A mutant that drops the
/// `last_title` comparison (sending `Title` unconditionally) fails on frames
/// one and three; a mutant that never calls `update_title` at all fails on
/// frame two.
#[test]
fn title_command_fires_only_on_a_real_change() {
    let ctx = egui::Context::default();
    let mut app = App::default();

    // Frame 1 — nothing has happened since construction: `App::default`
    // pre-seeds the title cache to match (see its own doc comment), so this
    // must send nothing at all.
    let out = ctx.run(raw_input(vec![]), |c| app.update_ui(c));
    assert!(
        !out.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .iter()
            .any(|c| matches!(c, egui::ViewportCommand::Title(_))),
        "an untouched app's first frame must send no Title command"
    );

    // Frame 2 — a real change.
    app.history
        .commit(Box::new(CreateLine::new(some_line())), &mut app.document);
    let out = ctx.run(raw_input(vec![]), |c| app.update_ui(c));
    assert!(
        out.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .contains(&egui::ViewportCommand::Title(
                "*Untitled.svg - LaserCAD v2".to_owned()
            )),
        "a real change must be visible on the very next frame"
    );

    let out = ctx.run(raw_input(vec![]), |c| app.update_ui(c));
    assert!(
        !out.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .iter()
            .any(|c| matches!(c, egui::ViewportCommand::Title(_))),
        "the title settled, so this idle frame must send nothing"
    );
}

// ---------------------------------------------------------------------------
// AC 3 — Save / Open Recent title before and after
// ---------------------------------------------------------------------------

/// AC 3 — a successful Save clears the `*`.
#[test]
fn successful_save_clears_the_star_in_the_title() {
    let dir = tempdir("save_success");
    let mut app = App {
        settings_path: Some(dir.join("settings.json")),
        autosave_path: Some(dir.join("autosave.json")),
        current_file: Some(dir.join("drawing.svg")),
        ..App::default()
    };
    app.history
        .commit(Box::new(CreateLine::new(some_line())), &mut app.document);
    assert_eq!(app.display_title(), "*drawing.svg - LaserCAD v2");

    app.action_save();

    assert_eq!(app.error_message, None, "the save must succeed");
    assert_eq!(app.display_title(), "drawing.svg - LaserCAD v2");
}

/// AC 3 — a failed Save (unwritable path) leaves the title exactly as it was.
#[test]
fn failed_save_leaves_the_title_unchanged() {
    let mut app = App {
        current_file: Some(PathBuf::from("/nonexistent_dir_lcv138/canary.svg")),
        ..App::default()
    };
    app.history
        .commit(Box::new(CreateLine::new(some_line())), &mut app.document);
    let title_before = app.display_title();

    app.action_save();

    assert!(app.error_message.is_some(), "the save must fail");
    assert_eq!(
        app.display_title(),
        title_before,
        "a failed save must not change the title"
    );
}

/// AC 3 — a successful Open Recent sets the title to the new file, star gone.
#[test]
fn successful_open_path_sets_the_title_to_the_new_file() {
    let dir = tempdir("open_path_success");
    let svg = dir.join("opened.svg");
    std::fs::write(&svg, VALID_SVG).unwrap();
    let mut app = App {
        settings_path: Some(dir.join("settings.json")),
        autosave_path: Some(dir.join("autosave.json")),
        ..App::default()
    };
    assert_eq!(app.display_title(), "Untitled.svg - LaserCAD v2");

    app.action_open_path(svg);

    assert_eq!(app.error_message, None, "the open must succeed");
    assert_eq!(app.display_title(), "opened.svg - LaserCAD v2");
}

/// AC 3 — a failed Open Recent (missing file) leaves the title unchanged.
#[test]
fn failed_open_path_leaves_the_title_unchanged() {
    let mut app = App {
        current_file: Some(PathBuf::from("original.svg")),
        ..App::default()
    };
    app.mark_saved();
    let title_before = app.display_title();

    app.action_open_path(PathBuf::from("/does/not/exist/lcv138.svg"));

    assert!(app.error_message.is_some(), "the open must fail");
    assert_eq!(app.display_title(), title_before);
}

/// AC 3 — cancelling the discard-confirmation dialog runs nothing: the parked
/// `OpenPath` never reaches `action_open_path`, so the title is untouched.
#[test]
fn cancelling_the_discard_dialog_leaves_the_title_unchanged() {
    // The parked path must be one that would genuinely *succeed* if opened —
    // a nonexistent path would leave the title unchanged either way (the
    // open itself would fail), which would let a Cancel-runs-Confirm-anyway
    // bug hide behind a second, unrelated failure.
    let dir = tempdir("cancel_title");
    let would_succeed = dir.join("would_open.svg");
    std::fs::write(&would_succeed, VALID_SVG).unwrap();

    let mut app = App {
        current_file: Some(PathBuf::from("keep.svg")),
        ..App::default()
    };
    app.mark_saved();
    app.history
        .commit(Box::new(CreateLine::new(some_line())), &mut app.document);
    let title_before = app.display_title();

    app.request_open_path(would_succeed.clone());
    assert_eq!(
        app.guard.pending_action,
        Some(PendingAction::OpenPath(would_succeed))
    );

    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |c| {
        lasercad::app::apply_dialog_result(c, &mut app, DialogResult::Cancelled);
    });

    assert!(app.guard.pending_action.is_none());
    assert_eq!(app.display_title(), title_before);
}

/// AC 3 — a cancelled Open… or Save As… dialog cannot have touched
/// `current_file`: both functions return on `None` **before** any
/// title-relevant mutation. Behavioural coverage is impossible here — `rfd`
/// is disarmed outside `crate::run` (ADR 0005) — so this is a source scan,
/// the same shape `src/io/file_actions.rs`'s own tests already use for the
/// dialog half of these two functions.
#[test]
fn cancelled_dialog_returns_before_any_title_relevant_mutation_source_scan() {
    let src = include_str!("../src/io/file_actions.rs");
    for (signature, guard) in [
        ("pub fn action_open(app: &mut App) {", "None => return,"),
        ("pub fn action_save_as(app: &mut App) {", "None => return,"),
    ] {
        let start = src
            .find(signature)
            .unwrap_or_else(|| panic!("{signature} must exist"));
        let body = &src[start..];
        let guard_at = body
            .find(guard)
            .unwrap_or_else(|| panic!("{signature} must guard on {guard:?}"));
        let mutation_at = body.find("current_file = Some").unwrap_or_else(|| {
            panic!("positive control: {signature} must eventually set current_file")
        });
        assert!(
            guard_at < mutation_at,
            "{signature}: a cancelled dialog must return before current_file is touched"
        );
    }
}

// ---------------------------------------------------------------------------
// AC 4 — the recovered-from-autosave label
// ---------------------------------------------------------------------------

/// AC 4 — with `recovered_from_autosave` set (the only way to reach it in a
/// test: `App::new()` cannot be called, ADR 0002 §A2), the label paints, a
/// stationary hover over it paints its explanatory tooltip, and neither
/// `dirty_since` nor `last_autosave_at` is touched by any of this.
///
/// The hover needs no manufactured delay: egui's `tooltip_delay` (0.5 s,
/// `style.rs`) is measured against `PointerState::last_move_time`, which
/// starts at `f64::NEG_INFINITY` and only ever advances once a real,
/// non-zero velocity is measured across several distinct positions
/// (egui-0.29.1 `input_state/mod.rs::PointerState::begin_pass`) — never true
/// here, since the pointer lands once and never moves again.
#[test]
fn recovered_badge_paints_with_its_hover_text_and_touches_nothing_else() {
    let ctx = egui::Context::default();
    let mut app = App {
        title: DocumentTitleState {
            recovered_from_autosave: true,
            ..Default::default()
        },
        ..App::default()
    };
    ctx.set_pixels_per_point(1.0);
    boot(&ctx, &mut app);

    let dirty_before = app.dirty_since;
    let autosave_before = app.last_autosave_at;

    let runs = paint::painted_runs(&ctx, &mut app);
    assert!(
        runs.iter()
            .any(|r| r.text.trim() == "Recovered (not saved)"),
        "the recovery label must be painted while the flag is set"
    );
    let pos = locate(&runs, "Recovered (not saved)");

    let _ = ctx.run(raw_input(vec![egui::Event::PointerMoved(pos)]), |c| {
        app.update_ui(c)
    });
    let out = ctx.run(raw_input(vec![]), |c| app.update_ui(c));
    let hover_runs = paint::runs_in(&out.shapes);
    assert!(
        hover_runs.iter().any(|r| r.text.contains("crash-safety")),
        "hovering the recovery label must paint its explanatory tooltip: {:?}",
        hover_runs.iter().map(|r| &r.text).collect::<Vec<_>>()
    );

    assert_eq!(
        app.dirty_since, dirty_before,
        "rendering the recovery label must not touch dirty_since"
    );
    assert_eq!(
        app.last_autosave_at, autosave_before,
        "rendering the recovery label must not touch last_autosave_at"
    );
}

/// LCV-138 amended AC 4 — a successful Save clears the recovered flag: the
/// drawing that came back from the crash-safety copy has now actually been
/// written to a file.
#[test]
fn action_save_clears_recovered_from_autosave() {
    let dir = tempdir("recovered_save");
    let mut app = App {
        settings_path: Some(dir.join("settings.json")),
        autosave_path: Some(dir.join("autosave.json")),
        current_file: Some(dir.join("drawing.svg")),
        title: DocumentTitleState {
            recovered_from_autosave: true,
            ..Default::default()
        },
        ..App::default()
    };
    app.history
        .commit(Box::new(CreateLine::new(some_line())), &mut app.document);

    app.action_save();

    assert_eq!(app.error_message, None, "the save must succeed");
    assert!(
        !app.title.recovered_from_autosave,
        "a successful save must clear the recovered label"
    );
}

/// LCV-138 amended AC 4 — a successful Open Recent clears the recovered
/// flag too: the operator has replaced the recovered drawing outright.
#[test]
fn action_open_path_clears_recovered_from_autosave() {
    let dir = tempdir("recovered_open_path");
    let svg = dir.join("opened.svg");
    std::fs::write(&svg, VALID_SVG).unwrap();
    let mut app = App {
        settings_path: Some(dir.join("settings.json")),
        autosave_path: Some(dir.join("autosave.json")),
        title: DocumentTitleState {
            recovered_from_autosave: true,
            ..Default::default()
        },
        ..App::default()
    };

    app.action_open_path(svg);

    assert_eq!(app.error_message, None, "the open must succeed");
    assert!(
        !app.title.recovered_from_autosave,
        "a successful Open Recent must clear the recovered label"
    );
}

/// LCV-138 amended AC 4 — File > New clears the recovered flag as well: the
/// operator has discarded the recovered drawing for a blank one.
#[test]
fn action_new_clears_recovered_from_autosave() {
    let dir = tempdir("recovered_new");
    let mut app = App {
        settings_path: Some(dir.join("settings.json")),
        autosave_path: Some(dir.join("autosave.json")),
        title: DocumentTitleState {
            recovered_from_autosave: true,
            ..Default::default()
        },
        ..App::default()
    };

    app.action_new();

    assert!(
        !app.title.recovered_from_autosave,
        "File > New must clear the recovered label"
    );
}

/// LCV-138 amended AC 4, the companion regression at the app-action level:
/// an autosave flush alone (`mark_clean()`, no Save/Open/New) must leave the
/// recovered flag set — a crash-safety write is not the operator saving,
/// matching AC 3's guarantee that autosave never clears the unsaved marker
/// either.
#[test]
fn autosave_flush_leaves_recovered_from_autosave_set() {
    let mut app = App {
        title: DocumentTitleState {
            recovered_from_autosave: true,
            ..Default::default()
        },
        ..App::default()
    };
    app.history
        .commit(Box::new(CreateLine::new(some_line())), &mut app.document);

    app.mark_clean(); // what flush_if_due calls after every write

    assert!(
        app.title.recovered_from_autosave,
        "an autosave flush must not clear the recovered label"
    );
}

// ---------------------------------------------------------------------------
// AC 5 — Open Recent disambiguation and honest failure
// ---------------------------------------------------------------------------

/// AC 5 — two entries sharing a basename in different directories both show
/// enough of their parent path to tell them apart; the bare, ambiguous
/// basename never appears on its own once it collides.
#[test]
fn open_recent_disambiguates_colliding_basenames_via_the_real_menu() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    ctx.set_pixels_per_point(1.0);
    boot(&ctx, &mut app);

    app.settings.recent_files = vec![
        "/home/op/cuts/plate.svg".to_owned(),
        "/home/op/marks/plate.svg".to_owned(),
    ];

    let runs = open_recent_submenu(&ctx, &mut app);
    let labels: Vec<&str> = runs.iter().map(|r| r.text.trim()).collect();
    assert!(labels.contains(&"cuts/plate.svg"), "{labels:?}");
    assert!(labels.contains(&"marks/plate.svg"), "{labels:?}");
    assert!(
        !labels.contains(&"plate.svg"),
        "the bare, ambiguous basename must not appear once it collides: {labels:?}"
    );
}

/// AC 5 — replaces the source-scan-only proof
/// (`recent_submenu_hover_text_is_the_full_path_source_scan`,
/// `src/ui/menubar.rs`) with a real painted hover, mirroring the
/// recovered-badge hover test above: a source scan proves the
/// `.on_hover_text(entry)` call is *written*, not that the tooltip actually
/// paints (AGENTS.md "a rendering acceptance criterion is not satisfied by a
/// source scan alone").
#[test]
fn open_recent_entry_hover_text_paints_the_full_path() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    ctx.set_pixels_per_point(1.0);
    boot(&ctx, &mut app);

    app.settings.recent_files = vec!["/home/op/jobs/plate.svg".to_owned()];

    let runs = open_recent_submenu(&ctx, &mut app);
    let pos = locate(&runs, "plate.svg");

    let _ = ctx.run(raw_input(vec![egui::Event::PointerMoved(pos)]), |c| {
        app.update_ui(c)
    });
    // Unlike the recovered-badge hover test above, the pointer here has
    // already moved twice before landing on this entry (onto "File", then
    // onto "Open Recent ▶"), so egui's pointer-velocity window
    // (`emath::History`, up to 0.1 s / 3 samples) is still warm and
    // `last_move_time` keeps advancing for a few more frames even though the
    // pointer itself has stopped. A handful of idle frames lets that window
    // flush and `tooltip_delay` (0.5 s, in ~1/60 s `predicted_dt` steps)
    // elapse for real, exactly as an operator's own stationary cursor would.
    let mut out = ctx.run(raw_input(vec![]), |c| app.update_ui(c));
    for _ in 0..45 {
        out = ctx.run(raw_input(vec![]), |c| app.update_ui(c));
    }
    let hover_runs = paint::runs_in(&out.shapes);
    assert!(
        hover_runs
            .iter()
            .any(|r| r.text.trim() == "/home/op/jobs/plate.svg"),
        "hovering an Open Recent entry must paint its full path: {:?}",
        hover_runs.iter().map(|r| &r.text).collect::<Vec<_>>()
    );
}

/// AC 5 — a missing file selected from Open Recent preserves the drawing,
/// history, `current_file` and the recent-files order, and surfaces
/// `error_message`; the failing entry is not removed from the list.
#[test]
fn open_recent_missing_file_preserves_state_and_surfaces_the_error() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    ctx.set_pixels_per_point(1.0);
    boot(&ctx, &mut app);

    app.current_file = Some(PathBuf::from("original.svg"));
    app.history
        .commit(Box::new(CreateLine::new(some_line())), &mut app.document);
    app.mark_saved();
    let before_entities = app.document.entities.clone();
    let before_revision = app.history.revision();

    let dir = tempdir("open_recent_missing");
    let missing = dir.join("does_not_exist.svg");
    let present = dir.join("present.svg");
    std::fs::write(&present, VALID_SVG).unwrap();
    // The failing entry sits *second*, deliberately not at the front: a
    // premature reorder-to-front bug (promoting on click, before the file is
    // even read) is invisible if the failing entry already sits at index 0,
    // since moving the front entry to the front is a no-op. Second is what
    // makes "order unchanged" a real assertion.
    let recent_before = vec![
        present.to_string_lossy().into_owned(),
        missing.to_string_lossy().into_owned(),
    ];
    app.settings.recent_files = recent_before.clone();

    let runs = open_recent_submenu(&ctx, &mut app);
    let missing_label = missing.file_name().unwrap().to_str().unwrap();
    let pos = locate(&runs, missing_label);
    click_button(&ctx, &mut app, pos);

    assert_eq!(
        app.document.entities, before_entities,
        "a failed open must not clear the drawing"
    );
    assert_eq!(app.history.revision(), before_revision);
    assert_eq!(app.current_file, Some(PathBuf::from("original.svg")));
    assert!(app.error_message.is_some(), "the failure must surface");
    assert_eq!(
        app.settings.recent_files, recent_before,
        "a failed open must not reorder the recent-files list"
    );
}

/// AC 5 — same shape, for an entry whose file exists but fails to parse.
#[test]
fn open_recent_malformed_svg_preserves_state_and_surfaces_the_error() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    ctx.set_pixels_per_point(1.0);
    boot(&ctx, &mut app);

    app.current_file = Some(PathBuf::from("original.svg"));
    app.history
        .commit(Box::new(CreateLine::new(some_line())), &mut app.document);
    app.mark_saved();
    let before_entities = app.document.entities.clone();
    let before_revision = app.history.revision();

    let dir = tempdir("open_recent_malformed");
    let malformed = dir.join("malformed.svg");
    let present = dir.join("present.svg");
    std::fs::write(&malformed, MALFORMED_SVG).unwrap();
    std::fs::write(&present, VALID_SVG).unwrap();
    // The failing entry sits second, not at the front — see the sibling
    // missing-file test above for why that is what makes "order unchanged" a
    // real assertion rather than a vacuous one.
    let recent_before = vec![
        present.to_string_lossy().into_owned(),
        malformed.to_string_lossy().into_owned(),
    ];
    app.settings.recent_files = recent_before.clone();

    let runs = open_recent_submenu(&ctx, &mut app);
    let label = malformed.file_name().unwrap().to_str().unwrap();
    let pos = locate(&runs, label);
    click_button(&ctx, &mut app, pos);

    assert_eq!(app.document.entities, before_entities);
    assert_eq!(app.history.revision(), before_revision);
    assert_eq!(app.current_file, Some(PathBuf::from("original.svg")));
    assert!(app.error_message.is_some());
    assert_eq!(app.settings.recent_files, recent_before);
}
