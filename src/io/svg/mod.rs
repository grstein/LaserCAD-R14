//! SVG export (LaserGRBL-compatible) and import (`roxmltree`).
//!
//! Kernel module — MUST NOT import `egui`, `eframe`, or `rfd`.
//!
//! Submodules arrive with demands LCV-056 (export) and LCV-057 (import);
//! LCV-114 split the root-`<svg>` header (the bed size) into its own module.

pub mod export;
pub mod header;
pub mod import;

pub use export::{export_svg, Preset};
pub use import::{import_svg, ImportedSvg, SvgImportError};
