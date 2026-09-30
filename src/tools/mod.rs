//! Tools: `Tool` trait, `ToolManager`, one file per drawing or modify tool.
//!
//! Tools never mutate `Document` directly — they construct a `Command` and call
//! `history.commit(cmd, doc)`. State machines stay inside individual tool files.

pub mod arc;
pub mod circle;
pub mod copy;
pub mod delete;
pub mod dist;
pub mod extend;
pub mod line;
pub mod manager;
pub mod move_;
pub mod pointer_event;
pub mod polyline;
pub mod rect;
pub mod select;
pub mod text;
pub mod tool;
pub mod trim;

pub use arc::ArcTool;
pub use circle::CircleTool;
pub use copy::CopyTool;
pub use delete::DeleteTool;
pub use dist::DistTool;
pub use extend::ExtendTool;
pub use line::LineTool;
pub use manager::ToolManager;
pub use move_::MoveTool;
pub use pointer_event::{PointerButton, PointerEvent};
pub use polyline::PolylineTool;
pub use rect::RectTool;
pub use select::SelectTool;
pub use text::TextTool;
pub use tool::Tool;
pub use trim::TrimTool;

use crate::cmdline::ToolKind;

/// The single map from tool identity to tool instance (ADR 0003 §A3).
///
/// Exhaustive `match`, no `_` arm: adding a `ToolKind` variant is a compile
/// error until it is mapped here. `src/ui/shortcuts.rs::TOOL_KEYS` and the
/// command-line alias table (`src/cmdline/parse.rs`) are two bindings into
/// the same `ToolKind`, both resolved through this one function — see the
/// LCV-110 demand's "identity problem" for why a second string-keyed map is
/// not an option.
pub fn make(kind: ToolKind) -> Box<dyn Tool> {
    match kind {
        ToolKind::Select => Box::new(SelectTool::default()),
        ToolKind::Line => Box::new(LineTool::default()),
        ToolKind::Polyline => Box::new(PolylineTool::default()),
        ToolKind::Rect => Box::new(RectTool::default()),
        ToolKind::Circle => Box::new(CircleTool::default()),
        ToolKind::Arc => Box::new(ArcTool::default()),
        ToolKind::Move => Box::new(MoveTool::default()),
        ToolKind::Delete => Box::new(DeleteTool),
        ToolKind::Trim => Box::new(TrimTool),
        ToolKind::Extend => Box::new(ExtendTool::default()),
        ToolKind::Text => Box::new(TextTool::default()),
        ToolKind::Copy => Box::new(CopyTool::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC 12, 13 — `make` covers every `ToolKind` variant and constructs the
    /// tool that name expects. The exhaustive `match` above already fails to
    /// compile if a variant is missing; this test additionally pins each
    /// variant to the right instance.
    #[test]
    fn make_covers_every_tool_kind() {
        assert_eq!(make(ToolKind::Select).name(), "Select");
        assert_eq!(make(ToolKind::Line).name(), "LINE");
        assert_eq!(make(ToolKind::Polyline).name(), "PLINE");
        assert_eq!(make(ToolKind::Rect).name(), "RECT");
        assert_eq!(make(ToolKind::Circle).name(), "CIRCLE");
        assert_eq!(make(ToolKind::Arc).name(), "ARC");
        assert_eq!(make(ToolKind::Move).name(), "MOVE");
        assert_eq!(make(ToolKind::Delete).name(), "ERASE");
        assert_eq!(make(ToolKind::Trim).name(), "TRIM");
        assert_eq!(make(ToolKind::Extend).name(), "EXTEND");
        assert_eq!(make(ToolKind::Text).name(), "TEXT");
        assert_eq!(make(ToolKind::Copy).name(), "COPY");
    }
}
