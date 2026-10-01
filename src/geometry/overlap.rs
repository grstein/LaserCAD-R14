//! Shared segments and shared arc spans (LCV-194): two primitives overlap
//! when they have infinitely many common points, which the `intersect`
//! routines refuse to list.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use crate::geometry::distance::Prim;

/// True when `a` and `b` share a segment or an arc span of positive length.
/// Collinear segments that only touch end to end share one point, not a
/// segment.
pub fn overlaps(a: Prim, b: Prim) -> bool {
    let _ = (a, b);
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Arc, Circle, Line, Vec2};
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
