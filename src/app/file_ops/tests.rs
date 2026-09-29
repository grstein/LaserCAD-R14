use super::*;
use crate::document::{CreateLine, Entity};
use crate::geometry::{Line, Vec2};

fn some_line() -> Line {
    Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0))
}

fn commit_a_line(app: &mut App) {
    app.history
        .commit(Box::new(CreateLine::new(some_line())), &mut app.document);
}

// -- AC 1, 2, 14 — has_unsaved_changes -----------------------------------

/// AC 1 — `App::default().guard.saved_revision` is `None`.
#[test]
fn app_default_saved_revision_is_none() {
    assert_eq!(App::default().guard.saved_revision, None);
}

/// AC 5 — `App::default().guard.pending_action` is `None`.
#[test]
fn app_default_pending_action_is_none() {
    assert_eq!(App::default().guard.pending_action, None);
}

/// AC 2 — the truth table in the demand body, one assertion per row.
/// Fresh `App::default()` per row so each is independent and unambiguous.
#[test]
fn has_unsaved_changes_truth_table() {
    // Row 1 — App::default(): blank document, saved_revision == None.
    let app = App::default();
    assert!(!app.has_unsaved_changes(), "row 1: blank + None is saved");

    // Row 2 — blank, one history.commit, saved_revision still None.
    let mut app = App::default();
    commit_a_line(&mut app);
    assert!(
        app.has_unsaved_changes(),
        "row 2: a committed entity with None must report unsaved"
    );

    // Row 3 — after mark_saved() at the current revision.
    app.mark_saved();
    assert!(!app.has_unsaved_changes(), "row 3: freshly marked saved");

    // Row 4 — after mark_saved() then one more commit.
    commit_a_line(&mut app);
    assert!(
        app.has_unsaved_changes(),
        "row 4: a commit after mark_saved must report unsaved"
    );

    // Row 5 — mark_saved() then mark_clean() with no intervening commit
    // (an autosave flush with nothing new to flush) stays saved; a commit
    // after that is unsaved again.
    let mut app = App::default();
    commit_a_line(&mut app);
    app.mark_saved();
    app.mark_clean();
    assert!(
        !app.has_unsaved_changes(),
        "row 5a: mark_clean alone must not un-save a saved document"
    );
    commit_a_line(&mut app);
    assert!(
        app.has_unsaved_changes(),
        "row 5b: a commit after that mark_clean is unsaved again"
    );

    // Row 6 — the regression row this demand exists to fix: mark_saved(),
    // one commit, then mark_clean() (the autosave flushed the change)
    // must still report unsaved. The change was autosaved, not saved.
    let mut app = App::default();
    app.mark_saved();
    commit_a_line(&mut app);
    app.mark_clean();
    assert!(
        app.has_unsaved_changes(),
        "row 6: autosaving a change must not count as saving it"
    );

    // Row 7 — after mark_saved() then history.undo: undoing away from the
    // saved revision is still unsaved (History::revision() is monotonic).
    let mut app = App::default();
    commit_a_line(&mut app);
    app.mark_saved();
    assert!(app.history.undo(&mut app.document));
    assert!(
        app.has_unsaved_changes(),
        "row 7: undo away from the saved revision is unsaved"
    );

    // Row 8 — saved_revision == None with entities pushed directly: the
    // autosave-recovery simulation (App::new() cannot be called from a
    // test, ADR 0002 §A2).
    let mut app = App::default();
    app.document.entities.push(Entity::Line(some_line()));
    assert!(
        app.has_unsaved_changes(),
        "row 8: a recovered document with entities and no saved_revision is unsaved"
    );
}

/// AC 14 — named regression test: an autosave flush must not suppress the
/// discard prompt. `dirty_since` returns to `None` on every flush
/// (`flush_if_due` calls `mark_clean()`), but `mark_clean()` never
/// touches `saved_revision`, so the predicate is unaffected. This is the
/// criterion "most likely to be optimised away" per the demand.
#[test]
fn autosave_flush_does_not_clear_unsaved_changes() {
    let mut app = App::default();
    app.mark_saved();
    commit_a_line(&mut app);

    app.mark_clean(); // what flush_if_due calls after every write

    assert!(app.dirty_since.is_none(), "the autosave signal is clean");
    assert!(
        app.has_unsaved_changes(),
        "dirty_since is the wrong signal: an autosaved-but-not-saved-to-\
             file document must still report unsaved changes"
    );
}

// -- AC 3, 4 — mark_saved -------------------------------------------------

/// AC 3 — mark_saved sets saved_revision to the current revision and
/// clears the autosave debounce via mark_clean.
#[test]
fn mark_saved_sets_revision_and_clears_dirty() {
    let mut app = App::default();
    commit_a_line(&mut app);
    app.sync_dirty();
    assert!(app.dirty_since.is_some());

    app.mark_saved();

    assert_eq!(app.guard.saved_revision, Some(app.history.revision()));
    assert!(app.dirty_since.is_none());
    assert_eq!(app.last_synced_revision, app.history.revision());
}

/// LCV-138 amended AC 4 — `mark_saved` is the only clearer of
/// `title.recovered_from_autosave`: once the document is certified safe
/// to discard, the "recovered and unsaved" label has stopped being true.
#[test]
fn mark_saved_clears_recovered_from_autosave() {
    let mut app = App {
        title: crate::app::DocumentTitleState {
            recovered_from_autosave: true,
            ..Default::default()
        },
        ..App::default()
    };

    app.mark_saved();

    assert!(!app.title.recovered_from_autosave);
}

/// LCV-138 amended AC 4, the companion regression: an autosave flush
/// (`mark_clean()` alone, with no `mark_saved()`) must leave the flag
/// set — a crash-safety write is not the operator saving, matching AC 3's
/// guarantee that autosave never clears the unsaved marker either.
#[test]
fn mark_clean_alone_does_not_clear_recovered_from_autosave() {
    let mut app = App {
        title: crate::app::DocumentTitleState {
            recovered_from_autosave: true,
            ..Default::default()
        },
        ..App::default()
    };

    app.mark_clean();

    assert!(app.title.recovered_from_autosave);
}

// -- AC 5, 6 — the guarded entry points ----------------------------------

/// AC 5, 6 — a clean document runs the action immediately and leaves
/// nothing parked.
#[test]
fn request_new_on_clean_document_acts_immediately() {
    let mut app = App::default();
    commit_a_line(&mut app);
    app.mark_saved();
    app.current_file = Some(PathBuf::from("clean.svg"));
    assert!(!app.has_unsaved_changes());

    app.request_new();

    assert_eq!(app.document.entity_count(), 0, "action_new must have run");
    assert!(app.current_file.is_none(), "action_new resets current_file");
    assert!(app.guard.pending_action.is_none());
}

/// AC 5, 6 — a dirty document parks `PendingAction::New` and changes
/// nothing: document, history and pending_action are the only things
/// touched.
#[test]
fn request_new_on_dirty_document_parks_and_changes_nothing() {
    let mut app = App::default();
    commit_a_line(&mut app);
    assert!(app.has_unsaved_changes());
    let revision_before = app.history.revision();

    app.request_new();

    assert_eq!(app.guard.pending_action, Some(PendingAction::New));
    assert_eq!(app.document.entity_count(), 1, "document must be untouched");
    assert_eq!(app.history.revision(), revision_before);
}

/// AC 5, 6 — a dirty document parks `PendingAction::Open` without ever
/// calling `action_open`, so no native `rfd` dialog opens and the test
/// cannot hang (ADR 0002 §A4 rule 1).
#[test]
fn request_open_on_dirty_document_parks_without_touching_the_filesystem() {
    let mut app = App::default();
    commit_a_line(&mut app);

    app.request_open();

    assert_eq!(app.guard.pending_action, Some(PendingAction::Open));
    assert_eq!(app.document.entity_count(), 1, "document must be untouched");
}

/// AC 6 — `request_exit` returns `false` and parks on a dirty document.
#[test]
fn request_exit_returns_false_and_parks_when_dirty() {
    let mut app = App::default();
    commit_a_line(&mut app);

    assert!(!app.request_exit());
    assert_eq!(app.guard.pending_action, Some(PendingAction::Exit));
}

/// AC 6 — `request_exit` returns `true` and parks nothing on a clean
/// document.
#[test]
fn request_exit_returns_true_when_clean() {
    let mut app = App::default();
    assert!(app.request_exit());
    assert!(app.guard.pending_action.is_none());
}

/// AC 6 — a second request while one is already parked is a no-op: it
/// neither swaps the parked action nor queues a new one.
#[test]
fn second_request_while_pending_is_a_no_op() {
    let mut app = App::default();
    commit_a_line(&mut app);

    app.request_new();
    assert_eq!(app.guard.pending_action, Some(PendingAction::New));

    app.request_open();
    assert_eq!(
        app.guard.pending_action,
        Some(PendingAction::New),
        "a second request must not replace the parked action"
    );

    assert!(!app.request_exit());
    assert_eq!(
        app.guard.pending_action,
        Some(PendingAction::New),
        "request_exit must not park while another action is already pending"
    );
}

// -- AC 15 — purity ------------------------------------------------------

/// AC 15 — `file_ops.rs` imports neither `eframe` nor `rfd`.
#[test]
fn file_ops_does_not_import_eframe_or_rfd() {
    let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/file_ops.rs"));
    for line in src.lines() {
        let trimmed = line.trim_start();
        assert!(
            !trimmed.starts_with("use eframe") && !trimmed.starts_with("use rfd"),
            "file_ops.rs must not import eframe or rfd: {trimmed:?}"
        );
    }
}

// -- AC 3, 4 — promoted static checks (review finding) -------------------
//
// Both were hand-run greps until a mutation review proved that "hand-run"
// is not a gate: adding `app.guard.saved_revision = Some(app.history.revision())`
// inside `flush_if_due`'s due branch passed all 746 library tests. A
// source scan, unlike a grep in a PR description, runs on every
// `cargo test`.

/// AC 3 — `src/app/autosave.rs` must never mention `saved_revision`.
/// Autosave (`flush_if_due` / `sync_dirty` / `mark_clean`) is crash
/// recovery, not a save — the file on disk, if any, is still stale — so
/// it must never learn about the safe-to-discard signal. This is the
/// exact regression a review mutation caught with zero test failures
/// before this test existed: `flush_if_due` is the one call site the
/// whole demand exists to keep honest, and nothing else guarded it.
#[test]
fn autosave_never_mentions_saved_revision() {
    let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/autosave.rs"));
    assert!(
        !src.contains("saved_revision"),
        "src/app/autosave.rs must not reference saved_revision at all (AC 3)"
    );
}

/// AC 4 — `src/io/file_actions.rs` must never call the bare
/// `mark_clean` the five file actions called before this demand; each
/// now calls `mark_saved`, which clears the autosave debounce *and*
/// marks the document safe to discard.
#[test]
fn file_actions_never_calls_mark_clean() {
    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/io/file_actions.rs"
    ));
    assert!(
        !src.contains("mark_clean"),
        "src/io/file_actions.rs must not call mark_clean (AC 4): every \
             file action must call mark_saved instead"
    );
}
