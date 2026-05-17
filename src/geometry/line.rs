//! Canonical line-segment value type in millimeter space.
//!
//! [`Line`] is the kernel's segment primitive: two endpoints in mm-space and a
//! small set of pure geometric helpers (length, direction, midpoint, bbox,
//! parametric point, closest-point / distance). It is a value type — no
//! identity, no document-level state. Persistence and history identification
//! belong to the `Entity` / `Document` layer (LCV-020+).
//!
//! Conventions match [`Vec2`]: `f64` mm coordinates, radians elsewhere in the
//! kernel, no `Eq` / `Hash` (transitively `f64`). Degenerate-direction
//! handling returns `Option<Vec2>` from [`Line::direction`], mirroring
//! [`Vec2::normalize`].
//!
//! Frozen by demand LCV-011.

use crate::geometry::epsilon::EPSILON;
use crate::geometry::vec2::Vec2;

/// A line segment in millimeter space, defined by two endpoints.
///
/// Field names match v1's TypeScript `geometry/line.ts` (`p1`, `p2`) so the
/// port is easy to cross-reference. Equality (`==`) is bit-exact `f64`
/// comparison on both endpoints; tolerance-aware checks should compare each
/// endpoint via [`Vec2::approx_eq`].
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Line {
    /// Start endpoint.
    pub p1: Vec2,
    /// End endpoint.
    pub p2: Vec2,
}

impl Line {
    /// Construct a line segment from its two endpoints.
    pub const fn new(p1: Vec2, p2: Vec2) -> Self {
        Self { p1, p2 }
    }

    /// Euclidean length of the segment: `|p2 - p1|`.
    pub fn length(&self) -> f64 {
        (self.p2 - self.p1).length()
    }

    /// Squared length of the segment. Cheaper than [`Line::length`] when only
    /// an ordering or a comparison-to-squared-radius is needed.
    pub fn length_squared(&self) -> f64 {
        (self.p2 - self.p1).length_squared()
    }

    /// Unit vector from `p1` toward `p2`, or `None` when the segment is
    /// degenerate (length at or below [`EPSILON`]).
    pub fn direction(&self) -> Option<Vec2> {
        (self.p2 - self.p1).normalize()
    }

    /// Midpoint of the segment: `(p1 + p2) / 2`.
    pub fn midpoint(&self) -> Vec2 {
        self.p1.lerp(self.p2, 0.5)
    }

    /// Axis-aligned bounding box of the segment as `(min, max)`.
    ///
    /// Invariant under endpoint ordering: swapping `p1` and `p2` yields the
    /// same `(min, max)` pair.
    pub fn bbox(&self) -> (Vec2, Vec2) {
        let min = Vec2::new(self.p1.x.min(self.p2.x), self.p1.y.min(self.p2.y));
        let max = Vec2::new(self.p1.x.max(self.p2.x), self.p1.y.max(self.p2.y));
        (min, max)
    }

    /// Parametric point along the (infinite) line through `p1` and `p2`:
    /// `t = 0.0` returns `p1`, `t = 1.0` returns `p2`. `t` is **not clamped**
    /// — values outside `[0, 1]` return the extrapolated infinite-line point.
    ///
    /// Callers that need segment-clamping use [`Line::closest_point`] instead.
    pub fn point_at(&self, t: f64) -> Vec2 {
        self.p1.lerp(self.p2, t)
    }

    /// Point on the segment closest to `p`.
    ///
    /// For a degenerate segment (length at or below [`EPSILON`]) returns
    /// `p1` regardless of `p`. Otherwise returns the foot of the
    /// perpendicular from `p`, clamped to the closed segment `[p1, p2]`.
    pub fn closest_point(&self, p: Vec2) -> Vec2 {
        let d = self.p2 - self.p1;
        let len_sq = d.length_squared();
        if len_sq <= EPSILON * EPSILON {
            return self.p1;
        }
        let t = (p - self.p1).dot(d) / len_sq;
        let t_clamped = t.clamp(0.0, 1.0);
        self.p1 + d * t_clamped
    }

    /// Euclidean distance from `p` to the closest point on the segment.
    pub fn distance_to_point(&self, p: Vec2) -> f64 {
        (p - self.closest_point(p)).length()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC#1 — `Line` has `pub` fields and can be constructed via struct literal.
    #[test]
    fn struct_literal_construction() {
        let l = Line {
            p1: Vec2::default(),
            p2: Vec2::new(1.0, 0.0),
        };
        assert_eq!(l.p1, Vec2::default());
        assert_eq!(l.p2, Vec2::new(1.0, 0.0));
    }

    /// AC#2 — exact arithmetic on the (3,4,5) Pythagorean triple.
    #[test]
    fn length_and_length_squared_345() {
        let l = Line::new(Vec2::new(0.0, 0.0), Vec2::new(3.0, 4.0));
        assert_eq!(l.length(), 5.0);
        assert_eq!(l.length_squared(), 25.0);
    }

    /// AC#3 — `direction` returns `None` on a degenerate segment and the
    /// expected unit vector on an axis-aligned segment.
    #[test]
    fn direction_none_for_degenerate_and_unit_for_axis_aligned() {
        assert!(Line::new(Vec2::default(), Vec2::default())
            .direction()
            .is_none());
        let dir = Line::new(Vec2::new(0.0, 0.0), Vec2::new(2.0, 0.0))
            .direction()
            .expect("non-degenerate direction");
        assert!(dir.approx_eq(Vec2::new(1.0, 0.0), EPSILON));
    }

    /// AC#4 — midpoint of a known segment.
    #[test]
    fn midpoint_of_known_segment() {
        let m = Line::new(Vec2::new(2.0, 4.0), Vec2::new(6.0, 8.0)).midpoint();
        assert!(m.approx_eq(Vec2::new(4.0, 6.0), EPSILON));
    }

    /// AC#5 — `bbox` is invariant under endpoint ordering.
    #[test]
    fn bbox_is_order_invariant() {
        let a = Line::new(Vec2::new(5.0, 1.0), Vec2::new(2.0, 7.0)).bbox();
        let b = Line::new(Vec2::new(2.0, 7.0), Vec2::new(5.0, 1.0)).bbox();
        assert_eq!(a, b);
        assert_eq!(a, (Vec2::new(2.0, 1.0), Vec2::new(5.0, 7.0)));
    }

    /// AC#6 — `point_at` at endpoints, midpoint, and an extrapolated `t`.
    #[test]
    fn point_at_endpoints_midpoint_and_extrapolated() {
        let l = Line::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
        assert_eq!(l.point_at(0.0), l.p1);
        assert_eq!(l.point_at(1.0), l.p2);
        assert!(l.point_at(0.5).approx_eq(l.midpoint(), EPSILON));
        assert!(l.point_at(2.0).approx_eq(Vec2::new(2.0, 0.0), EPSILON));
    }

    /// AC#7 — `closest_point` returns the foot of the perpendicular for a
    /// point off the line but within the segment's parametric range.
    #[test]
    fn closest_point_foot_of_perpendicular() {
        let l = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        let cp = l.closest_point(Vec2::new(3.0, 5.0));
        assert!(cp.approx_eq(Vec2::new(3.0, 0.0), EPSILON));
    }

    /// AC#8 — `closest_point` clamps to either segment endpoint when the
    /// perpendicular foot falls outside the segment.
    #[test]
    fn closest_point_clamps_to_endpoints() {
        let l = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        assert!(l
            .closest_point(Vec2::new(-2.0, 1.0))
            .approx_eq(Vec2::new(0.0, 0.0), EPSILON));
        assert!(l
            .closest_point(Vec2::new(15.0, -1.0))
            .approx_eq(Vec2::new(10.0, 0.0), EPSILON));
    }

    /// AC#9 — `closest_point` on a degenerate segment returns `p1`
    /// regardless of the query point.
    #[test]
    fn closest_point_on_degenerate_segment_returns_p1() {
        let p1 = Vec2::new(3.0, 7.0);
        let l = Line::new(p1, p1);
        assert_eq!(l.closest_point(Vec2::new(0.0, 0.0)), p1);
        assert_eq!(l.closest_point(Vec2::new(100.0, -50.0)), p1);
    }

    /// AC#10 — `distance_to_point` is `0` on the segment and equals the
    /// perpendicular distance otherwise.
    #[test]
    fn distance_to_point_zero_on_segment_and_perpendicular_distance() {
        let l = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        assert!(l.distance_to_point(Vec2::new(4.0, 0.0)).abs() <= EPSILON);
        assert_eq!(l.distance_to_point(Vec2::new(3.0, 5.0)), 5.0);
    }
}
