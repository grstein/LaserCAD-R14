//! Drawing check (LCV-190): open ends, gaps, duplicates, degenerate and
//! off-bed entities, found once and printed by every consumer.
//!
//! Read-only: nothing here mutates the [`Document`]. Only entities on layers
//! with Output on are checked; indices are document indices, zero-based.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use super::Document;
use crate::geometry::Vec2;

/// One problem the check found. Indices are zero-based document indices.
#[derive(Clone, Debug, PartialEq)]
pub enum Finding {
    /// A line or arc endpoint that meets no other endpoint.
    OpenEnd {
        /// The entity the endpoint belongs to.
        index: usize,
        /// The endpoint, mm.
        at: Vec2,
    },
    /// Two endpoints closer than [`GAP_MM`] that do not meet.
    Gap {
        /// The lower of the two entity indices.
        a: usize,
        /// The higher of the two entity indices.
        b: usize,
        /// Midpoint between the two endpoints, mm.
        mid: Vec2,
        /// Distance between the two endpoints, mm.
        width: f64,
    },
}

/// Endpoints closer than this (mm) that do not meet form a gap.
pub const GAP_MM: f64 = 0.5;

/// Every finding of one check, in kind order and by ascending index.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CheckReport {
    /// The findings, sorted by kind, then by index.
    pub findings: Vec<Finding>,
}

/// Check the drawing on Output-on layers and report what is wrong with it.
pub fn check_drawing(_doc: &Document) -> CheckReport {
    CheckReport::default()
}

#[cfg(test)]
mod tests;
