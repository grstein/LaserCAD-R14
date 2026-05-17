//! LaserCAD v2 library crate.
//!
//! Module boundaries and rules are documented in `AGENTS.md` at the repository
//! root. Kernel modules ([`geometry`], [`document`], [`io::svg`],
//! [`agent::classifier`], [`text`]) MUST NOT import `egui`, `eframe`, or `rfd`.

pub mod agent;
pub mod app;
pub mod document;
pub mod geometry;
pub mod io;
pub mod render;
pub mod text;
pub mod tools;
pub mod ui;
pub mod util;

/// Application entry point. Opens the bootstrap window.
///
/// Wired by [`crate::app::App`] — see `src/app.rs`.
pub fn run() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("LaserCAD v2 — bootstrap"),
        ..Default::default()
    };
    eframe::run_native(
        "LaserCAD v2",
        native_options,
        Box::new(|_cc| Ok(Box::<app::App>::default())),
    )
}
