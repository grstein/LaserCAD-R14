//! Axis-aligned rectangle value type in mm space plus the eight
//! contains/crosses predicates feeding the future SelectTool (LCV-042).
//!
//! Predicates split in two semantic families, mirroring AutoCAD's
//! window-box / crossing-box distinction:
//!
//! - `contains_*` — target is *fully* inside the rectangle.
//! - `crosses_*`  — target *overlaps* the rectangle in any way
//!   (including a target fully inside or merely touching an edge).
//!
//! Predicates are exact (no `EPSILON` slack on `contains_point`); the UI
//! applies pixel-radius tolerance before converting to world space.
//! [`EPSILON`] is only used where the math is distance-vs-radius.
//!
//! Frozen by demand LCV-015.

use crate::geometry::arc::Arc;
use crate::geometry::circle::Circle;
use crate::geometry::epsilon::EPSILON;
use crate::geometry::intersect;
use crate::geometry::line::Line;
use crate::geometry::vec2::Vec2;

/// Axis-aligned rectangle in millimeter space, with `min.x <= max.x` and
/// `min.y <= max.y` enforced by [`Rect::new`].
///
/// Field names mirror v1's TypeScript `geometry/rect.ts` (`min`, `max`).
/// Equality (`==`) is bit-exact `f64` comparison on both corners; no
/// `Eq` / `Hash` (transitively `f64`).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rect {
    /// Lower-left corner (smallest x and y).
    pub min: Vec2,
    /// Upper-right corner (largest x and y).
    pub max: Vec2,
}

impl Rect {
    /// Construct an axis-aligned rectangle from any two corner points.
    ///
    /// Auto-normalizes so `min = (min(p1.x, p2.x), min(p1.y, p2.y))` and
    /// `max = (max(p1.x, p2.x), max(p1.y, p2.y))`. Cannot be `const fn`
    /// because `f64::min` / `f64::max` are not `const`.
    pub fn new(p1: Vec2, p2: Vec2) -> Self {
        Self {
            min: Vec2::new(p1.x.min(p2.x), p1.y.min(p2.y)),
            max: Vec2::new(p1.x.max(p2.x), p1.y.max(p2.y)),
        }
    }

    /// Width: `max.x - min.x`. Always `>= 0` thanks to the constructor's
    /// auto-normalization.
    pub fn width(&self) -> f64 {
        self.max.x - self.min.x
    }

    /// Height: `max.y - min.y`. Always `>= 0`.
    pub fn height(&self) -> f64 {
        self.max.y - self.min.y
    }

    /// Bounding box of the rectangle as `(min, max)`. Trivial but kept for
    /// API symmetry with `Line::bbox`, `Circle::bbox`, `Arc::bbox`.
    pub fn bbox(&self) -> (Vec2, Vec2) {
        (self.min, self.max)
    }

    /// True iff `p` lies inside the closed rectangle (inclusive bounds).
    ///
    /// No `EPSILON` slack — picking tolerance is a UI concern, applied
    /// before conversion to world coordinates.
    pub fn contains_point(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    /// True iff both endpoints of `line` lie inside the rectangle.
    pub fn contains_line(&self, line: &Line) -> bool {
        self.contains_point(line.p1) && self.contains_point(line.p2)
    }

    /// True iff `line` overlaps the rectangle in any way: fully inside,
    /// partially inside, or merely touching an edge. Returns `true` if
    /// either endpoint is inside, or if the segment crosses any of the
    /// four rect edges (via [`intersect::line_line`]).
    pub fn crosses_line(&self, line: &Line) -> bool {
        if self.contains_point(line.p1) || self.contains_point(line.p2) {
            return true;
        }
        for edge in self.edges() {
            if intersect::line_line(line, &edge).is_some() {
                return true;
            }
        }
        false
    }

    /// True iff the circle's axis-aligned bounding box fits entirely inside
    /// the rectangle. For an axis-aligned rect this is exact for full
    /// containment of a circle.
    pub fn contains_circle(&self, c: &Circle) -> bool {
        self.min.x <= c.center.x - c.r
            && c.center.x + c.r <= self.max.x
            && self.min.y <= c.center.y - c.r
            && c.center.y + c.r <= self.max.y
    }

    /// True iff the rectangle and the disk bounded by `c` overlap (closest
    /// point on the rect to `c.center` lies within `c.r + EPSILON`).
    pub fn crosses_circle(&self, c: &Circle) -> bool {
        let cp = Vec2::new(
            c.center.x.clamp(self.min.x, self.max.x),
            c.center.y.clamp(self.min.y, self.max.y),
        );
        cp.distance(c.center) <= c.r + EPSILON
    }

    /// True iff the arc's axis-aligned bounding box fits entirely inside
    /// the rectangle. Uses [`Arc::bbox`] which already accounts for the
    /// cardinal extreme angles inside the arc's sweep.
    pub fn contains_arc(&self, a: &Arc) -> bool {
        let (amin, amax) = a.bbox();
        self.min.x <= amin.x && amax.x <= self.max.x && self.min.y <= amin.y && amax.y <= self.max.y
    }

    /// True iff the arc overlaps the rectangle in any way. Returns `true`
    /// if [`Rect::contains_arc`] holds, if either arc endpoint is inside
    /// the rect, or if any rect edge intersects the parent circle at an
    /// angle within the arc's sweep ([`Arc::contains_angle`]). A bbox
    /// disjointness check short-circuits the "entirely outside" case.
    pub fn crosses_arc(&self, a: &Arc) -> bool {
        if self.contains_arc(a) {
            return true;
        }
        let (amin, amax) = a.bbox();
        if !self.bboxes_overlap(amin, amax) {
            return false;
        }
        if self.contains_point(a.start_point()) || self.contains_point(a.end_point()) {
            return true;
        }
        let parent = Circle::new(a.center, a.r);
        for edge in self.edges() {
            for p in intersect::line_circle(&edge, &parent) {
                let v = p - a.center;
                let angle = v.y.atan2(v.x);
                if a.contains_angle(angle) {
                    return true;
                }
            }
        }
        false
    }

    /// Four edges of the rectangle, each as a `Line`, listed in CCW order
    /// starting from the bottom edge.
    fn edges(&self) -> [Line; 4] {
        let bl = self.min;
        let br = Vec2::new(self.max.x, self.min.y);
        let tr = self.max;
        let tl = Vec2::new(self.min.x, self.max.y);
        [
            Line::new(bl, br),
            Line::new(br, tr),
            Line::new(tr, tl),
            Line::new(tl, bl),
        ]
    }

    /// True iff this rect's bbox and the supplied `(min, max)` overlap on
    /// both axes (inclusive).
    fn bboxes_overlap(&self, other_min: Vec2, other_max: Vec2) -> bool {
        self.min.x <= other_max.x
            && other_min.x <= self.max.x
            && self.min.y <= other_max.y
            && other_min.y <= self.max.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::{FRAC_PI_2, PI};

    /// AC#1 — `Rect` has `pub` fields and can be constructed via struct literal.
    #[test]
    fn struct_literal_construction() {
        let r = Rect {
            min: Vec2::default(),
            max: Vec2::new(1.0, 1.0),
        };
        assert_eq!(r.min, Vec2::default());
        assert_eq!(r.max, Vec2::new(1.0, 1.0));
    }

    /// AC#2 — `new` auto-normalizes corners in any input order.
    #[test]
    fn new_auto_normalizes_corners() {
        let r = Rect::new(Vec2::new(5.0, 3.0), Vec2::new(1.0, 7.0));
        assert_eq!(r.min, Vec2::new(1.0, 3.0));
        assert_eq!(r.max, Vec2::new(5.0, 7.0));
    }

    /// AC#3 — `width` and `height` on the normalized rect from AC#2.
    #[test]
    fn width_and_height_of_normalized_rect() {
        let r = Rect::new(Vec2::new(5.0, 3.0), Vec2::new(1.0, 7.0));
        assert_eq!(r.width(), 4.0);
        assert_eq!(r.height(), 4.0);
    }

    /// AC#4 — `bbox` returns `(min, max)`.
    #[test]
    fn bbox_returns_min_max_tuple() {
        let r = Rect::new(Vec2::new(5.0, 3.0), Vec2::new(1.0, 7.0));
        assert_eq!(r.bbox(), (r.min, r.max));
    }

    /// AC#5 — `contains_point`: PASS inside + on boundary, FAIL outside.
    #[test]
    fn contains_point_inside_boundary_outside() {
        let r = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        assert!(r.contains_point(Vec2::new(5.0, 5.0)));
        assert!(r.contains_point(Vec2::new(10.0, 10.0))); // boundary inclusive
        assert!(!r.contains_point(Vec2::new(11.0, 5.0)));
    }

    /// AC#6 — `contains_line`: PASS both inside, FAIL one endpoint outside.
    #[test]
    fn contains_line_both_endpoints_inside_vs_one_outside() {
        let r = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        assert!(r.contains_line(&Line::new(Vec2::new(2.0, 2.0), Vec2::new(8.0, 8.0))));
        assert!(!r.contains_line(&Line::new(Vec2::new(2.0, 2.0), Vec2::new(15.0, 8.0))));
    }

    /// AC#7 — `crosses_line`: through-PASS, outside-FAIL, fully-inside-PASS.
    #[test]
    fn crosses_line_through_vs_outside_vs_fully_inside() {
        let r = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        // Segment crosses through the rect.
        assert!(r.crosses_line(&Line::new(Vec2::new(-5.0, 5.0), Vec2::new(15.0, 5.0))));
        // Segment entirely to the left.
        assert!(!r.crosses_line(&Line::new(Vec2::new(-5.0, 5.0), Vec2::new(-1.0, 5.0))));
        // Segment fully inside also counts as crossing.
        assert!(r.crosses_line(&Line::new(Vec2::new(2.0, 2.0), Vec2::new(8.0, 8.0))));
    }

    /// AC#8 — `contains_circle`: inscribed PASS, oversized FAIL.
    #[test]
    fn contains_circle_inscribed_vs_overflows() {
        let r = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        assert!(r.contains_circle(&Circle::new(Vec2::new(5.0, 5.0), 3.0)));
        assert!(!r.contains_circle(&Circle::new(Vec2::new(5.0, 5.0), 6.0)));
    }

    /// AC#9 — `crosses_circle`: overlap PASS, clear FAIL.
    #[test]
    fn crosses_circle_overlap_vs_clear() {
        let r = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        // Circle straddles the right edge.
        assert!(r.crosses_circle(&Circle::new(Vec2::new(11.0, 5.0), 2.0)));
        // Circle entirely outside.
        assert!(!r.crosses_circle(&Circle::new(Vec2::new(15.0, 5.0), 2.0)));
    }

    /// AC#10 — `contains_arc`: quarter-arc bbox inside PASS, overflows FAIL.
    #[test]
    fn contains_arc_quarter_inside_vs_overflows() {
        let r = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let inside = Arc::new(Vec2::new(5.0, 5.0), 2.0, 0.0, FRAC_PI_2, true);
        assert!(r.contains_arc(&inside));
        let outside = Arc::new(Vec2::new(9.0, 5.0), 2.0, 0.0, FRAC_PI_2, true);
        assert!(!r.contains_arc(&outside));
    }

    /// AC#11 — `crosses_arc`: endpoint inside PASS, entirely outside FAIL.
    /// Arc center (11,5) r=3, sweep [π, 3π/2] CCW: start (8,5) is inside,
    /// end (11,2) is outside the rect.
    #[test]
    fn crosses_arc_endpoint_inside_vs_entirely_outside() {
        let r = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let crossing = Arc::new(Vec2::new(11.0, 5.0), 3.0, PI, 3.0 * FRAC_PI_2, true);
        assert!(r.crosses_arc(&crossing));
        let far = Arc::new(Vec2::new(20.0, 20.0), 1.0, 0.0, FRAC_PI_2, true);
        assert!(!r.crosses_arc(&far));
    }

    /// Extra: an arc whose bbox overlaps the rect but whose sweep stays
    /// outside should not count as crossing (exercises the angle filter).
    /// Center (12,5) r=4, sweep [0, π/2]: start (16,5), end (12,9) both
    /// outside, and the parent's left extent at angle π is not in sweep.
    #[test]
    fn crosses_arc_bbox_overlaps_but_sweep_is_outside() {
        let r = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let a = Arc::new(Vec2::new(12.0, 5.0), 4.0, 0.0, FRAC_PI_2, true);
        assert!(!r.crosses_arc(&a));
    }
}
