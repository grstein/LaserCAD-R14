//! Top-level application state and egui wiring.
//!
//! For now the [`App`] is a placeholder — it shows a centered label so the
//! bootstrap demand (LCV-007) can prove the toolchain end-to-end. The real
//! drawing surface, command line, menubar, etc. arrive in later demands.

pub const MODULE: &str = "app";

#[derive(Default)]
pub struct App;

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(32.0);
                ui.heading("LaserCAD v2");
                ui.label("Bootstrap window. PLAN.md is the roadmap.");
            });
        });
    }
}
