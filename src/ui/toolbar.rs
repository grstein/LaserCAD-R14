//! Left-side tool rail: one painted-icon button per drawing / modify tool.
//!
//! Exposes [`draw_toolbar`], which renders the rail into an [`egui::Ui`] that
//! lives inside a `SidePanel::left`. Clicking a button activates the
//! corresponding tool via [`crate::tools::ToolManager::set_tool`].
//!
//! [`TOOLS`] is the single source of truth for every rail tool: it drives
//! this rail, the Tools menu in `crate::ui::menubar` and the shortcuts
//! dialog — there is no second list. LCV-110 resolves each entry through
//! `entry.kind` + [`tools::make`], the single tool-identity map (ADR 0003
//! §A3).
//!
//! LCV-140 put the rail and the AI toggle in one `ScrollArea` so a short
//! window scrolls instead of clipping. LCV-183 replaced the text buttons with
//! 32 pt icon buttons (`crate::ui::icons`) in two columns — the draw group
//! left, the modify group right — with the `AI` toggle below both, and a
//! tooltip naming label, key and command word ([`tool_hover_text`]). LCV-190
//! put a CHECK icon button beside `AI`. The
//! rail's fixed width lives in `src/app/panels.rs::RAIL_WIDTH`.

use crate::app::App;
use crate::cmdline::ToolKind;
use crate::tools;
use crate::ui::icons::{icon_button, modify, text_button};

mod table;
pub(crate) use table::{TOOLS, ToolEntry};

/// Index of the first "modify group" entry (`Move`): the draw tools (Select
/// … Text) fill the left column, the modify tools (Move … Dist) the right.
const MODIFY_GROUP_START: usize = 7;

/// Gap between rail buttons, across and down, in points (LCV-183).
pub(crate) const RAIL_GAP: f32 = 4.0;

/// The AI assistant toggle's visible text (LCV-183 AC 7, LCV-167's short
/// form): a 32 pt square button below both tool columns. Its hover text is
/// `"AI Assistant"`, set at the one call site in [`draw_toolbar`].
pub(crate) const AGENT_TOGGLE_LABEL: &str = "AI";

/// Hover text of the CHECK button beside `AI` (LCV-190): a command, not a
/// tool, so it is no `TOOLS` entry and has no key.
pub(crate) const CHECK_HOVER: &str = "Check — CHECK";

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

/// Render the tool rail into `ui`.
///
/// Two columns of [`icon_button`]s — the draw group (Select … Text) on the
/// left, the modify group (Move … Dist) on the right — then a separator and
/// a bottom row: the `AI` toggle, and beside it the CHECK button, which runs
/// [`App::run_check`] (LCV-190). The active tool's button is painted in the
/// selected fill. Clicking a button calls
/// [`ToolManager::set_tool`](crate::tools::ToolManager::set_tool), which
/// cancels any in-progress state on the old tool before switching.
///
/// **Call site**: inside a `SidePanel::left("toolbar")` added in
/// [`crate::app::App::update_ui`], *after* the bottom status-bar panel and
/// *before* the `CentralPanel`. The panel's width is fixed by the caller
/// (`src/app/panels.rs::RAIL_WIDTH`).
///
/// Everything — both columns and the toggle — sits in one `ScrollArea`
/// (LCV-140 AC 2, LCV-183 AC 9): a window too short for the rail scrolls it
/// instead of clipping. The toggle is deliberately *inside* the scroll area:
/// a `SidePanel` clips whatever overflows its rect with no way back, so
/// anything outside the one scrolling mechanism would be unreachable on a
/// cramped window.
pub fn draw_toolbar(ui: &mut egui::Ui, app: &mut App) {
    // `active_tool_name` returns `&'static str` — the borrow on `app` ends
    // immediately so the later mutable call to `set_tool` is safe.
    let active = app.tool_manager.active_tool_name();
    let agent_panel_open = app.agent.panel_open;
    let (draw, modify) = TOOLS.split_at(MODIFY_GROUP_START);

    // At most one of these can be true/Some per frame.
    let (clicked, agent_clicked, check_clicked) = egui::ScrollArea::vertical()
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::splat(RAIL_GAP);
            let mut clicked: Option<ToolKind> = None;
            ui.horizontal_top(|ui| {
                for group in [draw, modify] {
                    ui.vertical(|ui| {
                        for entry in group {
                            if icon_button(ui, active == entry.tool_name, entry.icon)
                                .on_hover_text(tool_hover_text(entry))
                                .clicked()
                            {
                                clicked = Some(entry.kind);
                            }
                        }
                    });
                }
            });

            ui.separator();
            let (agent_clicked, check_clicked) = ui
                .horizontal(|ui| {
                    let agent = text_button(ui, agent_panel_open, AGENT_TOGGLE_LABEL)
                        .on_hover_text("AI Assistant");
                    let check =
                        icon_button(ui, false, modify::check_drawing).on_hover_text(CHECK_HOVER);
                    (agent.clicked(), check.clicked())
                })
                .inner;

            (clicked, agent_clicked, check_clicked)
        })
        .inner;

    if let Some(kind) = clicked {
        app.tool_manager.set_tool(tools::make(kind));
    }
    if agent_clicked {
        app.agent.panel_open = !app.agent.panel_open;
    }
    if check_clicked {
        app.run_check();
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

    /// LCV-202 AC 5 — every `TOOLS` entry has its own row in the user
    /// guide's tool table: a line starting `| <label> |`.
    #[test]
    fn user_guide_has_a_row_for_every_tool() {
        let guide = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/user-guide.md"));
        let missing: Vec<&str> = TOOLS
            .iter()
            .map(|e| e.label)
            .filter(|label| {
                let row = format!("| {label} |");
                !guide.lines().any(|l| l.trim_start().starts_with(&row))
            })
            .collect();
        assert!(
            missing.is_empty(),
            "docs/user-guide.md lacks rows for {missing:?}"
        );
    }
}
