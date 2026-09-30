//! Arc-target trim and extend helpers (LCV-160).
//!
//! Positions on an arc are measured as *travel*: the angle swept from a
//! reference angle in a given direction, in `[0, 2π)`. [`trim_arc_at_points`]
//! measures from the start in the arc's own direction; [`extend_arc`] measures
//! from the grown endpoint, away from the other end. Both results keep the
//! target's center, radius and `ccw`; unchanged ends keep their exact angle.
//!
//! See the parent [`super`] module for the public commands wrapping these
//! routines.

use core::f64::consts::TAU;

use crate::document::Entity;
use crate::geometry::{Arc, EPSILON, Vec2};

/// Polar angle of `p` about `center`, in `(-π, π]`.
fn angle_of(center: Vec2, p: Vec2) -> f64 {
    let v = p - center;
    v.y.atan2(v.x)
}

/// Travel from angle `from` to angle `to`, CCW when `ccw`, in `[0, 2π)`.
fn travel(from: f64, to: f64, ccw: bool) -> f64 {
    if ccw {
        (to - from).rem_euclid(TAU)
    } else {
        (from - to).rem_euclid(TAU)
    }
}

/// Trim an [`Arc`] target at its cut points (LCV-160 AC 1): keep the sub-arc
/// between the cut points (or the arc's ends) around `keep`. Points off the
/// span or at the arc's own ends are ignored; `None` when none is left. A
/// `keep` off the span counts as the nearer end.
pub(crate) fn trim_arc_at_points(target: &Arc, pts: &[Vec2], keep: Vec2) -> Option<Entity> {
    let sweep = target.sweep_angle();
    let at = |p: Vec2| travel(target.start_angle, angle_of(target.center, p), target.ccw);
    let mut cuts: Vec<(f64, f64)> = pts
        .iter()
        .map(|&p| (at(p), angle_of(target.center, p)))
        .filter(|(u, _)| *u > EPSILON && *u < sweep - EPSILON)
        .collect();
    if cuts.is_empty() {
        return None;
    }
    cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut u_keep = at(keep);
    if u_keep > sweep {
        u_keep = if u_keep - sweep < TAU - u_keep {
            sweep
        } else {
            0.0
        };
    }
    let (mut lo, mut hi) = (target.start_angle, target.end_angle);
    for (u, angle) in cuts {
        if u_keep <= u + EPSILON {
            hi = angle;
            break;
        }
        lo = angle;
    }
    Some(Entity::Arc(Arc::new(
        target.center,
        target.r,
        lo,
        hi,
        target.ccw,
    )))
}

/// Grow endpoint `ep` (`0` = start, `1` = end) of an [`Arc`] along its own
/// circle, away from the other end, to the nearest of `pts` (LCV-160 AC 5).
/// Returns the grown arc and the travel in mm. A point at the endpoint
/// itself, or one reached only at or past `2π − sweep` (closing the arc into
/// a full turn, AC 7), is rejected; `None` when no point is left.
pub(crate) fn extend_arc(target: &Arc, pts: &[Vec2], ep: u8) -> Option<(Arc, f64)> {
    let limit = TAU - target.sweep_angle() - EPSILON;
    let (from, dir) = if ep == 0 {
        (target.start_angle, !target.ccw)
    } else {
        (target.end_angle, target.ccw)
    };
    let (u, angle) = pts
        .iter()
        .map(|&p| angle_of(target.center, p))
        .map(|angle| (travel(from, angle, dir), angle))
        .filter(|(u, _)| *u > EPSILON && *u < limit)
        .min_by(|a, b| a.0.total_cmp(&b.0))?;
    let mut grown = *target;
    if ep == 0 {
        grown.start_angle = angle;
    } else {
        grown.end_angle = angle;
    }
    Some((grown, u * target.r))
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::{FRAC_PI_2, PI};

    fn polar(r: f64, a: f64) -> Vec2 {
        Vec2::new(r * a.cos(), r * a.sin())
    }

    fn as_arc(e: Option<Entity>) -> Arc {
        match e {
            Some(Entity::Arc(a)) => a,
            other => panic!("expected Arc, got {other:?}"),
        }
    }

    fn near(a: Vec2, b: Vec2) -> bool {
        a.approx_eq(b, 1e-9)
    }

    /// AC 1 — a CW arc keeps its direction; the unchanged end keeps its
    /// exact angle.
    #[test]
    fn trim_cw_arc_keeps_direction_and_exact_end() {
        let target = Arc::new(Vec2::default(), 10.0, PI, 0.0, false);
        let cut = [polar(10.0, FRAC_PI_2)];
        let got = as_arc(trim_arc_at_points(&target, &cut, polar(10.0, 0.75 * PI)));
        assert_eq!((got.start_angle, got.ccw), (PI, false));
        assert!(near(got.end_point(), polar(10.0, FRAC_PI_2)));
        let got = as_arc(trim_arc_at_points(&target, &cut, polar(10.0, 0.25 * PI)));
        assert_eq!(got.end_angle, 0.0);
        assert!(near(got.start_point(), polar(10.0, FRAC_PI_2)));
    }

    /// AC 1 — an arc across ±π keeps the piece between two cut points.
    #[test]
    fn trim_arc_across_pi_between_two_cuts() {
        let target = Arc::new(Vec2::default(), 10.0, 0.75 * PI, -0.75 * PI, true);
        let cuts = [polar(10.0, 0.9 * PI), polar(10.0, -0.9 * PI)];
        let got = as_arc(trim_arc_at_points(&target, &cuts, polar(10.0, PI)));
        assert!(near(got.start_point(), cuts[0]) && near(got.end_point(), cuts[1]));
        assert!(got.ccw && got.contains_angle(PI));
    }

    /// Cut points at the arc's own ends or off its span are ignored, so a
    /// trim there is a no-op instead of an identical commit.
    #[test]
    fn trim_arc_ignores_own_ends_and_off_span_points() {
        let target = Arc::new(Vec2::default(), 10.0, 0.0, PI, true);
        let pts = [
            target.start_point(),
            target.end_point(),
            polar(10.0, -FRAC_PI_2),
        ];
        assert!(trim_arc_at_points(&target, &pts, polar(10.0, 1.0)).is_none());
    }

    /// A click just past an end (off the span) keeps that end's piece.
    #[test]
    fn trim_arc_keep_off_span_snaps_to_nearer_end() {
        let target = Arc::new(Vec2::default(), 10.0, 0.0, PI, true);
        let cut = [polar(10.0, FRAC_PI_2)];
        let got = as_arc(trim_arc_at_points(&target, &cut, polar(10.0, -0.1)));
        assert_eq!(got.start_angle, 0.0);
        let got = as_arc(trim_arc_at_points(&target, &cut, polar(10.0, PI + 0.1)));
        assert_eq!(got.end_angle, PI);
    }

    /// AC 5 — the end grows along the arc's direction, the start against
    /// it, to the nearest point; the travel is in mm.
    #[test]
    fn extend_arc_grows_away_from_other_end() {
        let target = Arc::new(Vec2::default(), 10.0, 0.0, FRAC_PI_2, true);
        let pts = [polar(10.0, 2.0 * PI / 3.0), polar(10.0, -2.0 * PI / 3.0)];
        let (end, mm) = extend_arc(&target, &pts, 1).expect("end reaches");
        assert!(near(end.end_point(), pts[0]) && end.start_angle == 0.0);
        assert!((mm - 10.0 * PI / 6.0).abs() < 1e-9);
        let (start, mm) = extend_arc(&target, &pts, 0).expect("start reaches");
        assert!(near(start.start_point(), pts[1]) && start.end_angle == FRAC_PI_2);
        assert!((mm - 10.0 * 2.0 * PI / 3.0).abs() < 1e-9);
    }

    /// AC 7 — a point only reachable by closing the full turn (on the arc's
    /// own span, or at its other end) is rejected, and so is the endpoint.
    #[test]
    fn extend_arc_rejects_full_turn() {
        let target = Arc::new(Vec2::default(), 10.0, 0.0, FRAC_PI_2, true);
        let pts = [
            polar(10.0, PI / 4.0),
            target.start_point(),
            target.end_point(),
        ];
        assert!(extend_arc(&target, &pts, 1).is_none());
        assert!(extend_arc(&target, &pts, 0).is_none());
    }
}
