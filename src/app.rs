//! Top-level application state and egui wiring.
//!
//! [`App`] owns the live editing state — the [`Document`], the
//! undo/redo [`History`], the [`Camera`] world↔screen transform, and
//! pointer-derived state such as [`last_cursor_world`](App::last_cursor_world).
//! All entity mutation goes through `App::commit` once it lands; tools never
//! mutate the document directly.
//!
//! The render pipeline (camera, grid, bed, entities, preview, snaps) attaches
//! to the `CentralPanel` viewport rect set up by [`App::update`].

use crate::document::{Document, History};
use crate::geometry::Vec2;
use crate::render::{Bed, Camera};

/// Factor applied per mouse-wheel notch. `> 1.0` zooms in; `< 1.0` zooms out.
const WHEEL_ZOOM_FACTOR: f64 = 1.1;

/// Live application state. Owned by the eframe runtime via
/// [`crate::run`] and ticked once per frame in [`App::update`].
#[derive(Default)]
pub struct App {
    /// The CAD document — entities, schema, bounds.
    pub document: Document,
    /// Undo/redo history stack (depth = `HISTORY_DEPTH`).
    pub history: History,
    /// World↔screen transform plus zoom and pan state.
    pub camera: Camera,
    /// Laser bed configuration: size and origin in world space.
    pub bed: Bed,
    /// Last known cursor position in world space, updated while hovering
    /// the viewport. `None` before the cursor first enters the panel.
    pub last_cursor_world: Option<Vec2>,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let (rect, response) =
                ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());

            // Sync the camera's viewport size before any draw call consumes it.
            self.camera.viewport_size_px = [rect.width(), rect.height()];

            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 0.0, egui::Color32::from_gray(24));
            painter.rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(1.0, egui::Color32::from_gray(64)),
            );

            crate::render::draw_grid(&painter, rect, &self.camera);
            crate::render::draw_bed(&painter, rect, &self.camera, &self.bed);
            crate::render::draw_entities(
                &painter,
                rect,
                &self.camera,
                &self.document.entities,
                crate::render::PaintOptions::default(),
            );
            crate::render::draw_selection_highlight(
                &painter,
                rect,
                &self.camera,
                &self.document.entities,
                &self.document.selection,
            );

            // --- pointer / camera interaction (LCV-032) ---
            if response.hovered() {
                if let Some(hover_pos) = response.hover_pos() {
                    self.last_cursor_world = Some(self.camera.screen_to_world(hover_pos));

                    // Wheel zoom around cursor.
                    let scroll_y = ctx.input(|i| i.smooth_scroll_delta.y);
                    if scroll_y != 0.0 {
                        let factor = if scroll_y > 0.0 {
                            WHEEL_ZOOM_FACTOR
                        } else {
                            1.0 / WHEEL_ZOOM_FACTOR
                        };
                        handle_wheel_zoom(&mut self.camera, hover_pos, factor);
                    }
                }
            }

            // Middle-button pan.
            if response.dragged_by(egui::PointerButton::Middle) {
                handle_pan(&mut self.camera, response.drag_delta());
            }

            // Zoom-extents keys.
            if ctx.input(|i| i.key_pressed(egui::Key::F))
                || ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Num0))
            {
                handle_zoom_extents(
                    &mut self.camera,
                    &self.document,
                    [rect.width(), rect.height()],
                );
            }

            // Always repaint so cursor-coords and smooth camera motion stay live.
            ctx.request_repaint();
        });
    }
}

// ---------------------------------------------------------------------------
// Testable helpers
// ---------------------------------------------------------------------------

/// Apply a wheel-zoom step to `camera` anchored at `screen_anchor`.
pub fn handle_wheel_zoom(camera: &mut Camera, screen_anchor: egui::Pos2, factor: f64) {
    camera.zoom_around(screen_anchor, factor);
}

/// Pan the camera by a screen-space delta (middle-drag).
pub fn handle_pan(camera: &mut Camera, delta_screen_px: egui::Vec2) {
    camera.pan(delta_screen_px);
}

/// Fit the document bounds to the viewport.
pub fn handle_zoom_extents(camera: &mut Camera, document: &Document, viewport_size: [f32; 2]) {
    camera.zoom_extents(document.bounds(), viewport_size);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LCV-030 AC#1 — `App::default()` produces an empty document and an
    /// empty history.
    #[test]
    fn app_default_constructs_with_empty_document_and_history() {
        let app = App::default();
        assert_eq!(app.document.entity_count(), 0);
        assert!(!app.history.can_undo());
        assert!(!app.history.can_redo());
    }

    /// LCV-031 AC#13 — `App` carries a `Camera` field and it defaults to
    /// [`Camera::default()`].
    #[test]
    fn app_default_camera_matches_camera_default() {
        let app = App::default();
        assert_eq!(app.camera, Camera::default());
        assert_eq!(app.camera.mm_per_px, 1.0);
    }

    /// LCV-032 AC#1 — `App` carries `last_cursor_world` defaulting to `None`.
    #[test]
    fn app_default_has_no_cursor_world() {
        let app = App::default();
        assert_eq!(app.last_cursor_world, None);
    }

    /// LCV-034 AC#7 — `App` carries a `Bed` field that defaults to
    /// [`crate::render::Bed::default()`].
    #[test]
    fn app_default_bed_matches_bed_default() {
        let app = App::default();
        assert_eq!(app.bed, crate::render::Bed::default());
        assert_eq!(app.bed.size_mm, [400.0, 400.0]);
    }

    /// LCV-032 AC#8 — wheel zoom helper with positive factor zooms in.
    #[test]
    fn wheel_zoom_dispatch_positive_scroll_zooms_in() {
        let mut cam = Camera {
            center_world: Vec2::new(0.0, 0.0),
            mm_per_px: 1.0,
            viewport_size_px: [800.0, 600.0],
        };
        handle_wheel_zoom(&mut cam, egui::Pos2::new(400.0, 300.0), 1.1);
        assert!(cam.mm_per_px < 1.0);
    }

    /// LCV-032 AC#8 — wheel zoom helper with factor < 1 zooms out.
    #[test]
    fn wheel_zoom_dispatch_negative_scroll_zooms_out() {
        let mut cam = Camera {
            center_world: Vec2::new(0.0, 0.0),
            mm_per_px: 1.0,
            viewport_size_px: [800.0, 600.0],
        };
        handle_wheel_zoom(&mut cam, egui::Pos2::new(400.0, 300.0), 1.0 / 1.1);
        assert!(cam.mm_per_px > 1.0);
    }

    /// LCV-032 AC#8 — wheel zoom helper with factor 1.0 is a no-op.
    #[test]
    fn wheel_zoom_dispatch_unity_scroll_is_noop() {
        let mut cam = Camera {
            center_world: Vec2::new(0.0, 0.0),
            mm_per_px: 1.0,
            viewport_size_px: [800.0, 600.0],
        };
        handle_wheel_zoom(&mut cam, egui::Pos2::new(400.0, 300.0), 1.0);
        assert!((cam.mm_per_px - 1.0).abs() < 1e-12);
    }

    /// LCV-032 AC#8 — zoom extents dispatch resets on empty document.
    #[test]
    fn zoom_extents_dispatch_on_empty_document_resets() {
        let mut cam = Camera {
            center_world: Vec2::new(7.0, -3.0),
            mm_per_px: 4.0,
            viewport_size_px: [10.0, 10.0],
        };
        let doc = Document::default();
        handle_zoom_extents(&mut cam, &doc, [800.0, 600.0]);
        assert_eq!(cam.center_world, Vec2::new(0.0, 0.0));
        assert_eq!(cam.mm_per_px, 1.0);
        assert_eq!(cam.viewport_size_px, [800.0, 600.0]);
    }
}
