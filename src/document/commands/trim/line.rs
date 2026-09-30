//! Line-target trim and line-target extend helpers: [`trim_line_at_points`]
//! keeps the piece around the click given any cutter's cut points;
//! [`extend_line_to_line`], [`extend_line_to_circle`] and
//! [`extend_line_to_arc`] (LCV-160) grow an endpoint to a boundary. See the
//! parent [`super`] module for the public commands wrapping these routines.

use super::parametric_t;
use crate::document::Entity;
use crate::geometry::{
    Arc, Circle, EPSILON, Line, Vec2, line_arc, line_circle, line_line_infinite,
};

/// Trim a [`Line`] target at its cut points `pts` (LCV-160): keep the
/// sub-segment between the two cut points (or target endpoints) around
/// `keep`, compared by parametric `t` on the target. Cut points at the
/// target's own endpoints are ignored; `None` when none is left.
pub(crate) fn trim_line_at_points(target: Line, pts: &[Vec2], keep: Vec2) -> Option<Entity> {
    let mut ts: Vec<(f64, Vec2)> = pts
        .iter()
        .map(|&p| (parametric_t(&target, p), p))
        .filter(|(t, _)| *t > EPSILON && *t < 1.0 - EPSILON)
        .collect();
    if ts.is_empty() {
        return None;
    }
    ts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let t_keep = parametric_t(&target, keep);
    let (mut lo, mut hi) = (target.p1, target.p2);
    for (t, p) in ts {
        if t_keep <= t + EPSILON {
            hi = p;
            break;
        }
        lo = p;
    }
    Some(Entity::Line(Line::new(lo, hi)))
}

/// Extend a target line to its infinite-line intersection with a boundary
/// [`Line`]. `endpoint = 0` extends `p1`, `endpoint = 1` extends `p2`.
/// No-op if the boundary is parallel or the intersection is behind the
/// chosen endpoint.
pub(crate) fn extend_line_to_line(target: Line, boundary: &Line, endpoint: u8) -> Option<Line> {
    let x = line_line_infinite(&target, boundary)?;
    apply_extend(target, x, endpoint)
}

/// Extend a target line to the nearest valid intersection with a boundary
/// [`Circle`]. Uses an infinite-line interpretation of the target: the
/// segment is extended along its own direction until it hits the circle.
/// Returns `None` if the line misses the circle or every intersection is
/// behind the chosen endpoint.
pub(crate) fn extend_line_to_circle(target: Line, boundary: &Circle, endpoint: u8) -> Option<Line> {
    let reach = reach_line(&target, boundary.center, boundary.r)?;
    extend_line_to_points(target, &line_circle(&reach, boundary), endpoint)
}

/// Extend a target line to the nearest cut point of its extension on a
/// boundary [`Arc`]'s span (LCV-160); crossings of the arc's parent circle
/// off the span do not count.
pub(crate) fn extend_line_to_arc(target: Line, boundary: &Arc, endpoint: u8) -> Option<Line> {
    let reach = reach_line(&target, boundary.center, boundary.r)?;
    extend_line_to_points(target, &line_arc(&reach, boundary), endpoint)
}

/// Move the chosen endpoint to the nearest point of `pts` that lies ahead
/// of it on the target's infinite line. `None` when every point is behind.
pub(crate) fn extend_line_to_points(target: Line, pts: &[Vec2], endpoint: u8) -> Option<Line> {
    let from = if endpoint == 0 { target.p1 } else { target.p2 };
    pts.iter()
        .filter_map(|&p| apply_extend(target, p, endpoint).map(|l| ((p - from).length(), l)))
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, l)| l)
}

/// A segment along the target's infinite line, long enough to cover both
/// endpoints and any crossing with the circle `(center, r)`: the slack
/// `+ len + 1.0` keeps the surrogate's `[0, 1]` window around `p1` and `p2`.
/// `None` for a degenerate target.
fn reach_line(target: &Line, center: Vec2, r: f64) -> Option<Line> {
    let d = target.p2 - target.p1;
    let len = d.length();
    if len <= EPSILON {
        return None;
    }
    let dir = d / len;
    let reach = (center - target.p1).length() + r + len + 1.0;
    Some(Line::new(target.p1 - dir * reach, target.p1 + dir * reach))
}

/// Move the chosen endpoint to `x`, but only if doing so lengthens the
/// segment. Returns `None` if `x` is on the wrong side of the endpoint (the
/// "behind the endpoint" no-op rule).
fn apply_extend(target: Line, x: Vec2, endpoint: u8) -> Option<Line> {
    let t = parametric_t(&target, x);
    match endpoint {
        0 => {
            if t > -EPSILON {
                return None;
            }
            Some(Line::new(x, target.p2))
        }
        1 => {
            if t < 1.0 + EPSILON {
                return None;
            }
            Some(Line::new(target.p1, x))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::{ExtendEntity, TrimEntity};
    use crate::document::commands::Command;
    use crate::document::{Document, Entity};
    use crate::geometry::{Circle, EPSILON, Line, Vec2};

    fn doc_with(entities: Vec<Entity>) -> Document {
        let mut doc = Document::default();
        entities.into_iter().for_each(|e| doc.push_current(e));
        doc
    }

    fn as_line(e: &Entity) -> Line {
        match e {
            Entity::Line(l) => *l,
            other => panic!("expected Line, got {other:?}"),
        }
    }

    fn line_approx_eq(a: Line, b: Line) -> bool {
        a.p1.approx_eq(b.p1, EPSILON) && a.p2.approx_eq(b.p2, EPSILON)
    }

    /// AC#2 — Trim Line x Line, keep the left side.
    #[test]
    fn trim_line_line_keep_left_side() {
        let target = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        let cutter = Line::new(Vec2::new(5.0, -1.0), Vec2::new(5.0, 1.0));
        let mut doc = doc_with(vec![Entity::Line(target), Entity::Line(cutter)]);
        let mut cmd = TrimEntity::new(0, 1, Vec2::new(3.0, 0.0));
        cmd.do_(&mut doc);
        assert!(line_approx_eq(
            as_line(&doc.entities[0]),
            Line::new(Vec2::new(0.0, 0.0), Vec2::new(5.0, 0.0))
        ));
        assert!(line_approx_eq(as_line(&doc.entities[1]), cutter));
        cmd.undo(&mut doc);
        assert!(line_approx_eq(as_line(&doc.entities[0]), target));
    }

    /// AC#3 — Trim Line x Line, keep the right side.
    #[test]
    fn trim_line_line_keep_right_side() {
        let target = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        let cutter = Line::new(Vec2::new(5.0, -1.0), Vec2::new(5.0, 1.0));
        let mut doc = doc_with(vec![Entity::Line(target), Entity::Line(cutter)]);
        let mut cmd = TrimEntity::new(0, 1, Vec2::new(7.0, 0.0));
        cmd.do_(&mut doc);
        assert!(line_approx_eq(
            as_line(&doc.entities[0]),
            Line::new(Vec2::new(5.0, 0.0), Vec2::new(10.0, 0.0))
        ));
        cmd.undo(&mut doc);
        assert!(line_approx_eq(as_line(&doc.entities[0]), target));
    }

    /// AC#4 — Trim Line x Circle, keep the middle (cutter carves out the
    /// middle).
    #[test]
    fn trim_line_circle_keep_middle() {
        let target = Line::new(Vec2::new(-5.0, 0.0), Vec2::new(5.0, 0.0));
        let cutter = Circle::new(Vec2::default(), 2.0);
        let mut doc = doc_with(vec![Entity::Line(target), Entity::Circle(cutter)]);
        let mut cmd = TrimEntity::new(0, 1, Vec2::new(0.0, 0.0));
        cmd.do_(&mut doc);
        assert!(line_approx_eq(
            as_line(&doc.entities[0]),
            Line::new(Vec2::new(-2.0, 0.0), Vec2::new(2.0, 0.0))
        ));
        cmd.undo(&mut doc);
        assert!(line_approx_eq(as_line(&doc.entities[0]), target));
    }

    /// AC#5 — Trim Line x Circle, keep an outer side.
    #[test]
    fn trim_line_circle_keep_outer_side() {
        let target = Line::new(Vec2::new(-5.0, 0.0), Vec2::new(5.0, 0.0));
        let cutter = Circle::new(Vec2::default(), 2.0);
        let mut doc = doc_with(vec![Entity::Line(target), Entity::Circle(cutter)]);
        let mut cmd = TrimEntity::new(0, 1, Vec2::new(-4.0, 0.0));
        cmd.do_(&mut doc);
        assert!(line_approx_eq(
            as_line(&doc.entities[0]),
            Line::new(Vec2::new(-5.0, 0.0), Vec2::new(-2.0, 0.0))
        ));
        cmd.undo(&mut doc);
        assert!(line_approx_eq(as_line(&doc.entities[0]), target));
    }

    /// AC#7 — no intersection ⇒ Trim is a no-op; undo is safe.
    #[test]
    fn trim_no_intersection_is_noop() {
        let target = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        let cutter = Line::new(Vec2::new(20.0, 20.0), Vec2::new(30.0, 30.0));
        let mut doc = doc_with(vec![Entity::Line(target), Entity::Line(cutter)]);
        let mut cmd = TrimEntity::new(0, 1, Vec2::new(5.0, 0.0));
        cmd.do_(&mut doc);
        assert!(line_approx_eq(as_line(&doc.entities[0]), target));
        cmd.undo(&mut doc);
        assert!(line_approx_eq(as_line(&doc.entities[0]), target));
    }

    /// AC#8 — Extend Line x Line moves the chosen endpoint to the boundary.
    #[test]
    fn extend_line_line_to_intersection() {
        let target = Line::new(Vec2::new(0.0, 0.0), Vec2::new(5.0, 0.0));
        let boundary = Line::new(Vec2::new(10.0, -1.0), Vec2::new(10.0, 1.0));
        let mut doc = doc_with(vec![Entity::Line(target), Entity::Line(boundary)]);
        let mut cmd = ExtendEntity::new(0, 1, 1);
        cmd.do_(&mut doc);
        assert!(line_approx_eq(
            as_line(&doc.entities[0]),
            Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0))
        ));
        cmd.undo(&mut doc);
        assert!(line_approx_eq(as_line(&doc.entities[0]), target));
    }

    /// AC#9 — Extend Line x Circle picks the nearest valid intersection.
    #[test]
    fn extend_line_circle_picks_nearest_intersection() {
        let target = Line::new(Vec2::new(-5.0, 0.0), Vec2::new(-3.0, 0.0));
        let boundary = Circle::new(Vec2::default(), 2.0);
        let mut doc = doc_with(vec![Entity::Line(target), Entity::Circle(boundary)]);
        let mut cmd = ExtendEntity::new(0, 1, 1);
        cmd.do_(&mut doc);
        let extended = as_line(&doc.entities[0]);
        assert!(extended.p1.approx_eq(Vec2::new(-5.0, 0.0), EPSILON));
        assert!(extended.p2.approx_eq(Vec2::new(-2.0, 0.0), EPSILON));
        cmd.undo(&mut doc);
        assert!(line_approx_eq(as_line(&doc.entities[0]), target));
    }

    /// AC#10 — extending toward an intersection behind the endpoint is a
    /// no-op; bonus: extending `p1` (endpoint = 0) symmetrically moves p1
    /// outward when the intersection is ahead of `p1` (negative `t`).
    #[test]
    fn extend_behind_endpoint_is_noop_and_p1_endpoint_works() {
        let target = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        let boundary = Line::new(Vec2::new(-5.0, -1.0), Vec2::new(-5.0, 1.0));
        let mut doc = doc_with(vec![Entity::Line(target), Entity::Line(boundary)]);
        let mut cmd = ExtendEntity::new(0, 1, 1);
        cmd.do_(&mut doc);
        assert!(line_approx_eq(as_line(&doc.entities[0]), target));

        // Bonus: same target + boundary but extend `p1` — the boundary at
        // `x = -5` is ahead of `p1 = (0,0)`, so p1 moves to `(-5, 0)`.
        let mut cmd = ExtendEntity::new(0, 1, 0);
        cmd.do_(&mut doc);
        assert!(line_approx_eq(
            as_line(&doc.entities[0]),
            Line::new(Vec2::new(-5.0, 0.0), Vec2::new(10.0, 0.0))
        ));
        cmd.undo(&mut doc);
        assert!(line_approx_eq(as_line(&doc.entities[0]), target));
    }

    /// LCV-160 AC 2 — the kept piece lies between the cut points around the
    /// click; with an arc cutter only its span points count.
    #[test]
    fn trim_line_at_points_keeps_piece_around_click() {
        use super::trim_line_at_points;
        use crate::geometry::{Arc, line_arc};
        let target = Line::new(Vec2::new(5.0, -20.0), Vec2::new(5.0, 20.0));
        let upper = Arc::new(Vec2::default(), 10.0, 0.0, core::f64::consts::PI, true);
        let pts = line_arc(&target, &upper);
        let got = trim_line_at_points(target, &pts, Vec2::new(5.0, -15.0));
        let y = 75.0f64.sqrt();
        let want = Line::new(Vec2::new(5.0, -20.0), Vec2::new(5.0, y));
        assert!(line_approx_eq(as_line(&got.expect("cut")), want));
        let three = [0.5, -0.4, 0.0].map(|t| Vec2::new(5.0, 20.0 * t));
        let got = trim_line_at_points(target, &three, Vec2::new(5.0, 3.0));
        let want = Line::new(Vec2::new(5.0, 0.0), Vec2::new(5.0, 10.0));
        assert!(line_approx_eq(as_line(&got.expect("cut")), want));
    }

    /// LCV-160 — cut points at the target's own endpoints are ignored, so a
    /// cutter that only touches an end changes nothing.
    #[test]
    fn trim_line_at_points_ignores_own_endpoints() {
        use super::trim_line_at_points;
        let target = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        let ends = [target.p1, target.p2];
        assert!(trim_line_at_points(target, &ends, Vec2::new(5.0, 0.0)).is_none());
        assert!(trim_line_at_points(target, &[], Vec2::new(5.0, 0.0)).is_none());
    }

    /// LCV-160 AC 6 — extending to an arc skips the parent circle's nearer
    /// crossing that lies off the span.
    #[test]
    fn extend_line_to_arc_uses_span_points_only() {
        use super::extend_line_to_arc;
        use crate::geometry::Arc;
        let target = Line::new(Vec2::new(5.0, -20.0), Vec2::new(5.0, -15.0));
        let upper = Arc::new(Vec2::default(), 10.0, 0.0, core::f64::consts::PI, true);
        let got = extend_line_to_arc(target, &upper, 1).expect("reaches the span");
        assert!(got.p2.approx_eq(Vec2::new(5.0, 75.0f64.sqrt()), 1e-9));
        assert!(extend_line_to_arc(target, &upper, 0).is_none());
    }
}
