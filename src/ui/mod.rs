//! UI chrome: menubar, toolbar, statusbar, command-line widget, modal dialogs,
//! keyboard shortcuts, theme.
//!
//! Submodules arrive with demands LCV-065 .. LCV-071.

pub mod statusbar;
pub use statusbar::{draw_statusbar, format_coords};

/// Placeholder module witness; retired when all LCV-06x submodules land.
pub const MODULE: &str = "ui";
