//! Persistence: settings, autosave, recent files, file dialogs, and SVG (see
//! [`svg`] submodule).
//!
//! `io::svg` is part of the kernel and MUST NOT import UI deps; the rest of
//! `io` (dialogs, recent, etc.) may use `rfd` and `directories`.

pub mod autosave;
pub mod dialogs;
pub mod export_layers;
pub mod file_actions;
pub mod recent;
pub mod settings;
mod settings_store;
pub mod svg;

pub use dialogs::{arm_native_dialogs, open_file_dialog, pick_folder_dialog, save_file_dialog};
pub use export_layers::{action_export_layers, layer_exports};
pub use file_actions::{action_new, action_open, action_open_path, action_save, action_save_as};
pub use recent::{OpenRecentError, open_recent, recent_files};
pub use svg::{ImportedSvg, SvgImportError, export_svg, import_svg};

#[cfg(test)]
mod tests {
    #[test]
    fn dialogs_reexported_from_io() {
        let _: fn() -> Option<std::path::PathBuf> = crate::io::open_file_dialog;
        let _: fn(&str) -> Option<std::path::PathBuf> = crate::io::save_file_dialog;
        let _: fn() -> Option<std::path::PathBuf> = crate::io::pick_folder_dialog;
    }
}
