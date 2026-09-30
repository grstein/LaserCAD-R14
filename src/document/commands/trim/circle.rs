//! Circle-target trim helper.
//!
//! Owns the case where the target is a [`Circle`]: given two cut points from
//! any cutter, the circle is replaced by an [`Arc`] spanning the side that
//! contains `keep_side_point`. A tangent (one intersection) is a no-op
//! because a circle cannot be split at a tangent — matches AutoCAD's
//! behavior.
//!
//! See the parent [`super`] module for the public command wrapping this
//! routine.

use crate::document::Entity;
use crate::geometry::{Arc, Circle, EPSILON, Line, Vec2, line_circle};

/// Trim a [`Circle`] target by a [`Line`] cutter (see
/// [`trim_circle_at_points`]).
pub(crate) fn trim_circle_by_line(target: &Circle, cutter: &Line, keep: Vec2) -> Option<Entity> {
    trim_circle_at_points(target, &line_circle(cutter, target), keep)
}

/// Trim a [`Circle`] target at its cut points (LCV-160 AC 3). Requires
/// exactly two points; returns the CCW [`Entity::Arc`] between them that
/// holds `keep`. `None` for zero, one (tangent) or more points.
pub(crate) fn trim_circle_at_points(target: &Circle, pts: &[Vec2], keep: Vec2) -> Option<Entity> {
    if pts.len() != 2 || pts[0].approx_eq(pts[1], EPSILON) {
        return None;
    }
    // Order intersections by angle ascending so the "first" arc goes CCW
    // from a -> b. Both angles sit in `(-π, π]` (atan2 range).
    let mut angled: Vec<(f64, Vec2)> = pts
        .iter()
        .map(|&p| {
            let v = p - target.center;
            (v.y.atan2(v.x), p)
        })
        .collect();
    angled.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(core::cmp::Ordering::Equal));
    let (start_angle, _) = angled[0];
    let (end_angle, _) = angled[1];
    // `keep` is classified by its angle relative to `target.center`. A keep
    // at the center is ambiguous — refuse to pick.
    let kv = keep - target.center;
    if kv.length_squared() <= EPSILON * EPSILON {
        return None;
    }
    let keep_angle = kv.y.atan2(kv.x);
    if angle_in_ccw_range(keep_angle, start_angle, end_angle) {
        Some(Entity::Arc(Arc::new(
            target.center,
            target.r,
            start_angle,
            end_angle,
            true,
        )))
    } else {
        Some(Entity::Arc(Arc::new(
            target.center,
            target.r,
            end_angle,
            start_angle,
            true,
        )))
    }
}

/// True iff `angle` lies in the CCW sweep from `start` to `end`. All inputs
/// in `(-π, π]` (atan2 range). Inclusive at both endpoints within `EPSILON`.
fn angle_in_ccw_range(angle: f64, start: f64, end: f64) -> bool {
    let two_pi = core::f64::consts::TAU;
    let sweep = (end - start).rem_euclid(two_pi);
    let delta = (angle - start).rem_euclid(two_pi);
    delta <= sweep + EPSILON
}

#[cfg(test)]
mod tests {
    use super::super::TrimEntity;
    use crate::document::commands::Command;
    use crate::document::{Document, Entity};
    use crate::geometry::{Arc, Circle, EPSILON, Line, Vec2};
    use core::f64::consts::PI;

    fn doc_with(entities: Vec<Entity>) -> Document {
        let mut doc = Document::default();
        entities.into_iter().for_each(|e| doc.push_current(e));
        doc
    }

    fn as_arc(e: &Entity) -> Arc {
        match e {
            Entity::Arc(a) => *a,
            other => panic!("expected Arc, got {other:?}"),
        }
    }

    /// AC#6 — Trim Circle x Line yields the upper semicircle arc.
    #[test]
    fn trim_circle_line_yields_arc() {
        let circle = Circle::new(Vec2::default(), 1.0);
        let cutter = Line::new(Vec2::new(-2.0, 0.0), Vec2::new(2.0, 0.0));
        let mut doc = doc_with(vec![Entity::Circle(circle), Entity::Line(cutter)]);
        let mut cmd = TrimEntity::new(0, 1, Vec2::new(0.0, 1.0));
        cmd.do_(&mut doc);
        let arc = as_arc(&doc.entities[0]);
        assert!(arc.center.approx_eq(Vec2::default(), EPSILON));
        assert!((arc.r - 1.0).abs() <= EPSILON);
        assert!((arc.start_angle - 0.0).abs() <= EPSILON);
        assert!((arc.end_angle - PI).abs() <= EPSILON);
        assert!(arc.ccw);
        cmd.undo(&mut doc);
        assert_eq!(doc.entities[0], Entity::Circle(circle));
    }

    /// Trim Circle x Line keeping the lower side returns the lower
    /// semicircle (start = π, end = 2π via wrap, ccw = true).
    #[test]
    fn trim_circle_line_keep_lower_half() {
        let circle = Circle::new(Vec2::default(), 1.0);
        let cutter = Line::new(Vec2::new(-2.0, 0.0), Vec2::new(2.0, 0.0));
        let mut doc = doc_with(vec![Entity::Circle(circle), Entity::Line(cutter)]);
        let mut cmd = TrimEntity::new(0, 1, Vec2::new(0.0, -1.0));
        cmd.do_(&mut doc);
        let arc = as_arc(&doc.entities[0]);
        // start_angle should be PI, end_angle 0 (other half, swept CCW).
        assert!((arc.start_angle - PI).abs() <= EPSILON);
        assert!(arc.end_angle.abs() <= EPSILON);
        assert!(arc.ccw);
    }

    /// LCV-160 AC 3 — two cut points give the arc holding the click; one
    /// point (tangent) or none is a no-op.
    #[test]
    fn trim_circle_at_points_needs_two_points() {
        use super::trim_circle_at_points;
        let circle = Circle::new(Vec2::default(), 10.0);
        let (a, b) = (
            Vec2::new(5.0, 75f64.sqrt()),
            Vec2::new(5.0, -(75f64.sqrt())),
        );
        let got = trim_circle_at_points(&circle, &[a, b], Vec2::new(-10.0, 0.0));
        let arc = as_arc(&got.expect("two points"));
        assert!(arc.contains_angle(PI));
        assert!(arc.start_point().approx_eq(a, 1e-9) && arc.end_point().approx_eq(b, 1e-9));
        let keep = Vec2::new(-10.0, 0.0);
        assert!(trim_circle_at_points(&circle, &[a], keep).is_none());
        assert!(trim_circle_at_points(&circle, &[], keep).is_none());
    }
}
