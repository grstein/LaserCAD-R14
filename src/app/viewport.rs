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

use super::{App, apply_ortho, resolve_snap};
use crate::document::Document;
use crate::geometry::Vec2;
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

        // Middle-button pan.
        if response.dragged_by(egui::PointerButton::Middle) {
            handle_pan(&mut app.camera, response.drag_delta());
        }

        // Input before painting (DESIGN.md F1, LCV-162 AC 4): the snap glyph
        // and the crosshair come from this frame's pointer, not the last.
        // `cursor` is the resolved world point, `None` off the canvas.
        let cursor = response
            .hover_pos()
            .filter(|_| response.hovered())
            .map(|hover_pos| handle_hover(ctx, app, rect, hover_pos));

        paint(ui, rect, app, cursor);

        // Ask for a follow-up frame only while the canvas is live — see
        // `viewport_is_live` and AGENTS.md §Event flow → Repaint policy. The
        // status-bar coordinates are unaffected: `last_cursor_world` is written
        // only inside `handle_hover`, which already runs under
        // `response.hovered()`.
        if viewport_is_live(&response, app) {
            ctx.request_repaint();
        }
    });
}

/// Is anything on the canvas moving, so that this frame needs a successor?
///
/// Exactly three terms and no fourth (LCV-120): the pointer is over the canvas,
/// a drag is in progress — `dragged()` covers every button, including the
/// middle-button pan handled just above the call — or a tool preview is on
/// screen. Deliberately **not** `ToolManager::anchor`: a tool that is armed and
/// waiting for its first click has nothing moving, and waking on it would keep
/// the app running for most of a session.
///
/// Called after [`paint`], which is what assigns `app.preview_entities` from
/// the active tool, so the third term reads this frame's preview rather than
/// the previous one's.
fn viewport_is_live(response: &egui::Response, app: &App) -> bool {
    response.hovered() || response.dragged() || !app.preview_entities.is_empty()
}

/// Paint the canvas background and the whole render pipeline into `rect`.
///
/// Paint order (LCV-137 AC 1): canvas background, bed background fill, the
/// grid (when enabled), the bed border and exterior overlay, entities,
/// selection, preview, snap marker. The bed's fill is painted BEFORE the
/// grid and its border/overlay AFTER, so the grid's lines land on top of the
/// fill and are visible inside the bed rather than painted over by it — the
/// two halves of what used to be one `draw_bed` call, split for this order
/// (LCV-137 AC 2, `src/render/bed.rs`).
fn paint(ui: &egui::Ui, rect: egui::Rect, app: &mut App, _cursor: Option<Vec2>) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, crate::ui::CANVAS_BG);
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0_f32, egui::Color32::from_gray(64)),
    );

    // The bed is the document's, rebuilt every frame (LCV-114 AC 4/AC 15):
    // no cached copy, so a `SetBedSize` shows up on the very next frame.
    let bed = crate::render::Bed::from_size_mm(app.document.bed_mm);
    crate::render::draw_bed_fill(&painter, rect, &app.camera, &bed);

    if app.grid_enabled {
        crate::render::draw_grid(&painter, rect, &app.camera);
    }

    crate::render::draw_bed(&painter, rect, &app.camera, &bed);
    crate::render::draw_entities(
        &painter,
        rect,
        &app.camera,
        &app.document,
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
/// Returns the resolved world point, after snap and Ortho.
fn handle_hover(
    ctx: &egui::Context,
    app: &mut App,
    rect: egui::Rect,
    hover_pos: egui::Pos2,
) -> Vec2 {
    // `hover_pos` is global (the whole window); `rect.min` is the viewport's
    // own origin, which is nonzero whenever a panel claims space before the
    // `CentralPanel` — always, since the menubar and toolbar always do, and
    // further when the agent panel is open. Every camera call below wants
    // the viewport-local point, so the subtraction happens exactly once,
    // here, and every caller below (snap, world_pos, wheel zoom) reuses the
    // one result (LCV-137 AC 3).
    let local_pos = hover_pos - rect.min.to_vec2();

    // `resolve_snap` takes the global `hover_pos` plus `rect` and does this
    // same subtraction internally.
    if app.snap_enabled {
        let (anchor, kinds) = (app.tool_manager.anchor(), app.settings.object_snaps);
        let entities = &app.document.entities;
        app.active_snap = resolve_snap(hover_pos, rect, &app.camera, entities, anchor, kinds);
    }
    let world_pos = app
        .active_snap
        .map(|s| s.point)
        .unwrap_or_else(|| app.camera.screen_to_world(local_pos));
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

    // Wheel zoom around cursor. `zoom_around` (reached through
    // `handle_wheel_zoom`) treats its anchor as viewport-local, so this
    // passes `local_pos` — computed once, above — never the global
    // `hover_pos` (LCV-137 AC 3/AC 4).
    let scroll_y = ctx.input(|i| i.smooth_scroll_delta.y);
    if scroll_y != 0.0 {
        let factor = if scroll_y > 0.0 {
            WHEEL_ZOOM_FACTOR
        } else {
            1.0 / WHEEL_ZOOM_FACTOR
        };
        handle_wheel_zoom(&mut app.camera, local_pos, factor);
    }
    world_pos
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
///
/// A tool's result line (LCV-159) is drained first, into `command_feedback`,
/// so the hand-over below cannot swallow it.
pub(super) fn poll_successor(app: &mut App) {
    if let Some(message) = app.tool_manager.take_message() {
        app.command_feedback = message;
    }
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
mod tests;
