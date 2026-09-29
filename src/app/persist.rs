//! The whole persistence surface of [`App`] — three methods, one file
//! (LCV-119, [ADR 0006](../../docs/adr/0006-real-user-paths-are-injected.md)).
//!
//! A real per-user filesystem location is resolved exactly once, at boot, and
//! carried as data on [`App`] (`settings_path`, `autosave_path`). Nothing
//! below boot resolves one. Besides `App::new`, this file is the **only**
//! reader of those two fields, so the invariant is one `grep` wide:
//!
//! ```text
//! grep -rn "settings_path\|autosave_path" src/
//! ```
//!
//! must match only `src/app/mod.rs` (the declarations), `src/app/init.rs`
//! (the two constructors), this file, and `#[cfg(test)]` modules. The test
//! `the_two_path_fields_have_only_three_readers` below enforces exactly that.
//!
//! **`None` means "this process does not persist".** The write is a no-op, not
//! a panic — the inverse of [ADR 0005](../../docs/adr/0005-native-dialogs-disarmed-by-default.md)'s
//! choice for `rfd`, and deliberately so: whether a write happened is directly
//! observable by the test that asked for it (point the path at a tempdir and
//! read the bytes back), so an absent write is a failed assertion, never a
//! false green. All production call sites already swallow write failures, so a
//! panic here would be louder than the real failure it guards.
//!
//! MUST NOT import `egui`, `eframe` or `rfd`.

use super::App;
use crate::io::autosave::{clear_autosave_at, save_autosave_to};
use crate::io::settings::save_to;

impl App {
    /// Write `settings` back to `settings_path`, if this process was given
    /// one.
    ///
    /// A failed write is swallowed and the session continues: an unwritable
    /// config directory must not abort a CAD job mid-cut. Called by the three
    /// file actions that touch the recent-files list, by the Agent Settings
    /// window on close, and by the Bed size… modal on OK.
    pub fn persist_settings(&self) {
        let Some(path) = self.settings_path.as_deref() else {
            return;
        };
        let _ = save_to(&self.settings, path);
    }

    /// Write the current document to `autosave_path`; returns whether a file
    /// was actually written.
    ///
    /// `false` covers both "this process does not persist" (`autosave_path`
    /// is `None`) and "the write failed". The caller —
    /// [`flush_if_due`](super::autosave::flush_if_due) — uses the result only
    /// to decide whether to stamp `last_autosave_at`, so the two cases are
    /// correctly indistinguishable: neither produced a recovery file.
    pub fn write_autosave(&self) -> bool {
        let Some(path) = self.autosave_path.as_deref() else {
            return false;
        };
        save_autosave_to(&self.document, path).is_ok()
    }

    /// Remove the autosave file at `autosave_path`, if this process was given
    /// one.
    ///
    /// Called by all five file actions: once the document is saved, opened or
    /// reset, the stale recovery file must not outlive it. Tolerates a missing
    /// file and any I/O error.
    pub fn clear_autosave(&self) {
        let Some(path) = self.autosave_path.as_deref() else {
            return;
        };
        clear_autosave_at(path);
    }
}

#[cfg(test)]
mod tests;
