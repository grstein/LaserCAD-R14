//! SVG export (LaserGRBL-compatible) and import (`roxmltree`).
//!
//! Kernel module — MUST NOT import `egui`, `eframe`, or `rfd`.
//!
//! Submodules arrive with demands LCV-056 (export) and LCV-057 (import).

pub mod export;
pub mod import;

pub use export::export_svg;
pub use import::{import_svg, SvgImportError};
