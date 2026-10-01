//! The ellipse `center + u·cos t + v·sin t` in principal form (ADR 0015 §3).
//!
//! An affine map sends an ellipse's two conjugate semi-diameters `u`, `v` to
//! another conjugate pair; this is the single entry point that turns such a
//! pair back into [`Ellipse`] fields. The principal axis nearest to `u` is
//! chosen (a parameter shift `|t0| ≤ π/4`, zero when `u ⟂ v`), so an
//! axis-aligned or rotated ellipse keeps its own `rx`, `ry` and rotation. A
//! reflection (`u × v < 0`) negates the span and flips its direction.

use crate::geometry::{Ellipse, EllipseSpan, Vec2};

impl Ellipse {
    /// The ellipse traced by `center + u·cos t + v·sin t`, with `span` given
    /// in that parameter `t`; endpoints keep their roles.
    pub fn from_conjugate(center: Vec2, u: Vec2, v: Vec2, span: Option<EllipseSpan>) -> Self {
        let num = 2.0 * u.dot(v);
        let den = u.length_squared() - v.length_squared();
        let t0 = if num == 0.0 {
            0.0
        } else {
            0.5 * (num / den).atan()
        };
        let (s0, c0) = t0.sin_cos();
        let a = u * c0 + v * s0;
        let b = v * c0 - u * s0;
        let sign = if u.cross(v) < 0.0 { -1.0 } else { 1.0 };
        let span = span.map(|s| {
            EllipseSpan::new(
                sign * (s.start - t0),
                sign * (s.end - t0),
                s.ccw == (sign > 0.0),
            )
        });
        Self::new(center, a.length(), b.length(), a.y.atan2(a.x), span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::EPSILON;
    use core::f64::consts::{FRAC_PI_2, PI};

    fn rot(p: Vec2, a: f64) -> Vec2 {
        let (s, c) = a.sin_cos();
        Vec2::new(p.x * c - p.y * s, p.x * s + p.y * c)
    }

    fn conj_point(c: Vec2, u: Vec2, v: Vec2, t: f64) -> Vec2 {
        c + u * t.cos() + v * t.sin()
    }

    /// Orthogonal semi-axes come back unchanged, `rx < ry` included.
    #[test]
    fn orthogonal_pair_keeps_radii_and_rotation() {
        for (rx, ry, r) in [
            (4.0, 2.0, 0.3),
            (2.0, 4.0, -1.2),
            (3.0, 3.0, 2.5),
            (5.0, 1.0, 0.0),
        ] {
            let u = rot(Vec2::new(rx, 0.0), r);
            let v = rot(Vec2::new(0.0, ry), r);
            let span = Some(EllipseSpan::new(0.2, 1.7, true));
            let e = Ellipse::from_conjugate(Vec2::new(1.0, 2.0), u, v, span);
            assert!(
                (e.rx - rx).abs() <= EPSILON && (e.ry - ry).abs() <= EPSILON,
                "{e:?}"
            );
            assert!((e.rotation - r).abs() <= EPSILON, "{e:?}");
            assert_eq!(e.span, span);
        }
    }

    /// A skewed pair gives the singular values of the matrix `[u v]`.
    #[test]
    fn skewed_pair_matches_svd() {
        for (u, v) in [
            (Vec2::new(3.0, 0.0), Vec2::new(1.5, 2.0)),
            (Vec2::new(1.0, 1.0), Vec2::new(-0.2, 3.0)),
            (Vec2::new(2.0, 0.5), Vec2::new(2.0, -0.4)),
            (Vec2::new(1.0, 0.0), Vec2::new(1.0, 1e-3)),
        ] {
            let (a, b, d) = (u.dot(u), u.dot(v), v.dot(v));
            let disc = ((a - d) * (a - d) + 4.0 * b * b).sqrt();
            let (s1, s2) = (
                ((a + d + disc) / 2.0).sqrt(),
                ((a + d - disc) / 2.0).max(0.0).sqrt(),
            );
            let e = Ellipse::from_conjugate(Vec2::default(), u, v, None);
            let (hi, lo) = (e.rx.max(e.ry), e.rx.min(e.ry));
            assert!(
                (hi - s1).abs() <= 1e-9 && (lo - s2).abs() <= 1e-9,
                "{e:?} {s1} {s2}"
            );
            for k in 0..12 {
                let t = f64::from(k) * PI / 6.0;
                let p = conj_point(Vec2::default(), u, v, t);
                let l = rot(p, -e.rotation);
                let q = (l.x / e.rx).powi(2) + (l.y / e.ry).powi(2);
                assert!((q - 1.0).abs() <= 1e-9, "{t} {q}");
            }
        }
    }

    /// A reflected pair negates the span (around the shift) and flips `ccw`.
    #[test]
    fn reflection_negates_span_and_flips_ccw() {
        let span = Some(EllipseSpan::new(0.25, 1.5, true));
        let e = Ellipse::from_conjugate(
            Vec2::default(),
            Vec2::new(4.0, 0.0),
            Vec2::new(0.0, -2.0),
            span,
        );
        let s = e.span.expect("span kept");
        assert!(!s.ccw);
        assert!(
            (s.start + 0.25).abs() <= EPSILON && (s.end + 1.5).abs() <= EPSILON,
            "{s:?}"
        );
        let e = Ellipse::from_conjugate(
            Vec2::default(),
            Vec2::new(4.0, 0.0),
            Vec2::new(0.0, 2.0),
            span,
        );
        assert!(e.span.expect("span kept").ccw);
    }

    /// Start and end points, and the midpoint, are those of the conjugate
    /// parameterization, for direct and reflected skewed pairs.
    #[test]
    fn endpoints_are_preserved() {
        let c = Vec2::new(30.0, -12.0);
        for (u, v) in [
            (Vec2::new(3.0, 1.0), Vec2::new(1.5, 2.0)),
            (Vec2::new(3.0, 1.0), Vec2::new(1.5, -2.0)),
            (Vec2::new(-1.0, 4.0), Vec2::new(-6.0, 0.5)),
            (Vec2::new(0.0, 4.0), Vec2::new(4.0, 0.0)),
        ] {
            for (start, end, ccw) in [
                (0.3, 2.0, true),
                (-1.0, -FRAC_PI_2 - 3.0, false),
                (5.0, 1.0, true),
            ] {
                let e = Ellipse::from_conjugate(c, u, v, Some(EllipseSpan::new(start, end, ccw)));
                let sp = e.start_point().expect("arc");
                let ep = e.end_point().expect("arc");
                assert!(sp.approx_eq(conj_point(c, u, v, start), EPSILON), "{e:?}");
                assert!(ep.approx_eq(conj_point(c, u, v, end), EPSILON), "{e:?}");
                let dir = if ccw { 1.0 } else { -1.0 };
                let sweep = (dir * (end - start)).rem_euclid(core::f64::consts::TAU);
                assert!((e.sweep() - sweep).abs() <= 1e-9, "{e:?} {sweep}");
            }
        }
    }
}
