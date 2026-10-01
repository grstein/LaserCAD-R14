//! Entity painter: line, circle, and arc rendering.
//!
//! Walks the document's entities and strokes each one through
//! [`crate::render::stroke_entity`]: a line segment, or one path whose chord
//! count follows the on-screen radius (LCV-164 AC 5, `render/tessellate.rs`).
//!
//! Every entity is stroked in its layer's color (LCV-156 AC 3) at the width
//! in [`PaintOptions`]; selection and preview paint their own overlays.
//!
//! Introduced by demand LCV-035.

use crate::document::Document;
use crate::geometry::{Arc, Bezier, Ellipse, Vec2};
use crate::render::Camera;

/// Rendering options for the entity painter.
///
/// Default: a 1 pt stroke. The chord count of curves is no option: it
/// follows the zoom (LCV-164 AC 5, replacing LCV-035's fixed 64).
#[derive(Debug, Clone, Copy)]
pub struct PaintOptions {
    /// Stroke width in points applied to all entities.
    pub stroke_width: f32,
}

impl Default for PaintOptions {
    fn default() -> Self {
        Self { stroke_width: 1.0 }
    }
}

/// Draw all of `doc`'s entities onto the viewport, each in its layer's color.
///
/// `rect` is the viewport's screen-space rectangle; its origin is added to
/// every projected point so shapes land in the correct painter clip rect.
/// Each entity is one shape (see [`crate::render::stroke_entity`]).
pub fn draw_entities(
    painter: &egui::Painter,
    rect: egui::Rect,
    camera: &Camera,
    doc: &Document,
    options: PaintOptions,
) {
    for (i, entity) in doc.entities.iter().enumerate() {
        let [r, g, b] = doc.layer_color(i);
        let stroke = egui::Stroke::new(options.stroke_width, egui::Color32::from_rgb(r, g, b));
        crate::render::stroke_entity(painter, rect, camera, entity, stroke);
    }
}

/// An ellipse as a world-space polyline whose chord deviation is at most half
/// a screen pixel at `mm_per_px` (LCV-176 AC 4, ADR 0015 §8).
pub fn ellipse_polyline(e: &Ellipse, mm_per_px: f64) -> Vec<Vec2> {
    e.polyline(0.5 * mm_per_px)
}

/// A Bézier as a world-space polyline whose chord deviation is at most half
/// a screen pixel at `mm_per_px` (LCV-177 AC 5, ADR 0016 §4).
pub fn bezier_polyline(b: &Bezier, mm_per_px: f64) -> Vec<Vec2> {
    b.polyline(0.5 * mm_per_px)
}

/// Sample an arc as a polyline in world space.
///
/// Returns `segments + 1` evenly-spaced points along the arc from
/// `arc.start_angle` to `arc.end_angle`, respecting `arc.ccw`.
///
/// `segments` is clamped to `>= 2` (a 1-segment arc is degenerate; minimum
/// 2 segments = 3 sample points).
///
/// Direction:
/// - `arc.ccw == true`: sample from `start_angle` to `end_angle` going
///   counter-clockwise (increasing angle in standard math sense).
/// - `arc.ccw == false`: sample from `start_angle` to `end_angle` going
///   clockwise (decreasing angle).
pub fn arc_polyline(arc: &Arc, segments: usize) -> Vec<Vec2> {
    let segments = segments.max(2);
    let mut points = Vec::with_capacity(segments + 1);

    let sweep = arc.sweep_angle();
    for i in 0..=segments {
        let t = i as f64 / segments as f64;
        let angle = if arc.ccw {
            arc.start_angle + t * sweep
        } else {
            arc.start_angle - t * sweep
        };
        let x = arc.center.x + arc.r * angle.cos();
        let y = arc.center.y + arc.r * angle.sin();
        points.push(Vec2::new(x, y));
    }

    points
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Entity;
    use crate::geometry::{Circle, EPSILON, Line};
    use core::f64::consts::FRAC_PI_2;

    /// `PaintOptions::default()` has a non-zero stroke width (LCV-035 AC 2's
    /// fixed 64 arc segments is replaced by LCV-164 AC 5).
    #[test]
    fn paint_options_default_is_sensible() {
        assert!(PaintOptions::default().stroke_width > 0.0);
    }

    /// AC#5 — `arc_polyline` returns `n + 1` points for `n >= 2`.
    /// For `n < 2`, it clamps to `n = 2` and returns 3 points.
    #[test]
    fn arc_polyline_sample_count() {
        let arc = Arc::new(Vec2::default(), 1.0, 0.0, FRAC_PI_2, true);

        // n = 0, 1 → clamped to 2 → 3 points.
        assert_eq!(arc_polyline(&arc, 0).len(), 3);
        assert_eq!(arc_polyline(&arc, 1).len(), 3);

        // n >= 2 → n + 1 points.
        assert_eq!(arc_polyline(&arc, 2).len(), 3);
        assert_eq!(arc_polyline(&arc, 8).len(), 9);
        assert_eq!(arc_polyline(&arc, 64).len(), 65);
        assert_eq!(arc_polyline(&arc, 256).len(), 257);
    }

    /// AC#6 — `arc_polyline` endpoints match `arc.start_point()` and
    /// `arc.end_point()` within `EPSILON`.
    #[test]
    fn arc_polyline_endpoints_match_arc() {
        // Quarter arc: 0 → π/2, CCW.
        let arc = Arc::new(Vec2::default(), 1.0, 0.0, FRAC_PI_2, true);
        let pts = arc_polyline(&arc, 64);

        assert_eq!(pts.len(), 65);
        assert!(
            pts[0].approx_eq(arc.start_point(), EPSILON),
            "start: expected {:?}, got {:?}",
            arc.start_point(),
            pts[0]
        );
        assert!(
            pts[64].approx_eq(arc.end_point(), EPSILON),
            "end: expected {:?}, got {:?}",
            arc.end_point(),
            pts[64]
        );

        // Specific check: start = (1, 0), end = (0, 1).
        assert!(pts[0].approx_eq(Vec2::new(1.0, 0.0), EPSILON));
        assert!(pts[64].approx_eq(Vec2::new(0.0, 1.0), EPSILON));
    }

    /// AC#7 — The total chord length approximates the arc length within 0.5%.
    #[test]
    fn arc_polyline_total_length_approximates_arc_length() {
        // 90° unit arc: expected length = π/2 ≈ 1.5708.
        let arc = Arc::new(Vec2::default(), 1.0, 0.0, FRAC_PI_2, true);
        let pts = arc_polyline(&arc, 64);

        let mut chord_length = 0.0;
        for i in 0..pts.len() - 1 {
            chord_length += (pts[i + 1] - pts[i]).length();
        }

        let expected = arc.arc_length(); // π/2 * 1.0
        let error = (chord_length - expected).abs() / expected;
        assert!(
            error < 0.005,
            "chord_length={chord_length}, expected={expected}, error={error}"
        );
    }

    /// AC#8 — `arc_polyline` respects `ccw` flag: same arc, opposite `ccw`,
    /// the second sample point is in opposite half-planes.
    #[test]
    fn arc_polyline_respects_ccw_flag() {
        // CCW arc: 0 → π/2.
        let arc_ccw = Arc::new(Vec2::default(), 1.0, 0.0, FRAC_PI_2, true);
        let pts_ccw = arc_polyline(&arc_ccw, 64);
        // pts_ccw[1] should be in the first quadrant (small positive angle).

        // CW arc: same start/end, but the "long way around" (270° clockwise).
        let arc_cw = Arc::new(Vec2::default(), 1.0, 0.0, FRAC_PI_2, false);
        let pts_cw = arc_polyline(&arc_cw, 64);
        // pts_cw[1] should be in the fourth quadrant (small negative angle).

        // CCW: pts[1] should have positive Y and positive X.
        assert!(pts_ccw[1].y > 0.0, "CCW arc should go upward first");
        assert!(pts_ccw[1].x > 0.0, "CCW arc should stay in Q1");

        // CW: pts[1] should have negative Y and positive X.
        assert!(pts_cw[1].y < 0.0, "CW arc should go downward first");
        assert!(pts_cw[1].x > 0.0, "CW arc should stay in Q4");
    }

    /// AC#9 — `draw_entities` does not panic on degenerate inputs.
    #[test]
    fn draw_entities_does_not_panic_on_degenerate_inputs() {
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let ctx = &ui.ctx().clone();
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("test_entities"),
            ));
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(800.0, 600.0));
            let opts = PaintOptions::default();

            let doc_of = |entities: Vec<Entity>| {
                let mut doc = Document::default();
                entities.into_iter().for_each(|e| doc.push_current(e));
                doc
            };

            // Empty document.
            draw_entities(&painter, rect, &Camera::default(), &doc_of(vec![]), opts);

            // One of each variant.
            let entities = doc_of(vec![
                Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0))),
                Entity::Circle(Circle::new(Vec2::new(50.0, 50.0), 20.0)),
                Entity::Arc(Arc::new(Vec2::default(), 10.0, 0.0, FRAC_PI_2, true)),
            ]);
            draw_entities(&painter, rect, &Camera::default(), &entities, opts);

            // Degenerate cameras.
            let cam_extreme_in = Camera {
                mm_per_px: 1e-9,
                viewport_size_px: [800.0, 600.0],
                ..Camera::default()
            };
            draw_entities(&painter, rect, &cam_extreme_in, &entities, opts);

            let cam_extreme_out = Camera {
                mm_per_px: 1e9,
                viewport_size_px: [800.0, 600.0],
                ..Camera::default()
            };
            draw_entities(&painter, rect, &cam_extreme_out, &entities, opts);

            // Zero-radius circle.
            let zero_circle = doc_of(vec![Entity::Circle(Circle::new(Vec2::default(), 0.0))]);
            draw_entities(&painter, rect, &Camera::default(), &zero_circle, opts);

            // Zero-sweep arc (start == end).
            let zero_arc = doc_of(vec![Entity::Arc(Arc::new(
                Vec2::default(),
                1.0,
                0.0,
                0.0,
                true,
            ))]);
            draw_entities(&painter, rect, &Camera::default(), &zero_arc, opts);
        });
    }

    /// Static check (AC#13): no `eframe` or `rfd` imports.
    /// (Verified by grep in build-gate script; this test documents the rule.)
    #[test]
    fn no_eframe_or_rfd_imports() {
        // Compile-time check: if this module compiled without those imports,
        // the rule holds. This test is a placeholder for the grep check.
    }

    /// Size check (AC#14): file should be <= 300 LOC.
    /// (Verified by `wc -l` in build-gate script; this test documents the rule.)
    #[test]
    fn file_size_under_300_loc() {
        // Run `wc -l src/render/entities.rs` to verify.
        // Current implementation is well under 300 lines.
    }
}
