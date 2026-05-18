//! Pure-Rust geometry kernel: `Vec2`, `Line`, `Circle`, `Arc`, intersections,
//! snap engine, rectangle predicates.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`. The kernel must remain testable
//! as a pure library. Reviewer enforces.
//!
//! Submodules arrive with demands LCV-010 .. LCV-017.

pub mod arc;
pub mod circle;
pub mod epsilon;
pub mod intersect;
pub mod line;
pub mod rect;
pub mod vec2;

pub use arc::Arc;
pub use circle::Circle;
pub use epsilon::EPSILON;
pub use intersect::{circle_circle, line_circle, line_line, line_line_infinite};
pub use line::Line;
pub use rect::Rect;
pub use vec2::Vec2;
