//! `geometry::Ellipse` (LCV-176 AC4, AC6, ADR 0015 §1, §8): parametric point
//! and span ends, span sweep and containment through the unit-circle rule,
//! the exact bounding box of a rotated or partial ellipse, the quadrant
//! vertices inside a span, and a polyline whose vertices lie on the curve with
//! a chord deviation within the requested tolerance.

use core::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};
use lasercad::geometry::{EPSILON, Ellipse, EllipseSpan, Vec2};
use proptest::prelude::*;
use proptest::test_runner::FileFailurePersistence;

const CASES: u32 = 128;

/// `CASES` cases; failing seeds persist under the repo-root
/// `proptest-regressions/`, since `tests/` holds only `it/` and `harness/`.
fn config() -> ProptestConfig {
    ProptestConfig {
        cases: CASES,
        failure_persistence: Some(Box::new(FileFailurePersistence::Direct(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/proptest-regressions/geometry/ellipse_props.txt"
        )))),
        ..ProptestConfig::default()
    }
}

fn deg(d: f64) -> f64 {
    d * PI / 180.0
}

fn span(start: f64, end: f64, ccw: bool) -> Option<EllipseSpan> {
    Some(EllipseSpan::new(start, end, ccw))
}

/// `point(t) = center + R(rotation)·(rx·cos t, ry·sin t)`; the span ends are
/// the points at its start and end parameters; a full ellipse has none.
#[test]
fn point_and_span_ends() {
    let e = Ellipse::new(Vec2::new(10.0, 5.0), 4.0, 2.0, FRAC_PI_2, None);
    assert!(e.point(0.0).approx_eq(Vec2::new(10.0, 9.0), EPSILON));
    assert!(e.point(FRAC_PI_2).approx_eq(Vec2::new(8.0, 5.0), EPSILON));
    assert!(e.start_point().is_none() && e.end_point().is_none());
    let a = Ellipse::new(
        Vec2::new(0.0, 0.0),
        4.0,
        2.0,
        0.0,
        span(0.0, FRAC_PI_2, true),
    );
    let s = a.start_point().expect("arc has a start");
    let t = a.end_point().expect("arc has an end");
    assert!(s.approx_eq(Vec2::new(4.0, 0.0), EPSILON), "{s:?}");
    assert!(t.approx_eq(Vec2::new(0.0, 2.0), EPSILON), "{t:?}");
}

/// Sweep and containment follow `Arc`'s wrap-around rule in the parameter,
/// for both directions; a full ellipse sweeps a whole turn and contains all.
#[test]
fn sweep_and_containment_wrap_cw_and_ccw() {
    let full = Ellipse::new(Vec2::default(), 3.0, 1.0, 0.3, None);
    assert!((full.sweep() - TAU).abs() <= EPSILON);
    assert!(full.contains_param(1.234) && full.contains_param(-5.0));

    let ccw = Ellipse::new(
        Vec2::default(),
        3.0,
        1.0,
        0.0,
        span(deg(350.0), deg(10.0), true),
    );
    assert!((ccw.sweep() - deg(20.0)).abs() <= EPSILON);
    assert!(ccw.contains_param(0.0) && ccw.contains_param(deg(5.0)));
    assert!(!ccw.contains_param(PI));

    let cw = Ellipse::new(
        Vec2::default(),
        3.0,
        1.0,
        0.0,
        span(deg(10.0), deg(350.0), false),
    );
    assert!((cw.sweep() - deg(20.0)).abs() <= EPSILON);
    assert!(cw.contains_param(0.0) && !cw.contains_param(PI));

    let big = Ellipse::new(Vec2::default(), 3.0, 1.0, 0.0, span(0.0, FRAC_PI_2, false));
    assert!((big.sweep() - 3.0 * FRAC_PI_2).abs() <= EPSILON);
    assert!(big.contains_param(PI) && !big.contains_param(FRAC_PI_4));
}

/// A full ellipse rotated by 45° has the half-width `sqrt((rx² + ry²) / 2)`
/// on both axes; an axis-aligned one has its radii.
#[test]
fn bbox_of_full_ellipse_rotated_and_aligned() {
    let e = Ellipse::new(Vec2::new(1.0, 2.0), 4.0, 2.0, FRAC_PI_4, None);
    let h = ((16.0 + 4.0) / 2.0f64).sqrt();
    let (min, max) = e.bbox();
    assert!(min.approx_eq(Vec2::new(1.0 - h, 2.0 - h), 1e-9), "{min:?}");
    assert!(max.approx_eq(Vec2::new(1.0 + h, 2.0 + h), 1e-9), "{max:?}");
    let (min, max) = Ellipse::new(Vec2::default(), 4.0, 2.0, 0.0, None).bbox();
    assert!(min.approx_eq(Vec2::new(-4.0, -2.0), EPSILON));
    assert!(max.approx_eq(Vec2::new(4.0, 2.0), EPSILON));
}

/// A partial span's box holds its ends and only the extremes it passes.
#[test]
fn bbox_of_partial_span_is_tight() {
    let a = Ellipse::new(Vec2::default(), 4.0, 2.0, 0.0, span(0.0, FRAC_PI_2, true));
    let (min, max) = a.bbox();
    assert!(min.approx_eq(Vec2::new(0.0, 0.0), EPSILON), "{min:?}");
    assert!(max.approx_eq(Vec2::new(4.0, 2.0), EPSILON), "{max:?}");
    let a = Ellipse::new(
        Vec2::default(),
        4.0,
        2.0,
        0.0,
        span(deg(10.0), deg(20.0), true),
    );
    let (min, max) = a.bbox();
    assert!(min.approx_eq(a.point(deg(20.0)).min_with(a.point(deg(10.0))), EPSILON));
    assert!(max.approx_eq(a.point(deg(10.0)).max_with(a.point(deg(20.0))), EPSILON));
}

/// Quadrants are the axis vertices at `t ∈ {0, π/2, π, 3π/2}` inside the span.
#[test]
fn quadrants_inside_the_span_only() {
    let full = Ellipse::new(Vec2::new(1.0, 1.0), 4.0, 2.0, FRAC_PI_2, None);
    let want = [(1.0, 5.0), (-1.0, 1.0), (1.0, -3.0), (3.0, 1.0)];
    assert_eq!(full.quadrants().len(), 4);
    for (q, (x, y)) in full.quadrants().into_iter().zip(want) {
        assert!(q.approx_eq(Vec2::new(x, y), EPSILON), "{q:?} vs ({x}, {y})");
    }
    let a = Ellipse::new(
        Vec2::default(),
        4.0,
        2.0,
        0.0,
        span(-0.1, FRAC_PI_2 + 0.1, true),
    );
    let q = a.quadrants();
    assert_eq!(q.len(), 2, "{q:?}");
    assert!(q[0].approx_eq(Vec2::new(4.0, 0.0), EPSILON));
    assert!(q[1].approx_eq(Vec2::new(0.0, 2.0), EPSILON));
    let cw = Ellipse::new(
        Vec2::default(),
        4.0,
        2.0,
        0.0,
        span(-0.1, FRAC_PI_2 + 0.1, false),
    );
    assert_eq!(cw.quadrants().len(), 2, "{:?}", cw.quadrants());
}

/// The fewest segments that keep the chord deviation within tolerance:
/// `10·(1 − cos(Δt/2)) ≤ 0.01` needs `Δt ≤ 0.08945`, so 71 segments.
#[test]
fn polyline_uses_the_fewest_segments_within_tolerance() {
    let e = Ellipse::new(Vec2::default(), 10.0, 5.0, 0.0, None);
    assert_eq!(e.polyline(0.01).len(), 72);
}

trait MinMax {
    fn min_with(self, o: Vec2) -> Vec2;
    fn max_with(self, o: Vec2) -> Vec2;
}

impl MinMax for Vec2 {
    fn min_with(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x.min(o.x), self.y.min(o.y))
    }
    fn max_with(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x.max(o.x), self.y.max(o.y))
    }
}

fn dist_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let d = b - a;
    let len2 = d.length_squared();
    if len2 == 0.0 {
        return p.distance(a);
    }
    let t = ((p - a).dot(d) / len2).clamp(0.0, 1.0);
    p.distance(a + d * t)
}

fn ellipse_case() -> impl Strategy<Value = Ellipse> {
    (
        (-200.0..200.0f64, -200.0..200.0f64),
        0.5..300.0f64,
        0.5..300.0f64,
        -10.0..10.0f64,
        prop::option::of((-10.0..10.0f64, 0.05..6.2f64, any::<bool>())),
    )
        .prop_map(|((cx, cy), rx, ry, rot, sp)| {
            let span =
                sp.map(|(s, sw, ccw)| EllipseSpan::new(s, if ccw { s + sw } else { s - sw }, ccw));
            Ellipse::new(Vec2::new(cx, cy), rx, ry, rot, span)
        })
}

proptest! {
    #![proptest_config(config())]

    /// Every vertex is on the curve; every sampled curve point lies within
    /// `tol` of the polyline; the polyline runs from the start to the end.
    #[test]
    fn polyline_on_curve_within_tolerance(e in ellipse_case(), tol in 0.001..2.0f64) {
        let pts = e.polyline(tol);
        prop_assert!(pts.len() >= 9 && pts.len() <= 4097, "{}", pts.len());
        let scale = e.rx.max(e.ry) + e.center.length();
        for v in &pts {
            let l = *v - e.center;
            let (s, c) = (-e.rotation).sin_cos();
            let local = Vec2::new(l.x * c - l.y * s, l.x * s + l.y * c);
            let t = (local.y / e.ry).atan2(local.x / e.rx);
            prop_assert!(e.point(t).distance(*v) <= 1e-12 * scale.max(1.0), "{v:?}");
        }
        let first = e.start_point().unwrap_or(pts[0]);
        prop_assert!(pts[0].approx_eq(first, 1e-9 * scale));
        if let Some(end) = e.end_point() {
            prop_assert!(pts[pts.len() - 1].approx_eq(end, 1e-9 * scale));
        }
        let span = e.span.unwrap_or(EllipseSpan::new(0.0, TAU, true));
        let dir = if span.ccw { 1.0 } else { -1.0 };
        for i in 0..=400 {
            let p = e.point(span.start + dir * e.sweep() * f64::from(i) / 400.0);
            let d = pts.windows(2).map(|w| dist_to_segment(p, w[0], w[1])).fold(f64::INFINITY, f64::min);
            prop_assert!(d <= tol * (1.0 + 1e-9) + 1e-9 * scale, "deviation {d} > {tol}");
        }
    }

    /// Every point of the span lies inside the bbox, and the box is attained.
    #[test]
    fn bbox_holds_every_point(e in ellipse_case()) {
        let (min, max) = e.bbox();
        let span = e.span.unwrap_or(EllipseSpan::new(0.0, TAU, true));
        let dir = if span.ccw { 1.0 } else { -1.0 };
        let slack = 1e-9 * (e.rx.max(e.ry) + e.center.length());
        let mut lo = Vec2::new(f64::INFINITY, f64::INFINITY);
        let mut hi = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for i in 0..=2000 {
            let p = e.point(span.start + dir * e.sweep() * f64::from(i) / 2000.0);
            prop_assert!(p.x >= min.x - slack && p.x <= max.x + slack, "{p:?} {min:?} {max:?}");
            prop_assert!(p.y >= min.y - slack && p.y <= max.y + slack, "{p:?} {min:?} {max:?}");
            lo = lo.min_with(p);
            hi = hi.max_with(p);
        }
        let gap = 1e-4 * e.rx.max(e.ry);
        prop_assert!(lo.x - min.x <= gap && lo.y - min.y <= gap && max.x - hi.x <= gap && max.y - hi.y <= gap);
    }
}
