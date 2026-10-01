//! Quadrant, perpendicular, tangent and nearest candidates (LCV-161).
//!
//! Split off [`super`] to keep the snap files under the 300-LOC kernel cap.
//! Every point lies on the entity itself: segment feet need `t ∈ [0, 1]`,
//! arc points must be inside the sweep ([`Arc::contains_angle`]).

use crate::geometry::arc::Arc;
use crate::geometry::circle::Circle;
use crate::geometry::epsilon::EPSILON;
use crate::geometry::vec2::Vec2;

use super::candidates::{Candidate, make_candidate};
use super::{SnapEntity, SnapKind};

/// Push the world-axis quadrant points of every circle, and of every arc
/// where the quadrant angle lies inside the sweep.
pub(super) fn collect_quadrants(entities: &[SnapEntity], out: &mut Vec<Candidate>) {
    for (idx, e) in entities.iter().enumerate() {
        let (center, r, arc) = match e {
            SnapEntity::Line(_) | SnapEntity::Ellipse(_) => continue,
            SnapEntity::Circle(c) => (c.center, c.r, None),
            SnapEntity::Arc(a) => (a.center, a.r, Some(a)),
        };
        let quadrants = [
            (0.0, Vec2::new(r, 0.0)),
            (core::f64::consts::FRAC_PI_2, Vec2::new(0.0, r)),
            (core::f64::consts::PI, Vec2::new(-r, 0.0)),
            (3.0 * core::f64::consts::FRAC_PI_2, Vec2::new(0.0, -r)),
        ];
        for (angle, offset) in quadrants {
            if arc.is_none_or(|a| a.contains_angle(angle)) {
                out.push(make_candidate(center + offset, SnapKind::Quadrant, idx));
            }
        }
    }
}

/// Push the feet of the perpendicular from `anchor`: on a segment (only
/// when the foot lies on it), and the two points of a circle or arc on the
/// line through its centre and `anchor` (none when `anchor` is the centre).
pub(super) fn collect_perpendicular(
    anchor: Vec2,
    entities: &[SnapEntity],
    out: &mut Vec<Candidate>,
) {
    for (idx, e) in entities.iter().enumerate() {
        let (center, r, arc) = match e {
            SnapEntity::Line(l) => {
                let d = l.p2 - l.p1;
                let len_sq = d.length_squared();
                if len_sq > EPSILON * EPSILON {
                    let t = (anchor - l.p1).dot(d) / len_sq;
                    if (-EPSILON..=1.0 + EPSILON).contains(&t) {
                        let foot = l.p1 + d * t.clamp(0.0, 1.0);
                        out.push(make_candidate(foot, SnapKind::Perpendicular, idx));
                    }
                }
                continue;
            }
            SnapEntity::Ellipse(_) => continue,
            SnapEntity::Circle(c) => (c.center, c.r, None),
            SnapEntity::Arc(a) => (a.center, a.r, Some(a)),
        };
        let v = anchor - center;
        if v.length() <= EPSILON {
            continue;
        }
        let base = v.y.atan2(v.x);
        push_on_circle(
            center,
            r,
            arc,
            [base, base + core::f64::consts::PI],
            SnapKind::Perpendicular,
            idx,
            out,
        );
    }
}

/// Push the tangent points from `anchor` on every circle and arc that
/// `anchor` lies strictly outside of (inside the sweep, for an arc).
pub(super) fn collect_tangent(anchor: Vec2, entities: &[SnapEntity], out: &mut Vec<Candidate>) {
    for (idx, e) in entities.iter().enumerate() {
        let (center, r, arc) = match e {
            SnapEntity::Line(_) | SnapEntity::Ellipse(_) => continue,
            SnapEntity::Circle(c) => (c.center, c.r, None),
            SnapEntity::Arc(a) => (a.center, a.r, Some(a)),
        };
        let v = anchor - center;
        let d = v.length();
        if d <= r + EPSILON {
            continue;
        }
        let base = v.y.atan2(v.x);
        let alpha = (r / d).acos();
        push_on_circle(
            center,
            r,
            arc,
            [base - alpha, base + alpha],
            SnapKind::Tangent,
            idx,
            out,
        );
    }
}

/// Push the points of the circle `(center, r)` at `angles`, keeping only
/// those inside `arc`'s sweep when `arc` is given.
fn push_on_circle(
    center: Vec2,
    r: f64,
    arc: Option<&Arc>,
    angles: [f64; 2],
    kind: SnapKind,
    idx: usize,
    out: &mut Vec<Candidate>,
) {
    for angle in angles {
        if arc.is_none_or(|a| a.contains_angle(angle)) {
            let point = Circle::new(center, r).point_at_angle(angle);
            out.push(make_candidate(point, kind, idx));
        }
    }
}

/// Push a Nearest candidate for every entity whose closest point to `world`
/// lies within `tolerance`, with its distance filled in.
pub(super) fn collect_nearest(
    world: Vec2,
    tolerance: f64,
    entities: &[SnapEntity],
    out: &mut Vec<Candidate>,
) {
    for (idx, e) in entities.iter().enumerate() {
        let point = match e {
            SnapEntity::Ellipse(_) => None,
            SnapEntity::Line(l) => Some(l.closest_point(world)),
            SnapEntity::Circle(c) => radial_point(c, world, None),
            SnapEntity::Arc(a) => radial_point(&Circle::new(a.center, a.r), world, Some(a))
                .or_else(|| {
                    // Outside the sweep the arc's closest point is its nearer end.
                    let (s, e) = (a.start_point(), a.end_point());
                    Some(if (s - world).length() <= (e - world).length() {
                        s
                    } else {
                        e
                    })
                }),
        };
        let Some(point) = point else { continue };
        let d = (point - world).length();
        if d <= tolerance {
            let mut c = make_candidate(point, SnapKind::Nearest, idx);
            c.distance = d;
            out.push(c);
        }
    }
}

/// Point of `circle` on the ray from its centre through `p`, if `p` is not
/// the centre and (for an arc) the ray's angle is inside the sweep.
fn radial_point(circle: &Circle, p: Vec2, arc: Option<&Arc>) -> Option<Vec2> {
    let v = p - circle.center;
    if v.length() <= EPSILON {
        return None;
    }
    let angle = v.y.atan2(v.x);
    arc.is_none_or(|a| a.contains_angle(angle))
        .then(|| circle.point_at_angle(angle))
}
