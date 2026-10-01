//! Nearest point on an ellipse and segment hits (ADR 0015 §7).
//!
//! The foot on the full ellipse is the robust bisection of Eberly,
//! "Distance from a point to an ellipse". On a span, the foot counts if its
//! parameter lies within the span, else the nearer span end wins. This is
//! exact while the point is closer to the curve than its smallest radius of
//! curvature; farther away it only ranks candidates.

use crate::geometry::{Circle, Ellipse, Line, Vec2, intersect};

/// Bisection steps; `f64` bisection always stops earlier at a fixed point.
const MAX_ITER: usize = 1100;

impl Ellipse {
    /// `p` in the ellipse's own frame: centered, `rx` along +X.
    fn local(&self, p: Vec2) -> Vec2 {
        let (s, c) = self.rotation.sin_cos();
        let d = p - self.center;
        Vec2::new(d.x * c + d.y * s, -d.x * s + d.y * c)
    }

    /// Parameter of the foot of `p` on the full ellipse.
    fn foot_param(&self, p: Vec2) -> f64 {
        let l = self.local(p);
        let (a, b) = (self.rx.abs(), self.ry.abs());
        let swap = a < b;
        let (e0, e1, y0, y1) = if swap {
            (b, a, l.y.abs(), l.x.abs())
        } else {
            (a, b, l.x.abs(), l.y.abs())
        };
        let (x0, x1) = first_quadrant_foot(e0, e1, y0, y1);
        let (fx, fy) = if swap { (x1, x0) } else { (x0, x1) };
        let (fx, fy) = (fx.copysign(l.x), fy.copysign(l.y));
        (fy / self.ry).atan2(fx / self.rx)
    }

    /// The point of the curve (span included) nearest to `p`.
    pub fn nearest(&self, p: Vec2) -> Vec2 {
        let t = self.foot_param(p);
        if self.contains_param(t) {
            return self.point(t);
        }
        match (self.start_point(), self.end_point()) {
            (Some(s), Some(e)) if p.distance(e) < p.distance(s) => e,
            (Some(s), _) => s,
            _ => self.point(t),
        }
    }

    /// Unsigned distance from `p` to the curve (span included).
    pub fn distance_to_point(&self, p: Vec2) -> f64 {
        p.distance(self.nearest(p))
    }

    /// True iff the segment meets the curve within the span: the segment is
    /// mapped into the unit-circle frame and tested against the unit circle.
    pub fn hits_segment(&self, seg: &Line) -> bool {
        let unit = |p: Vec2| {
            let l = self.local(p);
            Vec2::new(l.x / self.rx, l.y / self.ry)
        };
        let mapped = Line::new(unit(seg.p1), unit(seg.p2));
        intersect::line_circle(&mapped, &Circle::new(Vec2::default(), 1.0))
            .into_iter()
            .any(|q| self.contains_param(q.y.atan2(q.x)))
    }
}

/// Eberly's foot on `x²/e0² + y²/e1² = 1` with `e0 ≥ e1 > 0`, for a query
/// point `(y0, y1)` in the first quadrant.
fn first_quadrant_foot(e0: f64, e1: f64, y0: f64, y1: f64) -> (f64, f64) {
    if y1 > 0.0 {
        if y0 > 0.0 {
            let (z0, z1) = (y0 / e0, y1 / e1);
            let g = z0 * z0 + z1 * z1 - 1.0;
            if g == 0.0 {
                return (y0, y1);
            }
            let r0 = (e0 / e1) * (e0 / e1);
            let sbar = root(r0, z0, z1, g);
            return (r0 * y0 / (sbar + r0), y1 / (sbar + 1.0));
        }
        return (0.0, e1);
    }
    let (numer0, denom0) = (e0 * y0, e0 * e0 - e1 * e1);
    if numer0 < denom0 {
        let xde0 = numer0 / denom0;
        return (e0 * xde0, e1 * (1.0 - xde0 * xde0).max(0.0).sqrt());
    }
    (e0, 0.0)
}

/// Bisection root of `(r0·z0/(s+r0))² + (z1/(s+1))² − 1` (Eberly `GetRoot`).
fn root(r0: f64, z0: f64, z1: f64, g: f64) -> f64 {
    let n0 = r0 * z0;
    let mut s0 = z1 - 1.0;
    let mut s1 = if g < 0.0 { 0.0 } else { n0.hypot(z1) - 1.0 };
    let mut s = 0.0;
    for _ in 0..MAX_ITER {
        s = 0.5 * (s0 + s1);
        if s <= s0 || s >= s1 {
            break;
        }
        let (q0, q1) = (n0 / (s + r0), z1 / (s + 1.0));
        let g = q0 * q0 + q1 * q1 - 1.0;
        if g > 0.0 {
            s0 = s;
        } else if g < 0.0 {
            s1 = s;
        } else {
            break;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{EPSILON, EllipseSpan};
    use core::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

    fn thin() -> Ellipse {
        Ellipse::new(Vec2::new(5.0, -3.0), 20.0, 2.0, 0.4, None)
    }

    /// The foot of a point near the curve is the curve point it was pushed
    /// out from along the normal, outside and inside (within the smallest
    /// radius of curvature, 0.2 here), for every quadrant.
    #[test]
    fn nearest_on_full_ellipse_is_the_normal_foot() {
        for e in [thin(), Ellipse::new(Vec2::default(), 2.0, 20.0, -1.1, None)] {
            for k in 0..24 {
                let t = f64::from(k) * PI / 12.0 + 0.05;
                let on = e.point(t);
                let (s, c) = e.rotation.sin_cos();
                let (lx, ly) = (e.ry * t.cos(), e.rx * t.sin());
                let n = Vec2::new(lx * c - ly * s, lx * s + ly * c)
                    .normalize()
                    .expect("n");
                for d in [-0.1, 0.5] {
                    let f = e.nearest(on + n * d);
                    assert!(f.approx_eq(on, 1e-9), "{t} {d} {f:?} {on:?}");
                    assert!((e.distance_to_point(on + n * d) - d.abs()).abs() <= 1e-9);
                }
            }
        }
    }

    /// Points on the axes, and the center itself, find a minor-axis vertex
    /// (the center is `min(rx, ry)` away).
    #[test]
    fn nearest_on_axes_and_at_center() {
        let e = Ellipse::new(Vec2::default(), 4.0, 2.0, 0.0, None);
        assert!(
            e.nearest(Vec2::new(6.0, 0.0))
                .approx_eq(Vec2::new(4.0, 0.0), EPSILON)
        );
        assert!(
            e.nearest(Vec2::new(0.0, -5.0))
                .approx_eq(Vec2::new(0.0, -2.0), EPSILON)
        );
        assert!((e.distance_to_point(Vec2::default()) - 2.0).abs() <= EPSILON);
        let tall = Ellipse::new(Vec2::default(), 2.0, 4.0, 0.0, None);
        assert!((tall.distance_to_point(Vec2::default()) - 2.0).abs() <= EPSILON);
        let circle = Ellipse::new(Vec2::default(), 3.0, 3.0, 0.0, None);
        assert!((circle.distance_to_point(Vec2::new(1.0, 0.0)) - 2.0).abs() <= EPSILON);
    }

    /// On a span, a foot inside the span wins; a foot outside falls back to
    /// the nearer span end; the center of a quarter arc picks an end.
    #[test]
    fn nearest_on_span_inside_or_outside() {
        let a = Ellipse::new(
            Vec2::default(),
            4.0,
            2.0,
            0.0,
            Some(EllipseSpan::new(0.0, FRAC_PI_2, true)),
        );
        let inside = a.point(FRAC_PI_4);
        let full = Ellipse { span: None, ..a };
        assert_eq!(a.nearest(inside * 1.1), full.nearest(inside * 1.1));
        assert!(a.distance_to_point(inside) <= EPSILON);
        assert!(
            a.nearest(Vec2::new(-4.5, 0.1))
                .approx_eq(Vec2::new(0.0, 2.0), EPSILON)
        );
        assert!(
            a.nearest(Vec2::new(0.5, -3.0))
                .approx_eq(Vec2::new(4.0, 0.0), EPSILON)
        );
        assert!((a.distance_to_point(Vec2::default()) - 2.0).abs() <= EPSILON);
        let cw = Ellipse::new(
            Vec2::default(),
            4.0,
            2.0,
            0.0,
            Some(EllipseSpan::new(0.0, FRAC_PI_2, false)),
        );
        let q = Vec2::new(-4.5, 0.1);
        assert_eq!(cw.nearest(q), full.nearest(q));
        assert!(cw.nearest(q).approx_eq(Vec2::new(-4.0, 0.0), 0.1));
    }

    /// A segment crossing the curve hits; one inside, outside, or crossing
    /// only outside the span does not.
    #[test]
    fn hits_segment_respects_curve_and_span() {
        let e = Ellipse::new(Vec2::default(), 4.0, 2.0, 0.0, None);
        let seg =
            |a: (f64, f64), b: (f64, f64)| Line::new(Vec2::new(a.0, a.1), Vec2::new(b.0, b.1));
        assert!(e.hits_segment(&seg((3.0, 0.0), (5.0, 0.0))));
        assert!(e.hits_segment(&seg((-1.0, -3.0), (-1.0, 3.0))));
        assert!(!e.hits_segment(&seg((-1.0, 0.0), (1.0, 0.0))));
        assert!(!e.hits_segment(&seg((5.0, -3.0), (5.0, 3.0))));
        let a = Ellipse::new(
            Vec2::default(),
            4.0,
            2.0,
            0.0,
            Some(EllipseSpan::new(0.0, FRAC_PI_2, true)),
        );
        assert!(a.hits_segment(&seg((1.0, 0.0), (1.0, 3.0))));
        assert!(!a.hits_segment(&seg((1.0, 0.0), (1.0, -3.0))));
        let rotated = Ellipse::new(Vec2::new(10.0, 10.0), 4.0, 2.0, FRAC_PI_2, None);
        assert!(rotated.hits_segment(&seg((10.0, 13.0), (10.0, 15.0))));
        assert!(!rotated.hits_segment(&seg((13.0, 10.0), (15.0, 10.0))));
    }
}
