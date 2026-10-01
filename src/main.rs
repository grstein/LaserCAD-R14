//! Binary entry point. The real work lives in the library crate.
// Release builds on Windows use the GUI subsystem, so no console window opens
// beside the app; debug builds keep the console for logs (LCV-201).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() -> eframe::Result<()> {
    lasercad::run()
}
