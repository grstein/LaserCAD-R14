//! Cross-module integration tests for the geometry kernel (LCV-017).
//!
//! Each scenario walks at least two kernel modules end-to-end, catching the
//! "module A's change silently breaks module B's assumption" class of bug.
//! Per-type unit tests live next to each module in `src/geometry/*.rs`; this
//! file is a smoke-test surface, NOT a replacement.

use core::f64::consts::FRAC_PI_2;
use lasercad::geometry::{
    Arc, Circle, EPSILON, Line, Rect, SnapEntity, SnapKind, Vec2, circle_circle, line_circle, snap,
};

/// Scenario 1 — Line + Circle + intersect + snap.
///
/// A horizontal line meets a circle at two symmetric points. The intersection
/// routine yields both, and the snap engine (with the same entities) picks
/// the intersection nearest the cursor with `secondary_idx = Some(other)`.
#[test]
fn line_circle_intersect_then_snap() {
    let line = Line::new(Vec2::new(-10.0, 0.0), Vec2::new(10.0, 0.0));
    let circle = Circle::new(Vec2::new(0.0, 0.0), 5.0);
    let pts = line_circle(&line, &circle);
    assert_eq!(pts.len(), 2);
    let expected = [Vec2::new(-5.0, 0.0), Vec2::new(5.0, 0.0)];
    for p in &pts {
        assert!(expected.iter().any(|e| p.approx_eq(*e, EPSILON)));
    }

    let entities = [SnapEntity::Line(line), SnapEntity::Circle(circle)];
    let r = snap(Vec2::new(5.05, 0.05), 0.5, &entities).expect("snap near intersection");
    assert_eq!(r.kind, SnapKind::Intersection);
    assert!(r.point.approx_eq(Vec2::new(5.0, 0.0), EPSILON));
    assert_eq!(r.primary_idx, 0);
    assert_eq!(r.secondary_idx, Some(1));
}

/// Scenario 2 — Rect + Arc + Arc::bbox feeding Rect::contains_arc / crosses_arc.
///
/// A quarter arc tucked inside a rect is fully contained; translating the
/// arc rightward so its sweep pokes out of the rect makes `contains_arc`
/// false but `crosses_arc` true.
#[test]
fn rect_contains_then_crosses_arc_after_translation() {
    let r = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
    let inside = Arc::new(Vec2::new(5.0, 5.0), 2.0, 0.0, FRAC_PI_2, true);
    assert!(r.contains_arc(&inside));
    assert!(r.crosses_arc(&inside)); // fully-contained implies crossing.

    // Slide the arc so its +X extreme pokes outside the rect.
    let outside = Arc::new(Vec2::new(9.0, 5.0), 2.0, 0.0, FRAC_PI_2, true);
    assert!(!r.contains_arc(&outside));
    assert!(r.crosses_arc(&outside));
}

/// Scenario 3 — Line + Line + snap midpoint disambiguates by index.
///
/// Two non-overlapping lines have distinct midpoints. A cursor near one of
/// them must produce a `Midpoint` snap pointing at the correct primary line.
#[test]
fn snap_midpoint_distinguishes_two_lines() {
    let entities = [
        SnapEntity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(4.0, 0.0))),
        SnapEntity::Line(Line::new(Vec2::new(0.0, 10.0), Vec2::new(20.0, 10.0))),
    ];
    let r = snap(Vec2::new(10.05, 10.05), 0.5, &entities).expect("midpoint of entities[1]");
    assert_eq!(r.kind, SnapKind::Midpoint);
    assert!(r.point.approx_eq(Vec2::new(10.0, 10.0), EPSILON));
    assert_eq!(r.primary_idx, 1);
    assert_eq!(r.secondary_idx, None);
}

/// Scenario 4 — Circle + Circle + intersect + snap intersection.
///
/// Two overlapping circles intersect at two known points; the snap engine
/// fed both circles reports an `Intersection` candidate with
/// `secondary_idx = Some(other)` at the chosen pair.
#[test]
fn two_circles_intersect_then_snap() {
    let a = Circle::new(Vec2::new(0.0, 0.0), 5.0);
    let b = Circle::new(Vec2::new(8.0, 0.0), 5.0);
    let pts = circle_circle(&a, &b);
    assert_eq!(pts.len(), 2);
    let expected = [Vec2::new(4.0, 3.0), Vec2::new(4.0, -3.0)];
    for p in &pts {
        assert!(expected.iter().any(|e| p.approx_eq(*e, EPSILON)));
    }

    let entities = [SnapEntity::Circle(a), SnapEntity::Circle(b)];
    let r = snap(Vec2::new(4.05, 3.05), 0.5, &entities).expect("snap near intersection");
    assert_eq!(r.kind, SnapKind::Intersection);
    assert!(r.point.approx_eq(Vec2::new(4.0, 3.0), EPSILON));
    assert_eq!(r.primary_idx, 0);
    assert_eq!(r.secondary_idx, Some(1));
}

/// Scenario 5 — Vec2 + Line + Rect: a polyline-style bbox walk.
///
/// Build three connected segments using `Vec2` arithmetic, take each
/// segment's bbox via `Line::bbox`, derive the overall bbox, then assert
/// every segment is contained by the resulting `Rect`.
#[test]
fn polyline_walk_bbox_contains_each_segment() {
    let p0 = Vec2::new(0.0, 0.0);
    let step = Vec2::new(3.0, 0.0);
    let up = Vec2::new(0.0, 4.0);
    let p1 = p0 + step; // (3, 0)
    let p2 = p1 + up; // (3, 4)
    let p3 = p2 - step; // (0, 4)
    let segments = [Line::new(p0, p1), Line::new(p1, p2), Line::new(p2, p3)];

    let mut lo = p0;
    let mut hi = p0;
    for seg in &segments {
        let (smin, smax) = seg.bbox();
        lo = Vec2::new(lo.x.min(smin.x), lo.y.min(smin.y));
        hi = Vec2::new(hi.x.max(smax.x), hi.y.max(smax.y));
    }
    let rect = Rect::new(lo, hi);
    assert_eq!(rect.min, Vec2::new(0.0, 0.0));
    assert_eq!(rect.max, Vec2::new(3.0, 4.0));
    for seg in &segments {
        assert!(rect.contains_line(seg));
    }
}
