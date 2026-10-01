//! The canvas cursor (LCV-162): an R14 crosshair across the whole canvas and,
//! while a tool waits for an entity pick, a hollow pickbox around it.
//!
//! Painted last on the canvas, after the snap glyph, from the resolved
//! cursor point (after snap and Ortho). The OS cursor is hidden over the
//! canvas by `src/app/viewport.rs::draw`.
//!
//! MUST NOT import `eframe` or `rfd`.

use super::Camera;
use crate::geometry::Vec2;

/// Stroke width of the crosshair and the pickbox, in points.
const CURSOR_STROKE_PT: f32 = 1.0;

/// The `cursor` colour token (DESIGN.md §3): light gray, ≥3:1 on the bed.
pub fn cursor_color() -> egui::Color32 {
    egui::Color32::from_gray(220)
}

/// The two crosshair segments through `at`, horizontal first, each spanning
/// `rect` edge to edge.
fn crosshair_segments(rect: egui::Rect, at: egui::Pos2) -> [[egui::Pos2; 2]; 2] {
    [
        [egui::pos2(rect.min.x, at.y), egui::pos2(rect.max.x, at.y)],
        [egui::pos2(at.x, rect.min.y), egui::pos2(at.x, rect.max.y)],
    ]
}

/// The pickbox square centred on `at`, of side `2 × aperture_pt`.
fn pickbox_rect(at: egui::Pos2, aperture_pt: f32) -> egui::Rect {
    egui::Rect::from_center_size(at, egui::Vec2::splat(2.0 * aperture_pt))
}

/// Screen position of world point `cursor` inside the canvas `rect`.
fn screen_of(rect: egui::Rect, camera: &Camera, cursor: Vec2) -> egui::Pos2 {
    camera.world_to_screen(cursor) + rect.min.to_vec2()
}

/// Paint the crosshair: one horizontal and one vertical 1 pt line across
/// `rect` through the world point `cursor` (LCV-162 AC 2).
pub fn draw_crosshair(painter: &egui::Painter, rect: egui::Rect, camera: &Camera, cursor: Vec2) {
    let stroke = egui::Stroke::new(CURSOR_STROKE_PT, cursor_color());
    for segment in crosshair_segments(rect, screen_of(rect, camera, cursor)) {
        painter.line_segment(segment, stroke);
    }
}

/// Paint the hollow pickbox of side `2 × aperture_pt` centred on the world
/// point `cursor` (LCV-162 AC 5).
pub fn draw_pickbox(
    painter: &egui::Painter,
    rect: egui::Rect,
    camera: &Camera,
    cursor: Vec2,
    aperture_pt: f32,
) {
    let square = pickbox_rect(screen_of(rect, camera, cursor), aperture_pt);
    let stroke = egui::Stroke::new(CURSOR_STROKE_PT, cursor_color());
    painter.rect_stroke(square, 0.0, stroke, egui::StrokeKind::Middle);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG 2 relative luminance of an opaque colour.
    fn luminance(c: egui::Color32) -> f64 {
        let ch = |v: u8| {
            let v = f64::from(v) / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
    }

    /// LCV-162 AC 2 — the cursor colour is opaque and has ≥3:1 WCAG contrast
    /// on the bed fill (gray 40, DESIGN.md §3 `bed.fill`).
    #[test]
    fn cursor_color_contrasts_with_the_bed_fill() {
        let (c, bed) = (cursor_color(), egui::Color32::from_gray(40));
        assert_eq!(c.a(), 255);
        let (a, b) = (luminance(c), luminance(bed));
        let ratio = (a.max(b) + 0.05) / (a.min(b) + 0.05);
        assert!(ratio >= 3.0, "contrast {ratio:.2}:1");
    }

    /// LCV-162 AC 2 — both segments span the rect and cross at the point.
    #[test]
    fn crosshair_segments_span_the_rect_through_the_point() {
        let rect = egui::Rect::from_min_max(egui::pos2(10.0, 20.0), egui::pos2(110.0, 220.0));
        let [h, v] = crosshair_segments(rect, egui::pos2(40.0, 70.0));
        assert_eq!(h, [egui::pos2(10.0, 70.0), egui::pos2(110.0, 70.0)]);
        assert_eq!(v, [egui::pos2(40.0, 20.0), egui::pos2(40.0, 220.0)]);
    }

    /// LCV-162 AC 5 — the pickbox is a square of side twice the aperture.
    #[test]
    fn pickbox_is_twice_the_aperture_around_the_point() {
        let r = pickbox_rect(egui::pos2(40.0, 70.0), 5.0);
        assert_eq!(r.center(), egui::pos2(40.0, 70.0));
        assert_eq!(r.size(), egui::vec2(10.0, 10.0));
    }
}
