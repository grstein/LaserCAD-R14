//! Hit-testing helpers for [`super::SelectTool`] (LCV-042).
//!
//! Pure functions — no tool state, no egui, no rfd. Each function takes only
//! geometry and document values and returns a simple result.
//!
//! - **Pick helpers** resolve a point click to a `Vec<usize>` ready for
//!   [`crate::document::SelectionCommand`].
//! - **Box helpers** test each entity against a [`Rect`] for window (fully
//!   inside) or crossing (any overlap) selection modes.
//! - [`box_preview`] builds the four-edge rectangle as `Vec<Entity>` for the
//!   render pipeline.

use crate::document::Entity;
use crate::geometry::{Arc, Line, Rect, Vec2};

// ---------------------------------------------------------------------------
// Pick helpers
// ---------------------------------------------------------------------------

/// Resolve a point-pick at `pos`, respecting `shift` toggle semantics.
///
/// `current_sel` is the set of currently-selected indices; `radius_mm` is the
/// pick aperture at the live zoom. Returns the new selection index set ready
/// to pass to [`crate::document::SelectionCommand::new`].
pub(super) fn pick_resolve(
    pos: Vec2,
    shift: bool,
    entities: &[Entity],
    current_sel: Vec<usize>,
    radius_mm: f64,
) -> Vec<usize> {
    let picked = pick_closest(pos, entities, radius_mm);

    match (picked, shift) {
        // Hit + shift: toggle the entity in the current selection.
        (Some(idx), true) => {
            if current_sel.contains(&idx) {
                current_sel.into_iter().filter(|&i| i != idx).collect()
            } else {
                let mut sel = current_sel;
                sel.push(idx);
                sel
            }
        }
        // Hit + no shift: replace with just this entity.
        (Some(idx), false) => vec![idx],
        // Miss + shift: keep current selection unchanged.
        (None, true) => current_sel,
        // Miss + no shift: clear selection.
        (None, false) => vec![],
    }
}

/// Find the index of the closest entity within `radius_mm` of `pos`.
///
/// Returns `None` when there are no entities or all are farther than the
/// radius.
pub(super) fn pick_closest(pos: Vec2, entities: &[Entity], radius_mm: f64) -> Option<usize> {
    entities
        .iter()
        .enumerate()
        .map(|(i, e)| (i, entity_distance_to_point(e, pos)))
        .filter(|(_, d)| *d <= radius_mm)
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
}

// ---------------------------------------------------------------------------
// Distance helpers
// ---------------------------------------------------------------------------

/// Euclidean distance from `p` to the nearest point on `entity`'s stroke.
pub(super) fn entity_distance_to_point(entity: &Entity, p: Vec2) -> f64 {
    match entity {
        Entity::Line(l) => l.distance_to_point(p),
        Entity::Circle(c) => c.distance_to_point(p).abs(),
        Entity::Arc(a) => arc_distance_to_point(a, p),
        Entity::Ellipse(_) => f64::INFINITY,
    }
}

/// Distance from `p` to the nearest point on the arc's stroke.
///
/// If the perpendicular from `p` to the parent circle falls within the arc's
/// angular sweep, the distance is the absolute radial gap. Otherwise the
/// distance is the minimum of the distances to the two arc endpoints.
pub(super) fn arc_distance_to_point(arc: &Arc, p: Vec2) -> f64 {
    let v = p - arc.center;
    let angle = v.y.atan2(v.x);
    if arc.contains_angle(angle) {
        (v.length() - arc.r).abs()
    } else {
        let d_start = (p - arc.start_point()).length();
        let d_end = (p - arc.end_point()).length();
        d_start.min(d_end)
    }
}

// ---------------------------------------------------------------------------
// Box selection helpers
// ---------------------------------------------------------------------------

/// `true` iff `entity`'s geometry is **fully** inside `rect` (window mode).
pub(super) fn entity_in_window(entity: &Entity, rect: &Rect) -> bool {
    match entity {
        Entity::Line(l) => rect.contains_line(l),
        Entity::Circle(c) => rect.contains_circle(c),
        Entity::Arc(a) => rect.contains_arc(a),
        Entity::Ellipse(_) => false,
    }
}

/// `true` iff `entity`'s geometry **overlaps** `rect` (crossing mode).
pub(super) fn entity_in_crossing(entity: &Entity, rect: &Rect) -> bool {
    match entity {
        Entity::Line(l) => rect.crosses_line(l),
        Entity::Circle(c) => rect.crosses_circle(c),
        Entity::Arc(a) => rect.crosses_arc(a),
        Entity::Ellipse(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Preview helper
// ---------------------------------------------------------------------------

/// Build the four line-segment edges of a selection box as preview entities.
pub(super) fn box_preview(p1: Vec2, p2: Vec2) -> Vec<Entity> {
    let bl = Vec2::new(p1.x.min(p2.x), p1.y.min(p2.y));
    let tr = Vec2::new(p1.x.max(p2.x), p1.y.max(p2.y));
    let br = Vec2::new(tr.x, bl.y);
    let tl = Vec2::new(bl.x, tr.y);
    vec![
        Entity::Line(Line::new(bl, br)),
        Entity::Line(Line::new(br, tr)),
        Entity::Line(Line::new(tr, tl)),
        Entity::Line(Line::new(tl, bl)),
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Circle, Line, Vec2};

    /// arc_distance_to_point: point directly on the arc → distance ≈ 0.
    #[test]
    fn arc_distance_on_arc_is_zero() {
        use crate::geometry::Arc;
        use core::f64::consts::FRAC_PI_2;
        let arc = Arc::new(Vec2::new(0.0, 0.0), 5.0, 0.0, FRAC_PI_2, true);
        let angle = std::f64::consts::PI / 4.0;
        let on_arc = Vec2::new(5.0 * angle.cos(), 5.0 * angle.sin());
        let d = arc_distance_to_point(&arc, on_arc);
        assert!(d < 1e-10, "distance on arc should be ~0, got {d}");
    }

    /// arc_distance: point at endpoint uses endpoint distance.
    #[test]
    fn arc_distance_outside_sweep_uses_endpoint() {
        use crate::geometry::Arc;
        use core::f64::consts::FRAC_PI_2;
        // Arc from 0 to π/2. A point at angle π is outside the sweep.
        let arc = Arc::new(Vec2::new(0.0, 0.0), 5.0, 0.0, FRAC_PI_2, true);
        let p = Vec2::new(-5.0, 0.0); // at angle π, outside sweep
        let d = arc_distance_to_point(&arc, p);
        // Start point is (5, 0), end point is (0, 5).
        // dist to start = sqrt(100) = 10, dist to end = sqrt(25+25) ≈ 7.07
        assert!(
            (d - (50.0f64).sqrt()).abs() < 1e-10,
            "expected dist to end point"
        );
    }

    /// pick_closest returns None when entity is farther than the radius.
    #[test]
    fn pick_closest_returns_none_when_outside_radius() {
        let entities = vec![Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
        ))];
        assert!(pick_closest(Vec2::new(0.0, 6.0), &entities, 5.0).is_none());
        assert!(pick_closest(Vec2::new(5.0, 0.3), &entities, 0.25).is_none());
    }

    /// pick_closest returns Some(0) when entity is within the radius.
    #[test]
    fn pick_closest_returns_index_when_within_radius() {
        let entities = vec![Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
        ))];
        assert_eq!(pick_closest(Vec2::new(5.0, 1.0), &entities, 5.0), Some(0));
        assert_eq!(
            pick_closest(Vec2::new(5.0, 80.0), &entities, 100.0),
            Some(0)
        );
    }

    /// entity_in_window: fully-inside line passes, crossing line fails.
    #[test]
    fn entity_in_window_line_inside_vs_crossing() {
        let rect = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let inside = Entity::Line(Line::new(Vec2::new(2.0, 2.0), Vec2::new(8.0, 8.0)));
        let crossing = Entity::Line(Line::new(Vec2::new(5.0, 5.0), Vec2::new(15.0, 5.0)));
        assert!(entity_in_window(&inside, &rect));
        assert!(!entity_in_window(&crossing, &rect));
    }

    /// entity_in_crossing: crossing line passes, outside-only line fails.
    #[test]
    fn entity_in_crossing_line_crossing_vs_outside() {
        let rect = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let crossing = Entity::Line(Line::new(Vec2::new(5.0, 5.0), Vec2::new(15.0, 5.0)));
        let outside = Entity::Line(Line::new(Vec2::new(20.0, 20.0), Vec2::new(30.0, 30.0)));
        assert!(entity_in_crossing(&crossing, &rect));
        assert!(!entity_in_crossing(&outside, &rect));
    }

    /// entity_in_window: circle inscribed inside rect passes.
    #[test]
    fn entity_in_window_circle_inscribed() {
        let rect = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let c = Entity::Circle(Circle::new(Vec2::new(5.0, 5.0), 3.0));
        assert!(entity_in_window(&c, &rect));
        let big = Entity::Circle(Circle::new(Vec2::new(5.0, 5.0), 6.0));
        assert!(!entity_in_window(&big, &rect));
    }

    /// box_preview returns 4 line segments.
    #[test]
    fn box_preview_returns_four_lines() {
        let preview = box_preview(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        assert_eq!(preview.len(), 4);
        for e in &preview {
            assert!(matches!(e, Entity::Line(_)));
        }
    }
}
