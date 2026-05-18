//! Circle-circle intersection.
//!
//! See the parent module [`crate::geometry::intersect`] for the wider
//! contract and conventions.

use crate::geometry::circle::Circle;
use crate::geometry::epsilon::EPSILON;
use crate::geometry::vec2::Vec2;

/// Intersect two circles.
///
/// Returns a `Vec` of 0, 1, or 2 points:
///
/// - 0 when the circles are separated (`d > r_a + r_b + EPSILON`), nested
///   without touching (`d < |r_a − r_b| − EPSILON`), or concentric (centers
///   within `EPSILON` of each other, regardless of radius match —
///   coincident circles have infinitely many "intersections" and we refuse
///   to pick).
/// - 1 when externally tangent (`|d − (r_a + r_b)| <= EPSILON`) or
///   internally tangent (`|d − |r_a − r_b|| <= EPSILON`).
/// - 2 otherwise — the standard radical-axis pair.
pub fn circle_circle(a: &Circle, b: &Circle) -> Vec<Vec2> {
    let delta = b.center - a.center;
    let d_sq = delta.length_squared();
    if d_sq <= EPSILON * EPSILON {
        // Concentric — coincident-or-nested-at-center. Refuse to pick.
        return vec![];
    }
    let d = d_sq.sqrt();
    let sum_r = a.r + b.r;
    let diff_r = (a.r - b.r).abs();
    if d > sum_r + EPSILON || d < diff_r - EPSILON {
        return vec![];
    }
    // External or internal tangent within EPSILON: one point on the line of
    // centers at distance r_a from a.center toward b.center.
    if (d - sum_r).abs() <= EPSILON || (d - diff_r).abs() <= EPSILON {
        let dir = delta / d;
        return vec![a.center + dir * a.r];
    }
    // Two-point case via the radical-axis formula.
    let a_coef = (a.r * a.r - b.r * b.r + d_sq) / (2.0 * d);
    let h_sq = a.r * a.r - a_coef * a_coef;
    let h = if h_sq < 0.0 { 0.0 } else { h_sq.sqrt() };
    let dir = delta / d;
    let mid = a.center + dir * a_coef;
    let perp = Vec2::new(-dir.y, dir.x);
    vec![mid + perp * h, mid - perp * h]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Assert two `Vec<Vec2>` describe the same set of points within
    /// `EPSILON`, ignoring order.
    fn assert_same_set(got: &[Vec2], expected: &[Vec2]) {
        assert_eq!(
            got.len(),
            expected.len(),
            "got = {got:?}, expected = {expected:?}"
        );
        for exp in expected {
            assert!(
                got.iter().any(|g| g.approx_eq(*exp, EPSILON)),
                "missing {exp:?} in {got:?}",
            );
        }
    }

    /// AC#1 — the four public functions are reachable through the parent
    /// module's re-exports, exercising the declared signatures end-to-end.
    #[test]
    fn public_api_surface() {
        use crate::geometry::intersect::{
            circle_circle, line_circle, line_line, line_line_infinite,
        };
        use crate::geometry::line::Line;
        let la = Line::new(Vec2::default(), Vec2::new(1.0, 0.0));
        let lb = Line::new(Vec2::new(0.5, -0.5), Vec2::new(0.5, 0.5));
        let c1 = Circle::new(Vec2::default(), 1.0);
        let c2 = Circle::new(Vec2::new(1.5, 0.0), 1.0);
        let _: Option<Vec2> = line_line(&la, &lb);
        let _: Option<Vec2> = line_line_infinite(&la, &lb);
        let _: Vec<Vec2> = line_circle(&la, &c1);
        let _: Vec<Vec2> = circle_circle(&c1, &c2);
    }

    /// AC#12 — two circle-circle intersections (order-independent).
    #[test]
    fn circle_circle_two_intersections() {
        let a = Circle::new(Vec2::default(), 5.0);
        let b = Circle::new(Vec2::new(8.0, 0.0), 5.0);
        let pts = circle_circle(&a, &b);
        assert_same_set(&pts, &[Vec2::new(4.0, 3.0), Vec2::new(4.0, -3.0)]);
    }

    /// AC#13 — external tangent.
    #[test]
    fn circle_circle_external_tangent() {
        let a = Circle::new(Vec2::default(), 5.0);
        let b = Circle::new(Vec2::new(10.0, 0.0), 5.0);
        let pts = circle_circle(&a, &b);
        assert_same_set(&pts, &[Vec2::new(5.0, 0.0)]);
    }

    /// AC#14 — internal tangent.
    #[test]
    fn circle_circle_internal_tangent() {
        let a = Circle::new(Vec2::default(), 10.0);
        let b = Circle::new(Vec2::new(5.0, 0.0), 5.0);
        let pts = circle_circle(&a, &b);
        assert_same_set(&pts, &[Vec2::new(10.0, 0.0)]);
    }

    /// AC#15 — separated circles.
    #[test]
    fn circle_circle_separated_returns_empty() {
        let a = Circle::new(Vec2::default(), 1.0);
        let b = Circle::new(Vec2::new(10.0, 0.0), 1.0);
        assert!(circle_circle(&a, &b).is_empty());
    }

    /// AC#16 — nested, non-tangent.
    #[test]
    fn circle_circle_nested_returns_empty() {
        let a = Circle::new(Vec2::default(), 10.0);
        let b = Circle::new(Vec2::new(2.0, 0.0), 1.0);
        assert!(circle_circle(&a, &b).is_empty());
    }

    /// AC#17 — concentric circles: equal-radii (coincident) and unequal.
    #[test]
    fn circle_circle_concentric_returns_empty() {
        let a = Circle::new(Vec2::default(), 5.0);
        assert!(circle_circle(&a, &Circle::new(Vec2::default(), 5.0)).is_empty());
        assert!(circle_circle(&a, &Circle::new(Vec2::default(), 3.0)).is_empty());
    }
}
