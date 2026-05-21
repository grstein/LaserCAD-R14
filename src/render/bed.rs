//! Laser bed visualization: filled rectangle + outer overlay.
//!
//! Draws a light-colored bed rectangle at configurable world coordinates
//! with a dark translucent overlay covering everything outside the bed.
//! The visual contrast makes the printable area obvious at a glance.
//!
//! Introduced by demand LCV-034.

use crate::geometry::Vec2;
use crate::render::Camera;

/// Laser bed configuration: size and world-space origin.
///
/// - `size_mm[0]` = width along world X; `size_mm[1]` = height along world Y.
/// - `origin_world` is the world-space coordinate of the **lower-left** corner
///   of the bed (so the bed spans `[origin.x, origin.x + size_mm[0]]` ×
///   `[origin.y, origin.y + size_mm[1]]`).
///
/// Default: 400×400 mm bed at world origin, matching common GRBL hobbyist
/// machine sizes (Ortur LM2/LM3, Atomstack A5/A10).
#[derive(Debug, Clone, PartialEq)]
pub struct Bed {
    /// Bed size in millimeters `[width, height]`.
    pub size_mm: [f64; 2],
    /// World-space coordinate of the lower-left corner of the bed.
    pub origin_world: Vec2,
}

impl Default for Bed {
    fn default() -> Self {
        Self {
            size_mm: [400.0, 400.0],
            origin_world: Vec2::new(0.0, 0.0),
        }
    }
}

impl Bed {
    /// Return the four world-space corners of the bed in the order:
    /// `[bottom_left, bottom_right, top_right, top_left]`.
    pub fn corners(&self) -> [Vec2; 4] {
        let bl = self.origin_world;
        let br = Vec2::new(self.origin_world.x + self.size_mm[0], self.origin_world.y);
        let tr = Vec2::new(
            self.origin_world.x + self.size_mm[0],
            self.origin_world.y + self.size_mm[1],
        );
        let tl = Vec2::new(self.origin_world.x, self.origin_world.y + self.size_mm[1]);
        [bl, br, tr, tl]
    }
}

/// Draw the laser bed rectangle with a border and outer overlay.
///
/// **Call order**: this function must be called AFTER [`crate::render::draw_grid`]
/// and BEFORE any entity rendering so the bed sits over the grid but under
/// drawn geometry.
///
/// Drawing steps:
/// 1. Convert the four bed corners to screen space.
/// 2. Fill the bed rectangle with a slightly lighter gray than the canvas
///    background to visually distinguish the printable area.
/// 3. Stroke the bed border with a light gray line.
/// 4. Draw four overlay strips (top/bottom/left/right) around the bed, clipped
///    to `rect`, covering the area outside the bed with a translucent black
///    overlay to discourage out-of-bounds placement.
pub fn draw_bed(painter: &egui::Painter, rect: egui::Rect, camera: &Camera, bed: &Bed) {
    // Convert bed corners to screen space.
    let corners = bed.corners();
    let offset = rect.min.to_vec2();
    let screen_bl = camera.world_to_screen(corners[0]) + offset;
    let screen_tr = camera.world_to_screen(corners[2]) + offset;

    // Compute the bed's screen-space bounding rectangle.
    let bed_screen_rect = egui::Rect::from_two_pos(screen_bl, screen_tr);

    // Fill the bed rectangle with a lighter gray (gray(40)) than the canvas
    // background (gray(24)) to make the printable area stand out.
    painter.rect_filled(bed_screen_rect, 0.0, egui::Color32::from_gray(40));

    // Stroke the bed border with a light gray line.
    painter.rect_stroke(
        bed_screen_rect,
        0.0,
        egui::Stroke::new(1.5, egui::Color32::from_gray(160)),
    );

    // Draw a dark translucent overlay outside the bed area.
    // The overlay is composed of four strips (top, bottom, left, right) that
    // together cover `rect \ bed_screen_rect`.
    let overlay_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 96);

    // Top strip: from rect.min.y to bed_screen_rect.min.y.
    if rect.min.y < bed_screen_rect.min.y {
        let top_strip =
            egui::Rect::from_min_max(rect.min, egui::Pos2::new(rect.max.x, bed_screen_rect.min.y));
        painter.rect_filled(top_strip, 0.0, overlay_color);
    }

    // Bottom strip: from bed_screen_rect.max.y to rect.max.y.
    if bed_screen_rect.max.y < rect.max.y {
        let bottom_strip =
            egui::Rect::from_min_max(egui::Pos2::new(rect.min.x, bed_screen_rect.max.y), rect.max);
        painter.rect_filled(bottom_strip, 0.0, overlay_color);
    }

    // Left strip: from rect.min.x to bed_screen_rect.min.x, covering only the
    // vertical slice between the bed's top and bottom.
    if rect.min.x < bed_screen_rect.min.x {
        let left_strip = egui::Rect::from_min_max(
            egui::Pos2::new(rect.min.x, bed_screen_rect.min.y),
            egui::Pos2::new(bed_screen_rect.min.x, bed_screen_rect.max.y),
        );
        painter.rect_filled(left_strip, 0.0, overlay_color);
    }

    // Right strip: from bed_screen_rect.max.x to rect.max.x, covering only the
    // vertical slice between the bed's top and bottom.
    if bed_screen_rect.max.x < rect.max.x {
        let right_strip = egui::Rect::from_min_max(
            egui::Pos2::new(bed_screen_rect.max.x, bed_screen_rect.min.y),
            egui::Pos2::new(rect.max.x, bed_screen_rect.max.y),
        );
        painter.rect_filled(right_strip, 0.0, overlay_color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC#1 — `Bed` struct can be constructed with custom values.
    #[test]
    fn bed_struct_constructs() {
        let bed = Bed {
            size_mm: [100.0, 50.0],
            origin_world: Vec2::new(10.0, 20.0),
        };
        assert_eq!(bed.size_mm, [100.0, 50.0]);
        assert_eq!(bed.origin_world, Vec2::new(10.0, 20.0));
    }

    /// AC#2 — `Bed::default()` returns a 400×400 mm bed at the origin.
    #[test]
    fn bed_default_is_400x400_at_origin() {
        let bed = Bed::default();
        assert_eq!(bed.size_mm, [400.0, 400.0]);
        assert_eq!(bed.origin_world, Vec2::new(0.0, 0.0));
    }

    /// AC#3 — Default bed's corners are at `[(0,0), (400,0), (400,400), (0,400)]`.
    #[test]
    fn bed_corners_default() {
        let bed = Bed::default();
        let corners = bed.corners();
        assert_eq!(corners[0], Vec2::new(0.0, 0.0)); // bottom_left
        assert_eq!(corners[1], Vec2::new(400.0, 0.0)); // bottom_right
        assert_eq!(corners[2], Vec2::new(400.0, 400.0)); // top_right
        assert_eq!(corners[3], Vec2::new(0.0, 400.0)); // top_left
    }

    /// AC#3 (second case) — Offset bed's corners match the expected values.
    #[test]
    fn bed_corners_offset() {
        let bed = Bed {
            size_mm: [100.0, 50.0],
            origin_world: Vec2::new(10.0, 20.0),
        };
        let corners = bed.corners();
        assert_eq!(corners[0], Vec2::new(10.0, 20.0)); // bottom_left
        assert_eq!(corners[1], Vec2::new(110.0, 20.0)); // bottom_right
        assert_eq!(corners[2], Vec2::new(110.0, 70.0)); // top_right
        assert_eq!(corners[3], Vec2::new(10.0, 70.0)); // top_left
    }

    /// AC#5 — `draw_bed` does not panic on degenerate inputs.
    #[test]
    fn draw_bed_does_not_panic_on_degenerate_inputs() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("test_bed"),
            ));
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(800.0, 600.0));

            // Degenerate camera: zero viewport.
            let cam1 = Camera {
                viewport_size_px: [0.0, 0.0],
                ..Camera::default()
            };
            draw_bed(&painter, rect, &cam1, &Bed::default());

            // Degenerate camera: huge zoom-in.
            let cam2 = Camera {
                mm_per_px: 1e-9,
                viewport_size_px: [800.0, 600.0],
                ..Camera::default()
            };
            draw_bed(&painter, rect, &cam2, &Bed::default());

            // Degenerate camera: huge zoom-out.
            let cam3 = Camera {
                mm_per_px: 1e9,
                viewport_size_px: [800.0, 600.0],
                ..Camera::default()
            };
            draw_bed(&painter, rect, &cam3, &Bed::default());

            // Degenerate bed: zero size.
            let bed1 = Bed {
                size_mm: [0.0, 0.0],
                origin_world: Vec2::new(0.0, 0.0),
            };
            draw_bed(&painter, rect, &Camera::default(), &bed1);

            // Degenerate bed: absurdly large.
            let bed2 = Bed {
                size_mm: [1e9, 1e9],
                origin_world: Vec2::new(0.0, 0.0),
            };
            draw_bed(&painter, rect, &Camera::default(), &bed2);
        });
    }
}
