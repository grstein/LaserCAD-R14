//! Left-side toolbar: one selectable button per drawing / modify tool.
//!
//! Exposes [`draw_toolbar`], which renders a vertical stack of tool buttons
//! into an [`egui::Ui`] that lives inside a `SidePanel::left`.  Clicking a
//! button activates the corresponding tool via
//! [`crate::tools::ToolManager::set_tool`].
//!
//! [`TOOLS`] is the single source of truth for every v0.1.0 tool: it drives
//! this toolbar *and* the Tools menu in `crate::ui::menubar` — there is no
//! second list. One drawing tool that exists in `src/tools/` is deliberately
//! not in this table; LCV-106 decides its fate.
//!
//! Introduced by demand LCV-066; extended to the full eleven-tool set and
//! made the Tools-menu data source by LCV-104. LCV-110 replaced the old
//! string-keyed label lookup with `entry.kind` + [`tools::make`] — the
//! single tool-identity map (ADR 0003 §A3).

use crate::app::App;
use crate::cmdline::ToolKind;
use crate::tools;

/// One toolbar / Tools-menu entry.
///
/// `label` and `tool_name` diverge because several tools use uppercase
/// internal names (`"LINE"`, `"ERASE"`, …) while the UI shows mixed-case
/// labels. `shortcut` is the bare-key hint shown in the Tools menu (`None`
/// for `Select`, which has no keyboard binding).
pub(crate) struct ToolEntry {
    /// Text displayed on the toolbar button and in the Tools menu.
    pub(crate) label: &'static str,
    /// String returned by [`Tool::name`]; used to detect the active tool for
    /// highlight purposes.
    pub(crate) tool_name: &'static str,
    /// Keyboard shortcut hint (`src/ui/shortcuts.rs::TOOL_KEYS`), if any.
    pub(crate) shortcut: Option<&'static str>,
    /// The tool identity resolved via [`tools::make`] — the single
    /// identity-to-instance map (ADR 0003 §A3). Replaces the old
    /// string-keyed label lookup.
    pub(crate) kind: ToolKind,
}

/// Index of the first "modify group" entry (`Move`). The toolbar draws a
/// separator immediately before it, splitting the draw tools (Select …
/// Text) from the modify tools (Move … Delete).
const MODIFY_GROUP_START: usize = 7;

/// Toolbar / Tools-menu entries, in display order (LCV-104 acceptance
/// table). This is the eleven-tool v0.1.0 set.
pub(crate) const TOOLS: &[ToolEntry] = &[
    ToolEntry {
        label: "Select",
        tool_name: "Select",
        shortcut: None,
        kind: ToolKind::Select,
    },
    ToolEntry {
        label: "Line",
        tool_name: "LINE",
        shortcut: Some("L"),
        kind: ToolKind::Line,
    },
    ToolEntry {
        label: "Polyline",
        tool_name: "PLINE",
        shortcut: Some("P"),
        kind: ToolKind::Polyline,
    },
    ToolEntry {
        label: "Rect",
        tool_name: "RECT",
        shortcut: Some("R"),
        kind: ToolKind::Rect,
    },
    ToolEntry {
        label: "Circle",
        tool_name: "CIRCLE",
        shortcut: Some("C"),
        kind: ToolKind::Circle,
    },
    ToolEntry {
        label: "Arc",
        tool_name: "ARC",
        shortcut: Some("A"),
        kind: ToolKind::Arc,
    },
    ToolEntry {
        label: "Text",
        tool_name: "TEXT",
        shortcut: Some("D"),
        kind: ToolKind::Text,
    },
    ToolEntry {
        label: "Move",
        tool_name: "MOVE",
        shortcut: Some("M"),
        kind: ToolKind::Move,
    },
    ToolEntry {
        label: "Trim",
        tool_name: "TRIM",
        shortcut: Some("T"),
        kind: ToolKind::Trim,
    },
    ToolEntry {
        label: "Extend",
        tool_name: "EXTEND",
        shortcut: Some("X"),
        kind: ToolKind::Extend,
    },
    ToolEntry {
        label: "Delete",
        tool_name: "ERASE",
        shortcut: Some("E"),
        kind: ToolKind::Delete,
    },
];

/// Compile-time guarantee that the toolbar has at least one entry.
#[allow(clippy::len_zero)]
const _: () = assert!(TOOLS.len() >= 1);

/// Render the left-side toolbar into `ui`.
///
/// Draws one [`egui::SelectableLabel`] per tool, with a separator between the
/// draw group (Select … Text) and the modify group (Move … Delete). The
/// currently active tool button is rendered in its selected/highlighted
/// state. Clicking an inactive button calls
/// [`ToolManager::set_tool`](crate::tools::ToolManager::set_tool), which
/// cancels any in-progress state on the old tool before switching.
///
/// **Call site**: inside a `SidePanel::left("toolbar")` added in
/// [`crate::app::App::update_ui`], *after* the bottom status-bar panel and
/// *before* the `CentralPanel`.
pub fn draw_toolbar(ui: &mut egui::Ui, app: &mut App) {
    // `active_tool_name` returns `&'static str` — the borrow on `app` ends
    // immediately so the later mutable call to `set_tool` is safe.
    let active = app.tool_manager.active_tool_name();

    // Collect the kind of the clicked button (at most one per frame).
    let mut clicked: Option<ToolKind> = None;

    for (i, entry) in TOOLS.iter().enumerate() {
        if i == MODIFY_GROUP_START {
            ui.separator();
        }
        let is_active = active == entry.tool_name;
        if ui.selectable_label(is_active, entry.label).clicked() {
            clicked = Some(entry.kind);
        }
    }

    if let Some(kind) = clicked {
        app.tool_manager.set_tool(tools::make(kind));
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

    /// LCV-110 AC 12 — every `TOOLS` entry's `kind` resolves through
    /// `tools::make` to the tool that entry's `tool_name` names. Replaces
    /// the old string-keyed label lookup test.
    #[test]
    fn tools_make_covers_all_toolbar_entries() {
        for entry in TOOLS {
            assert_eq!(
                tools::make(entry.kind).name(),
                entry.tool_name,
                "tool name mismatch for label \"{}\"",
                entry.label
            );
        }
    }

    /// LCV-104 AC#3 — TextTool is reachable from the toolbar/menu table for
    /// the first time.
    #[test]
    fn text_tool_is_reachable_from_toolbar() {
        let entry = TOOLS
            .iter()
            .find(|e| e.label == "Text")
            .expect("\"Text\" must be a known toolbar entry");
        assert_eq!(tools::make(entry.kind).name(), "TEXT");
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

    /// LCV-104 AC#1, AC#4 — the table is exactly the eleven v0.1.0 tools, in
    /// the acceptance-criteria order.
    #[test]
    fn toolbar_table_is_the_v010_tool_set() {
        assert_eq!(TOOLS.len(), 11, "TOOLS must have exactly eleven entries");
        let labels: Vec<&str> = TOOLS.iter().map(|e| e.label).collect();
        assert_eq!(
            labels,
            vec![
                "Select", "Line", "Polyline", "Rect", "Circle", "Arc", "Text", "Move", "Trim",
                "Extend", "Delete",
            ]
        );
    }
}
