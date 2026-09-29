//! Property tests for the intersection kernel (`geometry::intersect`).
//!
//! Every point returned by `line_line`, `line_line_infinite`, `line_circle` and
//! `circle_circle` lies on both inputs, and swapping the inputs returns the same
//! set. The kernel has no arc intersection routine (arcs are skipped by the
//! snap engine), so arcs are covered by `arc_props.rs` instead.
//!
//! Tolerance: [`EPSILON`] mm, widened only where the kernel's contract widens
//! it — segment membership has `EPSILON` *parametric* slack at each end, which
//! is `EPSILON · length` in millimetres.

use lasercad::geometry::{
    Circle, EPSILON, Line, Vec2, circle_circle, line_circle, line_line, line_line_infinite,
};
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
            "/proptest-regressions/geometry/intersect_props.txt"
        )))),
        ..ProptestConfig::default()
    }
}

fn coord() -> impl Strategy<Value = f64> {
    -500.0..500.0f64
}

fn point() -> impl Strategy<Value = Vec2> {
    (coord(), coord()).prop_map(|(x, y)| Vec2::new(x, y))
}

fn segment() -> impl Strategy<Value = Line> {
    (point(), point()).prop_map(|(a, b)| Line::new(a, b))
}

fn circle() -> impl Strategy<Value = Circle> {
    (point(), 0.01..500.0f64).prop_map(|(c, r)| Circle::new(c, r))
}

/// Millimetre slack for "on segment `l`": the kernel's parametric
/// `EPSILON` slack at the ends, scaled to the segment's length.
fn seg_tol(l: &Line) -> f64 {
    EPSILON * l.length().max(1.0)
}

/// Distance from `p` to the infinite line through `l`.
fn dist_to_infinite(l: &Line, p: Vec2) -> f64 {
    let d = l.p2 - l.p1;
    (p - l.p1).cross(d).abs() / d.length()
}

/// `a` and `b` are the same set of points within `tol`, ignoring order.
fn same_set(a: &[Vec2], b: &[Vec2], tol: f64) -> bool {
    a.len() == b.len()
        && a.iter().all(|p| b.iter().any(|q| p.approx_eq(*q, tol)))
        && b.iter().all(|q| a.iter().any(|p| p.approx_eq(*q, tol)))
}

proptest! {
    #![proptest_config(config())]

    /// A segment-segment hit lies on both segments; the swap agrees.
    #[test]
    fn line_line_point_on_both_and_symmetric(a in segment(), b in segment()) {
        let ab = line_line(&a, &b);
        let ba = line_line(&b, &a);
        prop_assert_eq!(ab.is_some(), ba.is_some(), "ab={:?} ba={:?}", ab, ba);
        if let (Some(p), Some(q)) = (ab, ba) {
            prop_assert!(a.distance_to_point(p) <= seg_tol(&a), "p={:?} off a", p);
            prop_assert!(b.distance_to_point(p) <= seg_tol(&b), "p={:?} off b", p);
            prop_assert!(p.approx_eq(q, seg_tol(&a).max(seg_tol(&b))), "{:?} != {:?}", p, q);
        }
    }

    /// An infinite-line hit lies on both infinite lines; the swap agrees.
    #[test]
    fn line_line_infinite_point_on_both_and_symmetric(a in segment(), b in segment()) {
        let ab = line_line_infinite(&a, &b);
        let ba = line_line_infinite(&b, &a);
        prop_assert_eq!(ab.is_some(), ba.is_some(), "ab={:?} ba={:?}", ab, ba);
        if let (Some(p), Some(q)) = (ab, ba) {
            // The hit can be far outside the segments; scale by its distance.
            let tol = EPSILON * p.length().max(1.0);
            prop_assert!(dist_to_infinite(&a, p) <= tol, "p={:?} off a", p);
            prop_assert!(dist_to_infinite(&b, p) <= tol, "p={:?} off b", p);
            prop_assert!(p.approx_eq(q, tol), "{:?} != {:?}", p, q);
        }
    }

    /// Every segment-circle hit lies on the segment and on the circle.
    #[test]
    fn line_circle_points_on_both(l in segment(), c in circle()) {
        let hits = line_circle(&l, &c);
        prop_assert!(hits.len() <= 2, "{:?}", hits);
        for p in hits {
            prop_assert!(l.distance_to_point(p) <= seg_tol(&l), "p={:?} off segment", p);
            prop_assert!(c.distance_to_point(p).abs() <= EPSILON, "p={:?} off circle", p);
        }
    }

    /// Every circle-circle hit lies on both circles; the swap agrees.
    #[test]
    fn circle_circle_points_on_both_and_symmetric(a in circle(), b in circle()) {
        let ab = circle_circle(&a, &b);
        let ba = circle_circle(&b, &a);
        for p in &ab {
            prop_assert!(a.distance_to_point(*p).abs() <= EPSILON, "p={:?} off a", p);
            prop_assert!(b.distance_to_point(*p).abs() <= EPSILON, "p={:?} off b", p);
        }
        prop_assert!(same_set(&ab, &ba, EPSILON), "ab={:?} ba={:?}", ab, ba);
    }
}
