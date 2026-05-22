//! UI chrome: menubar, toolbar, statusbar, command-line widget, modal dialogs,
//! keyboard shortcuts, theme.
//!
//! Submodules arrive with demands LCV-065 .. LCV-071.

pub mod statusbar;
pub mod theme;
pub mod toolbar;
pub use statusbar::{draw_statusbar, format_coords};
pub use theme::{apply_theme, CANVAS_BG};
pub use toolbar::draw_toolbar;
