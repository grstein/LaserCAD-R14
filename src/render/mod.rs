//! Render layer: camera, viewport, grid, bed, entity painting, preview overlay,
//! snap markers. Bridges the pure kernel to egui's `Painter`.
//!
//! Submodules arrive with demands LCV-031 (camera) .. LCV-038 (snap markers).
//! The viewport rect itself is allocated by [`crate::app::App::update`] in
//! LCV-030; submodules consume that rect plus a [`egui::Painter`].

pub mod bed;
pub mod camera;
pub mod cursor;
pub mod entities;
pub mod grid;
pub mod palette;
pub mod preview;
pub mod raster;
pub mod selection;
pub mod snaps;

pub use bed::{Bed, draw_bed, draw_bed_fill};
pub use camera::Camera;
pub use cursor::{cursor_color, draw_crosshair, draw_pickbox};
pub use entities::{PaintOptions, arc_polyline, draw_entities, ellipse_polyline};
pub use grid::draw_grid;
pub use preview::{draw_dashed, draw_preview};
pub use selection::{draw_hover, draw_selection_highlight};
pub use snaps::draw_snap_marker;
