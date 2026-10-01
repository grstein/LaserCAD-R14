//! LCV-194 — the agent's `measure` tool: exact answers for every query on a
//! fixture drawing (AC 1–6), refusals in the LCV-192 shape (AC 8), and one
//! read-only step that does not trip the fence (AC 7).
//!
//! ADR 0002 §A2: `App::default()` only. ADR 0005: no dialog is armed and no
//! test sends `Ctrl+O` / `Ctrl+S`. ADR 0006: no per-user path is injected.

use lasercad::agent::{AgentAction, AgentOutcome, MeasureQuery, MeasureRequest, MeasureTargets};
use lasercad::app::{App, apply};
use lasercad::document::{AddLayer, CreateArc, CreateCircle, CreateLine};
use lasercad::geometry::{Arc, Circle, Line, Vec2};

use MeasureQuery::{Angle, Bbox, Distance, Intersections, Length};

fn line(app: &mut App, a: (f64, f64), b: (f64, f64)) {
    let l = Line::new(Vec2::new(a.0, a.1), Vec2::new(b.0, b.1));
    app.commit(Box::new(CreateLine::new(l)));
}

/// The fixture, by index:
/// 0 line (0,0)–(40,0); 1 line (40,0)–(40,20), touching 0;
/// 2 circle (20,10) r 5; 3 arc (20,20) r 10 from 0° to 180° on the
/// Output-off layer `Engrave`, bulging to y = 30;
/// 4 line (0,20)–(40,20), through both arc ends; 5 line (10,0)–(30,0), on 0;
/// 6 line (40,30)–(0,30), antiparallel to 0.
fn fixture() -> App {
    let mut app = App::default();
    line(&mut app, (0.0, 0.0), (40.0, 0.0));
    line(&mut app, (40.0, 0.0), (40.0, 20.0));
    let circle = Circle::new(Vec2::new(20.0, 10.0), 5.0);
    app.commit(Box::new(CreateCircle::new(circle)));
    app.commit(Box::new(AddLayer::new("Engrave", [0, 0, 255], false)));
    let engrave = app.document.layer_by_name("Engrave").expect("added").id;
    let arc = Arc::new(
        Vec2::new(20.0, 20.0),
        10.0,
        0.0,
        core::f64::consts::PI,
        true,
    );
    app.commit(Box::new(CreateArc::new(arc).on_layer(engrave)));
    line(&mut app, (0.0, 20.0), (40.0, 20.0));
    line(&mut app, (10.0, 0.0), (30.0, 0.0));
    line(&mut app, (40.0, 30.0), (0.0, 30.0));
    app
}

fn request(query: MeasureQuery, points: &[(f64, f64)], targets: MeasureTargets) -> AgentAction {
    let points = points.iter().map(|&(x, y)| Vec2::new(x, y)).collect();
    AgentAction::Measure(MeasureRequest {
        query,
        points,
        targets,
    })
}

fn at(indices: &[usize]) -> MeasureTargets {
    MeasureTargets::Indices(indices.to_vec())
}

/// The answer text, which must be `Ok`.
fn ok(
    app: &mut App,
    query: MeasureQuery,
    points: &[(f64, f64)],
    targets: MeasureTargets,
) -> String {
    match apply(app, &request(query, points, targets)) {
        AgentOutcome::Ok(text) => text,
        other => panic!("{other:?}"),
    }
}

/// The refusal text, which must be `Refused`.
fn refused(app: &mut App, query: MeasureQuery, targets: MeasureTargets) -> String {
    match apply(app, &request(query, &[], targets)) {
        AgentOutcome::Refused(text) => text,
        other => panic!("{other:?}"),
    }
}

/// AC 1 — point–point, point–entity and entity–entity, with touching
/// entities at 0; the points come in operand order, points first.
#[test]
fn distance_between_points_and_entities() {
    let mut app = fixture();
    let cases = [
        (
            ok(&mut app, Distance, &[(0.0, 0.0), (3.0, 4.0)], at(&[])),
            "distance: 5.000 mm, dx 3.000, dy 4.000, from (0.000, 0.000) to (3.000, 4.000)",
        ),
        (
            ok(&mut app, Distance, &[(20.0, 30.0)], at(&[2])),
            "distance: 15.000 mm, dx 0.000, dy -15.000, from (20.000, 30.000) to (20.000, 15.000)",
        ),
        (
            ok(&mut app, Distance, &[], at(&[0, 2])),
            "distance: 5.000 mm, dx 0.000, dy 5.000, from (20.000, 0.000) to (20.000, 5.000)",
        ),
        (
            ok(&mut app, Distance, &[], at(&[0, 1])),
            "distance: 0.000 mm, dx 0.000, dy 0.000, from (40.000, 0.000) to (40.000, 0.000)",
        ),
        (
            ok(&mut app, Distance, &[], MeasureTargets::Ids(vec![4, 1])),
            "distance: 20.000 mm, dx 0.000, dy -20.000, from (30.000, 20.000) to (30.000, 0.000)",
        ),
    ];
    for (got, want) in cases {
        assert_eq!(got, want);
    }
}

/// AC 2 — a line's length, an arc's arc length, a circle's circumference.
#[test]
fn length_of_a_line_an_arc_and_a_circle() {
    let mut app = fixture();
    assert_eq!(ok(&mut app, Length, &[], at(&[0])), "length: 40.000 mm");
    assert_eq!(ok(&mut app, Length, &[], at(&[3])), "length: 31.416 mm");
    assert_eq!(ok(&mut app, Length, &[], at(&[2])), "length: 31.416 mm");
    let by_id = ok(&mut app, Length, &[], MeasureTargets::Ids(vec![2]));
    assert_eq!(by_id, "length: 20.000 mm");
}

/// AC 3 — listed entities, and the whole drawing including the arc on the
/// Output-off layer and its bulge.
#[test]
fn bbox_of_listed_entities_and_of_the_whole_drawing() {
    let mut app = fixture();
    assert_eq!(
        ok(&mut app, Bbox, &[], at(&[0, 2])),
        "bbox: min (0.000, 0.000), max (40.000, 15.000), width 40.000 mm, height 15.000 mm"
    );
    assert_eq!(
        ok(&mut app, Bbox, &[], at(&[3])),
        "bbox: min (10.000, 20.000), max (30.000, 30.000), width 20.000 mm, height 10.000 mm"
    );
    assert_eq!(
        ok(&mut app, Bbox, &[], at(&[])),
        "bbox: min (0.000, 0.000), max (40.000, 30.000), width 40.000 mm, height 30.000 mm"
    );
}

/// AC 4 — an empty drawing.
#[test]
fn bbox_of_an_empty_drawing() {
    let mut app = App::default();
    assert_eq!(
        ok(&mut app, Bbox, &[], at(&[])),
        "bbox: the drawing is empty"
    );
}

/// AC 5 — crossing points, none, and an overlap.
#[test]
fn intersections_points_none_and_overlap() {
    let mut app = fixture();
    assert_eq!(
        ok(&mut app, Intersections, &[], at(&[4, 3])),
        "intersections: (10.000, 20.000); (30.000, 20.000)"
    );
    assert_eq!(
        ok(&mut app, Intersections, &[], at(&[0, 2])),
        "intersections: none"
    );
    assert_eq!(
        ok(&mut app, Intersections, &[], at(&[0, 5])),
        "intersections: overlap"
    );
    assert_eq!(
        ok(&mut app, Intersections, &[], at(&[0, 1])),
        "intersections: (40.000, 0.000)"
    );
}

/// AC 6 — directed and undirected: 90/270, parallel, antiparallel.
#[test]
fn angle_directed_and_undirected() {
    let mut app = fixture();
    let cases = [
        (
            &[0, 1],
            "angle: 90.000° counter-clockwise from the first line to the second; 90.000° between the lines",
        ),
        (
            &[1, 0],
            "angle: 270.000° counter-clockwise from the first line to the second; 90.000° between the lines",
        ),
        (
            &[0, 4],
            "angle: 0.000° counter-clockwise from the first line to the second; 0.000° between the lines",
        ),
        (
            &[0, 6],
            "angle: 180.000° counter-clockwise from the first line to the second; 0.000° between the lines",
        ),
    ];
    for (indices, want) in cases {
        assert_eq!(ok(&mut app, Angle, &[], at(indices)), want, "{indices:?}");
    }
}

/// AC 6 — a directed angle a hair under a full turn prints 0.000, and a
/// hair under zero prints no minus sign.
#[test]
fn angle_and_bbox_print_no_minus_zero_and_no_full_turn() {
    let mut app = App::default();
    line(&mut app, (0.0, 0.0), (40.0, 0.0));
    line(&mut app, (0.0, 0.0), (40.0, -0.00001));
    assert_eq!(
        ok(&mut app, Angle, &[], at(&[0, 1])),
        "angle: 0.000° counter-clockwise from the first line to the second; 0.000° between the lines"
    );
    assert_eq!(
        ok(&mut app, Bbox, &[], at(&[])),
        "bbox: min (0.000, 0.000), max (40.000, 0.000), width 40.000 mm, height 0.000 mm"
    );
}

/// AC 8 — kinds that do not fit, an unknown index and an unknown id.
#[test]
fn measure_refuses_wrong_kinds_and_unknown_entities() {
    let mut app = fixture();
    assert_eq!(
        refused(&mut app, Angle, at(&[0, 2])),
        "measure indices[1]: is a circle; expected a line"
    );
    assert_eq!(
        refused(&mut app, Angle, MeasureTargets::Ids(vec![4, 1])),
        "measure ids[0]: is an arc; expected a line"
    );
    assert_eq!(
        refused(&mut app, Length, at(&[99])),
        "measure indices[0]: 99 is out of range; expected 0..=6 (the drawing has 7 entities)"
    );
    assert_eq!(
        refused(&mut app, Intersections, MeasureTargets::Ids(vec![1, 999])),
        "measure ids[1]: e999 is not in the drawing; expected an id listed by query_entities"
    );
    line(&mut app, (5.0, 5.0), (5.0, 5.0));
    assert_eq!(
        refused(&mut app, Angle, at(&[0, 7])),
        "measure indices[1]: is a zero-length line; expected a line with two distinct endpoints"
    );
}
