//! UI chrome: menubar, toolbar, statusbar, command-line widget, modal dialogs,
//! keyboard shortcuts, theme.
//!
//! Submodules arrive with demands LCV-065 .. LCV-071.

pub mod dialogs;
pub mod statusbar;
pub mod theme;
pub use dialogs::{about_dialog, confirm_dialog, error_dialog, DialogResult};
pub use statusbar::{draw_statusbar, format_coords};
pub use theme::{apply_theme, CANVAS_BG};
