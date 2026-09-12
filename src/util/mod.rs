//! Cross-cutting utilities: units (mm canonical, bed constants, the world ↔
//! SVG Y mirror) and filesystem paths (via the `directories` crate when wired
//! in).
//!
//! Submodules arrive on demand, not preemptively.

pub mod units;

pub use units::{flip_y, BED_HEIGHT_MM, BED_WIDTH_MM};
