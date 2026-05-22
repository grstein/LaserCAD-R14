//! Canonical 2D vector / point type in millimeter space.
//!
//! [`Vec2`] is the single point/vector type used across the geometry kernel,
//! the document model, the command line, and SVG export. Coordinates are
//! `f64` millimeters. No `Eq` or `Hash` impls — `f64` cannot satisfy either,
//! and downstream callers must be explicit about epsilon comparisons via
//! [`Vec2::approx_eq`].
//!
//! Frozen by demand LCV-010.

use core::ops::{Add, Div, Mul, Neg, Sub};

use serde::{Deserialize, Serialize};

use crate::geometry::epsilon::EPSILON;

/// 2D point or vector in millimeter space.
///
/// Equality (`==`) is bit-exact `f64` comparison; use [`Vec2::approx_eq`] for
/// tolerance-aware checks. See `AGENTS.md` § "Units and types" for the
/// kernel-wide contract.
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    /// X coordinate in millimeters.
    pub x: f64,
    /// Y coordinate in millimeters.
    pub y: f64,
}

impl Vec2 {
    /// Construct a vector from its components.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Euclidean length: `sqrt(x² + y²)`.
    pub fn length(&self) -> f64 {
        self.length_squared().sqrt()
    }

    /// Squared length: `x² + y²`. Cheaper than [`Vec2::length`] when only an
    /// ordering or a comparison-to-squared-radius is needed.
    pub fn length_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    /// Euclidean distance to `other`.
    pub fn distance(&self, other: Vec2) -> f64 {
        (*self - other).length()
    }

    /// Squared distance to `other`. Cheaper than [`Vec2::distance`].
    pub fn distance_squared(&self, other: Vec2) -> f64 {
        (*self - other).length_squared()
    }

    /// Dot product: `self.x * other.x + self.y * other.y`.
    pub fn dot(&self, other: Vec2) -> f64 {
        self.x * other.x + self.y * other.y
    }

    /// 2D scalar cross product: `self.x * other.y - self.y * other.x`.
    ///
    /// Positive when `other` is counter-clockwise from `self`, negative when
    /// clockwise, zero when collinear.
    pub fn cross(&self, other: Vec2) -> f64 {
        self.x * other.y - self.y * other.x
    }

    /// Unit vector in the direction of `self`, or `None` when the length is
    /// at or below [`EPSILON`] (degenerate direction).
    pub fn normalize(&self) -> Option<Vec2> {
        let len = self.length();
        if len <= EPSILON {
            None
        } else {
            Some(Vec2::new(self.x / len, self.y / len))
        }
    }

    /// Linear interpolation: `self + (other - self) * t`. `t` is NOT clamped
    /// to `[0, 1]`; extrapolation is supported.
    pub fn lerp(&self, other: Vec2, t: f64) -> Vec2 {
        Vec2::new(
            self.x + (other.x - self.x) * t,
            self.y + (other.y - self.y) * t,
        )
    }

    /// True iff both coordinate differences are `<= eps` in absolute value.
    pub fn approx_eq(&self, other: Vec2, eps: f64) -> bool {
        (self.x - other.x).abs() <= eps && (self.y - other.y).abs() <= eps
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Mul<f64> for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: f64) -> Vec2 {
        Vec2::new(self.x * rhs, self.y * rhs)
    }
}

impl Mul<Vec2> for f64 {
    type Output = Vec2;
    fn mul(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self * rhs.x, self * rhs.y)
    }
}

impl Div<f64> for Vec2 {
    type Output = Vec2;
    fn div(self, rhs: f64) -> Vec2 {
        Vec2::new(self.x / rhs, self.y / rhs)
    }
}

impl Neg for Vec2 {
    type Output = Vec2;
    fn neg(self) -> Vec2 {
        Vec2::new(-self.x, -self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC#1 — `Vec2` has `pub` fields and can be constructed via struct literal.
    #[test]
    fn struct_literal_construction() {
        let v = Vec2 { x: 0.0, y: 0.0 };
        assert_eq!(v.x, 0.0);
        assert_eq!(v.y, 0.0);
    }

    /// AC#2, AC#3, AC#12 — exact arithmetic on the (3,4,5) Pythagorean triple.
    #[test]
    fn length_and_distance_of_345_triple() {
        assert_eq!(Vec2::new(3.0, 4.0).length(), 5.0);
        assert_eq!(Vec2::new(3.0, 4.0).length_squared(), 25.0);
        assert_eq!(Vec2::default().distance(Vec2::new(3.0, 4.0)), 5.0);
        assert_eq!(Vec2::default().distance_squared(Vec2::new(3.0, 4.0)), 25.0);
    }

    /// AC#4 — degenerate-length vectors normalize to `None`.
    #[test]
    fn normalize_returns_none_below_epsilon() {
        assert!(Vec2::default().normalize().is_none());
        assert!(Vec2::new(EPSILON / 2.0, 0.0).normalize().is_none());
    }

    /// AC#5 — a non-degenerate axis-aligned vector normalizes to the unit X.
    #[test]
    fn normalize_unit_x() {
        let n = Vec2::new(2.0, 0.0)
            .normalize()
            .expect("non-degenerate length");
        assert!(n.approx_eq(Vec2::new(1.0, 0.0), EPSILON));
    }

    /// AC#6 — 2D scalar cross product on basis vectors.
    #[test]
    fn cross_basis_vectors() {
        assert_eq!(Vec2::new(1.0, 0.0).cross(Vec2::new(0.0, 1.0)), 1.0);
        assert_eq!(Vec2::new(0.0, 1.0).cross(Vec2::new(1.0, 0.0)), -1.0);
    }

    /// AC#7 — dot product on a known pair.
    #[test]
    fn dot_product_known_pair() {
        assert_eq!(Vec2::new(1.0, 2.0).dot(Vec2::new(3.0, 4.0)), 11.0);
    }

    /// AC#8 — lerp at `t=0`, `t=0.5`, `t=1.0`.
    #[test]
    fn lerp_endpoints_and_midpoint() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(10.0, 0.0);
        assert!(a.lerp(b, 0.5).approx_eq(Vec2::new(5.0, 0.0), EPSILON));
        assert_eq!(a.lerp(b, 0.0), a);
        assert_eq!(a.lerp(b, 1.0), b);
    }

    /// AC#9 — operator overloads.
    #[test]
    fn operator_overloads() {
        assert_eq!(
            Vec2::new(1.0, 2.0) + Vec2::new(3.0, 4.0),
            Vec2::new(4.0, 6.0)
        );
        assert_eq!(
            Vec2::new(3.0, 4.0) - Vec2::new(1.0, 1.0),
            Vec2::new(2.0, 3.0)
        );
        assert_eq!(Vec2::new(2.0, 3.0) * 2.0, Vec2::new(4.0, 6.0));
        assert_eq!(2.0 * Vec2::new(2.0, 3.0), Vec2::new(4.0, 6.0));
        assert_eq!(Vec2::new(6.0, 8.0) / 2.0, Vec2::new(3.0, 4.0));
        assert_eq!(-Vec2::new(1.0, -2.0), Vec2::new(-1.0, 2.0));
    }

    /// AC#10 — `Default` returns the origin.
    #[test]
    fn default_is_origin() {
        assert_eq!(Vec2::default(), Vec2::new(0.0, 0.0));
    }

    /// AC#11 — `approx_eq` tolerance behavior.
    #[test]
    fn approx_eq_tolerance() {
        assert!(Vec2::new(1.0, 1.0).approx_eq(Vec2::new(1.0 + 1e-10, 1.0 - 1e-10), EPSILON));
        assert!(!Vec2::new(1.0, 1.0).approx_eq(Vec2::new(1.0 + 1e-8, 1.0), EPSILON));
    }
}
