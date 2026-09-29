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

/// The AI assistant toggle's visible label (LCV-140 AC 3): a short text
/// button replacing the old icon-only `"🤖"`, which depended on emoji-font
/// coverage the rest of the UI never assumes. Its hover text stays the
/// pre-existing `"AI Assistant"` (set at the one call site in
/// [`draw_toolbar`]) — only what is visible changes, not the tooltip or the
/// click behaviour.
pub(crate) const AGENT_TOGGLE_LABEL: &str = "Agent";

/// Hover-tooltip text for one `TOOLS` entry (LCV-140 AC 1 / AC 3): the
/// label, plus its keyboard shortcut in parentheses when
/// [`ToolEntry::shortcut`] names one. Derived entirely from `entry`'s own
/// two fields — never a second, hand-typed table — and never inventing a
/// key for `Select`, whose `shortcut` is `None`.
pub(crate) fn tool_hover_text(entry: &ToolEntry) -> String {
    match entry.shortcut {
        Some(key) => format!("{} (shortcut: {key})", entry.label),
        None => format!("{} (no keyboard shortcut)", entry.label),
    }
}

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

    // ── LCV-140 AC 1 / AC 3 — hover-hint inventory ─────────────────────────

    /// AC 1 — every hover hint traces back to *that entry's own* `label` and
    /// `shortcut` field: it always starts with the label, it names the
    /// shortcut when one exists, and — the `Select` case — it invents no key
    /// when `shortcut` is `None`. The expected string is rebuilt from the
    /// entry's own fields, never from a second, per-tool hand-typed table.
    #[test]
    fn tool_hover_text_traces_to_its_own_entry() {
        for entry in TOOLS {
            let hint = tool_hover_text(entry);
            let expected = match entry.shortcut {
                Some(key) => format!("{} (shortcut: {key})", entry.label),
                None => format!("{} (no keyboard shortcut)", entry.label),
            };
            assert_eq!(hint, expected, "entry {}", entry.label);
            assert!(
                hint.starts_with(entry.label),
                "hint for {} must start with its own label: {hint:?}",
                entry.label
            );
            if let Some(key) = entry.shortcut {
                assert!(
                    hint.contains(key),
                    "{}'s hint must name its shortcut {key}: {hint:?}",
                    entry.label
                );
            }
        }
    }

    /// AC 1 — `Select` (the one entry with `shortcut: None`) gets a hint that
    /// says so in plain words, never a fabricated key.
    #[test]
    fn select_hover_text_names_no_key() {
        let select = TOOLS.iter().find(|e| e.label == "Select").unwrap();
        assert_eq!(select.shortcut, None, "positive control");
        assert_eq!(tool_hover_text(select), "Select (no keyboard shortcut)");
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
