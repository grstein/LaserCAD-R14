//! SVG export (LaserGRBL-compatible) and import (`roxmltree`).
//!
//! Kernel module — MUST NOT import `egui`, `eframe`, or `rfd`.
//!
//! Submodules arrive with demands LCV-056 (export) and LCV-057 (import);
//! LCV-114 split the root-`<svg>` header (the bed size) into its own module.

mod css_color;
pub mod export;
pub mod header;
pub mod import;
mod layers;
mod length;
mod matrix;
mod path_data;
mod viewport;

pub use export::{export_layer_svg, export_svg};
pub use import::{ImportedSvg, SvgImportError, import_svg};
