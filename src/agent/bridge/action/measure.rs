//! [`MeasureRequest`]: the payload of `AgentAction::Measure` (LCV-194),
//! split out of `action.rs` for the LOC cap. Kernel-pure like its parent: no
//! `egui`, `eframe`, `rfd`, `reqwest` or `Document`.

use crate::geometry::Vec2;

/// What a `measure` call asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasureQuery {
    /// The minimum distance of two operands, with dx, dy and both points.
    Distance,
    /// A line's length, an arc's arc length or a circle's circumference.
    Length,
    /// The exact extents of the listed entities, or of the whole drawing.
    Bbox,
    /// Every crossing point of two entities, or their overlap.
    Intersections,
    /// The directed and the undirected angle of two lines.
    Angle,
}

impl MeasureQuery {
    /// Every query, in the order the schema lists them.
    pub const ALL: [MeasureQuery; 5] = [
        Self::Distance,
        Self::Length,
        Self::Bbox,
        Self::Intersections,
        Self::Angle,
    ];

    /// The wire name: `distance`, `length`, `bbox`, `intersections`, `angle`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Distance => "distance",
            Self::Length => "length",
            Self::Bbox => "bbox",
            Self::Intersections => "intersections",
            Self::Angle => "angle",
        }
    }
}

/// The entities a `measure` call names: positions or stable ids, never both.
/// Shape-checked at parse time (unique, at most 1000); resolved against the
/// live drawing at the apply site (ADR 0007 §D2a).
#[derive(Debug, Clone, PartialEq)]
pub enum MeasureTargets {
    /// Zero-based indices into `Document::entities`; empty when none given.
    Indices(Vec<usize>),
    /// The `N` of each `e<N>` (LCV-188).
    Ids(Vec<u64>),
}

/// One validated `measure` call: the query and its operands, whose counts
/// already fit the query.
#[derive(Debug, Clone, PartialEq)]
pub struct MeasureRequest {
    /// What to measure.
    pub query: MeasureQuery,
    /// Finite points in mm, in the order given; only `distance` takes any.
    pub points: Vec<Vec2>,
    /// The entities, in the order given.
    pub targets: MeasureTargets,
}
