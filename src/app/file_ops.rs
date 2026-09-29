//! Guarded destructive entry points: New, Open, Exit (LCV-113).
//!
//! Every destructive entry point in the app used to run unconditionally:
//! File > New / Ctrl+N discarded the document *and* the undo history without
//! asking, File > Open / Ctrl+O / Open Recent replaced them the moment the
//! native dialog returned, and File > Exit / the window close button ended
//! the process outright. This file owns the safe-to-discard signal, the
//! parked-action state machine's four guards and the five `App` file-action
//! wrappers; the egui half — rendering `confirm_dialog`
//! (`src/ui/dialogs.rs`), applying the operator's answer and polling the
//! window close button — lives in `src/app/discard.rs` (split out, ADR 0004
//! §"The `src/app/mod.rs` seam", when this file reached the 300-LOC
//! implementation cap and LCV-138 needed to add a line to it).
//!
//! Two mechanisms, kept apart on purpose:
//!
//! - [`App::has_unsaved_changes`] / [`App::mark_saved`] — the safe-to-discard
//!   signal. `UnsavedGuard::saved_revision` (`app.guard.saved_revision`,
//!   `src/app/unsaved_guard.rs`) is the `history.revision()` at which the
//!   document was last known safe to discard (just written to a file, just
//!   loaded from one, or just reset to blank). It is deliberately **not**
//!   `dirty_since`: that field is an 800 ms autosave debounce timer that
//!   returns to `None` on every flush (`src/app/autosave.rs::flush_if_due`),
//!   so a guard built on it would fire only inside the sub-second window
//!   between a change and the next autosave write. Autosave is crash
//!   recovery, not a save — the file on disk, if any, is still stale — so
//!   `flush_if_due` keeps calling `mark_clean()` alone and never learns about
//!   `saved_revision`.
//! - [`PendingAction`] / `UnsavedGuard::pending_action` — the destructive
//!   action parked while the confirmation dialog is up. `confirm_dialog` is
//!   immediate-mode, so this parked value *is* the whole state machine: the
//!   four `request_*` guards below run the action immediately on a clean
//!   document and park it on a dirty one; `discard.rs`'s
//!   `draw_discard_dialog` renders the dialog only while something is
//!   parked, and its `apply_dialog_result` is the one place that clears it,
//!   on both the Discard and the Cancel path.
//!
//! MUST NOT import `eframe` or `rfd`. `request_open` / `request_open_path`
//! reach `crate::io::action_open` / `action_open_path` only on a clean
//! document, so no path here can be reached in a headless test without the
//! test itself proving the document dirty first (ADR 0002 §A4 rule 1 /
//! ADR 0003 §F3 trap 1: those two functions open a blocking native `rfd`
//! dialog and would hang the run).

use std::path::PathBuf;

use super::App;

// ---------------------------------------------------------------------------
// The parked action
// ---------------------------------------------------------------------------

/// A destructive action parked while the discard-confirmation dialog is up.
///
/// `None` on `UnsavedGuard::pending_action` means no dialog is pending. Carrying
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
        match self.guard.saved_revision {
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
    ///
    /// Also the **only** clearer of `title.recovered_from_autosave` (LCV-138
    /// amended AC 4): once this call certifies the document safe to discard —
    /// actually written to a file, or replaced outright via New/Open(/Recent)
    /// — the label's claim ("recovered and unsaved") has stopped being true,
    /// so it must stop being shown. An autosave flush calls only
    /// `mark_clean()`, never this method, so a crash-safety write alone
    /// leaves the flag set — it is not the operator saving.
    pub fn mark_saved(&mut self) {
        self.guard.saved_revision = Some(self.history.revision());
        self.title.recovered_from_autosave = false;
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
        if self.guard.pending_action.is_some() {
            return;
        }
        if self.has_unsaved_changes() {
            self.guard.pending_action = Some(PendingAction::New);
        } else {
            self.action_new();
        }
    }

    /// Guarded File > Open… / Ctrl+O. Runs [`App::action_open`] — which opens
    /// the native file dialog — immediately on a clean document; parks
    /// [`PendingAction::Open`] on a dirty one without touching the
    /// filesystem. A no-op while another action is already parked.
    pub fn request_open(&mut self) {
        if self.guard.pending_action.is_some() {
            return;
        }
        if self.has_unsaved_changes() {
            self.guard.pending_action = Some(PendingAction::Open);
        } else {
            self.action_open();
        }
    }

    /// Guarded File > Open Recent entry. Runs [`App::action_open_path`]
    /// immediately on a clean document; parks
    /// [`PendingAction::OpenPath`]`(path)` on a dirty one. A no-op while
    /// another action is already parked.
    pub fn request_open_path(&mut self, path: PathBuf) {
        if self.guard.pending_action.is_some() {
            return;
        }
        if self.has_unsaved_changes() {
            self.guard.pending_action = Some(PendingAction::OpenPath(path));
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
        if self.guard.pending_action.is_some() {
            return false;
        }
        if self.has_unsaved_changes() {
            self.guard.pending_action = Some(PendingAction::Exit);
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
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
