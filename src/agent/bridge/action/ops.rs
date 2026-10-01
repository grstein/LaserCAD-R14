//! [`CaptureFrame`] and [`SetOp`]: the payloads of two `AgentAction`
//! variants, split out of `action.rs` for the LOC cap (LCV-191). Kernel-pure
//! like its parent: no `egui`, `eframe`, `rfd`, `reqwest` or `Document`.

/// What a [`AgentAction::CaptureCanvas`](super::AgentAction::CaptureCanvas) frames (LCV-187). Shape-checked
/// at parse time; finiteness and area are checked at the apply site.
#[derive(Debug, Clone, PartialEq)]
pub enum CaptureFrame {
    /// The operator's viewport, sized as it is (LCV-145).
    View,
    /// Every entity's extents plus a 5 % margin, longest edge 1024 px.
    Drawing,
    /// The world rectangle between two corners in mm, in any order,
    /// longest edge 1024 px.
    Region {
        /// First corner X, mm.
        x0: f64,
        /// First corner Y, mm.
        y0: f64,
        /// Opposite corner X, mm.
        x1: f64,
        /// Opposite corner Y, mm.
        y1: f64,
    },
}

/// The edit of an [`AgentAction::Set`](super::AgentAction::Set): a single-index edit without its
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

impl SetOp {
    /// The single-index tool this edit mirrors; names a set's refusals.
    pub(super) fn tool_name(&self) -> &'static str {
        match self {
            Self::Delete => "delete_entity",
            Self::Move { .. } => "move_entity",
            Self::Copy { .. } => "copy_entity",
            Self::Rotate { .. } => "rotate_entity",
            Self::Mirror { .. } => "mirror_entity",
            Self::Scale { .. } => "scale_entity",
        }
    }
}
