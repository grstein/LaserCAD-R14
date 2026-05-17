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
pub mod line;
pub mod vec2;

pub use arc::Arc;
pub use circle::Circle;
pub use epsilon::EPSILON;
pub use line::Line;
pub use vec2::Vec2;
