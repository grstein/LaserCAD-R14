//! UI chrome: menubar, toolbar, statusbar, command-line widget, modal dialogs,
//! keyboard shortcuts, theme.
//!
//! Submodules arrive with demands LCV-065 .. LCV-071.

pub mod command_destination;
pub mod command_line;
pub mod dialogs;
mod icons;
mod layer_combo;
pub mod layers_dialog;
pub mod menubar;
pub mod shortcuts;
pub mod shortcuts_dialog;
pub mod statusbar;
pub mod theme;
pub mod toolbar;
pub use command_destination::{
    LABEL_AI, LABEL_AI_BUSY, LABEL_AI_PROMPT_EMPTY, LABEL_AI_UNAVAILABLE, LABEL_CAD,
    LABEL_TOOL_INPUT, destination_label,
};
pub use command_line::draw_command_line;
pub use dialogs::{DialogResult, about_dialog, confirm_dialog, error_dialog};
pub use layers_dialog::draw_layers_dialog;
pub use menubar::draw_menubar;
pub use shortcuts::process_shortcuts;
pub use shortcuts_dialog::{SHORTCUT_GROUPS, ShortcutGroup, shortcuts_dialog, tool_rows};
pub use statusbar::{draw_statusbar, format_coords};
pub use theme::{CANVAS_BG, apply_theme};
pub use toolbar::draw_toolbar;
