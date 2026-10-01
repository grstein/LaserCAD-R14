//! Closest points of two bounded primitives (LCV-194): a point, a segment,
//! a circle or an arc, each taken as drawn, never as its extension.
//!
//! [`closest`] answers the pair of points, one on each primitive, at the
//! minimum distance. Primitives that touch, cross or overlap answer a shared
//! point (distance 0).
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use crate::geometry::arc::Arc;
use crate::geometry::circle::Circle;
use crate::geometry::intersect::{
    arc_arc, circle_arc, circle_circle, line_arc, line_circle, line_line,
};
use crate::geometry::line::Line;
use crate::geometry::vec2::Vec2;

/// One bounded primitive, by value.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Prim {
    /// A point in mm.
    Point(Vec2),
    /// A segment.
    Line(Line),
    /// A full circle.
    Circle(Circle),
    /// A proper arc, bounded by its span.
    Arc(Arc),
}

/// The two closest points, the first on `a` and the second on `b`.
///
/// A crossing or tangency answers that point twice. Otherwise the minimum is
/// taken over the candidate pairs (LCV-194 plan): every endpoint of one
/// primitive against the other, and the mutual-normal points of a curve on
/// the line through both centres, or through the foot of the perpendicular
/// for a segment, each kept only on the arc's span. The first minimum wins,
/// so the answer is deterministic.
pub fn closest(a: Prim, b: Prim) -> (Vec2, Vec2) {
    if let Some(&x) = intersections(a, b).first() {
        return (x, x);
    }
    let mut best = (a.anchor(), b.foot(a.anchor()));
    let mut keep = |p: Vec2, q: Vec2| {
        if p.distance_squared(q) < best.0.distance_squared(best.1) {
            best = (p, q);
        }
    };
    for p in a.candidates(b) {
        keep(p, b.foot(p));
    }
    for q in b.candidates(a) {
        keep(a.foot(q), q);
    }
    best
}

/// Every crossing or tangency point of two primitives, in the order the
/// `intersect` routines give them. A point never intersects here (its
/// distance is the foot); collinear or concentric overlaps give none.
pub fn intersections(a: Prim, b: Prim) -> Vec<Vec2> {
    use Prim::{Arc as A, Circle as C, Line as L};
    match (a, b) {
        (L(l), L(m)) => line_line(&l, &m).into_iter().collect(),
        (L(l), C(c)) | (C(c), L(l)) => line_circle(&l, &c),
        (L(l), A(r)) | (A(r), L(l)) => line_arc(&l, &r),
        (C(c), C(d)) => circle_circle(&c, &d),
        (C(c), A(r)) | (A(r), C(c)) => circle_arc(&c, &r),
        (A(r), A(s)) => arc_arc(&r, &s),
        _ => Vec::new(),
    }
}

impl Prim {
    /// One point that is surely on the primitive.
    fn anchor(self) -> Vec2 {
        match self {
            Self::Point(p) => p,
            Self::Line(l) => l.p1,
            Self::Circle(c) => c.center + Vec2::new(c.r, 0.0),
            Self::Arc(a) => a.start_point(),
        }
    }

    /// The point of the primitive closest to `p`; the centre of a curve
    /// answers its point at angle 0 (or the arc's start).
    fn foot(self, p: Vec2) -> Vec2 {
        match self {
            Self::Point(q) => q,
            Self::Line(l) => l.closest_point(p),
            Self::Circle(c) => radial(c.center, c.r, p),
            Self::Arc(a) => {
                let on = radial(a.center, a.r, p);
                let v = on - a.center;
                if (p - a.center).normalize().is_some() && a.contains_angle(v.y.atan2(v.x)) {
                    return on;
                }
                let (s, e) = (a.start_point(), a.end_point());
                if p.distance_squared(e) < p.distance_squared(s) {
                    e
                } else {
                    s
                }
            }
        }
    }

    /// The points of `self` that may be closest to `other`: its endpoints,
    /// plus, for a curve, its points on the line through its centre and
    /// the other curve's centre, or through the foot of the perpendicular on
    /// the other segment; an arc keeps only those on its span.
    fn candidates(self, other: Prim) -> Vec<Vec2> {
        let (center, r) = match self {
            Self::Point(p) => return vec![p],
            Self::Line(l) => return vec![l.p1, l.p2],
            Self::Circle(c) => (c.center, c.r),
            Self::Arc(a) => (a.center, a.r),
        };
        let toward = match other {
            Self::Circle(c) => (c.center - center).normalize(),
            Self::Arc(a) => (a.center - center).normalize(),
            Self::Line(l) => l.direction().map(|d| Vec2::new(-d.y, d.x)),
            Self::Point(_) => None,
        };
        let mut out = vec![self.anchor()];
        if let Self::Arc(a) = self {
            out.push(a.end_point());
        }
        if let Some(u) = toward {
            for p in [center + u * r, center - u * r] {
                let v = p - center;
                if !matches!(self, Self::Arc(a) if !a.contains_angle(v.y.atan2(v.x))) {
                    out.push(p);
                }
            }
        }
        out
    }
}

/// The point of the circle (`center`, `r`) closest to `p`; the centre
/// itself answers the point at angle 0.
fn radial(center: Vec2, r: f64, p: Vec2) -> Vec2 {
    center + (p - center).normalize().unwrap_or(Vec2::new(1.0, 0.0)) * r
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::{FRAC_PI_2, PI};

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }
    fn seg(x1: f64, y1: f64, x2: f64, y2: f64) -> Prim {
        Prim::Line(Line::new(v(x1, y1), v(x2, y2)))
    }
    fn circle(x: f64, y: f64, r: f64) -> Prim {
        Prim::Circle(Circle::new(v(x, y), r))
    }
    fn arc(x: f64, y: f64, r: f64, start: f64, end: f64) -> Prim {
        Prim::Arc(Arc::new(v(x, y), r, start, end, true))
    }

    /// The distance, both ways round, with the points on the right sides.
    fn dist(a: Prim, b: Prim) -> f64 {
        let (p, q) = closest(a, b);
        let (q2, p2) = closest(b, a);
        let d = p.distance(q);
        assert!((d - p2.distance(q2)).abs() < 1e-9, "{a:?} {b:?} asymmetric");
        d
    }

    /// `closest(a, b)` lands on `want` (both points, in order).
    fn pins(a: Prim, b: Prim, want: (Vec2, Vec2)) {
        let (p, q) = closest(a, b);
        assert!(
            p.approx_eq(want.0, 1e-9) && q.approx_eq(want.1, 1e-9),
            "{a:?} {b:?}: got {p:?} {q:?}, want {want:?}"
        );
    }

    #[test]
    fn crossing_segments_touch_at_the_crossing() {
        let (a, b) = (seg(0.0, 0.0, 10.0, 10.0), seg(0.0, 10.0, 10.0, 0.0));
        pins(a, b, (v(5.0, 5.0), v(5.0, 5.0)));
        let t = seg(5.0, 5.0, 5.0, 20.0);
        assert_eq!(dist(a, t), 0.0, "an endpoint on the other segment");
    }

    #[test]
    fn parallel_and_skew_segments() {
        let a = seg(0.0, 0.0, 10.0, 0.0);
        assert!((dist(a, seg(2.0, 3.0, 8.0, 3.0)) - 3.0).abs() < 1e-12);
        // Parallel, past the end: the endpoint-to-endpoint distance.
        pins(a, seg(13.0, 4.0, 20.0, 4.0), (v(10.0, 0.0), v(13.0, 4.0)));
        // Skew, not crossing: an endpoint of one to the other segment.
        pins(a, seg(4.0, 2.0, 6.0, 9.0), (v(4.0, 0.0), v(4.0, 2.0)));
        // Collinear and overlapping: zero.
        assert!(dist(a, seg(5.0, 0.0, 15.0, 0.0)) < 1e-12);
    }

    #[test]
    fn a_line_outside_inside_and_through_a_circle() {
        let c = circle(0.0, 0.0, 5.0);
        pins(seg(-10.0, 8.0, 10.0, 8.0), c, (v(0.0, 8.0), v(0.0, 5.0)));
        // Wholly inside: the nearer endpoint, radially out.
        pins(seg(0.0, 1.0, 0.0, 3.0), c, (v(0.0, 3.0), v(0.0, 5.0)));
        // Through: a crossing.
        assert_eq!(dist(seg(-10.0, 0.0, 10.0, 0.0), c), 0.0);
        // Tangent: distance 0.
        assert!(dist(seg(-10.0, 5.0, 10.0, 5.0), c) < 1e-9);
        // Outside, the foot beyond the segment: the endpoint.
        pins(seg(8.0, 0.0, 9.0, 0.0), c, (v(8.0, 0.0), v(5.0, 0.0)));
        // Through, no endpoint or radial candidate on the other: a crossing.
        let (p, q) = closest(seg(-10.0, 1.0, 10.0, 1.0), c);
        assert!(p.distance(q) < 1e-9 && (p.y - 1.0).abs() < 1e-9, "{p:?}");
        // A slanted line: the mutual normal runs along its perpendicular.
        pins(c, seg(10.0, 5.0, 2.0, 11.0), (v(3.0, 4.0), v(6.0, 8.0)));
    }

    #[test]
    fn a_line_whose_foot_misses_the_arc_span_takes_the_endpoint() {
        // Upper half; the line lies below it, so the arc's ends are closest.
        let upper = arc(0.0, 0.0, 5.0, 0.0, PI);
        pins(
            seg(-1.0, -3.0, 2.0, -3.0),
            upper,
            (v(2.0, -3.0), v(5.0, 0.0)),
        );
        // Inside the parent circle: the endpoint farthest out, radially.
        let out = v(3.0, 1.0) * (5.0 / 10f64.sqrt());
        pins(seg(1.0, 1.0, 3.0, 1.0), upper, (v(3.0, 1.0), out));
        // A line far away whose perpendicular tangent point is on the span.
        pins(
            seg(-10.0, 9.0, 10.0, 9.0),
            upper,
            (v(0.0, 9.0), v(0.0, 5.0)),
        );
    }

    #[test]
    fn circles_separate_nested_and_concentric() {
        let a = circle(0.0, 0.0, 2.0);
        pins(a, circle(10.0, 0.0, 3.0), (v(2.0, 0.0), v(7.0, 0.0)));
        pins(
            circle(0.0, 0.0, 10.0),
            circle(3.0, 0.0, 2.0),
            (v(10.0, 0.0), v(5.0, 0.0)),
        );
        assert!((dist(a, circle(0.0, 0.0, 5.0)) - 3.0).abs() < 1e-12);
        assert!(dist(a, circle(3.0, 0.0, 2.0)) < 1e-9, "crossing");
        assert!(dist(a, a) < 1e-12, "the same circle");
    }

    #[test]
    fn an_arc_and_an_arc() {
        let right = arc(0.0, 0.0, 2.0, -FRAC_PI_2, FRAC_PI_2);
        let left = arc(10.0, 0.0, 3.0, FRAC_PI_2, 1.5 * PI);
        pins(right, left, (v(2.0, 0.0), v(7.0, 0.0)));
        // Facing away: the mutual normals miss both spans.
        let away = arc(10.0, 1.0, 3.0, -FRAC_PI_2, FRAC_PI_2);
        let left_of_origin = arc(0.0, 0.0, 2.0, FRAC_PI_2, 1.5 * PI);
        pins(left_of_origin, away, (v(0.0, -2.0), v(10.0, -2.0)));
        // Concentric, disjoint spans: endpoint to endpoint radially aside.
        let quarter = arc(0.0, 0.0, 5.0, 0.0, FRAC_PI_2);
        let opposite = arc(0.0, 0.0, 3.0, PI, 1.5 * PI);
        let d = dist(quarter, opposite);
        assert!((d - v(5.0, 0.0).distance(v(0.0, -3.0))).abs() < 1e-9, "{d}");
        // Concentric, shared span: the radial gap.
        assert!((dist(quarter, arc(0.0, 0.0, 3.0, 0.5, 1.0)) - 2.0).abs() < 1e-9);
        // An arc on a circle: crossing.
        assert!(dist(right, circle(2.0, 0.0, 1.0)) < 1e-9);
        // Crossing arcs, no candidate on the crossing: the crossing point.
        let crossing = arc(6.0, 0.0, 5.0, FRAC_PI_2, PI);
        pins(
            arc(0.0, 0.0, 5.0, 0.0, PI),
            crossing,
            (v(3.0, 4.0), v(3.0, 4.0)),
        );
        // Neither centre at the origin: the line through both centres.
        let near = (v(1.8, 1.6), v(4.2, 3.4));
        pins(
            arc(1.0, 1.0, 1.0, -FRAC_PI_2, FRAC_PI_2),
            circle(5.0, 4.0, 1.0),
            near,
        );
        pins(circle(1.0, 1.0, 1.0), circle(5.0, 4.0, 1.0), near);
        let facing = arc(5.0, 4.0, 1.0, FRAC_PI_2, 1.5 * PI);
        pins(arc(1.0, 1.0, 1.0, -FRAC_PI_2, FRAC_PI_2), facing, near);
    }

    #[test]
    fn a_point_and_each_kind() {
        let p = Prim::Point(v(3.0, 4.0));
        pins(p, Prim::Point(v(0.0, 0.0)), (v(3.0, 4.0), v(0.0, 0.0)));
        pins(p, seg(0.0, 0.0, 10.0, 0.0), (v(3.0, 4.0), v(3.0, 0.0)));
        pins(p, circle(0.0, 0.0, 10.0), (v(3.0, 4.0), v(6.0, 8.0)));
        pins(p, circle(0.0, 0.0, 1.0), (v(3.0, 4.0), v(0.6, 0.8)));
        // On the arc's span, then off it (the nearer end).
        pins(
            p,
            arc(0.0, 0.0, 10.0, 0.0, FRAC_PI_2),
            (v(3.0, 4.0), v(6.0, 8.0)),
        );
        pins(
            p,
            arc(0.0, 0.0, 10.0, PI, 1.5 * PI),
            (v(3.0, 4.0), v(-10.0, 0.0)),
        );
        // The centre: any point of the circle is closest; the answer is fixed.
        let c = Prim::Point(v(0.0, 0.0));
        pins(circle(0.0, 0.0, 2.0), c, (v(2.0, 0.0), v(0.0, 0.0)));
        // An arc's centre off the origin: the start wins the tie of its ends.
        let off = arc(1.0, 1.0, 2.0, -FRAC_PI_2, FRAC_PI_2);
        pins(Prim::Point(v(1.0, 1.0)), off, (v(1.0, 1.0), v(1.0, -1.0)));
        assert!((dist(c, circle(0.0, 0.0, 2.0)) - 2.0).abs() < 1e-12);
        assert!((dist(c, arc(0.0, 0.0, 2.0, 1.0, 2.0)) - 2.0).abs() < 1e-12);
        assert_eq!(
            dist(Prim::Point(v(5.0, 0.0)), seg(0.0, 0.0, 10.0, 0.0)),
            0.0
        );
    }
}
