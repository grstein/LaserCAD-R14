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
pub fn closest(a: Prim, b: Prim) -> (Vec2, Vec2) {
    let _ = (a, b);
    (Vec2::default(), Vec2::default())
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
        assert!((dist(c, circle(0.0, 0.0, 2.0)) - 2.0).abs() < 1e-12);
        assert!((dist(c, arc(0.0, 0.0, 2.0, 1.0, 2.0)) - 2.0).abs() < 1e-12);
        assert_eq!(
            dist(Prim::Point(v(5.0, 0.0)), seg(0.0, 0.0, 10.0, 0.0)),
            0.0
        );
    }
}
