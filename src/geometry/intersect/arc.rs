//! Span-aware intersections involving an [`Arc`]: segment-arc, circle-arc
//! and arc-arc.
//!
//! Each routine solves the arc's parent circle with the circle routines of
//! the sibling modules, then keeps only the points whose angle lies on the
//! arc's span ([`Arc::contains_angle`], endpoints inclusive). A point on the
//! parent circle but outside the span is not an intersection. Introduced by
//! demand LCV-160.
//!
//! See the parent module [`crate::geometry::intersect`] for the wider
//! contract and conventions.

use super::{circle_circle, line_circle};
use crate::geometry::arc::Arc;
use crate::geometry::circle::Circle;
use crate::geometry::line::Line;
use crate::geometry::vec2::Vec2;

/// Intersect a **segment** with an arc. Returns 0, 1 or 2 points, each on
/// both the segment and the arc's span. Order is not guaranteed.
pub fn line_arc(line: &Line, arc: &Arc) -> Vec<Vec2> {
    on_span(arc, line_circle(line, &parent(arc)))
}

/// Intersect a full circle with an arc. Returns 0, 1 or 2 points on the
/// circle and on the arc's span; concentric pairs return none (see
/// [`circle_circle`]).
pub fn circle_arc(circle: &Circle, arc: &Arc) -> Vec<Vec2> {
    on_span(arc, circle_circle(circle, &parent(arc)))
}

/// Intersect two arcs. Returns 0, 1 or 2 points that lie on both spans;
/// concentric pairs return none (see [`circle_circle`]).
pub fn arc_arc(a: &Arc, b: &Arc) -> Vec<Vec2> {
    on_span(b, on_span(a, circle_circle(&parent(a), &parent(b))))
}

/// The arc's parent circle.
fn parent(arc: &Arc) -> Circle {
    Circle::new(arc.center, arc.r)
}

/// Keep the points whose polar angle about the arc's center is on its span.
fn on_span(arc: &Arc, pts: Vec<Vec2>) -> Vec<Vec2> {
    pts.into_iter()
        .filter(|p| {
            let v = *p - arc.center;
            arc.contains_angle(v.y.atan2(v.x))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::EPSILON;
    use core::f64::consts::{FRAC_PI_2, PI};

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    fn assert_same_set(got: &[Vec2], expected: &[Vec2]) {
        assert_eq!(got.len(), expected.len(), "got = {got:?}");
        for exp in expected {
            assert!(
                got.iter().any(|g| g.approx_eq(*exp, 1e-9)),
                "missing {exp:?} in {got:?}"
            );
        }
    }

    /// Upper half of the radius-5 circle, CCW from 0 to π.
    fn upper() -> Arc {
        Arc::new(Vec2::default(), 5.0, 0.0, PI, true)
    }

    /// The three routines are reachable through `geometry::`.
    #[test]
    fn public_api_surface() {
        use crate::geometry::{arc_arc, circle_arc, line_arc};
        let l = Line::new(v(-6.0, 1.0), v(6.0, 1.0));
        let _: Vec<Vec2> = line_arc(&l, &upper());
        let _: Vec<Vec2> = circle_arc(&Circle::new(v(5.0, 0.0), 2.0), &upper());
        let _: Vec<Vec2> = arc_arc(&upper(), &upper());
    }

    /// A horizontal line through the upper half only meets the span above
    /// the X axis; the mirror line below meets nothing.
    #[test]
    fn line_arc_keeps_only_span_points() {
        let above = Line::new(v(-6.0, 3.0), v(6.0, 3.0));
        assert_same_set(&line_arc(&above, &upper()), &[v(-4.0, 3.0), v(4.0, 3.0)]);
        let below = Line::new(v(-6.0, -3.0), v(6.0, -3.0));
        assert!(line_arc(&below, &upper()).is_empty());
        let vertical = Line::new(v(0.0, -6.0), v(0.0, 6.0));
        assert_same_set(&line_arc(&vertical, &upper()), &[v(0.0, 5.0)]);
    }

    /// A CW arc from π/2 down to −π/2 covers the right half.
    #[test]
    fn line_arc_cw_arc() {
        let right = Arc::new(Vec2::default(), 5.0, FRAC_PI_2, -FRAC_PI_2, false);
        let l = Line::new(v(-6.0, 3.0), v(6.0, 3.0));
        assert_same_set(&line_arc(&l, &right), &[v(4.0, 3.0)]);
    }

    /// A CCW arc from 3π/4 to −3π/4 spans the −X side, across the ±π wrap.
    #[test]
    fn line_arc_across_pi_wrap() {
        let left = Arc::new(Vec2::default(), 5.0, 0.75 * PI, -0.75 * PI, true);
        let axis = Line::new(v(-6.0, 0.0), v(6.0, 0.0));
        assert_same_set(&line_arc(&axis, &left), &[v(-5.0, 0.0)]);
    }

    /// A tangent line touching the span yields one point.
    #[test]
    fn line_arc_tangent() {
        let l = Line::new(v(-6.0, 5.0), v(6.0, 5.0));
        assert_same_set(&line_arc(&l, &upper()), &[v(0.0, 5.0)]);
    }

    /// A crossing exactly at an arc endpoint counts (inclusive span).
    #[test]
    fn line_arc_includes_endpoints() {
        let l = Line::new(v(-6.0, 0.0), v(6.0, 0.0));
        assert_same_set(&line_arc(&l, &upper()), &[v(-5.0, 0.0), v(5.0, 0.0)]);
    }

    /// A circle straddling the arc's +X end: only the upper crossing counts.
    #[test]
    fn circle_arc_filters_by_span() {
        let c = Circle::new(v(8.0, 0.0), 5.0);
        assert_same_set(&circle_arc(&c, &upper()), &[v(4.0, 3.0)]);
        assert!(circle_arc(&Circle::new(Vec2::default(), 5.0), &upper()).is_empty());
    }

    /// Both spans filter an arc-arc pair.
    #[test]
    fn arc_arc_filters_both_spans() {
        let other_upper = Arc::new(v(8.0, 0.0), 5.0, 0.0, PI, true);
        assert_same_set(&arc_arc(&upper(), &other_upper), &[v(4.0, 3.0)]);
        let other_lower = Arc::new(v(8.0, 0.0), 5.0, PI, 2.0 * PI, true);
        assert!(arc_arc(&upper(), &other_lower).is_empty());
        let tangent = Arc::new(v(10.0, 0.0), 5.0, FRAC_PI_2, 1.5 * PI, true);
        let right = Arc::new(Vec2::default(), 5.0, -FRAC_PI_2, FRAC_PI_2, true);
        let pts = arc_arc(&right, &tangent);
        assert_same_set(&pts, &[v(5.0, 0.0)]);
        assert!(pts[0].approx_eq(v(5.0, 0.0), EPSILON));
    }
}
