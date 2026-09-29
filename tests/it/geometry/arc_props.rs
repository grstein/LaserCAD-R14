//! Property tests for the `Arc` angle helpers (`geometry::arc`).
//!
//! An arc built from a start angle and a signed sweep agrees with itself:
//! `sweep_angle` returns that sweep, every angle inside it is contained, every
//! angle outside is not, the endpoints are the points at the start/end angles,
//! and every contained point lies inside `bbox`. Start angles are left
//! un-normalised on purpose, since `Arc` stores them losslessly.

use core::f64::consts::TAU;
use lasercad::geometry::{Arc, EPSILON, Vec2};
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
            "/proptest-regressions/geometry/arc_props.txt"
        )))),
        ..ProptestConfig::default()
    }
}

/// Keeps the sweep a proper arc with room on both sides for a margin.
const SWEEP_MARGIN: f64 = 0.01;

#[derive(Debug, Clone, Copy)]
struct Case {
    arc: Arc,
    start: f64,
    sweep: f64,
}

impl Case {
    /// The angle `frac` of the way along the sweep, in its direction.
    fn angle_at(&self, frac: f64) -> f64 {
        let signed = if self.arc.ccw {
            self.sweep
        } else {
            -self.sweep
        };
        self.start + signed * frac
    }

    /// An angle in the gap the sweep does not cover, `frac` of the way into it.
    fn angle_outside(&self, frac: f64) -> f64 {
        let gap = TAU - self.sweep;
        let signed = if self.arc.ccw { 1.0 } else { -1.0 };
        self.start + signed * (self.sweep + gap * frac)
    }
}

fn case() -> impl Strategy<Value = Case> {
    (
        (-500.0..500.0f64, -500.0..500.0f64),
        0.01..500.0f64,
        -20.0..20.0f64,
        SWEEP_MARGIN..(TAU - SWEEP_MARGIN),
        any::<bool>(),
    )
        .prop_map(|((x, y), r, start, sweep, ccw)| {
            let end = if ccw { start + sweep } else { start - sweep };
            Case {
                arc: Arc::new(Vec2::new(x, y), r, start, end, ccw),
                start,
                sweep,
            }
        })
}

fn point_at(arc: &Arc, angle: f64) -> Vec2 {
    arc.center + Vec2::new(angle.cos(), angle.sin()) * arc.r
}

fn in_bbox(bbox: (Vec2, Vec2), p: Vec2, tol: f64) -> bool {
    let (lo, hi) = bbox;
    p.x >= lo.x - tol && p.x <= hi.x + tol && p.y >= lo.y - tol && p.y <= hi.y + tol
}

proptest! {
    #![proptest_config(config())]

    /// `sweep_angle` returns the sweep the arc was built with.
    #[test]
    fn sweep_angle_matches_construction(c in case()) {
        let got = c.arc.sweep_angle();
        prop_assert!((got - c.sweep).abs() <= 1e-9, "sweep {} != {}", got, c.sweep);
    }

    /// Both endpoints and any angle inside the sweep are contained.
    #[test]
    fn angles_inside_sweep_are_contained(c in case(), frac in 0.0..=1.0f64) {
        prop_assert!(c.arc.contains_angle(c.arc.start_angle));
        prop_assert!(c.arc.contains_angle(c.arc.end_angle));
        let a = c.angle_at(frac);
        prop_assert!(c.arc.contains_angle(a), "angle {} not contained in {:?}", a, c.arc);
    }

    /// An angle in the uncovered gap, clear of both ends, is not contained.
    #[test]
    fn angles_outside_sweep_are_not_contained(c in case(), frac in 0.01..0.99f64) {
        let a = c.angle_outside(frac);
        prop_assert!(!c.arc.contains_angle(a), "angle {} contained in {:?}", a, c.arc);
    }

    /// The endpoints are the points at the start and end angles, on the circle.
    #[test]
    fn endpoints_match_start_and_end_angles(c in case()) {
        let tol = EPSILON * c.arc.r.max(1.0);
        let (sp, ep) = (c.arc.start_point(), c.arc.end_point());
        prop_assert!(sp.approx_eq(point_at(&c.arc, c.start), tol), "{:?}", sp);
        prop_assert!(ep.approx_eq(point_at(&c.arc, c.angle_at(1.0)), tol), "{:?}", ep);
        prop_assert!(((sp - c.arc.center).length() - c.arc.r).abs() <= tol);
        prop_assert!(((ep - c.arc.center).length() - c.arc.r).abs() <= tol);
    }

    /// Every point on the arc lies inside its bounding box.
    #[test]
    fn points_on_arc_lie_in_bbox(c in case(), frac in 0.0..=1.0f64) {
        let p = point_at(&c.arc, c.angle_at(frac));
        let tol = EPSILON * c.arc.r.max(1.0);
        prop_assert!(in_bbox(c.arc.bbox(), p, tol), "{:?} outside {:?}", p, c.arc.bbox());
    }
}
