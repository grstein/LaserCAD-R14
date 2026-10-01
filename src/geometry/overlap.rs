//! Shared segments and shared arc spans (LCV-194): two primitives overlap
//! when they have infinitely many common points, which the `intersect`
//! routines refuse to list.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use core::f64::consts::TAU;

use crate::geometry::distance::Prim;
use crate::geometry::epsilon::EPSILON;
use crate::geometry::line::Line;
use crate::geometry::vec2::Vec2;

/// True when `a` and `b` share a segment or an arc span of positive length.
/// Collinear segments that only touch end to end share one point, not a
/// segment.
pub fn overlaps(a: Prim, b: Prim) -> bool {
    if let (Prim::Line(l), Prim::Line(m)) = (a, b) {
        return segments(&l, &m);
    }
    match (span(a), span(b)) {
        (Some(x), Some(y)) => {
            x.0.approx_eq(y.0, EPSILON)
                && (x.1 - y.1).abs() <= EPSILON
                && shared(x.2, y.2) > EPSILON
        }
        _ => false,
    }
}

/// Collinear segments whose parameter ranges share more than a point.
fn segments(l: &Line, m: &Line) -> bool {
    let Some(d) = l.direction() else {
        return false;
    };
    let off = |p: Vec2| d.cross(p - l.p1).abs() <= EPSILON;
    if m.length() <= EPSILON || !off(m.p1) || !off(m.p2) {
        return false;
    }
    let (s, t) = (d.dot(m.p1 - l.p1), d.dot(m.p2 - l.p1));
    s.max(t).min(l.length()) - s.min(t).max(0.0) > EPSILON
}

/// A curve as `(centre, radius, (CCW start in [0, 2π), sweep))`; a circle
/// sweeps the whole turn.
fn span(p: Prim) -> Option<(Vec2, f64, (f64, f64))> {
    match p {
        Prim::Circle(c) => Some((c.center, c.r, (0.0, TAU))),
        Prim::Arc(a) => {
            let start = if a.ccw { a.start_angle } else { a.end_angle };
            Some((a.center, a.r, (start.rem_euclid(TAU), a.sweep_angle())))
        }
        _ => None,
    }
}

/// The angle two CCW spans share, the second taken where it starts and one
/// turn earlier so a span across the 0 wrap still meets the first.
fn shared(x: (f64, f64), y: (f64, f64)) -> f64 {
    let d = (y.0 - x.0).rem_euclid(TAU);
    [d, d - TAU]
        .into_iter()
        .map(|start| (start + y.1).min(x.1) - start.max(0.0))
        .fold(0.0, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Arc, Circle};
    use core::f64::consts::{FRAC_PI_2, PI};

    fn seg(x1: f64, y1: f64, x2: f64, y2: f64) -> Prim {
        Prim::Line(Line::new(Vec2::new(x1, y1), Vec2::new(x2, y2)))
    }
    fn circle(r: f64) -> Prim {
        Prim::Circle(Circle::new(Vec2::default(), r))
    }
    fn arc(r: f64, start: f64, end: f64, ccw: bool) -> Prim {
        Prim::Arc(Arc::new(Vec2::default(), r, start, end, ccw))
    }

    /// Both ways round.
    fn both(a: Prim, b: Prim) -> bool {
        let ab = overlaps(a, b);
        assert_eq!(ab, overlaps(b, a), "{a:?} {b:?} asymmetric");
        ab
    }

    #[test]
    fn collinear_segments_overlap_or_touch() {
        let a = seg(0.0, 0.0, 10.0, 0.0);
        assert!(both(a, seg(5.0, 0.0, 15.0, 0.0)));
        assert!(both(a, seg(12.0, 0.0, 3.0, 0.0)), "reversed");
        assert!(both(a, a), "the same segment");
        assert!(
            !both(a, seg(10.0, 0.0, 20.0, 0.0)),
            "end to end is one point"
        );
        assert!(!both(a, seg(11.0, 0.0, 20.0, 0.0)), "collinear, apart");
        assert!(!both(a, seg(0.0, 1.0, 10.0, 1.0)), "parallel");
        assert!(!both(a, seg(5.0, -5.0, 5.0, 5.0)), "crossing");
        let slanted = seg(0.0, 0.0, 4.0, 3.0);
        assert!(both(slanted, seg(8.0, 6.0, 2.0, 1.5)));
        assert!(!both(seg(0.0, 0.0, 0.0, 0.0), a), "a degenerate segment");
    }

    #[test]
    fn the_same_circle_overlaps() {
        assert!(both(circle(5.0), circle(5.0)));
        assert!(!both(circle(5.0), circle(4.0)), "concentric, other radius");
        let moved = Prim::Circle(Circle::new(Vec2::new(1.0, 0.0), 5.0));
        assert!(!both(circle(5.0), moved));
        assert!(!both(circle(5.0), seg(-5.0, 0.0, 5.0, 0.0)));
    }

    #[test]
    fn concentric_equal_radius_arcs_with_shared_or_disjoint_spans() {
        let quarter = arc(5.0, 0.0, FRAC_PI_2, true);
        assert!(both(quarter, arc(5.0, 1.0, 2.0, true)));
        assert!(both(quarter, arc(5.0, 1.0, -1.0, false)), "a CW arc");
        assert!(!both(quarter, arc(5.0, PI, 1.5 * PI, true)), "disjoint");
        assert!(!both(quarter, arc(5.0, FRAC_PI_2, PI, true)), "end to end");
        assert!(
            !both(quarter, arc(4.0, 0.0, FRAC_PI_2, true)),
            "other radius"
        );
        // Across the 0 wrap: 350°..10° and 5°..40°.
        let wrap = arc(5.0, -0.17, 0.17, true);
        assert!(both(wrap, arc(5.0, 0.09, 0.7, true)));
        assert!(both(wrap, arc(5.0, 6.0, 6.2, true)));
        assert!(!both(wrap, arc(5.0, 3.0, 3.5, true)));
    }

    #[test]
    fn an_arc_lying_on_a_circle() {
        let quarter = arc(5.0, 0.0, FRAC_PI_2, true);
        assert!(both(quarter, circle(5.0)));
        assert!(!both(quarter, circle(6.0)));
        assert!(
            !both(Prim::Point(Vec2::new(5.0, 0.0)), circle(5.0)),
            "a point"
        );
    }
}
