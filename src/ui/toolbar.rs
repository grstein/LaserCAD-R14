//! Left-side toolbar: one selectable button per drawing / modify tool.
//!
//! Exposes [`draw_toolbar`], which renders a vertical stack of tool buttons
//! into an [`egui::Ui`] that lives inside a `SidePanel::left`.  Clicking a
//! button activates the corresponding tool via
//! [`crate::tools::ToolManager::set_tool`].
//!
//! [`TOOLS`] is the single source of truth for every rail tool: it drives
//! this toolbar *and* the Tools menu in `crate::ui::menubar` — there is no
//! second list. One drawing tool that exists in `src/tools/` is deliberately
//! not in this table; LCV-106 decides its fate.
//!
//! Introduced by demand LCV-066; extended to the full eleven-tool set and
//! made the Tools-menu data source by LCV-104. LCV-110 replaced the old
//! string-keyed label lookup with `entry.kind` + [`tools::make`] — the
//! single tool-identity map (ADR 0003 §A3).
//!
//! LCV-140 added three things, all derived from the same [`TOOLS`] table (no
//! second, hand-typed one): every button's hover tooltip names its
//! [`ToolEntry::shortcut`] (via [`tool_hover_text`]); the rail wraps its
//! eleven entries — plus the agent toggle — in one `ScrollArea` so a short
//! window scrolls the whole list instead of clipping any of it; and the AI
//! assistant toggle shows a short visible label ([`AGENT_TOGGLE_LABEL`])
//! instead of the old icon-only `"🤖"`, keeping its existing hover text and
//! click behaviour. The rail's outer width cap itself is computed in
//! `src/app/panels.rs::toolbar_width` — a panel-sizing decision, not a
//! drawing one — from this same table.

use crate::app::App;
use crate::cmdline::ToolKind;
use crate::tools;

mod table;
pub(crate) use table::{TOOLS, ToolEntry};

/// Index of the first "modify group" entry (`Move`). The toolbar draws a
/// separator immediately before it, splitting the draw tools (Select …
/// Text) from the modify tools (Move … Dist).
const MODIFY_GROUP_START: usize = 7;

/// The AI assistant toggle's visible label (LCV-140 AC 3): a short text
/// button replacing the old icon-only `"🤖"`, which depended on emoji-font
/// coverage the rest of the UI never assumes. Its hover text stays the
/// pre-existing `"AI Assistant"` (set at the one call site in
/// [`draw_toolbar`]) — only what is visible changes, not the tooltip or the
/// click behaviour.
pub(crate) const AGENT_TOGGLE_LABEL: &str = "Agent";

/// Hover-tooltip text for one `TOOLS` entry (LCV-183 AC 6):
/// `<Label> — <key> · <WORD>`, or `<Label> — <WORD>` when the entry has no
/// key, where `WORD` is the command word — the entry's `tool_name`
/// uppercased. Derived entirely from `entry` — never a second, hand-typed
/// table — and never inventing a key for an entry whose `shortcut` is `None`.
pub(crate) fn tool_hover_text(entry: &ToolEntry) -> String {
    let word = entry.tool_name.to_uppercase();
    match entry.shortcut {
        Some(key) => format!("{} — {key} · {word}", entry.label),
        None => format!("{} — {word}", entry.label),
    }
}

/// Render the left-side toolbar into `ui`.
///
/// Draws one [`egui::SelectableLabel`] per tool, with a separator between the
/// draw group (Select … Text) and the modify group (Move … Dist). The
/// currently active tool button is rendered in its selected/highlighted
/// state. Clicking an inactive button calls
/// [`ToolManager::set_tool`](crate::tools::ToolManager::set_tool), which
/// cancels any in-progress state on the old tool before switching.
///
/// **Call site**: inside a `SidePanel::left("toolbar")` added in
/// [`crate::app::App::update_ui`], *after* the bottom status-bar panel and
/// *before* the `CentralPanel`. The panel's own width is capped by the
/// caller (`src/app/panels.rs::toolbar_width`, LCV-140 AC 2) — this function
/// only ever draws into whatever `ui` it is given.
///
/// The eleven `TOOLS` entries, a separator, then the agent toggle are all
/// wrapped in one `ScrollArea` (LCV-140 AC 2): egui only shows a scrollbar
/// and clips when the panel's available height is smaller than that whole
/// content, so a normal window never scrolls and a short one degrades to
/// scrolling instead of losing content. The toggle is deliberately *inside*
/// the scroll area, not pinned below it — a `SidePanel` clips whatever
/// overflows its own assigned rect with no way back, so anything left
/// outside the one scrolling mechanism on an extremely cramped window would
/// be unreachable, not merely hidden (LCV-140 review, mutation testing:
/// `tests/it/ui/compact_chrome_and_action_hints.rs::ac2_a_real_wheel_scroll_reaches_a_row_hidden_by_the_cramped_rail`
/// is the regression guard).
pub fn draw_toolbar(ui: &mut egui::Ui, app: &mut App) {
    // `active_tool_name` returns `&'static str` — the borrow on `app` ends
    // immediately so the later mutable call to `set_tool` is safe.
    let active = app.tool_manager.active_tool_name();
    let agent_panel_open = app.agent.panel_open;

    // At most one of these can be true/Some per frame.
    let (clicked, agent_clicked): (Option<ToolKind>, bool) = egui::ScrollArea::vertical()
        .show(ui, |ui| {
            let mut clicked: Option<ToolKind> = None;
            for (i, entry) in TOOLS.iter().enumerate() {
                if i == MODIFY_GROUP_START {
                    ui.separator();
                }
                let is_active = active == entry.tool_name;
                if ui
                    .selectable_label(is_active, entry.label)
                    .on_hover_text(tool_hover_text(entry))
                    .clicked()
                {
                    clicked = Some(entry.kind);
                }
            }

            // ── AI assistant toggle (LCV-080; visible label since LCV-140,
            // moved inside the scroll area by the same demand's review) ────
            ui.separator();
            let agent_clicked = ui
                .selectable_label(agent_panel_open, AGENT_TOGGLE_LABEL)
                .on_hover_text("AI Assistant")
                .clicked();

            (clicked, agent_clicked)
        })
        .inner;

    if let Some(kind) = clicked {
        app.tool_manager.set_tool(tools::make(kind));
    }
    if agent_clicked {
        app.agent.panel_open = !app.agent.panel_open;
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── LCV-140 Expected Tests — active-tool highlight ─────────────────────

    /// Activating each `TOOLS` entry's tool leaves it — and only it —
    /// matching the same `active == entry.tool_name` comparison
    /// `draw_toolbar` uses to decide which button paints highlighted. The
    /// real click-through-the-UI case (one entry, "Circle") lives in
    /// `tests/it/ui/compact_chrome_and_action_hints.rs`; this covers the
    /// same comparison for all eleven, which a single UI click cannot afford.
    #[test]
    fn activating_each_tool_highlights_only_itself() {
        let mut app = App::default();
        for entry in TOOLS {
            app.tool_manager.set_tool(tools::make(entry.kind));
            let active = app.tool_manager.active_tool_name();
            for other in TOOLS {
                let is_active = active == other.tool_name;
                assert_eq!(
                    is_active,
                    other.tool_name == entry.tool_name,
                    "activating {} must highlight only itself, not {}",
                    entry.label,
                    other.label
                );
            }
        }
    }

    // ── LCV-183 AC 6 — tooltip format ───────────────────────────────────

    /// The `TOOLS` entry labelled `label`.
    fn entry(label: &str) -> &'static ToolEntry {
        TOOLS
            .iter()
            .find(|e| e.label == label)
            .unwrap_or_else(|| panic!("{label} is a TOOLS entry"))
    }

    /// AC 6 — the spec's own examples, plus `Select` (no key, mixed-case
    /// `tool_name`).
    #[test]
    fn ac6_tooltip_names_label_key_and_command_word() {
        assert_eq!(tool_hover_text(entry("Select")), "Select — SELECT");
        assert_eq!(tool_hover_text(entry("Line")), "Line — L · LINE");
        assert_eq!(tool_hover_text(entry("Rotate")), "Rotate — ROTATE");
        assert_eq!(tool_hover_text(entry("Delete")), "Delete — E · ERASE");
    }

    /// AC 6 — every tooltip traces back to *that entry's own* fields, with no
    /// invented key for an entry whose `shortcut` is `None`.
    #[test]
    fn ac6_every_tooltip_traces_to_its_own_entry() {
        for e in TOOLS {
            let hint = tool_hover_text(e);
            let word = e.tool_name.to_uppercase();
            let expected = match e.shortcut {
                Some(key) => format!("{} — {key} · {word}", e.label),
                None => format!("{} — {word}", e.label),
            };
            assert_eq!(hint, expected, "entry {}", e.label);
            assert_eq!(hint.contains('·'), e.shortcut.is_some(), "{hint:?}");
        }
    }

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

    /// LCV-104 AC#1, AC#4 — the eleven v0.1.0 tools in the acceptance-criteria
    /// order, with the v0.3 edit commands after Move and DIST last.
    #[test]
    fn toolbar_table_is_the_v030_tool_set() {
        assert_eq!(TOOLS.len(), 16, "TOOLS must have exactly sixteen entries");
        let labels: Vec<&str> = TOOLS.iter().map(|e| e.label).collect();
        assert_eq!(
            labels,
            vec![
                "Select", "Line", "Polyline", "Rect", "Circle", "Arc", "Text", "Move", "Copy",
                "Rotate", "Mirror", "Scale", "Trim", "Extend", "Delete", "Dist",
            ]
        );
    }
}
