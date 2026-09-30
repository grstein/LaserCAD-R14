//! The canvas paint pipeline of [`super::draw`], split out of `viewport.rs`
//! at the LOC-cap seam named in the LCV-162 plan; no behaviour of its own.
//!
//! MUST NOT import `eframe` or `rfd`.

use super::App;
use crate::geometry::Vec2;

/// Paint the canvas background and the whole render pipeline into `rect`.
///
/// Paint order (LCV-137 AC 1): canvas background, bed background fill, the
/// grid (when enabled), the bed border and exterior overlay, entities,
/// selection, preview, snap marker, then the cursor (LCV-162): the pickbox
/// while the tool waits for an entity pick, and the crosshair through the
/// resolved `cursor` point as the very last shapes. `cursor` is `None` off
/// the canvas, which paints neither. The bed's fill is painted BEFORE the
/// grid and its border/overlay AFTER, so the grid's lines land on top of the
/// fill and are visible inside the bed rather than painted over by it — the
/// two halves of what used to be one `draw_bed` call, split for this order
/// (LCV-137 AC 2, `src/render/bed.rs`).
pub(super) fn paint(ui: &egui::Ui, rect: egui::Rect, app: &mut App, cursor: Option<Vec2>) {
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

    if let Some(cursor) = cursor {
        if app.tool_manager.wants_entity_pick() {
            let aperture = crate::tools::PICK_APERTURE_PT as f32;
            crate::render::draw_pickbox(&painter, rect, &app.camera, cursor, aperture);
        }
        crate::render::draw_crosshair(&painter, rect, &app.camera, cursor);
    }
}
