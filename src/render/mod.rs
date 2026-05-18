//! Render layer: camera, viewport, grid, bed, entity painting, preview overlay,
//! snap markers. Bridges the pure kernel to egui's `Painter`.
//!
//! Submodules arrive with demands LCV-031 (camera) .. LCV-038 (snap markers).
//! The viewport rect itself is allocated by [`crate::app::App::update`] in
//! LCV-030; submodules consume that rect plus a [`egui::Painter`].

pub mod camera;

pub use camera::Camera;
