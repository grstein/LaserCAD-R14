//! Line-target trim and line-target extend helpers — the four entity-pair
//! cases where the target is a [`Line`]: [`trim_line_by_line`] (Line cutter),
//! [`trim_line_by_circle`] (Circle cutter), [`extend_line_to_line`] (Line
//! boundary), [`extend_line_to_circle`] (Circle boundary). See the parent
//! [`super`] module for the public commands wrapping these routines.

use super::parametric_t;
use crate::document::Entity;
use crate::geometry::{line_circle, line_line, line_line_infinite, Circle, Line, Vec2, EPSILON};

/// Trim a [`Line`] target by a [`Line`] cutter. The cutter's strict-segment
/// intersection point `X` splits the target into `[p1, X]` and `[X, p2]`.
/// Returns whichever sub-segment contains `keep` (parametric test on the
/// target's infinite line); when ambiguous (`keep` exactly at `X`) the
/// deterministic choice is `[p1, X]`.
pub(crate) fn trim_line_by_line(target: Line, cutter: &Line, keep: Vec2) -> Option<Entity> {
    let x = line_line(&target, cutter)?;
    let t_x = parametric_t(&target, x);
    let t_keep = parametric_t(&target, keep);
    if t_keep <= t_x + EPSILON {
        Some(Entity::Line(Line::new(target.p1, x)))
    } else {
        Some(Entity::Line(Line::new(x, target.p2)))
    }
}

/// Trim a [`Line`] target by a [`Circle`] cutter. One intersection splits
/// the target into 2 sub-segments; two intersections split into 3 (the
/// "middle" sub-segment is what the cutter carves out — kept when `keep` is
/// inside the cutter). A tangent (one point) is treated like a single split.
/// Returns `None` when the cutter does not intersect the target.
pub(crate) fn trim_line_by_circle(target: Line, cutter: &Circle, keep: Vec2) -> Option<Entity> {
    let pts = line_circle(&target, cutter);
    if pts.is_empty() {
        return None;
    }
    let mut ts: Vec<(f64, Vec2)> = pts
        .into_iter()
        .map(|p| (parametric_t(&target, p), p))
        .collect();
    ts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(core::cmp::Ordering::Equal));
    let t_keep = parametric_t(&target, keep);
    match ts.len() {
        1 => {
            let (t_x, x) = ts[0];
            if t_keep <= t_x + EPSILON {
                Some(Entity::Line(Line::new(target.p1, x)))
            } else {
                Some(Entity::Line(Line::new(x, target.p2)))
            }
        }
        2 => {
            let (t_a, a) = ts[0];
            let (t_b, b) = ts[1];
            if t_keep < t_a - EPSILON {
                Some(Entity::Line(Line::new(target.p1, a)))
            } else if t_keep > t_b + EPSILON {
                Some(Entity::Line(Line::new(b, target.p2)))
            } else {
                Some(Entity::Line(Line::new(a, b)))
            }
        }
        _ => None,
    }
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
/// [`Circle`]. Uses an infinite-line interpretation: the segment is extended
/// along its own direction until it hits the circle. Returns `None` if the
/// infinite line misses the circle or every intersection is behind the
/// chosen endpoint.
pub(crate) fn extend_line_to_circle(target: Line, boundary: &Circle, endpoint: u8) -> Option<Line> {
    let d = target.p2 - target.p1;
    let len = d.length();
    if len <= EPSILON {
        return None;
    }
    let dir = d / len;
    // Reach far enough that any sensible boundary lies inside the surrogate
    // segment that `line_circle` will solve. `+ len + 1.0` is slack so the
    // parametric `[0, 1]` window of the surrogate covers both `p1` and `p2`.
    let reach = (boundary.center - target.p1).length() + boundary.r + len + 1.0;
    let surrogate = Line::new(target.p1 - dir * reach, target.p1 + dir * reach);
    let pts = line_circle(&surrogate, boundary);
    if pts.is_empty() {
        return None;
    }
    let endpoint_pos = if endpoint == 0 { target.p1 } else { target.p2 };
    let mut best: Option<(f64, Vec2)> = None;
    for p in pts {
        let t = parametric_t(&target, p);
        let valid = if endpoint == 0 {
            t <= -EPSILON
        } else {
            t >= 1.0 + EPSILON
        };
        if !valid {
            continue;
        }
        let dist = (p - endpoint_pos).length();
        if best.map(|(bd, _)| dist < bd).unwrap_or(true) {
            best = Some((dist, p));
        }
    }
    let (_, x) = best?;
    apply_extend(target, x, endpoint)
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
    use crate::geometry::{Circle, Line, Vec2, EPSILON};

    fn doc_with(entities: Vec<Entity>) -> Document {
        Document {
            entities,
            ..Document::default()
        }
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
}
