//! Persistence: settings, autosave, recent files, file dialogs, and SVG (see
//! [`svg`] submodule).
//!
//! `io::svg` is part of the kernel and MUST NOT import UI deps; the rest of
//! `io` (dialogs, recent, etc.) may use `rfd` and `directories`.
//!
//! Submodules arrive with demands LCV-055 .. LCV-062.

pub const MODULE: &str = "io";
pub mod autosave;
pub mod dialogs;
pub mod file_actions;
pub mod recent;
pub mod settings;
pub mod svg;

pub use autosave::{clear_autosave, load_autosave, save_autosave};
pub use dialogs::{open_file_dialog, pick_folder_dialog, save_file_dialog};
pub use file_actions::{action_new, action_open, action_save, action_save_as};
pub use recent::{open_recent, recent_files, OpenRecentError};
pub use svg::{import_svg, SvgImportError};

#[cfg(test)]
mod tests {
    #[test]
    fn dialogs_reexported_from_io() {
        let _: fn() -> Option<std::path::PathBuf> = crate::io::open_file_dialog;
        let _: fn(&str) -> Option<std::path::PathBuf> = crate::io::save_file_dialog;
        let _: fn() -> Option<std::path::PathBuf> = crate::io::pick_folder_dialog;
    }
}
