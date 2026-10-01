//! Pure-Rust geometry kernel: `Vec2`, `Line`, `Circle`, `Arc`, intersections,
//! snap engine, rectangle predicates.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`. The kernel must remain testable
//! as a pure library. Reviewer enforces.
//!
//! Submodules arrive with demands LCV-010 .. LCV-017.

pub mod arc;
pub mod circle;
pub mod distance;
pub mod epsilon;
pub mod intersect;
pub mod line;
pub mod rect;
pub mod snap;
pub mod transform;
pub mod vec2;

pub use arc::Arc;
pub use circle::Circle;
pub use distance::{Prim, closest};
pub use epsilon::EPSILON;
pub use intersect::{
    arc_arc, circle_arc, circle_circle, line_arc, line_circle, line_line, line_line_infinite,
};
pub use line::Line;
pub use rect::Rect;
pub use snap::{SnapEntity, SnapKind, SnapKinds, SnapResult, snap, snap_query};
pub use transform::Transform;
pub use vec2::Vec2;
