//! The canvas's one curve sampler (LCV-164 AC 5, AC 6).
//!
//! A circle or arc is cut into as many chords as its **on-screen** radius
//! needs for a sagitta of at most [`MAX_SAGITTA_PT`]:
//! `n = ⌈2π / (2·acos(1 − 0.25 / r_pt))⌉` per full turn, clamped to
//! [[`MIN_CHORDS`], [`MAX_CHORDS`]] ([`MIN_CHORDS`] when `r_pt ≤ 0.25`). An
//! arc gets `⌈n · sweep / 2π⌉` chords, at least 2. Big circles stay round
//! and tiny ones stay cheap.
//!
//! [`stroke_entity`] paints an entity as **one** shape — a line segment, a
//! closed path for a circle, an open path for an arc — so a translucent
//! overlay (selection halo, hover, preview) has no darker joints.
//!
//! MUST NOT import `eframe` or `rfd`.

use core::f64::consts::TAU;

use crate::document::Entity;
use crate::geometry::Vec2;
use crate::render::{Camera, arc_polyline, bezier_polyline, ellipse_polyline};

/// Largest on-screen sagitta of one chord, in points.
pub const MAX_SAGITTA_PT: f64 = 0.25;
/// Fewest chords per full turn.
pub const MIN_CHORDS: usize = 8;
/// Most chords per full turn.
pub const MAX_CHORDS: usize = 1024;

/// Chords per full turn for a curve whose radius is `r_pt` points on screen.
pub fn chords_per_turn(r_pt: f64) -> usize {
    // NaN goes to the floor too.
    if r_pt.is_nan() || r_pt <= MAX_SAGITTA_PT {
        return MIN_CHORDS;
    }
    let step = 2.0 * (1.0 - MAX_SAGITTA_PT / r_pt).acos();
    let n = (TAU / step).ceil().min(MAX_CHORDS as f64);
    // In [1, MAX_CHORDS] after the `min`, so the cast is exact.
    (n as usize).clamp(MIN_CHORDS, MAX_CHORDS)
}

/// `entity` sampled in world space at `camera`'s zoom: a line's two ends; a
/// circle's `n + 1` points, the last equal to the first; an arc's chord ends
/// from start to end; an ellipse or Bézier sampled to half a pixel (closed
/// for a full ellipse).
fn world_points(entity: &Entity, camera: &Camera) -> Vec<Vec2> {
    match entity {
        Entity::Line(l) => vec![l.p1, l.p2],
        Entity::Circle(c) => {
            let n = chords_per_turn(c.r / camera.mm_per_px);
            (0..=n)
                .map(|i| c.point_at_angle(i as f64 / n as f64 * TAU))
                .collect()
        }
        Entity::Arc(a) => {
            let n = chords_per_turn(a.r / camera.mm_per_px);
            let chords = (n as f64 * a.sweep_angle() / TAU).ceil();
            // `arc_polyline` floors the count at 2; the sweep is ≤ 2π, so
            // `chords ≤ n ≤ MAX_CHORDS` and the cast is exact.
            arc_polyline(a, chords.max(0.0) as usize)
        }
        // Half-pixel chord deviation (LCV-176 AC 4, LCV-177 AC 5).
        Entity::Ellipse(e) => ellipse_polyline(e, camera.mm_per_px),
        Entity::Bezier(b) => bezier_polyline(b, camera.mm_per_px),
    }
}

/// `entity`'s polyline in global screen points (`rect.min` added once): the
/// samples [`stroke_entity`] strokes, the closing point of a circle included.
pub fn screen_points(rect: egui::Rect, camera: &Camera, entity: &Entity) -> Vec<egui::Pos2> {
    let offset = rect.min.to_vec2();
    world_points(entity, camera)
        .into_iter()
        .map(|w| camera.world_to_screen(w) + offset)
        .collect()
}

/// Stroke `entity` as one shape: a line segment, a closed path (circle, full
/// ellipse) or an open path (arc, elliptical arc, Bézier).
pub fn stroke_entity(
    painter: &egui::Painter,
    rect: egui::Rect,
    camera: &Camera,
    entity: &Entity,
    stroke: egui::Stroke,
) {
    let mut points = screen_points(rect, camera, entity);
    // A line always samples to exactly its two ends (`world_points`).
    let shape = match entity {
        Entity::Line(_) => egui::Shape::line_segment([points[0], points[1]], stroke),
        Entity::Circle(_) => {
            points.pop();
            egui::Shape::closed_line(points, stroke)
        }
        Entity::Ellipse(e) if e.span.is_none() => {
            points.pop();
            egui::Shape::closed_line(points, stroke)
        }
        Entity::Arc(_) | Entity::Ellipse(_) | Entity::Bezier(_) => {
            egui::Shape::line(points, stroke)
        }
    };
    painter.add(shape);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Arc, Circle, Line};

    /// Sagitta of one chord of an `n`-chord turn at radius `r_pt`.
    fn sagitta(r_pt: f64, n: usize) -> f64 {
        r_pt * (1.0 - (core::f64::consts::PI / n as f64).cos())
    }

    #[test]
    fn chords_per_turn_keeps_the_sagitta_under_a_quarter_point() {
        for r_pt in [0.3, 1.0, 10.0, 100.0, 1000.0, 10_000.0, 50_000.0] {
            let n = chords_per_turn(r_pt);
            assert!((MIN_CHORDS..=MAX_CHORDS).contains(&n), "r={r_pt}: {n}");
            assert!(sagitta(r_pt, n) <= MAX_SAGITTA_PT + 1e-12, "r={r_pt}: {n}");
            if n > MIN_CHORDS {
                assert!(
                    sagitta(r_pt, n - 1) > MAX_SAGITTA_PT,
                    "r={r_pt}: {n} not minimal"
                );
            }
        }
    }

    #[test]
    fn chords_per_turn_clamps_both_ends() {
        for r_pt in [0.0, 0.1, 0.25, -1.0, f64::NAN] {
            assert_eq!(chords_per_turn(r_pt), MIN_CHORDS, "r={r_pt}");
        }
        for r_pt in [1e6, 1e12, f64::INFINITY] {
            assert_eq!(chords_per_turn(r_pt), MAX_CHORDS, "r={r_pt}");
        }
    }

    fn cam(mm_per_px: f64) -> Camera {
        Camera {
            mm_per_px,
            viewport_size_px: [800.0, 600.0],
            ..Camera::default()
        }
    }

    #[test]
    fn screen_points_close_a_circle_and_span_an_arc() {
        let rect = egui::Rect::from_min_size(egui::Pos2::new(10.0, 20.0), egui::Vec2::splat(100.0));
        let circle = Entity::Circle(Circle::new(Vec2::new(0.0, 0.0), 100.0));
        let pts = screen_points(rect, &cam(1.0), &circle);
        assert_eq!(pts.len(), chords_per_turn(100.0) + 1);
        assert!((pts[0] - pts[pts.len() - 1]).length() < 1e-3);

        let arc = Arc::new(Vec2::new(0.0, 0.0), 100.0, 0.0, core::f64::consts::PI, true);
        let pts = screen_points(rect, &cam(1.0), &Entity::Arc(arc));
        assert_eq!(pts.len(), chords_per_turn(100.0).div_ceil(2) + 1);
        let start = cam(1.0).world_to_screen(arc.start_point()) + rect.min.to_vec2();
        assert!((pts[0] - start).length() < 1e-3);

        let line = Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(5.0, 5.0)));
        assert_eq!(screen_points(rect, &cam(1.0), &line).len(), 2);
    }

    #[test]
    fn a_tiny_arc_keeps_two_chords() {
        let arc = Arc::new(Vec2::new(0.0, 0.0), 0.1, 0.0, 0.01, true);
        let pts = screen_points(egui::Rect::ZERO, &cam(1.0), &Entity::Arc(arc));
        assert_eq!(pts.len(), 3);
    }
}
