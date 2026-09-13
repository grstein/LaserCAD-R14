//! File-action wrappers on `App` (LCV-113 file split).
//!
//! Moved verbatim out of `src/app/mod.rs`, which was at 293 of the 300-LOC
//! implementation cap and could not absorb the LCV-113 discard-confirmation
//! feature otherwise (ADR 0002 "300-LOC cap" / "src/app/mod.rs" table). Pure
//! relocation: no behaviour change. The guarded `request_*` entry points and
//! the discard-confirmation dialog land in this same file in the next commit.
//!
//! MUST NOT import `eframe` or `rfd`.

use std::path::PathBuf;

use super::App;

impl App {
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
