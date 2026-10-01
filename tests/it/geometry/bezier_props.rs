//! `geometry::Bezier` (LCV-177 AC5, AC6, ADR 0016 §1, §4): Bernstein point
//! and ends for both degrees, `map` over every control point, the tight
//! bounding box from the derivative roots (never the control polygon), and a
//! polyline whose vertices lie on the curve with a chord deviation within the
//! requested tolerance (Wang's bound).

use lasercad::geometry::{Bezier, EPSILON, Vec2};
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
            "/proptest-regressions/geometry/bezier_props.txt"
        )))),
        ..ProptestConfig::default()
    }
}

fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

fn arch() -> Bezier {
    Bezier::Cubic([v(0.0, 0.0), v(0.0, 40.0), v(10.0, 40.0), v(10.0, 0.0)])
}

fn hump() -> Bezier {
    Bezier::Quadratic([v(0.0, 0.0), v(5.0, 20.0), v(10.0, 0.0)])
}

/// Tight box by dense sampling: `(min, max)` of 10 001 curve points.
fn sampled_box(b: &Bezier) -> (Vec2, Vec2) {
    let mut lo = v(f64::INFINITY, f64::INFINITY);
    let mut hi = v(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for i in 0..=10_000 {
        let p = b.point(f64::from(i) / 10_000.0);
        lo = v(lo.x.min(p.x), lo.y.min(p.y));
        hi = v(hi.x.max(p.x), hi.y.max(p.y));
    }
    (lo, hi)
}

/// `point(t)` is the Bernstein form; the ends are the first and last control
/// points, exactly; `points()` lists the controls in order.
#[test]
fn point_and_ends_for_both_degrees() {
    let c = arch();
    assert!(c.point(0.5).approx_eq(v(5.0, 30.0), EPSILON));
    assert!(c.point(0.25).approx_eq(v(1.5625, 22.5), EPSILON));
    assert_eq!(c.point(0.0), c.start());
    assert_eq!(c.point(1.0), c.end());
    assert_eq!((c.start(), c.end()), (v(0.0, 0.0), v(10.0, 0.0)));
    assert_eq!(c.points().len(), 4);
    let q = hump();
    assert!(q.point(0.5).approx_eq(v(5.0, 10.0), EPSILON));
    assert!(q.point(0.25).approx_eq(v(2.5, 7.5), EPSILON));
    assert_eq!((q.start(), q.end()), (v(0.0, 0.0), v(10.0, 0.0)));
    assert_eq!(q.points(), &[v(0.0, 0.0), v(5.0, 20.0), v(10.0, 0.0)]);
}

/// `map` sends every control point through `f` and keeps the degree.
#[test]
fn map_keeps_degree_and_maps_every_point() {
    let shift = |p: Vec2| v(2.0 * p.x + 1.0, p.y - 3.0);
    let c = arch().map(shift);
    assert_eq!(
        c,
        Bezier::Cubic([v(1.0, -3.0), v(1.0, 37.0), v(21.0, 37.0), v(21.0, -3.0)])
    );
    let q = hump().map(shift);
    assert_eq!(
        q,
        Bezier::Quadratic([v(1.0, -3.0), v(11.0, 17.0), v(21.0, -3.0)])
    );
}

/// The box holds the curve, not its control polygon: the arch peaks at 30
/// (its controls reach 40), the hump at 10 (controls reach 20).
#[test]
fn bbox_is_the_curve_not_the_control_polygon() {
    let (min, max) = arch().bbox();
    assert!(min.approx_eq(v(0.0, 0.0), EPSILON), "{min:?}");
    assert!(max.approx_eq(v(10.0, 30.0), EPSILON), "{max:?}");
    let (min, max) = hump().bbox();
    assert!(min.approx_eq(v(0.0, 0.0), EPSILON), "{min:?}");
    assert!(max.approx_eq(v(10.0, 10.0), EPSILON), "{max:?}");
}

/// An S-shaped cubic overshoots its ends on X at two interior parameters;
/// the box reaches both extrema.
#[test]
fn bbox_reaches_interior_axis_extrema() {
    let s = Bezier::Cubic([v(0.0, 0.0), v(-10.0, 10.0), v(20.0, 10.0), v(10.0, 0.0)]);
    let (min, max) = s.bbox();
    let (lo, hi) = sampled_box(&s);
    assert!(min.x < -1.0 && max.x > 11.0, "{min:?} {max:?}");
    assert!(
        min.approx_eq(lo, 1e-5) && max.approx_eq(hi, 1e-5),
        "{min:?} {lo:?}"
    );
}

/// X's derivative has a leading coefficient of (almost) zero: the root at
/// `t = 1/3` must still be found, giving `max.x = 0.5`, not the end's `0`.
#[test]
fn bbox_with_a_near_zero_leading_coefficient() {
    let b = Bezier::Cubic([v(0.0, 0.0), v(1.0, 1.0), v(0.5, 2.0), v(-1.5 + 1e-13, 3.0)]);
    let (min, max) = b.bbox();
    assert!((max.x - 0.5).abs() <= 1e-9, "{max:?}");
    assert!((min.x + 1.5).abs() <= 1e-9, "{min:?}");
    assert!((min.y, max.y) == (0.0, 3.0), "{min:?} {max:?}");
}

/// Wang's count `⌈√(d(d−1)·max‖Pᵢ − 2Pᵢ₊₁ + Pᵢ₊₂‖ / (8·tol))⌉`: the hump
/// needs `⌈√(2·40 / 0.08)⌉ = 32` segments, the arch `⌈√(6·√1700 / 0.8)⌉ = 18`.
#[test]
fn polyline_segment_count_is_wangs_bound() {
    assert_eq!(hump().polyline(0.01).len(), 33);
    assert_eq!(arch().polyline(0.1).len(), 19);
}

/// Evenly spaced collinear controls are a straight curve: one segment, from
/// the start to the end exactly. Tolerance zero is clamped, not infinite.
#[test]
fn straight_curve_is_one_segment() {
    let b = Bezier::Cubic([v(0.0, 0.0), v(1.0, 1.0), v(2.0, 2.0), v(3.0, 3.0)]);
    assert_eq!(b.polyline(0.01), vec![v(0.0, 0.0), v(3.0, 3.0)]);
    let q = Bezier::Quadratic([v(0.0, 0.0), v(1.0, 2.0), v(2.0, 4.0)]);
    assert_eq!(q.polyline(0.0), vec![v(0.0, 0.0), v(2.0, 4.0)]);
    assert_eq!(arch().polyline(0.0).len(), 4097);
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

fn pt() -> impl Strategy<Value = Vec2> {
    (-200.0..200.0f64, -200.0..200.0f64).prop_map(|(x, y)| v(x, y))
}

fn bezier_case() -> impl Strategy<Value = Bezier> {
    prop_oneof![
        (pt(), pt(), pt()).prop_map(|(a, b, c)| Bezier::Quadratic([a, b, c])),
        (pt(), pt(), pt(), pt()).prop_map(|(a, b, c, d)| Bezier::Cubic([a, b, c, d])),
    ]
}

proptest! {
    #![proptest_config(config())]

    /// Vertex `i` is `point(i/n)`; the ends are exact; every sampled curve
    /// point lies within `tol` of the polyline.
    #[test]
    fn polyline_on_curve_within_tolerance(b in bezier_case(), tol in 0.01..2.0f64) {
        let pts = b.polyline(tol);
        prop_assert!(pts.len() >= 2 && pts.len() <= 4097, "{}", pts.len());
        let n = (pts.len() - 1) as f64;
        for (i, p) in pts.iter().enumerate() {
            prop_assert!(p.approx_eq(b.point(i as f64 / n), 1e-12 * 400.0), "{i} {p:?}");
        }
        prop_assert_eq!(pts[0], b.start());
        prop_assert_eq!(pts[pts.len() - 1], b.end());
        for i in 0..=1000 {
            let p = b.point(f64::from(i) / 1000.0);
            let d = pts.windows(2).map(|w| dist_to_segment(p, w[0], w[1])).fold(f64::INFINITY, f64::min);
            prop_assert!(d <= tol * (1.0 + 1e-9) + 1e-9, "deviation {d} > {tol}");
        }
    }

    /// Every curve point lies inside the box, and the box is attained.
    #[test]
    fn bbox_holds_every_point(b in bezier_case()) {
        let (min, max) = b.bbox();
        let (lo, hi) = sampled_box(&b);
        let slack = 1e-9;
        prop_assert!(lo.x >= min.x - slack && lo.y >= min.y - slack, "{lo:?} {min:?}");
        prop_assert!(hi.x <= max.x + slack && hi.y <= max.y + slack, "{hi:?} {max:?}");
        prop_assert!(lo.approx_eq(min, 1e-3) && hi.approx_eq(max, 1e-3), "{min:?} {lo:?} {max:?} {hi:?}");
    }
}
