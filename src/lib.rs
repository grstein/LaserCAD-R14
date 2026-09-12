//! LaserCAD v2 library crate.
//!
//! Module boundaries and rules are documented in `AGENTS.md` at the repository
//! root. Kernel modules ([`geometry`], [`document`], [`io::svg`], [`text`])
//! MUST NOT import `egui`, `eframe`, or `rfd`.

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

/// Window title shown in the OS title bar.
///
/// Frozen by demand LCV-007 — the Phase 0 window contract — and amended by
/// LCV-105, which dropped the Phase 0 suffix now that the product ships.
/// Downstream demands (LCV-030, LCV-067) reference this constant by name
/// rather than copying the literal so the title stays single-sourced; any
/// future change follows the same route — the constant, its test and the
/// LCV-007 demand body move together.
pub const APP_TITLE: &str = "LaserCAD v2";

/// Default native window inner size in screen pixels.
///
/// Pixels (egui surface units), NOT millimeters — window size is a UI
/// concern, not a CAD-world quantity. Frozen by demand LCV-007.
pub const DEFAULT_WINDOW_SIZE: [f32; 2] = [1280.0, 800.0];

/// Application entry point. Opens the main window.
///
/// Wired by [`crate::app::App`] — see `src/app/mod.rs`. Returns the
/// [`eframe::Result`] from [`eframe::run_native`] unchanged so callers
/// (notably `src/main.rs`) can propagate it with `?`.
pub fn run() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(APP_TITLE)
            .with_inner_size(DEFAULT_WINDOW_SIZE)
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "LaserCAD v2",
        native_options,
        Box::new(|_cc| Ok(Box::new(app::App::new()))),
    )
}

#[cfg(test)]
mod tests {
    use super::{APP_TITLE, DEFAULT_WINDOW_SIZE};

    /// LCV-007 — freeze the window contract, as amended by LCV-105.
    ///
    /// The title literal and default inner size are part of the demand's
    /// acceptance criteria; this test catches any silent rename. The name is
    /// deliberately unchanged so `git log -S` keeps tracking the contract
    /// across the LCV-105 title change.
    #[test]
    fn bootstrap_window_contract() {
        assert_eq!(APP_TITLE, "LaserCAD v2");
        assert_eq!(DEFAULT_WINDOW_SIZE, [1280.0_f32, 800.0_f32]);
    }
}
