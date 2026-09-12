//! Tools: `Tool` trait, `ToolManager`, one file per drawing or modify tool.
//!
//! Tools never mutate `Document` directly — they construct a `Command` and call
//! `history.commit(cmd, doc)`. State machines stay inside individual tool files.

pub mod arc;
pub mod circle;
pub mod delete;
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
pub use delete::DeleteTool;
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
