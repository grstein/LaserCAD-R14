//! Laser bed visualization: filled rectangle + outer overlay.
//!
//! Draws a light-colored bed rectangle at configurable world coordinates
//! with a dark translucent overlay covering everything outside the bed.
//! The visual contrast makes the printable area obvious at a glance.
//!
//! The size drawn is **not** a constant: it is the open document's
//! `bed_mm`, passed in by the viewport through [`Bed::from_size_mm`]
//! (LCV-114). The constants below seed a blank document only.
//!
//! Introduced by demand LCV-034; parameterised by LCV-114.

use crate::geometry::Vec2;
use crate::render::Camera;
use crate::util::{DEFAULT_BED_HEIGHT_MM, DEFAULT_BED_WIDTH_MM};

/// Laser bed configuration: size and world-space origin.
///
/// - `size_mm[0]` = width along world X; `size_mm[1]` = height along world Y.
/// - `origin_world` is the world-space coordinate of the **lower-left** corner
///   of the bed (so the bed spans `[origin.x, origin.x + size_mm[0]]` ×
///   `[origin.y, origin.y + size_mm[1]]`).
///
/// Default: the blank-document seed ([`DEFAULT_BED_WIDTH_MM`] ×
/// [`DEFAULT_BED_HEIGHT_MM`], 400×400 mm) at the world origin, matching common
/// GRBL hobbyist machine sizes (Ortur LM2/LM3, Atomstack A5/A10). A live
/// viewport builds its `Bed` from `Document::bed_mm` via [`Bed::from_size_mm`],
/// which is the very height the SVG exporter mirrors around — so the bed drawn
/// here and the canvas written to file stay one number (LCV-100, LCV-114).
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
            size_mm: [DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM],
            origin_world: Vec2::new(0.0, 0.0),
        }
    }
}

impl Bed {
    /// Build a bed of `size_mm` millimetres anchored at the world origin.
    ///
    /// This is how the viewport turns the open document's
    /// [`bed_mm`](crate::document::Document::bed_mm) into something drawable
    /// (LCV-114 AC 3): the renderer holds no bed state of its own, so changing
    /// the document's bed is visible on the very next frame.
    ///
    /// The size is taken as given — the caller is responsible for having
    /// clamped it (see [`crate::util::clamp_bed_mm`]).
    pub fn from_size_mm(size_mm: [f64; 2]) -> Self {
        Self {
            size_mm,
            origin_world: Vec2::new(0.0, 0.0),
        }
    }

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

/// The bed's screen-space bounding rectangle: the four world corners
/// projected through `camera` (viewport-local) and shifted by `rect.min` to
/// land in the painter's global clip rect — the same `+ offset` every other
/// projected point in this render layer applies exactly once.
fn bed_screen_rect(rect: egui::Rect, camera: &Camera, bed: &Bed) -> egui::Rect {
    let corners = bed.corners();
    let offset = rect.min.to_vec2();
    let screen_bl = camera.world_to_screen(corners[0]) + offset;
    let screen_tr = camera.world_to_screen(corners[2]) + offset;
    egui::Rect::from_two_pos(screen_bl, screen_tr)
}

/// Fill the bed rectangle only — no border, no exterior overlay.
///
/// **Call order** (LCV-137 AC 1/AC 2): called BEFORE
/// [`crate::render::draw_grid`], so the grid's lines paint *on top of* this
/// fill and are visible inside the bed rather than painted over by it. Pair
/// with [`draw_bed`], which paints the border and exterior overlay — those
/// stay AFTER the grid so they still sit under drawn geometry, framing the
/// grid rather than erasing it.
pub fn draw_bed_fill(painter: &egui::Painter, rect: egui::Rect, camera: &Camera, bed: &Bed) {
    let bed_screen_rect = bed_screen_rect(rect, camera, bed);
    // Fill the bed rectangle with a lighter gray (gray(40)) than the canvas
    // background (gray(24)) to make the printable area stand out.
    painter.rect_filled(bed_screen_rect, 0.0, egui::Color32::from_gray(40));
}

/// Draw the laser bed's border and outer overlay — no fill.
///
/// **Call order** (LCV-137 AC 1/AC 2): called AFTER [`crate::render::draw_grid`]
/// (whose lines must be visible over [`draw_bed_fill`]'s background, painted
/// before it) and BEFORE any entity rendering, so the border and overlay sit
/// over the grid but under drawn geometry. The background fill itself is
/// [`draw_bed_fill`], called first, before the grid.
///
/// Drawing steps:
/// 1. Convert the four bed corners to screen space.
/// 2. Stroke the bed border with a light gray line.
/// 3. Draw four overlay strips (top/bottom/left/right) around the bed, clipped
///    to `rect`, covering the area outside the bed with a translucent black
///    overlay to discourage out-of-bounds placement.
pub fn draw_bed(painter: &egui::Painter, rect: egui::Rect, camera: &Camera, bed: &Bed) {
    let bed_screen_rect = bed_screen_rect(rect, camera, bed);

    // Stroke the bed border with a light gray line.
    painter.rect_stroke(
        bed_screen_rect,
        0.0,
        egui::Stroke::new(1.5_f32, egui::Color32::from_gray(160)),
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

/// Paint the origin marker at world (0,0): one open path from
/// [`ORIGIN_ARM_PT`](crate::render::palette::ORIGIN_ARM_PT) along +X, through
/// the origin, to the same length along +Y (up the screen), in the `origin`
/// token (LCV-164 AC 4). Sized in points, so it reads the same at every zoom.
///
/// **Call order**: right after [`draw_bed`], so it sits over the bed border
/// it lies on and under the entities.
pub fn draw_origin(painter: &egui::Painter, rect: egui::Rect, camera: &Camera) {
    use crate::render::palette::{ORIGIN, ORIGIN_ARM_PT, ORIGIN_WIDTH_PT};
    let o = camera.world_to_screen(Vec2::new(0.0, 0.0)) + rect.min.to_vec2();
    let points = vec![
        o + egui::Vec2::new(ORIGIN_ARM_PT, 0.0),
        o,
        o + egui::Vec2::new(0.0, -ORIGIN_ARM_PT),
    ];
    let stroke = egui::Stroke::new(ORIGIN_WIDTH_PT, ORIGIN);
    painter.add(egui::Shape::line(points, stroke));
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

    /// LCV-100 AC 3 — the default bed and the blank-document seed share one
    /// source of truth.
    #[test]
    fn bed_default_uses_shared_constants() {
        assert_eq!(
            Bed::default().size_mm,
            [DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM]
        );
    }

    /// LCV-114 AC 3 — an arbitrary document bed becomes a drawable bed at the
    /// world origin, with no clamping or reinterpretation on the way through.
    #[test]
    fn bed_from_size_mm_takes_the_document_bed_verbatim() {
        let bed = Bed::from_size_mm([300.0, 180.0]);
        assert_eq!(bed.size_mm, [300.0, 180.0]);
        assert_eq!(bed.origin_world, Vec2::new(0.0, 0.0));
        assert_eq!(bed.corners()[2], Vec2::new(300.0, 180.0));
        assert_eq!(
            Bed::from_size_mm([DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM]),
            Bed::default()
        );
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
