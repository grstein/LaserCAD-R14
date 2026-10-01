//! Canonical arc value type in millimeter space.
//!
//! [`Arc`] is the kernel's proper-arc primitive: a center, a radius, an
//! explicit start/end angle pair, and an orientation flag (`ccw`). All angles
//! are in radians, CCW-positive from the +X axis (AutoCAD `ARC` convention).
//! Storage is intentionally lossless: the constructor does NOT normalize the
//! supplied angles, so a caller that hands in `start_angle = 350°,
//! end_angle = 10°, ccw = true` keeps that exact representation. The methods
//! below handle wrap-around internally via `rem_euclid` on `2π`.
//!
//! `Arc` strictly represents *proper* arcs (sweep `< 2π`). Equal start and
//! end angles collapse to a zero-length arc; callers that need a closed loop
//! use [`crate::geometry::Circle`]. The full-circle bbox branch (sweep
//! `>= 2π - EPSILON`) exists only as a robustness guard for inputs that
//! deliberately trace a near-closed curve.
//!
//! Frozen by demand LCV-013.

use serde::{Deserialize, Serialize};

use crate::geometry::epsilon::EPSILON;
use crate::geometry::vec2::Vec2;
use core::f64::consts::{FRAC_PI_2, PI, TAU};

/// An arc in millimeter space, defined by a center, a radius, a start/end
/// angle pair (radians), and an orientation flag.
///
/// Field names match the original TypeScript `geometry/arc.ts`
/// (`center`, `r`, `startAngle`, `endAngle`, `ccw`) so the port is easy to
/// cross-reference. Equality (`==`) is bit-exact `f64` comparison on every
/// field; tolerance-aware checks should compare each component explicitly.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Arc {
    /// Center of the arc's parent circle.
    pub center: Vec2,
    /// Radius in millimeters.
    pub r: f64,
    /// Start angle in radians, CCW from +X.
    pub start_angle: f64,
    /// End angle in radians, CCW from +X.
    pub end_angle: f64,
    /// Orientation: `true` = swept CCW from `start_angle` to `end_angle`,
    /// `false` = swept CW.
    pub ccw: bool,
}

impl Arc {
    /// Construct an arc from its raw components.
    ///
    /// The constructor is **panic-free** and **non-normalizing**: `r`,
    /// `start_angle`, and `end_angle` are stored as-is. A meaningful arc
    /// requires `r > 0.0`; callers that need validation perform it
    /// themselves. The constructor stays `const` and trivial so it can be
    /// used in tests and `static` contexts.
    pub const fn new(center: Vec2, r: f64, start_angle: f64, end_angle: f64, ccw: bool) -> Self {
        Self {
            center,
            r,
            start_angle,
            end_angle,
            ccw,
        }
    }

    /// Point on the arc at `start_angle`:
    /// `center + (r·cos(start_angle), r·sin(start_angle))`.
    pub fn start_point(&self) -> Vec2 {
        self.point_at_angle(self.start_angle)
    }

    /// Point on the arc at `end_angle`:
    /// `center + (r·cos(end_angle), r·sin(end_angle))`.
    pub fn end_point(&self) -> Vec2 {
        self.point_at_angle(self.end_angle)
    }

    /// Magnitude of the angular sweep in radians, in `[0, 2π]`.
    ///
    /// Computed as the angular distance from `start_angle` to `end_angle` in
    /// the direction implied by [`Arc::ccw`]. A start equal to end (within
    /// `EPSILON`) returns `0.0` — to draw a full circle, use
    /// [`crate::geometry::Circle`].
    pub fn sweep_angle(&self) -> f64 {
        let start = self.start_angle.rem_euclid(TAU);
        let end = self.end_angle.rem_euclid(TAU);
        // Zero-length sentinel: equal angles → zero sweep, never full circle.
        if (start - end).abs() <= EPSILON || (start - end).abs() >= TAU - EPSILON {
            return 0.0;
        }
        if self.ccw {
            (end - start).rem_euclid(TAU)
        } else {
            (start - end).rem_euclid(TAU)
        }
    }

    /// Arc length: `r · sweep_angle()`.
    pub fn arc_length(&self) -> f64 {
        self.r * self.sweep_angle()
    }

    /// True iff `angle` (modulo `2π`) lies within the sweep from
    /// `start_angle` to `end_angle` in the direction implied by `ccw`.
    ///
    /// Both endpoints are inclusive within [`EPSILON`]. Wrap-around (e.g.,
    /// start `350°`, end `10°`, `ccw = true`) is handled correctly. A
    /// zero-length arc returns `true` only at its single boundary angle.
    pub fn contains_angle(&self, angle: f64) -> bool {
        let start = self.start_angle.rem_euclid(TAU);
        let a = angle.rem_euclid(TAU);
        let sweep = self.sweep_angle();
        // Single-point arc: only the boundary angle qualifies.
        if sweep <= EPSILON {
            return Self::angles_equivalent(a, start);
        }
        let delta = if self.ccw {
            (a - start).rem_euclid(TAU)
        } else {
            (start - a).rem_euclid(TAU)
        };
        // Inclusive on both ends within EPSILON, with wrap-around tolerance
        // for inputs that land just before `start` (delta ≈ 2π).
        delta <= sweep + EPSILON || delta >= TAU - EPSILON
    }

    /// Axis-aligned bounding box of the arc as `(min, max)`.
    ///
    /// Considers the four cardinal extreme angles (`0`, `π/2`, `π`, `3π/2`)
    /// and includes each one in the bbox iff [`Arc::contains_angle`] is true;
    /// always includes `start_point` and `end_point`. A near-full-circle arc
    /// (sweep `>= 2π - EPSILON`) returns the parent circle's bbox as a
    /// robustness guard.
    pub fn bbox(&self) -> (Vec2, Vec2) {
        // Full-circle robustness guard: a caller that deliberately traces a
        // near-closed curve gets the parent circle's bbox.
        let raw_sweep = if self.ccw {
            (self.end_angle - self.start_angle).rem_euclid(TAU)
        } else {
            (self.start_angle - self.end_angle).rem_euclid(TAU)
        };
        if raw_sweep >= TAU - EPSILON {
            let offset = Vec2::new(self.r, self.r);
            return (self.center - offset, self.center + offset);
        }

        let mut min = self.start_point();
        let mut max = min;
        let ep = self.end_point();
        min = Vec2::new(min.x.min(ep.x), min.y.min(ep.y));
        max = Vec2::new(max.x.max(ep.x), max.y.max(ep.y));

        for theta in [0.0, FRAC_PI_2, PI, 3.0 * FRAC_PI_2] {
            if self.contains_angle(theta) {
                let p = self.point_at_angle(theta);
                min = Vec2::new(min.x.min(p.x), min.y.min(p.y));
                max = Vec2::new(max.x.max(p.x), max.y.max(p.y));
            }
        }
        (min, max)
    }

    fn point_at_angle(&self, angle: f64) -> Vec2 {
        self.center + Vec2::new(self.r * angle.cos(), self.r * angle.sin())
    }

    /// True iff `a` and `b` (both already in `[0, 2π)`) are equivalent
    /// modulo `2π` within `EPSILON`, with wrap-around (`0` and `2π - tiny`).
    fn angles_equivalent(a: f64, b: f64) -> bool {
        let diff = (a - b).abs();
        diff <= EPSILON || diff >= TAU - EPSILON
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

    fn deg(d: f64) -> f64 {
        d * PI / 180.0
    }

    /// AC#1 — `Arc` has `pub` fields and can be constructed via struct literal.
    #[test]
    fn struct_literal_construction() {
        let a = Arc {
            center: Vec2::default(),
            r: 1.0,
            start_angle: 0.0,
            end_angle: 1.0,
            ccw: true,
        };
        assert_eq!(a.center, Vec2::default());
        assert_eq!(a.r, 1.0);
        assert_eq!(a.start_angle, 0.0);
        assert_eq!(a.end_angle, 1.0);
        assert!(a.ccw);
    }

    /// AC#2 — quarter-arc endpoints.
    #[test]
    fn quarter_arc_endpoints() {
        let a = Arc::new(Vec2::default(), 1.0, 0.0, FRAC_PI_2, true);
        assert!(a.start_point().approx_eq(Vec2::new(1.0, 0.0), EPSILON));
        assert!(a.end_point().approx_eq(Vec2::new(0.0, 1.0), EPSILON));
    }

    /// AC#3 — quarter-arc sweep and length.
    #[test]
    fn quarter_arc_sweep_and_length() {
        let a = Arc::new(Vec2::default(), 1.0, 0.0, FRAC_PI_2, true);
        assert!((a.sweep_angle() - FRAC_PI_2).abs() <= EPSILON);
        assert!((a.arc_length() - FRAC_PI_2).abs() <= EPSILON);
    }

    /// AC#4 — `contains_angle` inside / outside / both boundaries.
    #[test]
    fn quarter_arc_contains_angle_inside_outside_and_boundaries() {
        let a = Arc::new(Vec2::default(), 1.0, 0.0, FRAC_PI_2, true);
        assert!(a.contains_angle(FRAC_PI_4)); // inside
        assert!(!a.contains_angle(3.0 * FRAC_PI_4)); // outside
        assert!(a.contains_angle(0.0)); // start boundary, inclusive
        assert!(a.contains_angle(FRAC_PI_2)); // end boundary, inclusive
    }

    /// AC#5 — bbox of a quarter arc is the corner box, not the parent circle's bbox.
    #[test]
    fn quarter_arc_bbox_is_corner_not_parent_circle() {
        let a = Arc::new(Vec2::default(), 1.0, 0.0, FRAC_PI_2, true);
        let (min, max) = a.bbox();
        assert!(min.approx_eq(Vec2::new(0.0, 0.0), EPSILON));
        assert!(max.approx_eq(Vec2::new(1.0, 1.0), EPSILON));
    }

    /// AC#6 — CCW wrap-around across angle `0`.
    #[test]
    fn ccw_wrap_around_zero() {
        let a = Arc::new(Vec2::default(), 1.0, deg(350.0), deg(10.0), true);
        assert!((a.sweep_angle() - deg(20.0)).abs() <= EPSILON);
        assert!(a.contains_angle(0.0));
        assert!(!a.contains_angle(PI));
        assert!(a.contains_angle(deg(5.0)));
    }

    /// AC#7 — CW quarter arc swept from (0,1) to (1,0).
    #[test]
    fn cw_quarter_arc() {
        let a = Arc::new(Vec2::default(), 1.0, FRAC_PI_2, 0.0, false);
        assert!((a.sweep_angle() - FRAC_PI_2).abs() <= EPSILON);
        assert!(a.contains_angle(FRAC_PI_4));
        assert!(!a.contains_angle(3.0 * FRAC_PI_4));
    }

    /// AC#8 — near-full-circle bbox matches the parent circle's bbox.
    #[test]
    fn near_full_circle_bbox_matches_parent_circle() {
        let a = Arc::new(Vec2::default(), 1.0, 0.0, TAU - EPSILON / 2.0, true);
        let (min, max) = a.bbox();
        assert!(min.approx_eq(Vec2::new(-1.0, -1.0), EPSILON));
        assert!(max.approx_eq(Vec2::new(1.0, 1.0), EPSILON));
    }

    /// AC#9 — bbox includes a cardinal extreme (`+X`) inside the sweep.
    #[test]
    fn bbox_includes_cardinal_extreme_inside_sweep() {
        let a = Arc::new(Vec2::default(), 1.0, -FRAC_PI_4, FRAC_PI_4, true);
        let (min, max) = a.bbox();
        assert!((max.x - 1.0).abs() <= EPSILON);
        assert!((min.x - FRAC_PI_4.cos()).abs() <= EPSILON);
        assert!((min.y - -FRAC_PI_4.sin()).abs() <= EPSILON);
        assert!((max.y - FRAC_PI_4.sin()).abs() <= EPSILON);
    }

    /// AC#10 — zero-length sweep collapses to a single boundary point.
    #[test]
    fn zero_length_arc_sweep_zero_and_boundary_contains() {
        let a = Arc::new(Vec2::default(), 1.0, FRAC_PI_4, FRAC_PI_4, true);
        assert_eq!(a.sweep_angle(), 0.0);
        assert_eq!(a.arc_length(), 0.0);
        assert!(a.contains_angle(FRAC_PI_4));
    }
}
