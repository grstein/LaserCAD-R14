//! Pure-Rust 2D intersection routines for the kernel.
//!
//! This module owns the four canonical intersection cases LaserCAD needs:
//! segment-segment (strict and infinite-line variants), segment-circle, and
//! circle-circle. The snap engine (LCV-016) and the future trim/extend tools
//! (Phase 4) consume these routines; consolidating them here ensures
//! consistent floating-point classification of parallels, tangents, and
//! near-coincidences.
//!
//! Conventions:
//!
//! - All coordinates are millimeters (kernel canonical units).
//! - "Near" classifications (parallel, tangent, coincident) use
//!   [`crate::geometry::EPSILON`] from LCV-010.
//! - Owned return types: `Option<Vec2>` for the at-most-one cases,
//!   `Vec<Vec2>` (length 0, 1, or 2) for the multi-point cases.
//! - No ordering guarantees on returned vectors; callers that need order
//!   sort the result themselves.
//!
//! Split into two submodules to honor the kernel's 300-LOC-per-file cap:
//!
//! - [`line`] — `line_line`, `line_line_infinite`, `line_circle`.
//! - [`circle`] — `circle_circle`.
//! - [`arc`] — `line_arc`, `circle_arc`, `arc_arc`, filtered by the arc's
//!   span (LCV-160).
//!
//! Frozen by demand LCV-014.

pub mod arc;
pub mod circle;
pub mod line;

pub use arc::{arc_arc, circle_arc, line_arc};
pub use circle::circle_circle;
pub use line::{line_circle, line_line, line_line_infinite};
