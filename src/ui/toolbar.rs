//! Left-side toolbar: one selectable button per drawing / modify tool.
//!
//! Exposes [`draw_toolbar`], which renders a vertical stack of tool buttons
//! into an [`egui::Ui`] that lives inside a `SidePanel::left`.  Clicking a
//! button activates the corresponding tool via
//! [`crate::tools::ToolManager::set_tool`].
//!
//! Only tools that exist in the current codebase are included.  Rect, Text,
//! and Move buttons will be added when those tools land (LCV-045 et al.).
//!
//! Introduced by demand LCV-066.

use crate::app::App;
use crate::tools::{ArcTool, CircleTool, DeleteTool, LineTool, PolylineTool, SelectTool, Tool};

/// Pairing of a user-facing button label with the tool's canonical `name()`.
///
/// The two values diverge because several tools use uppercase internal names
/// (`"LINE"`, `"ERASE"`, …) while the toolbar shows mixed-case labels.
struct ToolEntry {
    /// Text displayed on the toolbar button.
    label: &'static str,
    /// String returned by [`Tool::name`]; used to detect the active tool for
    /// highlight purposes.
    tool_name: &'static str,
}

/// Toolbar entries in top-to-bottom display order.
const TOOLS: &[ToolEntry] = &[
    ToolEntry {
        label: "Select",
        tool_name: "Select",
    },
    ToolEntry {
        label: "Line",
        tool_name: "LINE",
    },
    ToolEntry {
        label: "Polyline",
        tool_name: "PLINE",
    },
    ToolEntry {
        label: "Circle",
        tool_name: "CIRCLE",
    },
    ToolEntry {
        label: "Arc",
        tool_name: "ARC",
    },
    ToolEntry {
        label: "Delete",
        tool_name: "ERASE",
    },
];

/// Compile-time guarantee that the toolbar has at least one entry.
#[allow(clippy::len_zero)]
const _: () = assert!(TOOLS.len() >= 1);

/// Construct a fresh tool instance keyed on the button **label**.
///
/// Returns `None` for unknown labels (forward-compatible guard).
fn make_tool(label: &str) -> Option<Box<dyn Tool>> {
    match label {
        "Select" => Some(Box::new(SelectTool::default())),
        "Line" => Some(Box::new(LineTool::default())),
        "Polyline" => Some(Box::new(PolylineTool::default())),
        "Circle" => Some(Box::new(CircleTool::default())),
        "Arc" => Some(Box::new(ArcTool::default())),
        "Delete" => Some(Box::new(DeleteTool)),
        _ => None,
    }
}

/// Render the left-side toolbar into `ui`.
///
/// Draws one [`egui::SelectableLabel`] per tool.  The currently active tool
/// button is rendered in its selected/highlighted state.  Clicking an inactive
/// button calls [`ToolManager::set_tool`](crate::tools::ToolManager::set_tool),
/// which cancels any in-progress state on the old tool before switching.
///
/// **Call site**: inside a `SidePanel::left("toolbar")` added in
/// [`crate::app::App::update`], *after* the bottom status-bar panel and
/// *before* the `CentralPanel`.
pub fn draw_toolbar(ui: &mut egui::Ui, app: &mut App) {
    // `active_tool_name` returns `&'static str` — the borrow on `app` ends
    // immediately so the later mutable call to `set_tool` is safe.
    let active = app.tool_manager.active_tool_name();

    // Collect the label of the clicked button (at most one per frame).
    let mut clicked: Option<&'static str> = None;

    for entry in TOOLS {
        let is_active = active == entry.tool_name;
        if ui.selectable_label(is_active, entry.label).clicked() {
            clicked = Some(entry.label);
        }
    }

    if let Some(label) = clicked {
        if let Some(tool) = make_tool(label) {
            app.tool_manager.set_tool(tool);
        }
    }

    // ── AI assistant toggle (LCV-080) ─────────────────────────────────────────
    ui.separator();
    if ui
        .selectable_label(app.agent_panel_open, "🤖")
        .on_hover_text("AI Assistant")
        .clicked()
    {
        app.agent_panel_open = !app.agent_panel_open;
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// LCV-066 AC — `make_tool` returns `Some` for every label in `TOOLS`,
    /// and the resulting tool's `name()` matches the entry's `tool_name`.
    #[test]
    fn make_tool_covers_all_toolbar_entries() {
        for entry in TOOLS {
            let tool = make_tool(entry.label);
            assert!(
                tool.is_some(),
                "make_tool(\"{}\") should return Some, got None",
                entry.label
            );
            assert_eq!(
                tool.unwrap().name(),
                entry.tool_name,
                "tool name mismatch for label \"{}\"",
                entry.label
            );
        }
    }

    /// LCV-066 AC — `make_tool` returns `None` for an unknown label.
    #[test]
    fn make_tool_unknown_label_returns_none() {
        assert!(make_tool("NonExistentTool").is_none());
    }

    /// LCV-066 AC — default `App` active tool is "Select", which corresponds
    /// to the first toolbar entry.
    #[test]
    fn default_app_active_tool_matches_first_entry() {
        let app = App::default();
        assert_eq!(app.tool_manager.active_tool_name(), TOOLS[0].tool_name);
    }

    /// LCV-066 AC — all toolbar labels are unique (no duplicate buttons).
    #[test]
    fn toolbar_labels_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for entry in TOOLS {
            assert!(
                seen.insert(entry.label),
                "duplicate toolbar label: \"{}\"",
                entry.label
            );
        }
    }

    /// LCV-066 AC — all toolbar tool_names are unique (no two entries map to
    /// the same active tool).
    #[test]
    fn toolbar_tool_names_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for entry in TOOLS {
            assert!(
                seen.insert(entry.tool_name),
                "duplicate toolbar tool_name: \"{}\"",
                entry.tool_name
            );
        }
    }
}
