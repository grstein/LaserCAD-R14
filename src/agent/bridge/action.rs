//! [`AgentAction`]: the vocabulary of `bridge.rs`, split out for the LOC cap
//! (LCV-182). Kernel-pure like its parent: no `egui`, `eframe`, `rfd`,
//! `reqwest` or `Document`.

use crate::agent::drawing::DrawingItem;

mod measure;
mod ops;
pub use measure::{MeasureQuery, MeasureRequest, MeasureTargets};
pub use ops::{CaptureFrame, SetOp};

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
/// and in every outcome string (ADR 0007 §D5). [`AgentAction::ById`] names
/// entities by their stable id instead (LCV-188, ADR 0014).
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
    /// [`AgentAction::Set`] addressed by stable ids, `e<N>` on the wire
    /// (LCV-188, ADR 0014 §7): resolved to indices at the apply site.
    ById {
        /// The `N` of each `e<N>`: 1..=1000, unique, each ≥ 1, shape-checked
        /// at parse time; an id that is not live is refused at the apply site.
        ids: Vec<u64>,
        /// The edit applied to every listed entity.
        op: SetOp,
    },
    /// Read back every entity in the drawing. Commits nothing.
    QueryEntities,
    /// Read back the current selection. Commits nothing.
    QuerySelection,
    /// Run the drawing check (LCV-190). Commits nothing.
    CheckDrawing,
    /// Measure distances, lengths, extents, crossings or angles (LCV-194).
    /// Commits nothing.
    Measure(MeasureRequest),
    /// Render the drawing, framed as asked, into a grayscale PNG (LCV-145,
    /// LCV-187, ADR 0011). Commits nothing; one step like any action.
    CaptureCanvas(CaptureFrame),
    /// Ask whether a request carrying canvas images may go to `endpoint` /
    /// `model` (ADR 0011 item 10). **Not a step**: it bypasses the fence and
    /// the step counter. Never carries the API key.
    AuthorizeUpload {
        /// The endpoint the turn was started with.
        endpoint: String,
        /// The model the turn was started with.
        model: String,
    },
    /// A transcript `note` from the loop: what happened to one canvas image
    /// once its request returned (LCV-187, ADR 0011 item 10). **Not a
    /// step**, like `AuthorizeUpload`; answered `Ok`.
    Note(String),
    /// The model answered one request, which carried `captures` authorised
    /// canvas images (LCV-193). **Not a step**, like `AuthorizeUpload`;
    /// answered `Ok` with no transcript row.
    Replied {
        /// Authorised image parts the answered request carried.
        captures: u32,
    },
    /// What the model should be told about the drawing after a batch that
    /// ran unfenced (LCV-195). **Not a step**, like `AuthorizeUpload`: the UI
    /// answers it from the live document, outside the fence and the tally;
    /// `Ok("")` adds nothing.
    Feedback,
    /// May the loop remind the model to verify its drawing before the turn
    /// ends (LCV-197)? **Not a step**, like `AuthorizeUpload`: `Ok` is yes,
    /// anything else is no.
    VerifyDue,
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

    /// The tool every apply-site refusal names; a set names its edit tool.
    pub fn tool_name(&self) -> &str {
        match self {
            Self::CreateLine { .. } => "create_line",
            Self::CreateCircle { .. } => "create_circle",
            Self::CreateArc { .. } => "create_arc",
            Self::Delete { .. } => "delete_entity",
            Self::Move { .. } => "move_entity",
            Self::Copy { .. } => "copy_entity",
            Self::Rotate { .. } => "rotate_entity",
            Self::Mirror { .. } => "mirror_entity",
            Self::Scale { .. } => "scale_entity",
            Self::Set { op, .. } | Self::ById { op, .. } => op.tool_name(),
            Self::QueryEntities => "query_entities",
            Self::QuerySelection => "query_selection",
            Self::CheckDrawing => "check_drawing",
            Self::Measure(_) => "measure",
            Self::CaptureCanvas(_)
            | Self::AuthorizeUpload { .. }
            | Self::Note(_)
            | Self::Replied { .. } => "capture_canvas",
            // Not a tool: the loop asks it, the model never calls it.
            Self::Feedback | Self::VerifyDue => "",
            Self::CreateDrawing { .. } => "create_drawing",
            Self::Malformed { tool, .. } => tool,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AgentAction as A, CaptureFrame, MeasureQuery, MeasureRequest, MeasureTargets, SetOp,
    };

    /// LCV-192 AC 2 — every action names the tool the model called; a set
    /// names its edit tool.
    #[test]
    fn every_action_names_its_tool() {
        let (x, i) = (0.0, 0);
        let set = |op| A::Set {
            indices: vec![i],
            op,
        };
        let cases = [
            (
                A::CreateLine {
                    x1: x,
                    y1: x,
                    x2: x,
                    y2: x,
                    layer: None,
                },
                "create_line",
            ),
            (
                A::CreateCircle {
                    cx: x,
                    cy: x,
                    r: 1.0,
                    layer: None,
                },
                "create_circle",
            ),
            (
                A::CreateArc {
                    cx: x,
                    cy: x,
                    r: 1.0,
                    start: x,
                    end: x,
                    ccw: true,
                    layer: None,
                },
                "create_arc",
            ),
            (A::Delete { index: i }, "delete_entity"),
            (
                A::Move {
                    index: i,
                    dx: x,
                    dy: x,
                },
                "move_entity",
            ),
            (
                A::Copy {
                    index: i,
                    dx: x,
                    dy: x,
                },
                "copy_entity",
            ),
            (
                A::Rotate {
                    index: i,
                    x,
                    y: x,
                    angle: x,
                },
                "rotate_entity",
            ),
            (
                A::Mirror {
                    index: i,
                    x1: x,
                    y1: x,
                    x2: x,
                    y2: x,
                    erase_source: true,
                },
                "mirror_entity",
            ),
            (
                A::Scale {
                    index: i,
                    x,
                    y: x,
                    factor: 2.0,
                },
                "scale_entity",
            ),
            (set(SetOp::Delete), "delete_entity"),
            (set(SetOp::Move { dx: x, dy: x }), "move_entity"),
            (set(SetOp::Copy { dx: x, dy: x }), "copy_entity"),
            (set(SetOp::Rotate { x, y: x, angle: x }), "rotate_entity"),
            (
                set(SetOp::Mirror {
                    x1: x,
                    y1: x,
                    x2: x,
                    y2: x,
                    erase_source: false,
                }),
                "mirror_entity",
            ),
            (
                set(SetOp::Scale {
                    x,
                    y: x,
                    factor: 2.0,
                }),
                "scale_entity",
            ),
            (set(SetOp::Layer { layer: "L".into() }), "set_layer"),
            (A::QueryEntities, "query_entities"),
            (A::QuerySelection, "query_selection"),
            (A::CheckDrawing, "check_drawing"),
            (
                A::Measure(MeasureRequest {
                    query: MeasureQuery::Bbox,
                    points: vec![],
                    targets: MeasureTargets::Indices(vec![]),
                }),
                "measure",
            ),
            (A::CaptureCanvas(CaptureFrame::View), "capture_canvas"),
            (
                A::AuthorizeUpload {
                    endpoint: "e".into(),
                    model: "m".into(),
                },
                "capture_canvas",
            ),
            (A::Note("n".into()), "capture_canvas"),
            (A::Replied { captures: 1 }, "capture_canvas"),
            (A::Feedback, ""),
            (
                A::CreateDrawing {
                    items: vec![],
                    layer: None,
                },
                "create_drawing",
            ),
            (
                A::Malformed {
                    tool: "draw_unicorn".into(),
                    reason: "r".into(),
                },
                "draw_unicorn",
            ),
        ];
        for (action, name) in cases {
            assert_eq!(action.tool_name(), name, "{action:?}");
        }
    }

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
