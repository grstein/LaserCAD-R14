//! UI chrome: menubar, toolbar, statusbar, command-line widget, modal dialogs,
//! keyboard shortcuts, theme.
//!
//! Submodules arrive with demands LCV-065 .. LCV-071.

pub mod command_destination;
pub mod command_line;
pub mod dialogs;
mod layer_combo;
pub mod layers_dialog;
pub mod menubar;
pub mod shortcuts;
pub mod shortcuts_dialog;
pub mod statusbar;
pub mod theme;
pub mod toolbar;
pub use command_destination::{
    destination_label, LABEL_AI, LABEL_AI_BUSY, LABEL_AI_PROMPT_EMPTY, LABEL_AI_UNAVAILABLE,
    LABEL_CAD, LABEL_TOOL_INPUT,
};
pub use command_line::draw_command_line;
pub use dialogs::{about_dialog, confirm_dialog, error_dialog, DialogResult};
pub use layers_dialog::draw_layers_dialog;
pub use menubar::draw_menubar;
pub use shortcuts::process_shortcuts;
pub use shortcuts_dialog::{shortcuts_dialog, tool_rows, ShortcutGroup, SHORTCUT_GROUPS};
pub use statusbar::{draw_statusbar, format_coords};
pub use theme::{apply_theme, CANVAS_BG};
pub use toolbar::draw_toolbar;
