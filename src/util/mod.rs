//! Cross-cutting utilities: units (mm canonical, the default-bed constants and
//! their range, the world ↔ SVG Y mirror) and filesystem paths (via the
//! `directories` crate when wired in).
//!
//! Submodules arrive on demand, not preemptively.

pub mod units;

pub use units::{
    clamp_bed_mm, flip_y, BED_MAX_MM, BED_MIN_MM, DEFAULT_BED_HEIGHT_MM, DEFAULT_BED_WIDTH_MM,
};
