//! The `CentralPanel` viewport: painting, pointer routing, and camera actions.
//!
//! [`draw`] is the whole canvas phase of a frame — camera sync, the render
//! pipeline (grid, bed, entities, selection, preview, snap marker), pointer
//! events into the active tool, wheel zoom and middle-drag pan. It is called
//! once per frame from [`App::update_ui`](super::App::update_ui).
//!
//! No key is read here: `src/app/input.rs` is the single keyboard gate
//! (LCV-103 / ADR 0002 §A6). A `ctx.input(|i| i.key_*)` call in this file is a
//! double dispatch (defect D4) and a review blocker.
//!
//! MUST NOT import `eframe` or `rfd`.

use super::{apply_ortho, resolve_snap, App};
use crate::document::Document;
use crate::render::Camera;
use crate::tools::{PointerButton, PointerEvent};

/// Factor applied per mouse-wheel notch. `> 1.0` zooms in; `< 1.0` zooms out.
const WHEEL_ZOOM_FACTOR: f64 = 1.1;

/// Render the canvas and route pointer input for one frame.
pub fn draw(ctx: &egui::Context, app: &mut App) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let (rect, response) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());

        // Sync the camera's viewport size before any draw call consumes it.
        app.camera.viewport_size_px = [rect.width(), rect.height()];

        paint(ui, rect, app);

        // --- pointer / camera interaction (LCV-032 / LCV-041) ---
        if response.hovered() {
            if let Some(hover_pos) = response.hover_pos() {
                handle_hover(ctx, app, rect, hover_pos);
            }
        }

        // Middle-button pan.
        if response.dragged_by(egui::PointerButton::Middle) {
            handle_pan(&mut app.camera, response.drag_delta());
        }

        // Always repaint so cursor-coords and smooth camera motion stay live.
        ctx.request_repaint();
    });
}

/// Paint the canvas background and the whole render pipeline into `rect`.
fn paint(ui: &egui::Ui, rect: egui::Rect, app: &mut App) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, crate::ui::CANVAS_BG);
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, egui::Color32::from_gray(64)),
    );

    if app.grid_enabled {
        crate::render::draw_grid(&painter, rect, &app.camera);
    }
    // The bed is the document's, rebuilt every frame (LCV-114 AC 4/AC 15):
    // no cached copy, so a `SetBedSize` shows up on the very next frame.
    let bed = crate::render::Bed::from_size_mm(app.document.bed_mm);
    crate::render::draw_bed(&painter, rect, &app.camera, &bed);
    crate::render::draw_entities(
        &painter,
        rect,
        &app.camera,
        &app.document.entities,
        crate::render::PaintOptions::default(),
    );
    crate::render::draw_selection_highlight(
        &painter,
        rect,
        &app.camera,
        &app.document.entities,
        &app.document.selection,
    );

    // Update preview from tool (LCV-040 AC#9).
    app.preview_entities = app.tool_manager.preview();

    crate::render::draw_preview(&painter, rect, &app.camera, &app.preview_entities);

    if let Some(snap) = &app.active_snap {
        crate::render::draw_snap_marker(&painter, rect, &app.camera, snap);
    }
}

/// Resolve the cursor position and route pointer events while the viewport is
/// hovered: snap, ortho lock, press / move / release, and wheel zoom.
fn handle_hover(ctx: &egui::Context, app: &mut App, rect: egui::Rect, hover_pos: egui::Pos2) {
    // Resolve snap; viewport-local pos = hover_pos - rect.min.
    if app.snap_enabled {
        app.active_snap = resolve_snap(hover_pos, rect, &app.camera, &app.document.entities);
    }
    let world_pos = app
        .active_snap
        .map(|s| s.point)
        .unwrap_or_else(|| app.camera.screen_to_world(hover_pos - rect.min.to_vec2()));
    // Ortho lock (LCV-053): clamp to nearest cardinal axis from the active
    // tool's anchor, when ortho mode is active.
    let world_pos = if app.ortho_enabled {
        match app.tool_manager.anchor() {
            Some(anchor) => apply_ortho(anchor, world_pos),
            None => world_pos,
        }
    } else {
        world_pos
    };
    app.last_cursor_world = Some(world_pos);

    // Pointer events (LCV-041).
    if ctx.input(|i| i.pointer.primary_pressed()) {
        let shift = ctx.input(|i| i.modifiers.shift);
        send_pointer(
            app,
            PointerEvent::Press {
                world_pos,
                button: PointerButton::Primary,
                shift,
            },
        );
        // LCV-049: poll for a successor tool after press events.
        poll_successor(app);
    }
    send_pointer(app, PointerEvent::Move { world_pos });
    if ctx.input(|i| i.pointer.primary_released()) {
        let shift = ctx.input(|i| i.modifiers.shift);
        send_pointer(
            app,
            PointerEvent::Release {
                world_pos,
                button: PointerButton::Primary,
                shift,
            },
        );
        // LCV-049: poll for a successor tool after release events.
        poll_successor(app);
    }

    // Wheel zoom around cursor.
    let scroll_y = ctx.input(|i| i.smooth_scroll_delta.y);
    if scroll_y != 0.0 {
        let factor = if scroll_y > 0.0 {
            WHEEL_ZOOM_FACTOR
        } else {
            1.0 / WHEEL_ZOOM_FACTOR
        };
        handle_wheel_zoom(&mut app.camera, hover_pos, factor);
    }
}

/// Hand one pointer event to the active tool.
fn send_pointer(app: &mut App, event: PointerEvent) {
    app.tool_manager
        .on_pointer_event(&event, &mut app.document, &mut app.history);
}

/// Honour a tool succession request (LCV-049): a tool that finished may name
/// the tool that replaces it. Polled after press and release, never after a
/// move — and, since LCV-111, after a consumed command-line input, so the
/// typed path hands over exactly like the pointer path (`pub(super)` for
/// `super::cmdline`; there is deliberately only one copy of this body).
pub(super) fn poll_successor(app: &mut App) {
    if let Some(t) = app.tool_manager.take_successor() {
        app.tool_manager.set_tool(t);
    }
}

/// Apply a wheel-zoom step to `camera` anchored at `screen_anchor`.
pub fn handle_wheel_zoom(camera: &mut Camera, screen_anchor: egui::Pos2, factor: f64) {
    camera.zoom_around(screen_anchor, factor);
}

/// Pan the camera by a screen-space delta (middle-drag).
pub fn handle_pan(camera: &mut Camera, delta_screen_px: egui::Vec2) {
    camera.pan(delta_screen_px);
}

/// Fit the document bounds to the viewport.
///
/// No-ops on a zero-area viewport (LCV-103). `Camera::default()` carries the
/// `[0.0, 0.0]` sentinel until the first `CentralPanel` syncs a real size, and
/// the keyboard gate runs before that panel — zooming to a zero-area rect
/// would reset the camera for no reason.
pub fn handle_zoom_extents(camera: &mut Camera, document: &Document, viewport_size: [f32; 2]) {
    if viewport_size[0] <= 0.0 || viewport_size[1] <= 0.0 {
        return;
    }
    camera.zoom_extents(document.bounds(), viewport_size);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Vec2;

    /// LCV-114 AC 4/AC 15 — the canvas builds its bed from the document
    /// every frame and keeps no copy, so a `SetBedSize` is visible on the
    /// next frame with no cache to invalidate.
    ///
    /// A bounded source scan: `draw_viewport` needs a live `egui::Ui` and a
    /// painter, and the property at stake is *where the size comes from*.
    /// Each claim carries a positive control over the same slice, so an
    /// absence assertion cannot pass vacuously.
    #[test]
    fn the_canvas_bed_comes_from_the_document() {
        let src = include_str!("viewport.rs");
        let cfg_test_at = src
            .find("\n#[cfg(test)]")
            .expect("viewport.rs must have a test module to bound the scan");
        let implementation = &src[..cfg_test_at];
        assert!(
            implementation.contains("crate::render::draw_bed(&painter, rect, &app.camera, &bed)"),
            "positive control: the canvas must draw the bed"
        );
        assert!(
            implementation.contains("crate::render::Bed::from_size_mm(app.document.bed_mm)"),
            "the drawn bed must be built from the document (AC 4)"
        );
        assert!(
            !implementation.contains("app.bed"),
            "there is no App-owned bed to read (AC 4)"
        );
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

    /// LCV-103 AC#14 — `handle_zoom_extents` no-ops on a zero-area viewport.
    ///
    /// The keyboard gate runs before the `CentralPanel`, so on frame 0 it
    /// reads the `[0.0, 0.0]` sentinel of a fresh `Camera`. Pressing `F`
    /// then must leave the camera exactly as it was.
    #[test]
    fn zoom_extents_noop_on_zero_area_viewport() {
        let mut cam = Camera {
            center_world: Vec2::new(7.0, -3.0),
            mm_per_px: 4.0,
            viewport_size_px: [0.0, 0.0],
        };
        let doc = Document::default();
        handle_zoom_extents(&mut cam, &doc, [0.0, 0.0]);
        assert_eq!(cam.center_world, Vec2::new(7.0, -3.0));
        assert_eq!(cam.mm_per_px, 4.0);
        assert_eq!(cam.viewport_size_px, [0.0, 0.0]);

        // A viewport with one zero dimension is equally degenerate.
        handle_zoom_extents(&mut cam, &doc, [800.0, 0.0]);
        assert_eq!(cam.mm_per_px, 4.0);
    }
}
