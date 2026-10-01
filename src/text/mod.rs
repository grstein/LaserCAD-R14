//! Hershey single-stroke font data and ASCII text → line-geometry layout;
//! outline fonts for SVG text import ([`fonts`], LCV-179).
//!
//! # Overview
//!
//! Converts a UTF-8 string into a flat `Vec<Entity::Line>` using the classic
//! public-domain Hershey Simplex Roman single-stroke font.  The primary
//! entry-point is [`layout_text`].
//!
//! # Kernel-purity contract
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.  This module is part of the
//! pure-Rust kernel and must remain testable in headless / CLI / WASM contexts.
//!
//! Introduced by demand LCV-055.

pub mod fonts;
pub mod hershey;
pub mod hershey_data;
pub mod layout;

pub use fonts::{FaceId, FontBook};
pub use hershey::CAP_HEIGHT_HERSHEY;
pub use layout::layout_text;
