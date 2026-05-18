//! Line-based intersection routines: segment-segment (strict and infinite),
//! and segment-circle.
//!
//! See the parent module [`crate::geometry::intersect`] for the wider
//! contract and conventions.

use crate::geometry::circle::Circle;
use crate::geometry::epsilon::EPSILON;
use crate::geometry::line::Line;
use crate::geometry::vec2::Vec2;

/// Intersect two **segments**.
///
/// Returns `Some(point)` iff the two segments are non-parallel and the
/// unique infinite-line intersection lies within both segments (with
/// [`EPSILON`] parametric slack at the endpoints).
///
/// Returns `None` when:
///
/// - The two infinite lines are parallel (cross-product magnitude
///   `<= EPSILON`). Coincident (overlapping) segments are a special case of
///   parallel and also return `None`; reporting the overlap region is out
///   of scope (see LCV-014 notes).
/// - The infinite-line intersection point lies outside either segment's
///   parametric `[0, 1]` range, accounting for `EPSILON` slack.
pub fn line_line(a: &Line, b: &Line) -> Option<Vec2> {
    let (t, u, p) = solve_line_line(a, b)?;
    if (-EPSILON..=1.0 + EPSILON).contains(&t) && (-EPSILON..=1.0 + EPSILON).contains(&u) {
        Some(p)
    } else {
        None
    }
}

/// Intersect two **infinite lines** through the supplied segments' endpoints.
///
/// Returns `Some(point)` for any non-parallel pair. Returns `None` when the
/// two lines are parallel — including the coincident case, where infinitely
/// many intersection points exist and the function refuses to pick one
/// arbitrarily.
pub fn line_line_infinite(a: &Line, b: &Line) -> Option<Vec2> {
    solve_line_line(a, b).map(|(_, _, p)| p)
}

/// Intersect a **segment** with a circle.
///
/// Returns a `Vec` of 0, 1, or 2 points:
///
/// - 0 when the infinite line misses the circle, or when every infinite-line
///   intersection lies outside the segment's parametric `[0, 1]` range
///   (with [`EPSILON`] slack).
/// - 1 when the line is tangent within `EPSILON`, and the tangent point
///   lies within the segment.
/// - 2 when the line is a secant and both intersection points lie within
///   the segment.
///
/// Order is not guaranteed; callers that need a specific order sort the
/// result themselves.
pub fn line_circle(line: &Line, circle: &Circle) -> Vec<Vec2> {
    let d = line.p2 - line.p1;
    let len_sq = d.length_squared();
    if len_sq <= EPSILON * EPSILON {
        // Degenerate segment: treat as a point on (or off) the circle.
        let dist = (line.p1 - circle.center).length();
        if (dist - circle.r).abs() <= EPSILON {
            return vec![line.p1];
        }
        return vec![];
    }
    let f = line.p1 - circle.center;
    // Quadratic in t: (d·d) t² + 2 (f·d) t + (f·f − r²) = 0.
    let a = len_sq;
    let b_coef = 2.0 * f.dot(d);
    let c = f.length_squared() - circle.r * circle.r;
    let disc = b_coef * b_coef - 4.0 * a * c;
    if disc < -EPSILON {
        return vec![];
    }
    let mut out = Vec::with_capacity(2);
    if disc.abs() <= EPSILON {
        // Tangent.
        let t = -b_coef / (2.0 * a);
        if (-EPSILON..=1.0 + EPSILON).contains(&t) {
            out.push(line.p1 + d * t);
        }
        return out;
    }
    let sqrt_disc = disc.sqrt();
    let t1 = (-b_coef - sqrt_disc) / (2.0 * a);
    let t2 = (-b_coef + sqrt_disc) / (2.0 * a);
    if (-EPSILON..=1.0 + EPSILON).contains(&t1) {
        out.push(line.p1 + d * t1);
    }
    if (-EPSILON..=1.0 + EPSILON).contains(&t2) {
        out.push(line.p1 + d * t2);
    }
    out
}

/// Shared parametric solver for line-line intersections.
///
/// Returns `(t, u, point)` where `point = a.p1 + t (a.p2 − a.p1) = b.p1 + u
/// (b.p2 − b.p1)`. Returns `None` when the two direction vectors are
/// parallel (`|d_a × d_b| <= EPSILON`).
fn solve_line_line(a: &Line, b: &Line) -> Option<(f64, f64, Vec2)> {
    let da = a.p2 - a.p1;
    let db = b.p2 - b.p1;
    let denom = da.cross(db);
    if denom.abs() <= EPSILON {
        return None;
    }
    let delta = b.p1 - a.p1;
    let t = delta.cross(db) / denom;
    let u = delta.cross(da) / denom;
    Some((t, u, a.p1 + da * t))
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

    /// AC#2 — clearly crossing segments return the expected intersection.
    #[test]
    fn line_line_crossing_returns_intersection() {
        let a = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        let b = Line::new(Vec2::new(5.0, -5.0), Vec2::new(5.0, 5.0));
        let p = line_line(&a, &b).expect("segments cross");
        assert!(p.approx_eq(Vec2::new(5.0, 0.0), EPSILON));
    }

    /// AC#3 — parallel, non-coincident segments return `None`.
    #[test]
    fn line_line_parallel_returns_none() {
        let a = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        let b = Line::new(Vec2::new(0.0, 1.0), Vec2::new(10.0, 1.0));
        assert!(line_line(&a, &b).is_none());
    }

    /// AC#4 — coincident (overlapping) segments return `None` (documented
    /// contract: overlap reporting is out of scope).
    #[test]
    fn line_line_coincident_returns_none() {
        let a = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        let b = Line::new(Vec2::new(2.0, 0.0), Vec2::new(8.0, 0.0));
        assert!(line_line(&a, &b).is_none());
    }

    /// AC#5 — infinite-line intersection lies outside the segment ⇒ `None`.
    #[test]
    fn line_line_outside_segment_returns_none() {
        let a = Line::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
        let b = Line::new(Vec2::new(5.0, -5.0), Vec2::new(5.0, 5.0));
        assert!(line_line(&a, &b).is_none());
    }

    /// AC#6 — same setup as AC#5 but `line_line_infinite` returns the
    /// extended-line intersection.
    #[test]
    fn line_line_infinite_returns_intersection_outside_segment() {
        let a = Line::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
        let b = Line::new(Vec2::new(5.0, -5.0), Vec2::new(5.0, 5.0));
        let p = line_line_infinite(&a, &b).expect("non-parallel infinite lines");
        assert!(p.approx_eq(Vec2::new(5.0, 0.0), EPSILON));
    }

    /// AC#7 — parallel infinite lines return `None`.
    #[test]
    fn line_line_infinite_parallel_returns_none() {
        let a = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        let b = Line::new(Vec2::new(0.0, 1.0), Vec2::new(10.0, 1.0));
        assert!(line_line_infinite(&a, &b).is_none());
    }

    /// AC#8 — secant: two intersection points (order-independent).
    #[test]
    fn line_circle_secant_returns_two_points() {
        let line = Line::new(Vec2::new(-10.0, 0.0), Vec2::new(10.0, 0.0));
        let circle = Circle::new(Vec2::default(), 5.0);
        let pts = line_circle(&line, &circle);
        assert_same_set(&pts, &[Vec2::new(-5.0, 0.0), Vec2::new(5.0, 0.0)]);
    }

    /// AC#9 — tangent: exactly one intersection point.
    #[test]
    fn line_circle_tangent_returns_one_point() {
        let line = Line::new(Vec2::new(-10.0, 5.0), Vec2::new(10.0, 5.0));
        let circle = Circle::new(Vec2::default(), 5.0);
        let pts = line_circle(&line, &circle);
        assert_same_set(&pts, &[Vec2::new(0.0, 5.0)]);
    }

    /// AC#10 — no intersection: empty Vec.
    #[test]
    fn line_circle_no_intersection_returns_empty() {
        let line = Line::new(Vec2::new(-10.0, 6.0), Vec2::new(10.0, 6.0));
        let circle = Circle::new(Vec2::default(), 5.0);
        assert!(line_circle(&line, &circle).is_empty());
    }

    /// AC#11 — infinite line is secant but the segment lies entirely outside
    /// the circle ⇒ empty Vec.
    ///
    /// Note: the demand's literal sample (`Line(Vec2(6,-10), Vec2(6,-1))`,
    /// `Circle(Vec2(0,0), 10)`) describes a segment that actually crosses
    /// the circle at `(6, -8)` (since `y = -8 ∈ [-10, -1]`). To honor the
    /// AC's *stated intent* — "infinite-line intersects circle, but the
    /// segment lies entirely outside" — this test uses
    /// `Line(Vec2(6,-10), Vec2(6,-9))` so both endpoints sit outside the
    /// `r = 10` circle and the segment never crosses it.
    #[test]
    fn line_circle_intersections_outside_segment_returns_empty() {
        let line = Line::new(Vec2::new(6.0, -10.0), Vec2::new(6.0, -9.0));
        let circle = Circle::new(Vec2::default(), 10.0);
        // Sanity: the infinite line through x = 6 *does* hit the circle.
        let infinite_pts = line_circle(
            &Line::new(Vec2::new(6.0, -100.0), Vec2::new(6.0, 100.0)),
            &circle,
        );
        assert_eq!(infinite_pts.len(), 2);
        // The segment, however, lies entirely outside.
        assert!(line_circle(&line, &circle).is_empty());
    }
}
