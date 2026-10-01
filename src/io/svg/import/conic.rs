//! Ellipses and elliptical arcs on import (LCV-176, ADR 0015 §3).
//!
//! [`center_arc`] turns an SVG `A` segment into its centre form in user
//! space (SVG 2 §F.6.5, radii corrected per §F.6.6) as a conjugate pair.
//! [`conic_entity`] maps any such pair through the CTM and the Y mirror and
//! builds the entity with [`Ellipse::from_conjugate`]; a result whose radii
//! differ by at most [`EPSILON`] collapses to a [`Circle`] or [`Arc`].
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use super::to_world;
use crate::document::entity::Entity;
use crate::geometry::{Arc, Circle, EPSILON, Ellipse, EllipseSpan, Vec2};
use crate::io::svg::viewport::Ctx;

/// An ellipse in conjugate form, `center + u·cos t + v·sin t`, with an
/// optional span in that parameter `t`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Conic {
    /// The centre.
    pub(super) center: Vec2,
    /// The semi-diameter at `t = 0`.
    pub(super) u: Vec2,
    /// The semi-diameter at `t = π/2`.
    pub(super) v: Vec2,
    /// The span; `None` is the whole curve.
    pub(super) span: Option<EllipseSpan>,
}

/// The arc `A rx ry phi large sweep` from `from` to `to`, in user space.
/// `None` when the endpoints coincide or a radius is zero (the caller draws
/// a line); negative radii count as positive, radii too small for the chord
/// scale up together (SVG 2 §F.6.6).
pub(super) fn center_arc(
    from: Vec2,
    to: Vec2,
    radii: (f64, f64),
    phi_deg: f64,
    large: bool,
    sweep: bool,
) -> Option<Conic> {
    let (mut rx, mut ry) = (radii.0.abs(), radii.1.abs());
    if from.distance(to) < EPSILON || rx < EPSILON || ry < EPSILON {
        return None;
    }
    let (s, c) = phi_deg.to_radians().sin_cos();
    let h = (from - to) * 0.5;
    let (x1, y1) = (c * h.x + s * h.y, -s * h.x + c * h.y);
    let lambda = (x1 / rx).powi(2) + (y1 / ry).powi(2);
    if lambda > 1.0 {
        rx *= lambda.sqrt();
        ry *= lambda.sqrt();
    }
    let (rx2, ry2) = (rx * rx, ry * ry);
    let den = rx2 * y1 * y1 + ry2 * x1 * x1;
    let sq = ((rx2 * ry2 - den) / den).max(0.0).sqrt();
    let coef = if large == sweep { -sq } else { sq };
    let (cx1, cy1) = (coef * rx * y1 / ry, -coef * ry * x1 / rx);
    let mid = (from + to) * 0.5;
    let center = mid + Vec2::new(c * cx1 - s * cy1, s * cx1 + c * cy1);
    let start = ((y1 - cy1) / ry).atan2((x1 - cx1) / rx);
    let end = ((-y1 - cy1) / ry).atan2((-x1 - cx1) / rx);
    Some(Conic {
        center,
        u: Vec2::new(rx * c, rx * s),
        v: Vec2::new(-ry * s, ry * c),
        span: Some(EllipseSpan::new(start, end, sweep)),
    })
}

/// `conic` (user space) as a world entity: mapped through `ctx.ctm`,
/// un-mirrored around `bed_h`, in principal form. `None` when the result is
/// degenerate or not finite.
pub(super) fn conic_entity(ctx: &Ctx, conic: Conic, bed_h: f64) -> Option<Entity> {
    let center = to_world(ctx, conic.center, bed_h);
    let m = &ctx.ctm;
    // The linear part of the CTM, then the Y mirror.
    let map = |w: Vec2| Vec2::new(m.a * w.x + m.c * w.y, -(m.b * w.x + m.d * w.y));
    let e = Ellipse::from_conjugate(center, map(conic.u), map(conic.v), conic.span);
    let finite = [e.center.x, e.center.y, e.rx, e.ry, e.rotation]
        .iter()
        .all(|x| x.is_finite());
    if !finite || e.rx < EPSILON || e.ry < EPSILON {
        return None;
    }
    if (e.rx - e.ry).abs() > EPSILON {
        return Some(Entity::Ellipse(e));
    }
    // Circular: the polar angle is the parameter plus the rotation.
    Some(match e.span {
        None => Entity::Circle(Circle::new(e.center, e.rx)),
        Some(s) => Entity::Arc(Arc::new(
            e.center,
            e.rx,
            s.start + e.rotation,
            s.end + e.rotation,
            s.ccw,
        )),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::svg::matrix::Matrix;

    fn ctx(ctm: Matrix) -> Ctx {
        Ctx {
            ctm,
            viewport: [100.0, 100.0],
        }
    }

    /// The centre form passes through both endpoints and puts the centre on
    /// the side the flags choose.
    #[test]
    fn center_arc_meets_both_endpoints() {
        let (a, b) = (Vec2::new(50.0, 100.0), Vec2::new(90.0, 120.0));
        for (large, sweep) in [(false, false), (false, true), (true, false), (true, true)] {
            let k = center_arc(a, b, (40.0, -20.0), 30.0, large, sweep).expect("arc");
            let span = k.span.expect("span");
            let at = |t: f64| k.center + k.u * t.cos() + k.v * t.sin();
            assert!(at(span.start).approx_eq(a, 1e-9), "{large} {sweep}");
            assert!(at(span.end).approx_eq(b, 1e-9), "{large} {sweep}");
            assert_eq!(span.ccw, sweep);
            assert!((k.u.length() - 40.0).abs() < 1e-9 && (k.v.length() - 20.0).abs() < 1e-9);
        }
        assert_eq!(center_arc(a, a, (4.0, 2.0), 0.0, false, true), None);
        assert_eq!(center_arc(a, b, (0.0, 2.0), 0.0, false, true), None);
        assert_eq!(center_arc(a, b, (4.0, 1e-12), 0.0, false, true), None);
    }

    /// Short radii scale up by `sqrt(λ)`; the centre is the chord midpoint.
    #[test]
    fn center_arc_corrects_short_radii() {
        let (a, b) = (Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0));
        let k = center_arc(a, b, (4.0, 2.0), 0.0, false, true).expect("arc");
        assert!(k.center.approx_eq(Vec2::new(50.0, 0.0), 1e-9));
        assert!((k.u.length() - 50.0).abs() < 1e-9 && (k.v.length() - 25.0).abs() < 1e-9);
        // A vertical chord: λ = (0/4)² + (50/2)², so both radii scale by 25.
        let up = center_arc(a, Vec2::new(0.0, 100.0), (4.0, 2.0), 0.0, false, true);
        let k = up.expect("arc");
        assert!(k.center.approx_eq(Vec2::new(0.0, 50.0), 1e-9));
        assert!((k.u.length() - 100.0).abs() < 1e-9 && (k.v.length() - 50.0).abs() < 1e-9);
    }

    /// The identity CTM mirrors Y only; equal radii collapse to a circle or
    /// an arc with polar angles; non-finite or degenerate maps yield nothing.
    #[test]
    fn conic_entity_maps_mirrors_and_collapses() {
        let id = ctx(Matrix::IDENTITY);
        let k = Conic {
            center: Vec2::new(10.0, 20.0),
            u: Vec2::new(4.0, 0.0),
            v: Vec2::new(0.0, 2.0),
            span: None,
        };
        match conic_entity(&id, k, 100.0) {
            Some(Entity::Ellipse(e)) => {
                assert!(e.center.approx_eq(Vec2::new(10.0, 80.0), 1e-12));
                assert!((e.rx - 4.0).abs() < 1e-12 && (e.ry - 2.0).abs() < 1e-12);
            }
            other => panic!("{other:?}"),
        }
        let circle = Conic {
            v: Vec2::new(0.0, 4.0),
            ..k
        };
        assert!(matches!(
            conic_entity(&id, circle, 100.0),
            Some(Entity::Circle(c)) if (c.r - 4.0).abs() < 1e-12
        ));
        let arc = Conic {
            span: Some(EllipseSpan::new(0.0, 1.0, true)),
            ..circle
        };
        let Some(Entity::Arc(a)) = conic_entity(&id, arc, 100.0) else {
            panic!("an arc");
        };
        // Mirrored: the arc runs from angle 0 to −1, clockwise.
        assert!(a.start_point().approx_eq(Vec2::new(14.0, 80.0), 1e-12));
        let end = Vec2::new(10.0 + 4.0 * 1f64.cos(), 80.0 - 4.0 * 1f64.sin());
        assert!(a.end_point().approx_eq(end, 1e-12) && !a.ccw, "{a:?}");
        let flat = ctx(Matrix::scale(1.0, 0.0));
        assert_eq!(conic_entity(&flat, k, 100.0), None);
        let nan = Conic {
            center: Vec2::new(f64::NAN, 0.0),
            ..k
        };
        assert_eq!(conic_entity(&id, nan, 100.0), None);
        let thin = Conic {
            u: Vec2::new(1e-12, 0.0),
            ..k
        };
        assert_eq!(conic_entity(&id, thin, 100.0), None);
    }

    /// A turned circular conic collapses to an arc whose polar angles are
    /// its parameters plus the rotation: the arc ends where the conic does.
    #[test]
    fn a_turned_circular_arc_keeps_its_ends() {
        let turned = Conic {
            center: Vec2::new(10.0, 20.0),
            u: Vec2::new(0.0, 4.0),
            v: Vec2::new(-4.0, 0.0),
            span: Some(EllipseSpan::new(0.3, 1.2, true)),
        };
        let Some(Entity::Arc(a)) = conic_entity(&ctx(Matrix::IDENTITY), turned, 100.0) else {
            panic!("an arc");
        };
        let world = |t: f64| {
            let p = turned.center + turned.u * t.cos() + turned.v * t.sin();
            Vec2::new(p.x, 100.0 - p.y)
        };
        assert!(a.start_point().approx_eq(world(0.3), 1e-12), "{a:?}");
        assert!(a.end_point().approx_eq(world(1.2), 1e-12), "{a:?}");
    }
}
