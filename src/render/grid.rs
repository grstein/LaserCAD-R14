//! Responsive grid renderer.
//!
//! Draws a world-coordinate aligned orthogonal grid whose minor spacing
//! snaps to a 1-2-5 decade ladder so the screen spacing stays readable at
//! every zoom level.

use crate::geometry::Vec2;
use crate::render::{Camera, palette};

/// Minimum desired screen spacing between minor grid lines (pixels).
const MIN_SCREEN_PX: f64 = 8.0;

/// Clamp exponent for the spacing ladder (`10^k` where `k` is clamped).
const K_MIN: i32 = -6;
const K_MAX: i32 = 6;

/// Return the minor grid spacing in millimetres for the current zoom.
///
/// Chooses the smallest member of `{10^k, 2·10^k, 5·10^k}` such that
/// `spacing_mm / mm_per_px >= MIN_SCREEN_PX`.  The exponent `k` is
/// clamped to `[K_MIN, K_MAX]`.
pub(crate) fn pick_minor_spacing_mm(mm_per_px: f64) -> f64 {
    debug_assert!(mm_per_px > 0.0, "mm_per_px must be > 0");
    let mut k = K_MIN;
    loop {
        let decade = 10_f64.powi(k);
        for m in [1.0, 2.0, 5.0] {
            let spacing = m * decade;
            if spacing / mm_per_px >= MIN_SCREEN_PX {
                return spacing;
            }
        }
        if k >= K_MAX {
            return 5.0 * decade; // largest clamped value
        }
        k += 1;
    }
}

/// Fixed major step: every 10th minor line is major.
pub(crate) fn pick_major_step(_minor_spacing_mm: f64) -> u32 {
    10
}

/// Draw the grid into the viewport.
///
/// `rect` is the viewport's screen-space rectangle — in *global* (window)
/// screen coordinates, since that is what every caller in this tree holds.
/// `Camera::screen_to_world` / `Camera::world_to_screen` operate on
/// *viewport-local* coordinates (origin at the canvas's own top-left), so
/// every global position is converted by subtracting `rect.min` exactly once
/// before it reaches the camera, and every local result the camera hands
/// back is converted the other way — `+ rect.min` — exactly once before it
/// reaches the painter. LCV-137 AC 3.
pub fn draw_grid(painter: &egui::Painter, rect: egui::Rect, camera: &Camera) {
    let minor_mm = pick_minor_spacing_mm(camera.mm_per_px);
    let major_step = pick_major_step(minor_mm) as i64;

    // Visible world bounds. `rect.min`/`rect.max` are global; `screen_to_world`
    // wants the viewport-local corners instead — `(0, 0)` and the rect's own
    // size — never the global rect itself (LCV-137 AC 3 — before this fix
    // this line passed the global rect straight through, drifting the
    // visible grid by the toolbar width and menubar height whenever the
    // viewport did not start at the window's top-left corner).
    let local_tl = egui::Pos2::ZERO;
    let local_br = (rect.max - rect.min).to_pos2();
    let world_tl = camera.screen_to_world(local_tl);
    let world_br = camera.screen_to_world(local_br);
    let world_left = world_tl.x.min(world_br.x);
    let world_right = world_tl.x.max(world_br.x);
    let world_bottom = world_tl.y.min(world_br.y);
    let world_top = world_tl.y.max(world_br.y);

    let offset = rect.min.to_vec2();
    // 1 pt on a pixel centre (LCV-164 AC 1): a 1 pt line straddling two
    // pixels paints as two half-bright ones.
    let minor_stroke = egui::Stroke::new(1.0_f32, palette::GRID_MINOR);
    let major_stroke = egui::Stroke::new(1.0_f32, palette::GRID_MAJOR);
    let snap = |v: f32| painter.round_to_pixel_center(v);

    // Vertical lines (constant world X).
    let mut x = (world_left / minor_mm).floor() * minor_mm;
    while x <= world_right + minor_mm * 0.5 {
        let idx = (x / minor_mm).round() as i64;
        let stroke = if idx % major_step == 0 {
            major_stroke
        } else {
            minor_stroke
        };
        let mut p0 = camera.world_to_screen(Vec2::new(x, world_bottom)) + offset;
        let mut p1 = camera.world_to_screen(Vec2::new(x, world_top)) + offset;
        (p0.x, p1.x) = (snap(p0.x), snap(p0.x));
        painter.line_segment([p0, p1], stroke);
        x += minor_mm;
    }

    // Horizontal lines (constant world Y).
    let mut y = (world_bottom / minor_mm).floor() * minor_mm;
    while y <= world_top + minor_mm * 0.5 {
        let idx = (y / minor_mm).round() as i64;
        let stroke = if idx % major_step == 0 {
            major_stroke
        } else {
            minor_stroke
        };
        let mut p0 = camera.world_to_screen(Vec2::new(world_left, y)) + offset;
        let mut p1 = camera.world_to_screen(Vec2::new(world_right, y)) + offset;
        (p0.y, p1.y) = (snap(p0.y), snap(p0.y));
        painter.line_segment([p0, p1], stroke);
        y += minor_mm;
    }
}

#[cfg(test)]
mod tests;
