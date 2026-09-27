//! Render layer: camera, viewport, grid, bed, entity painting, preview overlay,
//! snap markers. Bridges the pure kernel to egui's `Painter`.
//!
//! Submodules arrive with demands LCV-031 (camera) .. LCV-038 (snap markers).
//! The viewport rect itself is allocated by [`crate::app::App::update`] in
//! LCV-030; submodules consume that rect plus a [`egui::Painter`].

pub mod bed;
pub mod camera;
pub mod entities;
pub mod grid;
pub mod preview;
pub mod selection;
pub mod snaps;

pub use bed::{draw_bed, draw_bed_fill, Bed};
pub use camera::Camera;
pub use entities::{arc_polyline, draw_entities, PaintOptions};
pub use grid::draw_grid;
pub use preview::draw_preview;
pub use selection::draw_selection_highlight;
pub use snaps::draw_snap_marker;
