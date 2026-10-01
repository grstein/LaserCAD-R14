//! Derivative roots of one Bézier coordinate (ADR 0016 §4).
//!
//! The derivative of a degree-`d` Bézier coordinate is, up to the factor
//! `d`, the degree-`d−1` Bernstein polynomial of the differences
//! `Δᵢ = cᵢ₊₁ − cᵢ`; its roots in `(0, 1)` split `[0, 1]` into monotone
//! pieces, each crossing a given line of that axis at most once.

use crate::geometry::{Bezier, EPSILON, Line, Vec2};

/// Bisection steps; `f64` bisection on `[0, 1]` stops earlier at a fixed point.
const BISECT_STEPS: usize = 64;

/// One coordinate of a point.
type Coord = fn(Vec2) -> f64;

impl Bezier {
    /// True iff the curve meets the axis-aligned segment `edge` (vertical when
    /// its ends share `x` within `EPSILON`, else horizontal), ends inclusive within `EPSILON`.
    /// Each monotone piece of the curve along the edge's normal axis is
    /// bisected for its one crossing of the edge line.
    pub fn crosses_axis_segment(&self, edge: &Line) -> bool {
        let vertical = (edge.p1.x - edge.p2.x).abs() <= EPSILON;
        let (along, across): (Coord, Coord) = if vertical {
            (|p| p.x, |p| p.y)
        } else {
            (|p| p.y, |p| p.x)
        };
        let value = along(edge.p1);
        let (lo, hi) = {
            let (a, b) = (across(edge.p1), across(edge.p2));
            (a.min(b) - EPSILON, a.max(b) + EPSILON)
        };
        let coords: Vec<f64> = self.points().iter().map(|&p| along(p)).collect();
        let f = |t: f64| along(self.point(t)) - value;
        let mut cuts = vec![0.0];
        cuts.extend(derivative_roots(&coords));
        cuts.push(1.0);
        cuts.windows(2).any(|w| {
            crossing(f, w[0], w[1]).is_some_and(|t| (lo..=hi).contains(&across(self.point(t))))
        })
    }
}

/// The parameter where `f` changes sign (or is zero) on `[t0, t1]`, by
/// bisection; `None` when `f` keeps one strict sign at both ends.
fn crossing(f: impl Fn(f64) -> f64, mut t0: f64, mut t1: f64) -> Option<f64> {
    let (f0, f1) = (f(t0), f(t1));
    if f0 == 0.0 {
        return Some(t0);
    }
    if f1 == 0.0 {
        return Some(t1);
    }
    if f0.signum() == f1.signum() {
        return None;
    }
    for _ in 0..BISECT_STEPS {
        let mid = 0.5 * (t0 + t1);
        if f(mid).signum() == f0.signum() {
            t0 = mid;
        } else {
            t1 = mid;
        }
    }
    Some(0.5 * (t0 + t1))
}

/// Roots in the open interval `(0, 1)` of the derivative of the Bézier
/// coordinate with control values `c` (3 or 4 values), in ascending order.
pub(super) fn derivative_roots(c: &[f64]) -> Vec<f64> {
    let d: Vec<f64> = c.windows(2).map(|w| w[1] - w[0]).collect();
    // Power form `a·t² + b·t + k` of the Bernstein polynomial of `d`.
    let (a, b, k) = match d.as_slice() {
        [d0, d1] => (0.0, d1 - d0, *d0),
        [d0, d1, d2] => (d0 - 2.0 * d1 + d2, 2.0 * (d1 - d0), *d0),
        _ => return Vec::new(),
    };
    let mut roots = quadratic_roots(a, b, k);
    roots.retain(|t| *t > 0.0 && *t < 1.0);
    roots.sort_by(f64::total_cmp);
    roots
}

/// Real roots of `a·t² + b·t + k`; a leading coefficient within `EPSILON`
/// of zero falls back to the linear root. Uses the cancellation-free form.
fn quadratic_roots(a: f64, b: f64, k: f64) -> Vec<f64> {
    if a.abs() <= EPSILON {
        return if b == 0.0 { Vec::new() } else { vec![-k / b] };
    }
    let disc = b * b - 4.0 * a * k;
    if disc < 0.0 {
        return Vec::new();
    }
    let q = -0.5 * (b + disc.sqrt().copysign(b));
    if q == 0.0 {
        return vec![0.0];
    }
    vec![q / a, k / q]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arch() -> Bezier {
        Bezier::Cubic([
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 40.0),
            Vec2::new(10.0, 40.0),
            Vec2::new(10.0, 0.0),
        ])
    }

    fn seg(a: (f64, f64), b: (f64, f64)) -> Line {
        Line::new(Vec2::new(a.0, a.1), Vec2::new(b.0, b.1))
    }

    /// The arch peaks at `y = 30` and crosses `y = 10` twice, near both ends.
    #[test]
    fn crossings_two_one_and_zero() {
        let b = arch();
        // Two crossings of the line, both inside the edge.
        assert!(b.crosses_axis_segment(&seg((-5.0, 10.0), (15.0, 10.0))));
        // Two crossings of the line; only the second piece's is inside.
        assert!(b.crosses_axis_segment(&seg((8.0, 10.0), (15.0, 10.0))));
        assert!(b.crosses_axis_segment(&seg((-5.0, 10.0), (2.0, 10.0))));
        // Two crossings of the line, neither inside the edge.
        assert!(!b.crosses_axis_segment(&seg((4.0, 10.0), (6.0, 10.0))));
        // Zero crossings: only the control polygon reaches y = 35.
        assert!(!b.crosses_axis_segment(&seg((-5.0, 35.0), (15.0, 35.0))));
        // One crossing of a vertical line, at the peak (5, 30).
        assert!(b.crosses_axis_segment(&seg((5.0, 20.0), (5.0, 40.0))));
        assert!(b.crosses_axis_segment(&seg((5.0, 40.0), (5.0, 30.0))));
        assert!(!b.crosses_axis_segment(&seg((5.0, 0.0), (5.0, 29.9))));
        // An edge through an end of the curve counts.
        assert!(b.crosses_axis_segment(&seg((10.0, -1.0), (10.0, 0.0))));
    }

    /// A quadratic crosses a low edge and misses one above its peak or beside it.
    #[test]
    fn quadratic_crossing_and_miss() {
        let q = Bezier::Quadratic([
            Vec2::new(0.0, 0.0),
            Vec2::new(5.0, 20.0),
            Vec2::new(10.0, 0.0),
        ]);
        assert!(q.crosses_axis_segment(&seg((0.0, 5.0), (3.0, 5.0))));
        assert!(!q.crosses_axis_segment(&seg((0.0, 10.5), (10.0, 10.5))));
        assert!(!q.crosses_axis_segment(&seg((11.0, -5.0), (11.0, 5.0))));
    }

    /// Roots of the derivative: none for a straight coordinate, the linear
    /// root for a quadratic, both of a cubic in ascending order.
    #[test]
    fn derivative_roots_by_degree() {
        assert!(derivative_roots(&[0.0, 1.0, 2.0, 3.0]).is_empty());
        assert_eq!(derivative_roots(&[0.0, 20.0, 0.0]), vec![0.5]);
        let r = derivative_roots(&[0.0, -10.0, 20.0, 10.0]);
        let s = 0.5f64.sqrt() / 2.0;
        assert_eq!(r.len(), 2);
        assert!((r[0] - (0.5 - s)).abs() <= 1e-12 && (r[1] - (0.5 + s)).abs() <= 1e-12);
        assert!(derivative_roots(&[0.0, 1.0, 3.0, 6.0]).is_empty());
    }
}
