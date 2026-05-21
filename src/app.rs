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

use crate::document::{Command, Document, Entity, History};
use crate::geometry::{SnapResult, Vec2};
use crate::render::{Bed, Camera};
use crate::tools::ToolManager;

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
    /// Preview entities: the in-progress tool geometry painted with a
    /// translucent amber stroke. Defaults to empty; the Phase-4 Tool trait
    /// (LCV-040) will populate this each frame.
    pub preview_entities: Vec<Entity>,
    /// Active snap result: the snapped point and kind computed by the snap
    /// engine. Defaults to `None`; LCV-054 (tool snap integration) writes it.
    pub active_snap: Option<SnapResult>,
    /// Tool manager: owns the active tool and routes pointer + keyboard events.
    /// Initialized to `SelectTool` by default (LCV-040).
    pub tool_manager: ToolManager,
}

impl App {
    /// Commit a command to the document and history stack.
    ///
    /// This is the **only legal path** for tools to mutate the document.
    /// The command is executed via `history.commit(cmd, &mut document)`.
    ///
    /// LCV-040 AC#7, AC#8.
    pub fn commit(&mut self, cmd: Box<dyn Command>) {
        self.history.commit(cmd, &mut self.document);
    }
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

            // Update preview from tool (LCV-040 AC#9).
            self.preview_entities = self.tool_manager.preview();

            crate::render::draw_preview(&painter, rect, &self.camera, &self.preview_entities);

            if let Some(snap) = &self.active_snap {
                crate::render::draw_snap_marker(&painter, rect, &self.camera, snap);
            }

            // --- pointer / camera interaction (LCV-032) ---
            if response.hovered() {
                if let Some(hover_pos) = response.hover_pos() {
                    let world_pos = self.camera.screen_to_world(hover_pos);
                    self.last_cursor_world = Some(world_pos);

                    // Tool pointer events (LCV-040 AC#10-12).
                    // We need to split the borrow: extract tool_manager temporarily.
                    let mut tm = std::mem::take(&mut self.tool_manager);

                    // Pointer down.
                    if ctx.input(|i| i.pointer.primary_pressed()) {
                        tm.handle_pointer_down(world_pos, self);
                    }

                    // Pointer move (always called while hovering).
                    tm.handle_pointer_move(world_pos, self);

                    // Pointer up.
                    if ctx.input(|i| i.pointer.primary_released()) {
                        tm.handle_pointer_up(world_pos, self);
                    }

                    // Restore tool_manager.
                    self.tool_manager = tm;

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

            // Escape key to tool (LCV-040 AC#13).
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                let mut tm = std::mem::take(&mut self.tool_manager);
                tm.handle_key(egui::Key::Escape, self);
                self.tool_manager = tm;
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

    /// LCV-037 AC#7 — `App` carries `preview_entities` defaulting to empty.
    #[test]
    fn app_default_has_empty_preview_entities() {
        let app = App::default();
        assert!(app.preview_entities.is_empty());
    }

    /// LCV-040 AC#6 — `App::default().tool_manager` has `SelectTool` active.
    #[test]
    fn app_default_tool_manager_has_select() {
        let app = App::default();
        assert_eq!(app.tool_manager.active_tool_name(), "Select");
    }

    /// LCV-040 AC#7, AC#8 — `App::commit` adds entity and pushes onto history.
    #[test]
    fn app_commit_adds_entity_and_pushes_history() {
        use crate::document::CreateLine;
        use crate::geometry::Line;

        let mut app = App::default();
        assert_eq!(app.document.entity_count(), 0);
        assert!(!app.history.can_undo());

        let line = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        app.commit(Box::new(CreateLine::new(line)));

        assert_eq!(app.document.entity_count(), 1);
        assert!(app.history.can_undo());
    }
}
