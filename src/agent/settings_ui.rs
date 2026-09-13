//! egui panel for editing agent connection settings.
//!
//! One of the **two** files in `src/agent/` that may import `egui` — the other
//! is `panel.rs` (ADR 0007 §D8). Every other `agent/` submodule is kernel-pure,
//! and no file here may import `eframe` or `rfd`.

use egui;

use crate::io::settings::Settings;

/// Draw the agent-settings form into `ui`.
///
/// Renders two labelled rows in a two-column grid:
/// - **Endpoint URL** — a plain single-line text field editing `settings.agent_endpoint`.
/// - **API Key** — a password-masked single-line field editing `settings.agent_api_key`
///   (shown as bullet characters •; excluded from egui's accessibility text).
///
/// Both fields have a minimum width of 320 logical pixels.
///
/// Returns `true` if either field was modified in this frame, `false` otherwise.
pub fn draw_agent_settings(ui: &mut egui::Ui, settings: &mut Settings) -> bool {
    let mut changed = false;

    egui::Grid::new("agent_settings_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Endpoint URL");
            let resp = ui.add(
                egui::TextEdit::singleline(&mut settings.agent_endpoint)
                    .min_size(egui::vec2(320.0, 0.0)),
            );
            if resp.changed() {
                changed = true;
            }
            ui.end_row();

            ui.label("API Key");
            let resp = ui.add(
                egui::TextEdit::singleline(&mut settings.agent_api_key)
                    .password(true)
                    .min_size(egui::vec2(320.0, 0.0)),
            );
            if resp.changed() {
                changed = true;
            }
            ui.end_row();
        });

    changed
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// §T3 — AC 6 — no synthetic input → returns false.
    #[test]
    fn no_input_returns_false() {
        let ctx = egui::Context::default();
        let mut settings = Settings::default();
        let mut changed = false;
        let _output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                changed = draw_agent_settings(ui, &mut settings);
            });
        });
        assert!(!changed);
    }
}
