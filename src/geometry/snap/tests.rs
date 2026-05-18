//! Unit tests for the snap engine. Mirrors the acceptance criteria from
//! demand LCV-016.

use super::*;
use core::f64::consts::FRAC_PI_2;

/// AC#1 — public surface is constructible via literal syntax.
#[test]
fn public_surface_constructible_via_literal() {
    let _kind = SnapKind::Endpoint;
    let _ent = SnapEntity::Line(Line::new(Vec2::default(), Vec2::new(1.0, 0.0)));
    let _res = SnapResult {
        point: Vec2::default(),
        kind: SnapKind::Endpoint,
        primary_idx: 0,
        secondary_idx: None,
    };
}

/// AC#2 — empty entity slice returns `None`.
#[test]
fn snap_empty_entities_returns_none() {
    assert!(snap(Vec2::default(), 1.0, &[]).is_none());
}

/// AC#3 — every candidate lies outside the tolerance ring ⇒ `None`.
#[test]
fn snap_outside_tolerance_returns_none() {
    let entities = [SnapEntity::Line(Line::new(
        Vec2::new(10.0, 10.0),
        Vec2::new(20.0, 10.0),
    ))];
    assert!(snap(Vec2::default(), 1.0, &entities).is_none());
}

/// AC#4 — endpoint of a line snaps when the cursor is within tolerance.
#[test]
fn snap_endpoint_of_line() {
    let entities = [SnapEntity::Line(Line::new(
        Vec2::new(10.0, 10.0),
        Vec2::new(20.0, 10.0),
    ))];
    let r = snap(Vec2::new(10.1, 10.1), 1.0, &entities).expect("expected endpoint snap");
    assert!(r.point.approx_eq(Vec2::new(10.0, 10.0), EPSILON));
    assert_eq!(r.kind, SnapKind::Endpoint);
    assert_eq!(r.primary_idx, 0);
    assert_eq!(r.secondary_idx, None);
}

/// AC#5 — midpoint of a line snaps when the cursor is within tolerance and
/// no closer endpoint exists.
#[test]
fn snap_midpoint_of_line() {
    let entities = [SnapEntity::Line(Line::new(
        Vec2::default(),
        Vec2::new(10.0, 0.0),
    ))];
    let r = snap(Vec2::new(5.1, 0.1), 1.0, &entities).expect("expected midpoint snap");
    assert!(r.point.approx_eq(Vec2::new(5.0, 0.0), EPSILON));
    assert_eq!(r.kind, SnapKind::Midpoint);
    assert_eq!(r.primary_idx, 0);
    assert_eq!(r.secondary_idx, None);
}

/// AC#6 — center of a circle snaps when the cursor is near.
#[test]
fn snap_center_of_circle() {
    let entities = [SnapEntity::Circle(Circle::new(Vec2::new(5.0, 5.0), 3.0))];
    let r = snap(Vec2::new(5.2, 4.9), 1.0, &entities).expect("expected center snap");
    assert!(r.point.approx_eq(Vec2::new(5.0, 5.0), EPSILON));
    assert_eq!(r.kind, SnapKind::Center);
    assert_eq!(r.primary_idx, 0);
    assert_eq!(r.secondary_idx, None);
}

/// AC#7 — line-line intersection produces an `Intersection` candidate with
/// `secondary_idx = Some(higher)`.
#[test]
fn snap_intersection_of_two_lines() {
    let entities = [
        SnapEntity::Line(Line::new(Vec2::default(), Vec2::new(10.0, 0.0))),
        SnapEntity::Line(Line::new(Vec2::new(5.0, -5.0), Vec2::new(5.0, 5.0))),
    ];
    // The cursor at (5.1, 0.1) is closer to the intersection (5, 0) than to
    // any other candidate (midpoint of entities[0] is also at (5, 0), but
    // Intersection beats Midpoint on the priority tie-break).
    let r = snap(Vec2::new(5.1, 0.1), 1.0, &entities).expect("expected intersection snap");
    assert!(r.point.approx_eq(Vec2::new(5.0, 0.0), EPSILON));
    assert_eq!(r.kind, SnapKind::Intersection);
    assert_eq!(r.primary_idx, 0);
    assert_eq!(r.secondary_idx, Some(1));
}

/// AC#8 — Endpoint beats Midpoint on equal-distance tie.
#[test]
fn tie_endpoint_beats_midpoint() {
    let entities = [
        SnapEntity::Line(Line::new(Vec2::default(), Vec2::new(10.0, 0.0))),
        SnapEntity::Line(Line::new(Vec2::new(5.0, 0.0), Vec2::new(5.0, 10.0))),
    ];
    // Cursor at (5, 0) is exactly on entities[0].midpoint() and on
    // entities[1].p1 (endpoint).
    let r = snap(Vec2::new(5.0, 0.0), 1.0, &entities).expect("expected snap");
    assert_eq!(r.kind, SnapKind::Endpoint);
    assert_eq!(r.primary_idx, 1);
}

/// AC#9 — Intersection beats Midpoint on equal-distance tie.
#[test]
fn tie_intersection_beats_midpoint() {
    let entities = [
        SnapEntity::Line(Line::new(Vec2::default(), Vec2::new(10.0, 0.0))),
        SnapEntity::Line(Line::new(Vec2::new(5.0, -5.0), Vec2::new(5.0, 5.0))),
    ];
    // Cursor at (5, 0) sits on both entities[0].midpoint and the line-line
    // intersection. Intersection wins.
    let r = snap(Vec2::new(5.0, 0.0), 1.0, &entities).expect("expected snap");
    assert_eq!(r.kind, SnapKind::Intersection);
}

/// AC#10 — Endpoint beats Intersection on equal-distance tie.
#[test]
fn tie_endpoint_beats_intersection() {
    let entities = [
        SnapEntity::Line(Line::new(Vec2::default(), Vec2::new(10.0, 0.0))),
        SnapEntity::Line(Line::new(Vec2::new(0.0, -5.0), Vec2::new(0.0, 5.0))),
    ];
    // Cursor at (0, 0) sits on entities[0].p1 (endpoint) and on the
    // line-line intersection. Endpoint wins.
    let r = snap(Vec2::default(), 1.0, &entities).expect("expected snap");
    assert_eq!(r.kind, SnapKind::Endpoint);
}

/// AC#11 — a closer candidate of a lower-priority kind beats a farther
/// candidate of a higher-priority kind; priority only breaks distance ties.
#[test]
fn closer_non_priority_wins_over_farther_priority() {
    // entities[0] midpoint at (5, 0) — distance 0.1 from cursor (5.1, 0).
    // entities[1] endpoint at (5.5, 0) — distance 0.4 from cursor.
    let entities = [
        SnapEntity::Line(Line::new(Vec2::default(), Vec2::new(10.0, 0.0))),
        SnapEntity::Line(Line::new(Vec2::new(5.5, 0.0), Vec2::new(5.5, 10.0))),
    ];
    let r = snap(Vec2::new(5.1, 0.0), 1.0, &entities).expect("expected snap");
    assert_eq!(r.kind, SnapKind::Midpoint);
    assert!(r.point.approx_eq(Vec2::new(5.0, 0.0), EPSILON));
}

/// AC#12 — two equal-priority, equal-distance candidates: lower
/// `primary_idx` wins.
#[test]
fn equal_priority_equal_distance_returns_lower_index() {
    // Two lines that share endpoint (0, 0). Cursor at (0, 0).
    let entities = [
        SnapEntity::Line(Line::new(Vec2::default(), Vec2::new(10.0, 0.0))),
        SnapEntity::Line(Line::new(Vec2::default(), Vec2::new(0.0, 10.0))),
    ];
    let r = snap(Vec2::default(), 1.0, &entities).expect("expected snap");
    assert_eq!(r.kind, SnapKind::Endpoint);
    assert_eq!(r.primary_idx, 0);
}

/// AC#13 — arc start_point is an endpoint candidate.
#[test]
fn snap_arc_endpoint() {
    let entities = [SnapEntity::Arc(Arc::new(
        Vec2::default(),
        1.0,
        0.0,
        FRAC_PI_2,
        true,
    ))];
    let r = snap(Vec2::new(1.05, 0.05), 1.0, &entities).expect("expected arc endpoint snap");
    assert!(r.point.approx_eq(Vec2::new(1.0, 0.0), EPSILON));
    assert_eq!(r.kind, SnapKind::Endpoint);
    assert_eq!(r.primary_idx, 0);
    assert_eq!(r.secondary_idx, None);
}

/// AC#14 — arc center is a center candidate.
#[test]
fn snap_arc_center() {
    let entities = [SnapEntity::Arc(Arc::new(
        Vec2::default(),
        1.0,
        0.0,
        FRAC_PI_2,
        true,
    ))];
    let r = snap(Vec2::new(0.1, 0.1), 1.0, &entities).expect("expected arc center snap");
    assert!(r.point.approx_eq(Vec2::default(), EPSILON));
    assert_eq!(r.kind, SnapKind::Center);
    assert_eq!(r.primary_idx, 0);
    assert_eq!(r.secondary_idx, None);
}

/// AC#15 — arc-involving pairs do NOT produce Intersection candidates.
#[test]
fn arc_involving_intersection_pairs_skipped() {
    // A quarter-arc on the unit circle (in the first quadrant) and a
    // horizontal line that visibly crosses it at (cos(π/4), sin(π/4)).
    let y = FRAC_PI_2.sin() / 2.0_f64.sqrt();
    let entities = [
        SnapEntity::Arc(Arc::new(Vec2::default(), 1.0, 0.0, FRAC_PI_2, true)),
        SnapEntity::Line(Line::new(Vec2::new(-2.0, y), Vec2::new(2.0, y))),
    ];
    // Cursor near the visible arc-line crossing (about (0.707, 0.707)).
    let cursor = Vec2::new(0.71, 0.71);
    if let Some(r) = snap(cursor, 0.5, &entities) {
        assert_ne!(
            r.kind,
            SnapKind::Intersection,
            "arc-involving pair must not produce an Intersection candidate"
        );
    }
}
