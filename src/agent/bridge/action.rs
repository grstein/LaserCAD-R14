//! [`AgentAction`]: the vocabulary of `bridge.rs`, split out for the LOC cap
//! (LCV-182). Kernel-pure like its parent: no `egui`, `eframe`, `rfd`,
//! `reqwest` or `Document`.

use crate::agent::drawing::DrawingItem;

/// One thing the model wants done to the drawing.
///
/// Produced by `crate::agent::tools::parse_tool_call` from a validated tool
/// call, applied by `crate::app::agent_apply::apply`. Deliberately **not** a
/// `serde` deserialization target (ADR 0007 §Alternatives): deriving it would
/// throw away the hand-rolled per-field checks and messages that are most of
/// `tools.rs`'s value.
///
/// `index` is a **positional** handle into `Document::entities`, so a delete
/// renumbers every higher index down by one. That is disclosed in the prompt
/// and in every outcome string (ADR 0007 §D5) until stable ids land.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentAction {
    /// Append a line between two endpoints, in mm.
    CreateLine {
        /// First endpoint X, mm.
        x1: f64,
        /// First endpoint Y, mm.
        y1: f64,
        /// Second endpoint X, mm.
        x2: f64,
        /// Second endpoint Y, mm.
        y2: f64,
        /// Existing layer name to draw on; `None` = the current layer
        /// (LCV-156). Shape-checked only; resolved at the apply site.
        layer: Option<String>,
    },
    /// Append a full circle, centre in mm and radius in mm.
    CreateCircle {
        /// Centre X, mm.
        cx: f64,
        /// Centre Y, mm.
        cy: f64,
        /// Radius, mm. Positive and finite — checked at parse time.
        r: f64,
        /// Existing layer name to draw on; `None` = the current layer
        /// (LCV-156). Shape-checked only; resolved at the apply site.
        layer: Option<String>,
    },
    /// Append a circular arc. Angles are **radians** (`0` = +X axis).
    CreateArc {
        /// Centre X, mm.
        cx: f64,
        /// Centre Y, mm.
        cy: f64,
        /// Radius, mm. Positive and finite — checked at parse time.
        r: f64,
        /// Start angle, radians.
        start: f64,
        /// End angle, radians.
        end: f64,
        /// `true` for a counter-clockwise sweep.
        ccw: bool,
        /// Existing layer name to draw on; `None` = the current layer
        /// (LCV-156). Shape-checked only; resolved at the apply site.
        layer: Option<String>,
    },
    /// Delete the entity at this positional index.
    Delete {
        /// Zero-based index into `Document::entities`.
        index: usize,
    },
    /// Translate one entity by `(dx, dy)` mm.
    Move {
        /// Zero-based index into `Document::entities`.
        index: usize,
        /// X translation, mm.
        dx: f64,
        /// Y translation, mm.
        dy: f64,
    },
    /// Append a copy of one entity translated by `(dx, dy)` mm, on the
    /// source's layer (LCV-157).
    Copy {
        /// Zero-based index into `Document::entities`.
        index: usize,
        /// X translation, mm.
        dx: f64,
        /// Y translation, mm.
        dy: f64,
    },
    /// Rotate one entity about `(x, y)` mm by `angle` radians, CCW positive
    /// (LCV-158).
    Rotate {
        /// Zero-based index into `Document::entities`.
        index: usize,
        /// Base point X, mm.
        x: f64,
        /// Base point Y, mm.
        y: f64,
        /// Rotation angle, radians, CCW positive.
        angle: f64,
    },
    /// Mirror one entity across the line `(x1, y1)`–`(x2, y2)` mm, replacing
    /// it or appending the image on its layer (LCV-181).
    Mirror {
        /// Zero-based index into `Document::entities`.
        index: usize,
        /// Mirror line points, mm.
        x1: f64,
        /// See `x1`.
        y1: f64,
        /// See `x1`.
        x2: f64,
        /// See `x1`.
        y2: f64,
        /// `true` replaces the source; `false` keeps it.
        erase_source: bool,
    },
    /// Scale one entity about `(x, y)` mm by `factor` (LCV-182).
    Scale {
        /// Zero-based index into `Document::entities`.
        index: usize,
        /// Base point X, mm.
        x: f64,
        /// Base point Y, mm.
        y: f64,
        /// Uniform scale factor. Positive and finite — checked at parse time.
        factor: f64,
    },
    /// Apply one edit to a whole set of entities as one command and one step
    /// (LCV-186): one base point or axis, copies appended in ascending
    /// source order.
    Set {
        /// Zero-based indices into `Document::entities`: 1..=1000, unique,
        /// shape-checked at parse time; range-checked at the apply site.
        indices: Vec<usize>,
        /// The edit applied to every listed entity.
        op: SetOp,
    },
    /// Read back every entity in the drawing. Commits nothing.
    QueryEntities,
    /// Read back the current selection. Commits nothing.
    QuerySelection,
    /// Render the drawing as framed in the viewport into a grayscale PNG
    /// (LCV-145, ADR 0011). Commits nothing; one step like any action.
    CaptureCanvas,
    /// Ask whether a request carrying canvas images may go to `endpoint` /
    /// `model` (ADR 0011 item 10). **Not a step**: it bypasses the fence and
    /// the step counter. Never carries the API key.
    AuthorizeUpload {
        /// The endpoint the turn was started with.
        endpoint: String,
        /// The model the turn was started with.
        model: String,
    },
    /// Append a whole validated batch as one command (LCV-144, ADR 0010).
    CreateDrawing {
        /// The entities, in order; 1..=1000, already shape-checked.
        items: Vec<DrawingItem>,
        /// Existing layer name for the whole batch; `None` = the current
        /// layer (LCV-156, ADR 0012 §6).
        layer: Option<String>,
    },
    /// A tool call that failed its shape check in the worker — JSON syntax,
    /// unknown tool, or any `ToolCallError` (ADR 0007 §D15). The UI answers
    /// `Refused(reason)` without reading the document; it is still a step.
    Malformed {
        /// The tool name as the model sent it.
        tool: String,
        /// Names the tool and the field; never echoes the raw arguments.
        reason: String,
    },
}

/// The edit of an [`AgentAction::Set`]: a single-index edit without its
/// `index`, with the same units and already-validated arguments (LCV-186).
#[derive(Debug, Clone, PartialEq)]
pub enum SetOp {
    /// `delete_entity`.
    Delete,
    /// `move_entity`: translate by `(dx, dy)` mm.
    Move {
        /// X translation, mm.
        dx: f64,
        /// Y translation, mm.
        dy: f64,
    },
    /// `copy_entity`: append translated copies on the sources' layers.
    Copy {
        /// X translation, mm.
        dx: f64,
        /// Y translation, mm.
        dy: f64,
    },
    /// `rotate_entity`: rotate about `(x, y)` mm by `angle` radians.
    Rotate {
        /// Base point X, mm.
        x: f64,
        /// Base point Y, mm.
        y: f64,
        /// Rotation angle, radians, CCW positive.
        angle: f64,
    },
    /// `mirror_entity`: mirror across `(x1, y1)`–`(x2, y2)` mm.
    Mirror {
        /// Mirror line points, mm.
        x1: f64,
        /// See `x1`.
        y1: f64,
        /// See `x1`.
        x2: f64,
        /// See `x1`.
        y2: f64,
        /// `true` replaces the sources; `false` keeps them.
        erase_source: bool,
    },
    /// `scale_entity`: scale about `(x, y)` mm by `factor` (> 0).
    Scale {
        /// Base point X, mm.
        x: f64,
        /// Base point Y, mm.
        y: f64,
        /// Uniform scale factor, positive and finite.
        factor: f64,
    },
}

impl AgentAction {
    /// The layer name a creation action asks for, if any (LCV-156).
    pub fn layer(&self) -> Option<&str> {
        match self {
            Self::CreateLine { layer, .. }
            | Self::CreateCircle { layer, .. }
            | Self::CreateArc { layer, .. }
            | Self::CreateDrawing { layer, .. } => layer.as_deref(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    /// AC 7 — the action enum is hand-built, never deserialized. A
    /// `Deserialize` derive here would let a future demand delete `tools.rs`'s
    /// per-field checks without noticing.
    #[test]
    fn agent_action_carries_no_deserialize_derive() {
        let src = include_str!("action.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("action.rs must have a bare #[cfg(test)] marker");
        let implementation = &src[..at];

        assert!(
            implementation.contains(concat!("pub enum ", "AgentAction")),
            "positive control: the enum must be declared in this file"
        );
        assert!(
            !implementation.contains(concat!("Deser", "ialize")),
            "AC 7: AgentAction must not be a serde deserialization target"
        );
    }
}
