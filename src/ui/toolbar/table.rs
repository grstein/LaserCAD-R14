//! The rail's tool table (LCV-104, LCV-183): [`ToolEntry`] and [`TOOLS`],
//! re-exported by `crate::ui::toolbar` so every `toolbar::TOOLS` path keeps
//! working. Data only.

use crate::cmdline::ToolKind;
use crate::ui::icons::{IconFn, draw, modify};

/// One toolbar / Tools-menu entry.
///
/// `label` and `tool_name` diverge because several tools use uppercase
/// internal names (`"LINE"`, `"ERASE"`, …) while the UI shows mixed-case
/// labels. `shortcut` is the bare-key hint shown in the Tools menu (`None`
/// for `Select`, which has no keyboard binding).
pub(crate) struct ToolEntry {
    /// Text shown in the Tools menu and the rail button tooltip.
    pub(crate) label: &'static str,
    /// String returned by [`crate::tools::Tool::name`]; used to detect the active tool for
    /// highlight purposes.
    pub(crate) tool_name: &'static str,
    /// Keyboard shortcut hint (`src/ui/shortcuts.rs::TOOL_KEYS`), if any.
    pub(crate) shortcut: Option<&'static str>,
    /// The tool identity resolved via [`crate::tools::make`] — the single
    /// identity-to-instance map (ADR 0003 §A3). Replaces the old
    /// string-keyed label lookup.
    pub(crate) kind: ToolKind,
    /// The rail button's painted icon (LCV-183).
    pub(crate) icon: IconFn,
}

/// Toolbar / Tools-menu entries, in display order (LCV-104 acceptance
/// table): the eleven v0.1.0 tools plus the v0.3 edit commands (COPY, ROTATE,
/// MIRROR, SCALE) after MOVE and the DIST query last. The v0.3 entries have
/// no bare-letter shortcut (ADR 0003 amendment 5).
pub(crate) const TOOLS: &[ToolEntry] = &[
    ToolEntry {
        label: "Select",
        tool_name: "Select",
        shortcut: None,
        kind: ToolKind::Select,
        icon: draw::select,
    },
    ToolEntry {
        label: "Line",
        tool_name: "LINE",
        shortcut: Some("L"),
        kind: ToolKind::Line,
        icon: draw::line,
    },
    ToolEntry {
        label: "Polyline",
        tool_name: "PLINE",
        shortcut: Some("P"),
        kind: ToolKind::Polyline,
        icon: draw::polyline,
    },
    ToolEntry {
        label: "Rect",
        tool_name: "RECT",
        shortcut: Some("R"),
        kind: ToolKind::Rect,
        icon: draw::rect,
    },
    ToolEntry {
        label: "Circle",
        tool_name: "CIRCLE",
        shortcut: Some("C"),
        kind: ToolKind::Circle,
        icon: draw::circle,
    },
    ToolEntry {
        label: "Arc",
        tool_name: "ARC",
        shortcut: Some("A"),
        kind: ToolKind::Arc,
        icon: draw::arc,
    },
    ToolEntry {
        label: "Text",
        tool_name: "TEXT",
        shortcut: Some("D"),
        kind: ToolKind::Text,
        icon: draw::text,
    },
    ToolEntry {
        label: "Move",
        tool_name: "MOVE",
        shortcut: Some("M"),
        kind: ToolKind::Move,
        icon: modify::move_,
    },
    ToolEntry {
        label: "Copy",
        tool_name: "COPY",
        shortcut: None,
        kind: ToolKind::Copy,
        icon: modify::copy,
    },
    ToolEntry {
        label: "Rotate",
        tool_name: "ROTATE",
        shortcut: None,
        kind: ToolKind::Rotate,
        icon: modify::rotate,
    },
    ToolEntry {
        label: "Mirror",
        tool_name: "MIRROR",
        shortcut: None,
        kind: ToolKind::Mirror,
        icon: modify::mirror,
    },
    ToolEntry {
        label: "Scale",
        tool_name: "SCALE",
        shortcut: None,
        kind: ToolKind::Scale,
        icon: modify::scale,
    },
    ToolEntry {
        label: "Trim",
        tool_name: "TRIM",
        shortcut: Some("T"),
        kind: ToolKind::Trim,
        icon: modify::trim,
    },
    ToolEntry {
        label: "Extend",
        tool_name: "EXTEND",
        shortcut: Some("X"),
        kind: ToolKind::Extend,
        icon: modify::extend,
    },
    ToolEntry {
        label: "Delete",
        tool_name: "ERASE",
        shortcut: Some("E"),
        kind: ToolKind::Delete,
        icon: modify::delete,
    },
    ToolEntry {
        label: "Dist",
        tool_name: "DIST",
        shortcut: None,
        kind: ToolKind::Dist,
        icon: modify::dist,
    },
];

/// Compile-time guarantee that the toolbar has at least one entry.
const _: () = assert!(!TOOLS.is_empty());
