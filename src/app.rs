//! Top-level application state and egui wiring.
//!
//! [`App`] owns the live editing state — currently the [`Document`] and the
//! undo/redo [`History`]. Later demands extend the struct (camera in LCV-031,
//! cursor in LCV-032, active tool in Phase 4, etc.). All entity mutation goes
//! through `App::commit` once it lands; tools never mutate the document
//! directly.
//!
//! The render pipeline (camera, grid, bed, entities, preview, snaps) attaches
//! to the `CentralPanel` viewport rect set up by [`App::update`]. The body of
//! `update` is intentionally minimal right now: it claims the full panel rect
//! as a viewport area and fills it with a dark gray placeholder so subsequent
//! Phase-3 demands have a surface to paint into.

use crate::document::{Document, History};

/// Live application state. Owned by the eframe runtime via
/// [`crate::run`] and ticked once per frame in [`App::update`].
///
/// TODO(LCV-031): add `camera: Camera` for world ↔ screen mapping and
/// zoom/pan state. Held off here so this demand can land without depending
/// on Camera math.
#[derive(Default)]
pub struct App {
    /// The CAD document — entities, schema, bounds.
    pub document: Document,
    /// Undo/redo history stack (depth = `HISTORY_DEPTH`).
    pub history: History,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Claim the full available rect as the viewport surface. The
            // render pipeline (LCV-031..LCV-038) paints into this rect.
            // `Sense::hover()` is the minimum needed for the panel to lay
            // out; LCV-032 widens it to `click_and_drag` for pointer input.
            let (rect, _response) =
                ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
            let painter = ui.painter_at(rect);

            // Dark canvas marker — placeholder visual until the grid (LCV-033)
            // and bed (LCV-034) renderers land. The final theme is owned by
            // LCV-071; the gray values here are arbitrary markers.
            painter.rect_filled(rect, 0.0, egui::Color32::from_gray(24));
            painter.rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(1.0, egui::Color32::from_gray(64)),
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::App;

    /// LCV-030 AC#1 — `App::default()` produces an empty document and an
    /// empty history. This is the contract every later demand relies on
    /// when it adds fields to `App`.
    #[test]
    fn app_default_constructs_with_empty_document_and_history() {
        let app = App::default();
        assert_eq!(app.document.entity_count(), 0);
        assert!(!app.history.can_undo());
        assert!(!app.history.can_redo());
    }
}
