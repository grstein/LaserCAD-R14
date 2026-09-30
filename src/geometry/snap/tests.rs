//! Unit tests for the snap engine. Mirrors the acceptance criteria from
//! demand LCV-016.

use super::*;
use core::f64::consts::{FRAC_PI_2, FRAC_PI_4};

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

// ── LCV-161: snap_query, Quadrant, Nearest, per-kind toggles ─────────────

/// Default kinds plus Nearest.
fn with_nearest() -> SnapKinds {
    SnapKinds {
        nearest: true,
        ..SnapKinds::default()
    }
}

/// LCV-161 AC8 — every kind is on by default except Nearest.
#[test]
fn snap_kinds_default_all_on_except_nearest() {
    let k = SnapKinds::default();
    for kind in [
        SnapKind::Endpoint,
        SnapKind::Midpoint,
        SnapKind::Center,
        SnapKind::Intersection,
        SnapKind::Quadrant,
        SnapKind::Perpendicular,
        SnapKind::Tangent,
    ] {
        assert!(k.contains(kind), "{kind:?} should be on by default");
    }
    assert!(!k.contains(SnapKind::Nearest));
}

/// LCV-161 AC1 — a circle's 90° point snaps as Quadrant.
#[test]
fn quadrant_on_circle() {
    let entities = [SnapEntity::Circle(Circle::new(Vec2::default(), 10.0))];
    let r = snap_query(
        Vec2::new(0.2, 10.1),
        1.0,
        &entities,
        None,
        SnapKinds::default(),
    )
    .expect("expected quadrant snap");
    assert_eq!(r.kind, SnapKind::Quadrant);
    assert!(r.point.approx_eq(Vec2::new(0.0, 10.0), 1e-9));
    assert_eq!(r.primary_idx, 0);
    assert_eq!(r.secondary_idx, None);
    // All four world-axis quadrants count.
    for q in [
        Vec2::new(10.0, 0.0),
        Vec2::new(-10.0, 0.0),
        Vec2::new(0.0, -10.0),
    ] {
        let r = snap_query(q, 0.5, &entities, None, SnapKinds::default()).expect("quadrant");
        assert_eq!(r.kind, SnapKind::Quadrant);
        assert!(r.point.approx_eq(q, 1e-9));
    }
}

/// LCV-161 AC1 — for an arc, only quadrant angles inside its sweep count.
#[test]
fn quadrant_on_arc_only_inside_sweep() {
    let arc = Arc::new(Vec2::default(), 10.0, FRAC_PI_4, 3.0 * FRAC_PI_4, true);
    let entities = [SnapEntity::Arc(arc)];
    let r = snap_query(
        Vec2::new(0.1, 9.8),
        1.0,
        &entities,
        None,
        SnapKinds::default(),
    )
    .expect("90° is inside the sweep");
    assert_eq!(r.kind, SnapKind::Quadrant);
    assert!(r.point.approx_eq(Vec2::new(0.0, 10.0), 1e-9));
    // 180° lies outside the sweep: no candidate there.
    let far = snap_query(
        Vec2::new(-10.0, 0.1),
        1.0,
        &entities,
        None,
        SnapKinds::default(),
    );
    assert!(far.is_none(), "quadrant outside the sweep must not snap");
}

/// LCV-161 AC5 — Nearest snaps to the closest point of a line only when on.
#[test]
fn nearest_on_line_only_when_enabled() {
    let entities = [SnapEntity::Line(Line::new(
        Vec2::default(),
        Vec2::new(100.0, 0.0),
    ))];
    let cursor = Vec2::new(30.0, 0.5);
    assert!(snap_query(cursor, 1.0, &entities, None, SnapKinds::default()).is_none());
    let r = snap_query(cursor, 1.0, &entities, None, with_nearest()).expect("nearest");
    assert_eq!(r.kind, SnapKind::Nearest);
    assert!(r.point.approx_eq(Vec2::new(30.0, 0.0), 1e-9));
    assert_eq!(r.primary_idx, 0);
}

/// LCV-161 AC5 — Nearest on circles and arcs, and the nearest entity wins.
#[test]
fn nearest_on_circle_and_arc_picks_closest_entity() {
    let entities = [
        SnapEntity::Circle(Circle::new(Vec2::default(), 10.0)),
        SnapEntity::Arc(Arc::new(Vec2::default(), 11.0, 0.0, FRAC_PI_2, true)),
    ];
    let dir = Vec2::new(1.0, 1.0) * (1.0 / 2.0_f64.sqrt());
    // 10.4 from the centre: 0.4 from the circle, 0.6 from the arc.
    let r = snap_query(dir * 10.4, 1.0, &entities, None, with_nearest()).expect("nearest");
    assert_eq!(r.kind, SnapKind::Nearest);
    assert_eq!(r.primary_idx, 0);
    assert!(r.point.approx_eq(dir * 10.0, 1e-9));
    // 10.7 from the centre: the arc is closer.
    let r = snap_query(dir * 10.7, 1.0, &entities, None, with_nearest()).expect("nearest");
    assert_eq!(r.primary_idx, 1);
    assert!(r.point.approx_eq(dir * 11.0, 1e-9));
    // Outside the arc's sweep only the circle is a Nearest candidate.
    let r = snap_query(dir * -10.7, 1.0, &entities, None, with_nearest()).expect("nearest");
    assert_eq!(r.primary_idx, 0);
}

/// LCV-161 AC5 — outside an arc's sweep, Nearest takes the arc's nearer end.
#[test]
fn nearest_on_arc_outside_sweep_is_nearer_endpoint() {
    let entities = [SnapEntity::Arc(Arc::new(
        Vec2::default(),
        10.0,
        0.0,
        FRAC_PI_2,
        true,
    ))];
    let kinds = only(SnapKind::Nearest);
    let r = snap_query(Vec2::new(10.3, -0.4), 1.0, &entities, None, kinds).expect("nearest");
    assert_eq!(r.kind, SnapKind::Nearest);
    assert!(r.point.approx_eq(Vec2::new(10.0, 0.0), 1e-9));
}

/// LCV-161 AC5 — Nearest never shadows a real candidate within the aperture.
#[test]
fn nearest_does_not_shadow_endpoint_in_range() {
    let entities = [SnapEntity::Line(Line::new(
        Vec2::default(),
        Vec2::new(100.0, 0.0),
    ))];
    // The line is 0.1 away, the endpoint 0.61 away: the endpoint still wins.
    let r = snap_query(Vec2::new(0.6, 0.1), 1.0, &entities, None, with_nearest()).expect("snap");
    assert_eq!(r.kind, SnapKind::Endpoint);
}

/// LCV-161 AC6 — distance ties break Endpoint > Intersection > Midpoint >
/// Center > Quadrant > Perpendicular > Tangent (Nearest never ties: it is
/// only computed when nothing else is in range).
#[test]
fn tie_order_covers_new_kinds() {
    let order = [
        SnapKind::Endpoint,
        SnapKind::Intersection,
        SnapKind::Midpoint,
        SnapKind::Center,
        SnapKind::Quadrant,
        SnapKind::Perpendicular,
        SnapKind::Tangent,
        SnapKind::Nearest,
    ];
    for w in order.windows(2) {
        assert!(
            priority(w[0]) < priority(w[1]),
            "{:?} before {:?}",
            w[0],
            w[1]
        );
    }
}

/// LCV-161 AC6 — Center beats Quadrant on a geometric tie.
#[test]
fn tie_center_beats_quadrant() {
    let entities = [
        SnapEntity::Circle(Circle::new(Vec2::default(), 5.0)),
        SnapEntity::Circle(Circle::new(Vec2::new(5.0, 0.0), 2.0)),
    ];
    let r = snap_query(
        Vec2::new(5.0, 0.0),
        1.0,
        &entities,
        None,
        SnapKinds::default(),
    )
    .expect("snap");
    assert_eq!(r.kind, SnapKind::Center);
    assert_eq!(r.primary_idx, 1);
}

/// LCV-161 AC9 — a disabled kind gives no candidate.
#[test]
fn disabled_kind_gives_no_candidate() {
    let line = [SnapEntity::Line(Line::new(
        Vec2::default(),
        Vec2::new(1.0, 0.0),
    ))];
    let no_end = SnapKinds {
        endpoint: false,
        ..SnapKinds::default()
    };
    let r = snap_query(Vec2::new(0.0, 0.1), 1.0, &line, None, no_end).expect("midpoint");
    assert_eq!(r.kind, SnapKind::Midpoint);

    let circle = [SnapEntity::Circle(Circle::new(Vec2::default(), 10.0))];
    let no_quad = SnapKinds {
        quadrant: false,
        ..SnapKinds::default()
    };
    assert!(snap_query(Vec2::new(0.0, 10.1), 1.0, &circle, None, no_quad).is_none());

    let mut none = SnapKinds::default();
    for kind in [
        SnapKind::Endpoint,
        SnapKind::Midpoint,
        SnapKind::Center,
        SnapKind::Intersection,
        SnapKind::Quadrant,
        SnapKind::Perpendicular,
        SnapKind::Tangent,
        SnapKind::Nearest,
    ] {
        none.set(kind, false);
    }
    assert!(snap_query(Vec2::new(0.0, 0.1), 1.0, &line, None, none).is_none());
    assert!(snap_query(Vec2::new(0.0, 10.1), 1.0, &circle, None, none).is_none());
}

// ── LCV-161: Perpendicular and Tangent from the tool anchor ──────────────

/// A set with only `kind` enabled.
fn only(kind: SnapKind) -> SnapKinds {
    let mut k = SnapKinds::default();
    for other in [
        SnapKind::Endpoint,
        SnapKind::Midpoint,
        SnapKind::Center,
        SnapKind::Intersection,
        SnapKind::Quadrant,
        SnapKind::Perpendicular,
        SnapKind::Tangent,
        SnapKind::Nearest,
    ] {
        k.set(other, other == kind);
    }
    k
}

/// LCV-161 AC2 — the foot on a segment snaps; a foot on the extension does not.
#[test]
fn perpendicular_foot_on_segment_only() {
    let entities = [SnapEntity::Line(Line::new(
        Vec2::default(),
        Vec2::new(10.0, 0.0),
    ))];
    let anchor = Some(Vec2::new(3.0, 8.0));
    let r = snap_query(
        Vec2::new(3.2, 0.3),
        1.0,
        &entities,
        anchor,
        SnapKinds::default(),
    )
    .expect("perpendicular foot");
    assert_eq!(r.kind, SnapKind::Perpendicular);
    assert!(r.point.approx_eq(Vec2::new(3.0, 0.0), 1e-9));
    assert_eq!(r.primary_idx, 0);
    // Foot at (15, 0) lies on the extension: no candidate.
    let ext = snap_query(
        Vec2::new(15.0, 0.2),
        1.0,
        &entities,
        Some(Vec2::new(15.0, 8.0)),
        SnapKinds::default(),
    );
    assert!(ext.is_none());
}

/// LCV-161 AC2 — a circle gives two feet on the line through centre and anchor.
#[test]
fn perpendicular_two_feet_on_circle() {
    let entities = [SnapEntity::Circle(Circle::new(Vec2::default(), 5.0))];
    let anchor = Some(Vec2::new(20.0, 20.0));
    let h = 5.0 / 2.0_f64.sqrt();
    for foot in [Vec2::new(h, h), Vec2::new(-h, -h)] {
        let cursor = foot + Vec2::new(0.2, -0.1);
        let r = snap_query(cursor, 1.0, &entities, anchor, SnapKinds::default()).expect("foot");
        assert_eq!(r.kind, SnapKind::Perpendicular);
        assert!(r.point.approx_eq(foot, 1e-9));
    }
}

/// LCV-161 AC2 — on an arc only the feet inside the sweep count.
#[test]
fn perpendicular_feet_on_arc_inside_sweep_only() {
    let entities = [SnapEntity::Arc(Arc::new(
        Vec2::default(),
        5.0,
        0.0,
        FRAC_PI_2,
        true,
    ))];
    let anchor = Some(Vec2::new(20.0, 20.0));
    let h = 5.0 / 2.0_f64.sqrt();
    let r = snap_query(
        Vec2::new(h, h),
        1.0,
        &entities,
        anchor,
        SnapKinds::default(),
    )
    .expect("foot inside the sweep");
    assert_eq!(r.kind, SnapKind::Perpendicular);
    let outside = snap_query(
        Vec2::new(-h, -h),
        1.0,
        &entities,
        anchor,
        SnapKinds::default(),
    );
    assert!(outside.is_none());
}

/// LCV-161 AC3 — two tangent points on a circle from an outside anchor.
#[test]
fn tangent_points_on_circle_from_outside_anchor() {
    let entities = [SnapEntity::Circle(Circle::new(Vec2::default(), 5.0))];
    let anchor = Some(Vec2::new(10.0, 0.0));
    let y = 5.0 * (FRAC_PI_2 / 1.5).sin(); // 5·sin 60°
    for t in [Vec2::new(2.5, y), Vec2::new(2.5, -y)] {
        let r = snap_query(
            t + Vec2::new(-0.2, 0.1),
            1.0,
            &entities,
            anchor,
            SnapKinds::default(),
        )
        .expect("tangent");
        assert_eq!(r.kind, SnapKind::Tangent);
        assert!(r.point.approx_eq(t, 1e-9));
        assert_eq!(r.primary_idx, 0);
    }
}

/// LCV-161 AC3 — on an arc only tangent points inside the sweep count.
#[test]
fn tangent_points_on_arc_inside_sweep_only() {
    let entities = [SnapEntity::Arc(Arc::new(
        Vec2::default(),
        5.0,
        0.0,
        FRAC_PI_2,
        true,
    ))];
    let anchor = Some(Vec2::new(10.0, 0.0));
    let y = 5.0 * (FRAC_PI_2 / 1.5).sin();
    let r = snap_query(
        Vec2::new(2.5, y),
        1.0,
        &entities,
        anchor,
        SnapKinds::default(),
    )
    .expect("tangent inside the sweep");
    assert_eq!(r.kind, SnapKind::Tangent);
    let outside = snap_query(
        Vec2::new(2.5, -y),
        1.0,
        &entities,
        anchor,
        SnapKinds::default(),
    );
    assert!(outside.is_none());
}

/// LCV-161 AC4 — no anchor, anchor on or inside the circle, or anchor at
/// the centre: no Tangent / Perpendicular candidate.
#[test]
fn anchored_kinds_need_a_usable_anchor() {
    let entities = [SnapEntity::Circle(Circle::new(Vec2::default(), 5.0))];
    let y = 5.0 * (FRAC_PI_2 / 1.5).sin();
    let tangent_pt = Vec2::new(2.5, y);
    // No anchor.
    for kind in [SnapKind::Perpendicular, SnapKind::Tangent] {
        assert!(snap_query(tangent_pt, 1.0, &entities, None, only(kind)).is_none());
    }
    // Anchor on the circle (at the 60° point) or inside it: no tangent.
    for anchor in [tangent_pt, Vec2::new(1.0, 1.0)] {
        for cursor in [tangent_pt, Vec2::new(2.5, -y), Vec2::new(-5.0, 0.0)] {
            let r = snap_query(
                cursor,
                1.0,
                &entities,
                Some(anchor),
                only(SnapKind::Tangent),
            );
            assert!(r.is_none(), "anchor {anchor:?} cursor {cursor:?}");
        }
    }
    // Anchor at the centre: no perpendicular foot on the circle.
    let r = snap_query(
        tangent_pt,
        1.0,
        &entities,
        Some(Vec2::default()),
        only(SnapKind::Perpendicular),
    );
    assert!(r.is_none());
}

/// LCV-161 AC2/AC3 — feet and tangent points outside the aperture do not count.
#[test]
fn anchored_candidates_respect_the_aperture() {
    let entities = [SnapEntity::Circle(Circle::new(Vec2::default(), 5.0))];
    let y = 5.0 * (FRAC_PI_2 / 1.5).sin();
    let cursor = Vec2::new(2.5 + 1.5, y);
    let anchor = Some(Vec2::new(10.0, 0.0));
    assert!(snap_query(cursor, 1.0, &entities, anchor, only(SnapKind::Tangent)).is_none());
    let foot_cursor = Vec2::new(-5.0, 1.5);
    assert!(
        snap_query(
            foot_cursor,
            1.0,
            &entities,
            anchor,
            only(SnapKind::Perpendicular)
        )
        .is_none()
    );
}

/// LCV-161 AC6 — Quadrant beats Perpendicular, Perpendicular beats Tangent.
#[test]
fn tie_quadrant_perpendicular_tangent() {
    // Anchor on the +X axis: the circle's perpendicular foot is its quadrant.
    let circle = [SnapEntity::Circle(Circle::new(Vec2::default(), 5.0))];
    let r = snap_query(
        Vec2::new(5.0, 0.0),
        1.0,
        &circle,
        Some(Vec2::new(20.0, 0.0)),
        SnapKinds::default(),
    )
    .expect("snap");
    assert_eq!(r.kind, SnapKind::Quadrant);

    // A radial segment through the arc's tangent point T: the anchor's foot
    // on the segment is T too (AT ⟂ CT). Arc-line intersections are skipped.
    let u = Vec2::new(0.5, 3.0_f64.sqrt() / 2.0);
    let t = u * 5.0;
    let entities = [
        SnapEntity::Arc(Arc::new(Vec2::default(), 5.0, 0.0, FRAC_PI_2, true)),
        SnapEntity::Line(Line::new(u * 2.5, u * 10.0)),
    ];
    let r = snap_query(
        t,
        1.0,
        &entities,
        Some(Vec2::new(10.0, 0.0)),
        SnapKinds::default(),
    )
    .expect("snap");
    assert_eq!(r.kind, SnapKind::Perpendicular);
    assert_eq!(r.primary_idx, 1);
    let r = snap_query(
        t,
        1.0,
        &entities,
        Some(Vec2::new(10.0, 0.0)),
        only(SnapKind::Tangent),
    )
    .expect("tangent alone");
    assert_eq!(r.kind, SnapKind::Tangent);
    assert_eq!(r.primary_idx, 0);
}
