//! UI chrome: menubar, toolbar, statusbar, command-line widget, modal dialogs,
//! keyboard shortcuts, theme.
//!
//! Submodules arrive with demands LCV-065 .. LCV-071.

pub mod command_line;
pub mod dialogs;
pub mod menubar;
pub mod shortcuts;
pub mod statusbar;
pub mod theme;
pub mod toolbar;
pub use command_line::draw_command_line;
pub use dialogs::{
    about_dialog, confirm_dialog, error_dialog, shortcuts_dialog, tool_rows, DialogResult,
    ShortcutGroup, SHORTCUT_GROUPS,
};
pub use menubar::draw_menubar;
pub use shortcuts::process_shortcuts;
pub use statusbar::{draw_statusbar, format_coords};
pub use theme::{apply_theme, CANVAS_BG};
pub use toolbar::draw_toolbar;
