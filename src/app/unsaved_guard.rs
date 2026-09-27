//! The unsaved-changes guard's state (ADR 0004 §"The `src/app/mod.rs` seam",
//! seam A) — split out of `src/app/mod.rs` when LCV-138's new `title` field
//! took it to the 300-line implementation cap with no headroom left.
//!
//! [`UnsavedGuard`] groups the three fields `src/app/file_ops.rs`'s guard
//! state machine owns and no other phase writes: the last known
//! safe-to-discard revision, the destructive action parked behind the
//! discard-confirmation dialog, and whether a parked `Exit` has already been
//! confirmed. `PendingAction` stays declared in `file_ops.rs`, and
//! `App::has_unsaved_changes` / `App::mark_saved` stay there too — only the
//! storage moved, and the inner field names are unchanged
//! (`app.guard.saved_revision`, not a rename).
//!
//! MUST NOT import `eframe` or `rfd`.

use super::PendingAction;

/// The unsaved-changes guard's state — grouped the way `AgentState` and
/// `DocumentTitleState` are, so `App` gains one field instead of three.
#[derive(Default)]
pub struct UnsavedGuard {
    /// The `history.revision()` at which the document was last known safe to
    /// discard: just written to a file, just loaded from one, or just reset
    /// to blank (LCV-113). `None` means "never was" — a fresh unsaved
    /// document or one recovered from autosave at boot, both unsaved per
    /// [`App::has_unsaved_changes`](super::App::has_unsaved_changes). **Not**
    /// the autosave signal: unlike `dirty_since`, autosave (`mark_clean`)
    /// never touches this field. The only writer is
    /// [`App::mark_saved`](super::App::mark_saved).
    pub saved_revision: Option<u64>,
    /// A destructive action parked while the discard-confirmation dialog is
    /// up (LCV-113). `None` means no dialog is pending; see
    /// `src/app/file_ops.rs`.
    pub pending_action: Option<PendingAction>,
    /// Set once a parked `Exit` is confirmed; never cleared afterward
    /// (LCV-136). `ViewportCommand::Close` does not synchronously destroy the
    /// window: egui-winit's `process_viewport_command` only records a fresh
    /// `ViewportEvent::Close` on the viewport, which makes
    /// `close_requested()` report `true` again on the *next* frame — the
    /// same close request, re-delivered. Without this latch,
    /// `poll_close_request` would run `request_exit` a second time, find the
    /// document still dirty (`Exit` performs no save), re-park
    /// `PendingAction::Exit`, and send `CancelClose` — cancelling the very
    /// close the operator just confirmed, forever. See
    /// `src/app/discard.rs::poll_close_request`. The latch assumes the
    /// window tears down once `Close` is sent uncancelled (eframe 0.29.1
    /// behaviour).
    pub exit_confirmed: bool,
}
