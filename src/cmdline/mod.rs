//! Command-line parser kernel: grammar, tool/toggle/zoom identities, and the
//! 50-entry recall ring behind the AutoCAD-style command line.
//!
//! Binding contract: ADR 0003 §A (`docs/adr/0003-command-line-input-contract.md`).
//! This module is **pure kernel**: it imports only [`crate::geometry`] and
//! `std`, joining the AGENTS.md purity list alongside `geometry/`,
//! `document/`, `io/svg/` and `text/`. It knows nothing about `egui`,
//! `eframe`, `rfd`, `crate::document::Document` or `crate::tools::Tool` — the
//! whole point is that the grammar is testable with no UI context at all.
//!
//! Wiring this parser's output into the tools and the command-line widget is
//! LCV-111's job, not this module's: nothing here mutates a `Document` and
//! nothing here knows what a `Tool` is.

mod history;
mod parse;

pub use history::CommandHistory;
pub use parse::{parse, parse_number};

use crate::geometry::Vec2;

/// The result of parsing one line of command-line input.
///
/// [`parse`] is **total**: it never panics and never returns an error.
/// Unrecognised or malformed input becomes [`CommandInput::Unknown`], never a
/// `Result::Err` — see ADR 0003 §A2 and product decision 5 in the LCV-110
/// demand body.
#[derive(Debug, Clone, PartialEq)]
pub enum CommandInput {
    /// An absolute point, e.g. `"50,25"`. World-space millimetres, Y-up.
    Point(Vec2),
    /// An offset from the active anchor, e.g. `"@10,-5"`. Millimetres.
    Relative(Vec2),
    /// A bare magnitude, e.g. `"37.5"`. Millimetres. Not validated against
    /// any tool's notion of a legal radius or length — that is the tool's
    /// business (LCV-111), not the grammar's.
    Distance(f64),
    /// A tool-activation alias, e.g. `"l"`, `"text"`.
    Tool(ToolKind),
    /// A toggle alias, e.g. `"snap"`, `"grid"`, `"ortho"`.
    Toggle(ToggleKind),
    /// A zoom command, e.g. `"zoom in"`, `"ze"`.
    Zoom(ZoomKind),
    /// The field was blank (empty or whitespace-only).
    ///
    /// Distinct from `Unknown(String::new())`: pressing Enter on a blank
    /// command line is a meaningful "accept" keystroke in R14 (LCV-111
    /// routes it to the active tool), not an error to report.
    Empty,
    /// Anything else. The payload is the **trimmed** raw text in its
    /// **original case** — never the lowercased matching key — so a caller
    /// can echo `Unknown command: "<text>"` without keeping its own copy.
    Unknown(String),
}

/// Every tool the command line (and the keyboard) can name.
///
/// Fieldless by design: the parser must not know what a `Tool` is.
/// [`crate::tools::make`] is the single map from a `ToolKind` to a
/// `Box<dyn Tool>` instance (ADR 0003 §A3) — the keyboard table
/// (`src/ui/shortcuts.rs::TOOL_KEYS`) and this alias table are two bindings
/// into the same enum, not two independent maps to two different instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    /// Selection tool. No command-line alias letter collision: `"s"`.
    Select,
    /// LINE — draw a straight segment.
    Line,
    /// PLINE — draw a chain of connected segments.
    Polyline,
    /// RECT — draw an axis-aligned rectangle.
    Rect,
    /// CIRCLE — draw a circle by center and radius.
    Circle,
    /// ARC — draw a circular arc.
    Arc,
    /// MOVE — relocate the current selection.
    Move,
    /// ERASE — delete the current selection. Bound to the bare `E` key and
    /// aliased to the command-line letter `"e"` (product decision 2 — `e` is
    /// ERASE, not EXTEND, matching AutoCAD R14's `E`/`EX` split).
    Delete,
    /// TRIM — shorten an entity at an intersection.
    Trim,
    /// EXTEND — lengthen an entity to a boundary. **No command-line alias**
    /// in this demand (product decision 2; ADR 0003 §A2 already excludes
    /// it) — do not add `"e"` → `Extend` back.
    Extend,
    /// TEXT — place Hershey-font text.
    Text,
}

/// A view / drawing-aid toggle the command line can flip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleKind {
    /// Object snap.
    Snap,
    /// The background grid.
    Grid,
    /// Orthogonal (axis-locked) cursor constraint.
    Ortho,
}

/// A zoom command the command line can issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoomKind {
    /// Zoom in one step.
    In,
    /// Zoom out one step.
    Out,
    /// Zoom to fit all entities (or the bed, if the document is empty).
    Extents,
}
