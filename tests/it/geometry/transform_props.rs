//! `geometry::Transform::Rotate` (LCV-158 AC6): fixed cases for a point, a
//! line, a circle and an arc, and property tests that a rotation is rigid.
//!
//! A rotation keeps line lengths, radii and arc sweeps, maps line endpoints
//! and centers like points, and lands the rotated arc's start and end points
//! on the rotated original endpoints.

use core::f64::consts::{FRAC_PI_2, PI, TAU};
use lasercad::geometry::{Arc, Circle, EPSILON, Line, Transform, Vec2};
use proptest::prelude::*;
use proptest::test_runner::FileFailurePersistence;

const CASES: u32 = 256;

/// `CASES` cases; failing seeds persist under the repo-root
/// `proptest-regressions/`, since `tests/` holds only `it/` and `harness/`.
fn config() -> ProptestConfig {
    ProptestConfig {
        cases: CASES,
        failure_persistence: Some(Box::new(FileFailurePersistence::Direct(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/proptest-regressions/geometry/transform_props.txt"
        )))),
        ..ProptestConfig::default()
    }
}

fn rotate(x: f64, y: f64, angle: f64) -> Transform {
    Transform::Rotate {
        base: Vec2::new(x, y),
        angle,
    }
}

/// A quarter turn about (1, 1) sends (3, 1) to (1, 3).
#[test]
fn rotate_point_quarter_turn_about_base() {
    let p = rotate(1.0, 1.0, FRAC_PI_2).point(Vec2::new(3.0, 1.0));
    assert!(p.approx_eq(Vec2::new(1.0, 3.0), EPSILON), "{p:?}");
}

/// The base point itself does not move.
#[test]
fn rotate_point_keeps_the_base() {
    let base = Vec2::new(-4.0, 7.5);
    let p = rotate(base.x, base.y, 1.234).point(base);
    assert!(p.approx_eq(base, EPSILON), "{p:?}");
}

/// A line maps both endpoints, in order.
#[test]
fn rotate_line_maps_both_endpoints() {
    let l = rotate(0.0, 0.0, PI).line(Line::new(Vec2::new(1.0, 0.0), Vec2::new(2.0, 3.0)));
    assert!(l.p1.approx_eq(Vec2::new(-1.0, 0.0), EPSILON), "{l:?}");
    assert!(l.p2.approx_eq(Vec2::new(-2.0, -3.0), EPSILON), "{l:?}");
}

/// A circle maps its center and keeps its radius bit-exact.
#[test]
fn rotate_circle_maps_center_keeps_radius() {
    let c = rotate(0.0, 0.0, FRAC_PI_2).circle(Circle::new(Vec2::new(5.0, 0.0), 2.5));
    assert!(c.center.approx_eq(Vec2::new(0.0, 5.0), EPSILON), "{c:?}");
    assert_eq!(c.r, 2.5);
}

/// An arc maps its center, keeps radius and orientation, and adds the angle
/// to both start and end angles.
#[test]
fn rotate_arc_adds_angle_to_start_and_end() {
    let src = Arc::new(Vec2::new(10.0, 0.0), 1.5, 0.25, 1.0, false);
    let a = rotate(0.0, 0.0, FRAC_PI_2).arc(src);
    assert!(a.center.approx_eq(Vec2::new(0.0, 10.0), EPSILON), "{a:?}");
    assert_eq!(a.r, 1.5);
    assert!(
        (a.start_angle - (0.25 + FRAC_PI_2)).abs() <= EPSILON,
        "{a:?}"
    );
    assert!((a.end_angle - (1.0 + FRAC_PI_2)).abs() <= EPSILON, "{a:?}");
    assert!(!a.ccw);
}

/// AC8 — whole turns within `EPSILON` are the identity; anything else is not.
#[test]
fn rotate_identity_is_a_whole_turn() {
    for angle in [0.0, TAU, -2.0 * TAU, 1e-12, TAU - 1e-12] {
        assert!(rotate(3.0, 4.0, angle).is_identity(), "{angle}");
    }
    for angle in [1e-6, PI, -FRAC_PI_2, TAU - 1e-6] {
        assert!(!rotate(3.0, 4.0, angle).is_identity(), "{angle}");
    }
}

fn point() -> impl Strategy<Value = Vec2> {
    (-500.0..500.0f64, -500.0..500.0f64).prop_map(|(x, y)| Vec2::new(x, y))
}

fn transform() -> impl Strategy<Value = Transform> {
    (point(), -2.0 * TAU..2.0 * TAU).prop_map(|(base, angle)| Transform::Rotate { base, angle })
}

fn arc() -> impl Strategy<Value = Arc> {
    (
        point(),
        0.01..500.0f64,
        -20.0..20.0f64,
        0.01..(TAU - 0.01),
        any::<bool>(),
    )
        .prop_map(|(center, r, start, sweep, ccw)| {
            let end = if ccw { start + sweep } else { start - sweep };
            Arc::new(center, r, start, end, ccw)
        })
}

/// Coordinates stay within ~1500 mm of the origin, so a scaled `EPSILON`
/// covers the rounding of one rotation.
const TOL: f64 = EPSILON * 1e1;

proptest! {
    #![proptest_config(config())]

    /// Distances between any two points are kept.
    #[test]
    fn rotation_keeps_distances(t in transform(), p in point(), q in point()) {
        let d = t.point(p).distance(t.point(q));
        prop_assert!((d - p.distance(q)).abs() <= TOL, "{} vs {}", d, p.distance(q));
    }

    /// Line length is kept and the endpoints map as points.
    #[test]
    fn rotation_keeps_line_length(t in transform(), p in point(), q in point()) {
        let src = Line::new(p, q);
        let l = t.line(src);
        prop_assert!((l.length() - src.length()).abs() <= TOL);
        prop_assert_eq!(l.p1, t.point(p));
        prop_assert_eq!(l.p2, t.point(q));
    }

    /// Circle radius is kept exactly and the center maps as a point.
    #[test]
    fn rotation_keeps_circle_radius(t in transform(), c in point(), r in 0.01..500.0f64) {
        let out = t.circle(Circle::new(c, r));
        prop_assert_eq!(out.r, r);
        prop_assert_eq!(out.center, t.point(c));
    }

    /// An arc keeps its radius, orientation and sweep, and its endpoints land
    /// on the rotated original endpoints.
    #[test]
    fn rotation_keeps_arc_sweep_and_moves_endpoints(t in transform(), a in arc()) {
        let out = t.arc(a);
        prop_assert_eq!(out.r, a.r);
        prop_assert_eq!(out.ccw, a.ccw);
        prop_assert_eq!(out.center, t.point(a.center));
        prop_assert!((out.sweep_angle() - a.sweep_angle()).abs() <= TOL);
        prop_assert!(out.start_point().approx_eq(t.point(a.start_point()), TOL));
        prop_assert!(out.end_point().approx_eq(t.point(a.end_point()), TOL));
    }
}
