//! Canonical circle value type in millimeter space.
//!
//! [`Circle`] is the kernel's circle primitive: a center point and a radius,
//! plus a small set of pure geometric helpers (circumference, area, bbox,
//! point-at-angle, containment, signed distance). It is a value type — no
//! identity, no document-level state. Persistence and history identification
//! belong to the `Entity` / `Document` layer (LCV-020+).
//!
//! Conventions match [`Vec2`] and [`Line`]: `f64` mm coordinates, radians
//! elsewhere in the kernel (CCW from the positive X axis), no `Eq` / `Hash`
//! (transitively `f64`). The constructor is intentionally `const fn` and
//! non-validating so it can be used in `const` contexts (e.g., test
//! fixtures). Validation of `r` is left to callers — the doc comment is the
//! single source of truth for the "r > 0" guidance.
//!
//! Frozen by demand LCV-012.

use serde::{Deserialize, Serialize};

use crate::geometry::vec2::Vec2;

/// A circle in millimeter space, defined by a center and a radius.
///
/// Field names match the original TypeScript `geometry/circle.ts` (`center`, `r`) so
/// the port is easy to cross-reference. Equality (`==`) is bit-exact `f64`
/// comparison on the center and the radius; tolerance-aware checks should
/// compare each component explicitly.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Circle {
    /// Center of the circle.
    pub center: Vec2,
    /// Radius in millimeters.
    pub r: f64,
}

impl Circle {
    /// Construct a circle from a center and radius.
    ///
    /// The constructor is **panic-free** — `r` is not validated. A meaningful
    /// circle requires `r > 0.0`; callers that need validation perform it
    /// themselves. The constructor stays `const` and trivial so it can be
    /// used in tests and `static` contexts.
    pub const fn new(center: Vec2, r: f64) -> Self {
        Self { center, r }
    }

    /// Circumference of the circle: `2π · r`.
    pub fn circumference(&self) -> f64 {
        2.0 * core::f64::consts::PI * self.r
    }

    /// Area of the disk bounded by the circle: `π · r²`.
    pub fn area(&self) -> f64 {
        core::f64::consts::PI * self.r * self.r
    }

    /// Axis-aligned bounding box of the circle as `(min, max)`.
    ///
    /// `min = center - (r, r)`, `max = center + (r, r)`. Tight for `r >= 0`.
    pub fn bbox(&self) -> (Vec2, Vec2) {
        let offset = Vec2::new(self.r, self.r);
        (self.center - offset, self.center + offset)
    }

    /// Point on the circle at the given angle.
    ///
    /// `angle_rad` is in radians, measured CCW from the positive X axis
    /// (AutoCAD convention). Returns `center + (r·cos(angle), r·sin(angle))`.
    pub fn point_at_angle(&self, angle_rad: f64) -> Vec2 {
        self.center + Vec2::new(self.r * angle_rad.cos(), self.r * angle_rad.sin())
    }

    /// True iff `p` is within the closed disk, with `eps` slack on the
    /// boundary: `(p - center).length() <= r + eps`.
    ///
    /// The boundary counts as "inside" within `eps`. Callers that want a
    /// strict interior test pass a negative `eps`, or test
    /// [`Circle::distance_to_point`] directly.
    pub fn contains_point(&self, p: Vec2, eps: f64) -> bool {
        (p - self.center).length() <= self.r + eps
    }

    /// **Signed** distance from `p` to the circle: `(p - center).length() - r`.
    ///
    /// - Positive when `p` is outside the circle.
    /// - `0.0` on the circle.
    /// - Negative when `p` is inside the circle.
    ///
    /// Matches the convention used by GIS and CSG libraries. Callers wanting
    /// an unsigned distance call `.abs()`.
    pub fn distance_to_point(&self, p: Vec2) -> f64 {
        (p - self.center).length() - self.r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::epsilon::EPSILON;
    use core::f64::consts::{FRAC_PI_2, PI};

    /// AC#1 — `Circle` has `pub` fields and can be constructed via struct literal.
    #[test]
    fn struct_literal_construction() {
        let c = Circle {
            center: Vec2::default(),
            r: 1.0,
        };
        assert_eq!(c.center, Vec2::default());
        assert_eq!(c.r, 1.0);
    }

    /// AC#2, AC#3 — `bbox` at origin and offset.
    #[test]
    fn bbox_origin_and_offset_circle() {
        assert_eq!(
            Circle::new(Vec2::default(), 5.0).bbox(),
            (Vec2::new(-5.0, -5.0), Vec2::new(5.0, 5.0))
        );
        assert_eq!(
            Circle::new(Vec2::new(10.0, 20.0), 3.0).bbox(),
            (Vec2::new(7.0, 17.0), Vec2::new(13.0, 23.0))
        );
    }

    /// AC#4, AC#5, AC#6 — `point_at_angle` at cardinal directions.
    #[test]
    fn point_at_angle_cardinals() {
        let c = Circle::new(Vec2::default(), 5.0);
        assert!(
            c.point_at_angle(0.0)
                .approx_eq(Vec2::new(5.0, 0.0), EPSILON)
        );
        assert!(
            c.point_at_angle(FRAC_PI_2)
                .approx_eq(Vec2::new(0.0, 5.0), EPSILON)
        );
        assert!(
            c.point_at_angle(PI)
                .approx_eq(Vec2::new(-5.0, 0.0), EPSILON)
        );
        // Bonus: 3π/2 sits at (0, -5).
        assert!(
            c.point_at_angle(3.0 * FRAC_PI_2)
                .approx_eq(Vec2::new(0.0, -5.0), EPSILON)
        );
    }

    /// AC#7 — the center is inside the circle.
    #[test]
    fn contains_point_center() {
        let c = Circle::new(Vec2::default(), 5.0);
        assert!(c.contains_point(Vec2::default(), EPSILON));
    }

    /// AC#8, AC#9 — boundary is inside within `eps`; outside-by-more-than-eps is outside.
    #[test]
    fn contains_point_boundary_and_outside() {
        let c = Circle::new(Vec2::default(), 5.0);
        assert!(c.contains_point(Vec2::new(5.0, 0.0), EPSILON));
        assert!(!c.contains_point(Vec2::new(5.0 + 1e-6, 0.0), EPSILON));
    }

    /// AC#10, AC#11, AC#12 — signed distance: outside positive, inside negative, boundary zero.
    #[test]
    fn signed_distance_inside_outside_and_boundary() {
        let c = Circle::new(Vec2::default(), 5.0);
        assert_eq!(c.distance_to_point(Vec2::new(6.0, 0.0)), 1.0);
        assert_eq!(c.distance_to_point(Vec2::new(4.0, 0.0)), -1.0);
        assert!(c.distance_to_point(Vec2::new(5.0, 0.0)).abs() <= EPSILON);
    }

    /// AC#13 — circumference and area of an r=2 circle.
    #[test]
    fn circumference_and_area_of_radius_2() {
        let c = Circle::new(Vec2::default(), 2.0);
        assert!((c.circumference() - 4.0 * PI).abs() <= EPSILON);
        assert!((c.area() - 4.0 * PI).abs() <= EPSILON);
    }
}
