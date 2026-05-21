//! Responsive grid renderer.
//!
//! Draws a world-coordinate aligned orthogonal grid whose minor spacing
//! snaps to a 1-2-5 decade ladder so the screen spacing stays readable at
//! every zoom level.

use crate::geometry::Vec2;
use crate::render::Camera;

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
/// `rect` is the viewport's screen-space rectangle; its origin is added to
/// every projected point so shapes land in the correct painter clip rect.
pub fn draw_grid(painter: &egui::Painter, rect: egui::Rect, camera: &Camera) {
    let minor_mm = pick_minor_spacing_mm(camera.mm_per_px);
    let major_step = pick_major_step(minor_mm) as i64;

    // Visible world bounds.
    let world_tl = camera.screen_to_world(rect.min);
    let world_br = camera.screen_to_world(rect.max);
    let world_left = world_tl.x.min(world_br.x);
    let world_right = world_tl.x.max(world_br.x);
    let world_bottom = world_tl.y.min(world_br.y);
    let world_top = world_tl.y.max(world_br.y);

    let offset = rect.min.to_vec2();
    let minor_stroke = egui::Stroke::new(0.5, egui::Color32::from_gray(48));
    let major_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(96));

    // Vertical lines (constant world X).
    let mut x = (world_left / minor_mm).floor() * minor_mm;
    while x <= world_right + minor_mm * 0.5 {
        let idx = (x / minor_mm).round() as i64;
        let stroke = if idx % major_step == 0 {
            major_stroke
        } else {
            minor_stroke
        };
        let p0 = camera.world_to_screen(Vec2::new(x, world_bottom)) + offset;
        let p1 = camera.world_to_screen(Vec2::new(x, world_top)) + offset;
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
        let p0 = camera.world_to_screen(Vec2::new(world_left, y)) + offset;
        let p1 = camera.world_to_screen(Vec2::new(world_right, y)) + offset;
        painter.line_segment([p0, p1], stroke);
        y += minor_mm;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_public_api_exists() {
        let s = pick_minor_spacing_mm(1.0);
        assert!(s.is_finite() && s > 0.0);
        assert_eq!(pick_major_step(s), 10);
    }

    #[test]
    fn minor_spacing_at_unit_zoom_is_10mm() {
        assert_eq!(pick_minor_spacing_mm(1.0), 10.0);
    }

    #[test]
    fn minor_spacing_at_zoom_in_is_1mm() {
        assert_eq!(pick_minor_spacing_mm(0.1), 1.0);
    }

    #[test]
    fn minor_spacing_at_zoom_out_is_100mm() {
        assert_eq!(pick_minor_spacing_mm(10.0), 100.0);
    }

    #[test]
    fn minor_spacing_at_half_unit_zoom_is_5mm() {
        assert_eq!(pick_minor_spacing_mm(0.5), 5.0);
    }

    #[test]
    fn minor_spacing_at_micro_zoom() {
        assert_eq!(pick_minor_spacing_mm(0.001), 0.01);
    }

    #[test]
    fn minor_spacing_clamped_at_extreme_zoom() {
        let small = pick_minor_spacing_mm(1e-9);
        assert!(small >= 1e-6);
        assert!(small.is_finite());
        let large = pick_minor_spacing_mm(1e9);
        assert!(large >= 1e6);
        assert!(large.is_finite());
    }

    #[test]
    fn major_step_is_ten() {
        assert_eq!(pick_major_step(1.0), 10);
        assert_eq!(pick_major_step(100.0), 10);
    }

    #[test]
    fn draw_grid_does_not_panic_on_degenerate_cameras() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("test"),
            ));
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(800.0, 600.0));

            let cams = [
                Camera {
                    viewport_size_px: [0.0, 0.0],
                    ..Camera::default()
                },
                Camera {
                    mm_per_px: 1e-9,
                    viewport_size_px: [800.0, 600.0],
                    ..Camera::default()
                },
                Camera {
                    mm_per_px: 1e9,
                    viewport_size_px: [800.0, 600.0],
                    ..Camera::default()
                },
                Camera {
                    center_world: Vec2::new(1e6, 1e6),
                    viewport_size_px: [800.0, 600.0],
                    ..Camera::default()
                },
            ];
            for cam in &cams {
                draw_grid(&painter, rect, cam);
            }
        });
    }
}
