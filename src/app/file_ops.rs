//! Guarded destructive entry points: New, Open, Exit (LCV-113).
//!
//! Every destructive entry point in the app used to run unconditionally:
//! File > New / Ctrl+N discarded the document *and* the undo history without
//! asking, File > Open / Ctrl+O / Open Recent replaced them the moment the
//! native dialog returned, and File > Exit / the window close button ended
//! the process outright. This file wires the existing, previously-uncalled
//! `confirm_dialog` (`src/ui/dialogs.rs`) to all three, alongside the five
//! `App` file-action wrappers already living here (moved out of
//! `src/app/mod.rs` in the previous commit to make room under the 300-LOC
//! implementation cap).
//!
//! Two mechanisms, kept apart on purpose:
//!
//! - [`App::has_unsaved_changes`] / [`App::mark_saved`] — the safe-to-discard
//!   signal. `App::saved_revision` is the `history.revision()` at which the
//!   document was last known safe to discard (just written to a file, just
//!   loaded from one, or just reset to blank). It is deliberately **not**
//!   `dirty_since`: that field is an 800 ms autosave debounce timer that
//!   returns to `None` on every flush (`src/app/autosave.rs::flush_if_due`),
//!   so a guard built on it would fire only inside the sub-second window
//!   between a change and the next autosave write. Autosave is crash
//!   recovery, not a save — the file on disk, if any, is still stale — so
//!   `flush_if_due` keeps calling `mark_clean()` alone and never learns about
//!   `saved_revision`.
//! - [`PendingAction`] / [`App::pending_action`] — the destructive action
//!   parked while the confirmation dialog is up. `confirm_dialog` is
//!   immediate-mode, so this parked value *is* the whole state machine: the
//!   four `request_*` guards below run the action immediately on a clean
//!   document and park it on a dirty one, [`draw_discard_dialog`] renders the
//!   dialog only while something is parked, and [`apply_dialog_result`] is
//!   the one place that clears it, on both the Discard and the Cancel path.
//!
//! MUST NOT import `eframe` or `rfd`. `request_open` / `request_open_path`
//! reach `crate::io::action_open` / `action_open_path` only on a clean
//! document, so no path here can be reached in a headless test without the
//! test itself proving the document dirty first (ADR 0002 §A4 rule 1 /
//! ADR 0003 §F3 trap 1: those two functions open a blocking native `rfd`
//! dialog and would hang the run).

use std::path::PathBuf;

use crate::ui::DialogResult;

use super::App;

// ---------------------------------------------------------------------------
// The parked action
// ---------------------------------------------------------------------------

/// A destructive action parked while the discard-confirmation dialog is up.
///
/// `None` on [`App::pending_action`] means no dialog is pending. Carrying
/// `OpenPath`'s argument here (rather than re-deriving it from
/// `App::current_file` or similar) is what lets Open Recent's guard be a
/// single variant instead of a special case.
#[derive(Debug, PartialEq)]
pub enum PendingAction {
    /// File > New / Ctrl+N.
    New,
    /// File > Open… / Ctrl+O.
    Open,
    /// One File > Open Recent entry, carrying the path chosen.
    OpenPath(PathBuf),
    /// File > Exit / the window close button.
    Exit,
}

impl App {
    // -----------------------------------------------------------------------
    // The safe-to-discard signal
    // -----------------------------------------------------------------------

    /// `true` when discarding the in-memory document right now would lose
    /// work. See the module header for why this is not `dirty_since`.
    ///
    /// - `saved_revision == Some(r)`: unsaved iff the document has moved on
    ///   from revision `r` since the last save/load/reset.
    /// - `saved_revision == None`: the document was never marked safe (a
    ///   fresh unsaved document, or one recovered from autosave at boot), so
    ///   it is unsaved iff it is non-empty. An empty, never-saved document
    ///   has nothing to lose.
    pub fn has_unsaved_changes(&self) -> bool {
        match self.saved_revision {
            Some(saved) => self.history.revision() != saved,
            None => self.document.entity_count() > 0,
        }
    }

    /// Mark the document safe to discard at its current revision, and clear
    /// the autosave debounce (`mark_clean`) along with it.
    ///
    /// The **only** writer of `saved_revision`. Called by the five file
    /// actions in `src/io/file_actions.rs` — `action_new`, `action_open`,
    /// `action_open_path`, `action_save`, `action_save_as` — in place of the
    /// bare `mark_clean()` they used to call, always after any `history`
    /// replacement so `history.revision()` reads the fresh history's value.
    pub fn mark_saved(&mut self) {
        self.saved_revision = Some(self.history.revision());
        self.mark_clean();
    }

    // -----------------------------------------------------------------------
    // Guarded entry points
    // -----------------------------------------------------------------------

    /// Guarded File > New / Ctrl+N. Runs [`App::action_new`] immediately on a
    /// clean document; parks [`PendingAction::New`] on a dirty one, changing
    /// nothing else. A no-op while another action is already parked, so a
    /// second Ctrl+N cannot queue a second action or swap the parked one.
    pub fn request_new(&mut self) {
        if self.pending_action.is_some() {
            return;
        }
        if self.has_unsaved_changes() {
            self.pending_action = Some(PendingAction::New);
        } else {
            self.action_new();
        }
    }

    /// Guarded File > Open… / Ctrl+O. Runs [`App::action_open`] — which opens
    /// the native file dialog — immediately on a clean document; parks
    /// [`PendingAction::Open`] on a dirty one without touching the
    /// filesystem. A no-op while another action is already parked.
    pub fn request_open(&mut self) {
        if self.pending_action.is_some() {
            return;
        }
        if self.has_unsaved_changes() {
            self.pending_action = Some(PendingAction::Open);
        } else {
            self.action_open();
        }
    }

    /// Guarded File > Open Recent entry. Runs [`App::action_open_path`]
    /// immediately on a clean document; parks
    /// [`PendingAction::OpenPath`]`(path)` on a dirty one. A no-op while
    /// another action is already parked.
    pub fn request_open_path(&mut self, path: PathBuf) {
        if self.pending_action.is_some() {
            return;
        }
        if self.has_unsaved_changes() {
            self.pending_action = Some(PendingAction::OpenPath(path));
        } else {
            self.action_open_path(path);
        }
    }

    /// Guarded File > Exit / the window close button.
    ///
    /// Returns `true` when the document is clean — the caller must close the
    /// window — and `false` after parking [`PendingAction::Exit`], meaning
    /// the caller must cancel the close. `bool`, not unit like the other
    /// three guards, because this is the only one whose *action* needs an
    /// `egui::Context` (`ViewportCommand::Close`) that the caller already
    /// holds; the other three perform their action directly. A no-op
    /// returning `false` while another action is already parked, so a second
    /// click on the window X cannot park on top of an already-parked action.
    pub fn request_exit(&mut self) -> bool {
        if self.pending_action.is_some() {
            return false;
        }
        if self.has_unsaved_changes() {
            self.pending_action = Some(PendingAction::Exit);
            false
        } else {
            true
        }
    }

    // -----------------------------------------------------------------------
    // File actions — thin `App` wrappers over `crate::io::file_actions`
    // (moved verbatim from `src/app/mod.rs`; see ADR 0002 "300-LOC cap")
    // -----------------------------------------------------------------------

    /// Create a new, empty document (LCV-062).
    pub fn action_new(&mut self) {
        crate::io::action_new(self);
    }

    /// Open a document from disk via a file dialog (LCV-062).
    pub fn action_open(&mut self) {
        crate::io::action_open(self);
    }

    /// Load a document from a known path (LCV-065, used by Open Recent).
    pub fn action_open_path(&mut self, path: PathBuf) {
        crate::io::action_open_path(self, path);
    }

    /// Save the current document to disk (LCV-062).
    pub fn action_save(&mut self) {
        crate::io::action_save(self);
    }

    /// Save the current document to a new path via a save dialog (LCV-065).
    pub fn action_save_as(&mut self) {
        crate::io::action_save_as(self);
    }
}

// ---------------------------------------------------------------------------
// The dialog driver
// ---------------------------------------------------------------------------

/// Render the discard-confirmation dialog. Renders nothing while
/// `App::pending_action` is `None`; on a click, delegates the decision to
/// [`apply_dialog_result`].
///
/// Called from [`super::panels::draw_dialogs`]. The render and the decision
/// are split on purpose (not inlined here): it is what makes
/// [`apply_dialog_result`] testable directly, with no simulated pointer
/// click, which is how the Discard and Cancel behaviour is actually covered.
pub fn draw_discard_dialog(ctx: &egui::Context, app: &mut App) {
    if app.pending_action.is_none() {
        return;
    }
    if let Some(result) = crate::ui::confirm_dialog(
        ctx,
        "Discard unsaved changes?",
        "The current drawing has unsaved changes. Continuing will discard them and the undo history.",
        "Discard",
        "Cancel",
    ) {
        apply_dialog_result(ctx, app, result);
    }
}

/// Apply the operator's answer to the parked action.
///
/// Takes `App::pending_action` unconditionally, so both branches clear it —
/// forgetting to clear it on either one would leave a dialog that reopens
/// every frame. On [`DialogResult::Confirmed`] the parked action runs exactly
/// once (`Exit` sends `ViewportCommand::Close`); on
/// [`DialogResult::Cancelled`] nothing else happens and no viewport command
/// is sent.
pub fn apply_dialog_result(ctx: &egui::Context, app: &mut App, result: DialogResult) {
    let action = app.pending_action.take();
    if result != DialogResult::Confirmed {
        return;
    }
    match action {
        Some(PendingAction::New) => app.action_new(),
        Some(PendingAction::Open) => app.action_open(),
        Some(PendingAction::OpenPath(path)) => app.action_open_path(path),
        Some(PendingAction::Exit) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
        None => {}
    }
}

// ---------------------------------------------------------------------------
// The window close button
// ---------------------------------------------------------------------------

/// Poll the window close button (the viewport X) once per frame.
///
/// Called from [`App::update_ui`](super::App::update_ui), immediately before
/// [`super::panels::draw_dialogs`]. When
/// `ctx.input(|i| i.viewport().close_requested())` is true this calls
/// [`App::request_exit`]; if that returns `false` (the document is dirty),
/// `ViewportCommand::CancelClose` is sent in the *same* frame — verified
/// against eframe 0.29.1 (`native/epi_integration.rs:284-295`): the
/// integration checks that frame's `viewport_output[ROOT].commands` for
/// `CancelClose` and closes the window otherwise. On a clean document nothing
/// is sent and the window closes normally.
pub fn poll_close_request(ctx: &egui::Context, app: &mut App) {
    if !ctx.input(|i| i.viewport().close_requested()) {
        return;
    }
    if !app.request_exit() {
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
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

    /// AC 1 — `App::default().saved_revision` is `None`.
    #[test]
    fn app_default_saved_revision_is_none() {
        assert_eq!(App::default().saved_revision, None);
    }

    /// AC 5 — `App::default().pending_action` is `None`.
    #[test]
    fn app_default_pending_action_is_none() {
        assert_eq!(App::default().pending_action, None);
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

        assert_eq!(app.saved_revision, Some(app.history.revision()));
        assert!(app.dirty_since.is_none());
        assert_eq!(app.last_synced_revision, app.history.revision());
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
        assert!(app.pending_action.is_none());
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

        assert_eq!(app.pending_action, Some(PendingAction::New));
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

        assert_eq!(app.pending_action, Some(PendingAction::Open));
        assert_eq!(app.document.entity_count(), 1, "document must be untouched");
    }

    /// AC 6 — `request_exit` returns `false` and parks on a dirty document.
    #[test]
    fn request_exit_returns_false_and_parks_when_dirty() {
        let mut app = App::default();
        commit_a_line(&mut app);

        assert!(!app.request_exit());
        assert_eq!(app.pending_action, Some(PendingAction::Exit));
    }

    /// AC 6 — `request_exit` returns `true` and parks nothing on a clean
    /// document.
    #[test]
    fn request_exit_returns_true_when_clean() {
        let mut app = App::default();
        assert!(app.request_exit());
        assert!(app.pending_action.is_none());
    }

    /// AC 6 — a second request while one is already parked is a no-op: it
    /// neither swaps the parked action nor queues a new one.
    #[test]
    fn second_request_while_pending_is_a_no_op() {
        let mut app = App::default();
        commit_a_line(&mut app);

        app.request_new();
        assert_eq!(app.pending_action, Some(PendingAction::New));

        app.request_open();
        assert_eq!(
            app.pending_action,
            Some(PendingAction::New),
            "a second request must not replace the parked action"
        );

        assert!(!app.request_exit());
        assert_eq!(
            app.pending_action,
            Some(PendingAction::New),
            "request_exit must not park while another action is already pending"
        );
    }

    // -- AC 12, 13 — apply_dialog_result --------------------------------------

    /// AC 12 — Confirmed runs the parked action exactly once and clears
    /// `pending_action`, for both a document action (`New`) and `Exit`
    /// (asserted via the returned `FullOutput`'s viewport commands, which is
    /// why this drives `apply_dialog_result` inside a `ctx.run` closure).
    #[test]
    fn confirm_runs_the_parked_action_once() {
        let ctx = egui::Context::default();

        let mut app = App::default();
        commit_a_line(&mut app);
        app.pending_action = Some(PendingAction::New);
        let out = ctx.run(egui::RawInput::default(), |ctx| {
            apply_dialog_result(ctx, &mut app, DialogResult::Confirmed);
        });
        assert!(app.pending_action.is_none());
        assert_eq!(
            app.document.entity_count(),
            0,
            "action_new must have run exactly once"
        );
        assert!(!out.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .contains(&egui::ViewportCommand::Close));

        let mut exit_app = App {
            pending_action: Some(PendingAction::Exit),
            ..App::default()
        };
        let out = ctx.run(egui::RawInput::default(), |ctx| {
            apply_dialog_result(ctx, &mut exit_app, DialogResult::Confirmed);
        });
        assert!(exit_app.pending_action.is_none());
        assert!(
            out.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .contains(&egui::ViewportCommand::Close),
            "a confirmed Exit must send ViewportCommand::Close"
        );
    }

    /// AC 13 — Cancelled clears `pending_action` and leaves every other field
    /// exactly as it was, sending no viewport command even when the parked
    /// action was `Exit`.
    #[test]
    fn cancel_restores_nothing_and_clears_the_pending_action() {
        let ctx = egui::Context::default();
        let mut app = App::default();
        commit_a_line(&mut app);
        app.mark_saved();
        app.current_file = Some(PathBuf::from("keep.svg"));
        app.pending_action = Some(PendingAction::Exit);

        let entity_count = app.document.entity_count();
        let revision = app.history.revision();
        let current_file = app.current_file.clone();
        let saved_revision = app.saved_revision;
        let dirty_since = app.dirty_since;

        let out = ctx.run(egui::RawInput::default(), |ctx| {
            apply_dialog_result(ctx, &mut app, DialogResult::Cancelled);
        });

        assert!(app.pending_action.is_none());
        assert_eq!(app.document.entity_count(), entity_count);
        assert_eq!(app.history.revision(), revision);
        assert_eq!(app.current_file, current_file);
        assert_eq!(app.saved_revision, saved_revision);
        assert_eq!(app.dirty_since, dirty_since);
        assert!(
            !out.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .contains(&egui::ViewportCommand::Close),
            "Cancel must never send a viewport command"
        );
    }

    // -- AC 15 — purity ------------------------------------------------------

    /// AC 15 — `file_ops.rs` imports neither `eframe` nor `rfd`.
    #[test]
    fn file_ops_does_not_import_eframe_or_rfd() {
        let src = include_str!("file_ops.rs");
        for line in src.lines() {
            let trimmed = line.trim_start();
            assert!(
                !trimmed.starts_with("use eframe") && !trimmed.starts_with("use rfd"),
                "file_ops.rs must not import eframe or rfd: {trimmed:?}"
            );
        }
    }
}
