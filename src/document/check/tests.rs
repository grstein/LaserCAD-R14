use std::f64::consts::{FRAC_PI_2, PI};

use super::*;
use crate::document::{AddLayer, Command, Entity};
use crate::geometry::{Arc, Circle, EPSILON, Line};

const BED: [f64; 2] = [400.0, 300.0];

fn p(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

fn line(x1: f64, y1: f64, x2: f64, y2: f64) -> Entity {
    Entity::Line(Line::new(p(x1, y1), p(x2, y2)))
}

fn arc(cx: f64, cy: f64, r: f64, a0: f64, a1: f64, ccw: bool) -> Entity {
    Entity::Arc(Arc::new(p(cx, cy), r, a0, a1, ccw))
}

fn circle(cx: f64, cy: f64, r: f64) -> Entity {
    Entity::Circle(Circle::new(p(cx, cy), r))
}

fn doc_of(entities: &[Entity]) -> Document {
    let mut doc = Document::with_bed(BED);
    for e in entities {
        doc.push_current(*e);
    }
    doc
}

/// Rectangle (10,10)-(60,40) as four chained lines; `dx` shifts the start
/// of the closing side so the last corner opens by `dx` mm.
fn rectangle(dx: f64) -> Vec<Entity> {
    vec![
        line(10.0, 10.0, 60.0, 10.0),
        line(60.0, 10.0, 60.0, 40.0),
        line(60.0, 40.0, 10.0, 40.0),
        line(10.0, 40.0, 10.0 + dx, 10.0),
    ]
}

fn open_ends(r: &CheckReport) -> Vec<(usize, Vec2)> {
    r.findings
        .iter()
        .filter_map(|f| match f {
            Finding::OpenEnd { index, at } => Some((*index, *at)),
            _ => None,
        })
        .collect()
}

fn gaps(r: &CheckReport) -> Vec<(usize, usize, f64)> {
    r.findings
        .iter()
        .filter_map(|f| match f {
            Finding::Gap { a, b, width, .. } => Some((*a, *b, *width)),
            _ => None,
        })
        .collect()
}

/// AC 2 — an open L-polyline has its two far ends open.
#[test]
fn open_l_polyline_has_two_open_ends() {
    let doc = doc_of(&[line(10.0, 10.0, 50.0, 10.0), line(50.0, 10.0, 50.0, 40.0)]);
    let r = check_drawing(&doc);
    assert_eq!(open_ends(&r), vec![(0, p(10.0, 10.0)), (1, p(50.0, 40.0))]);
    assert_eq!(r.findings.len(), 2);
}

/// AC 2 — a closed rectangle reports nothing.
#[test]
fn closed_rectangle_is_clean() {
    assert_eq!(
        check_drawing(&doc_of(&rectangle(0.0))),
        CheckReport::default()
    );
}

/// AC 2 — a closed contour of lines and a CCW fillet arc reports nothing.
#[test]
fn line_and_arc_fillet_contour_is_clean() {
    let doc = doc_of(&[
        line(10.0, 10.0, 50.0, 10.0),
        arc(50.0, 20.0, 10.0, -FRAC_PI_2, 0.0, true),
        line(60.0, 20.0, 60.0, 60.0),
        line(60.0, 60.0, 10.0, 60.0),
        line(10.0, 60.0, 10.0, 10.0),
    ]);
    assert_eq!(check_drawing(&doc), CheckReport::default());
}

/// AC 3 — a 0.3 mm opening is one gap, not two open ends.
#[test]
fn small_opening_is_one_gap() {
    let r = check_drawing(&doc_of(&rectangle(0.3)));
    let g = gaps(&r);
    assert_eq!(g.len(), 1);
    assert_eq!((g[0].0, g[0].1), (0, 3));
    assert!((g[0].2 - 0.3).abs() < 1e-9);
    assert!(open_ends(&r).is_empty());
    assert_eq!(r.findings.len(), 1);
}

/// AC 3 — the gap carries its midpoint.
#[test]
fn gap_midpoint_lies_between_the_ends() {
    let r = check_drawing(&doc_of(&rectangle(0.3)));
    let Some(Finding::Gap { mid, .. }) = r.findings.first() else {
        panic!("expected a gap, got {r:?}");
    };
    assert!(mid.approx_eq(p(10.15, 10.0), 1e-9), "{mid:?}");
}

/// AC 2/3 — a 0.6 mm opening is two open ends, no gap.
#[test]
fn wide_opening_is_two_open_ends() {
    let r = check_drawing(&doc_of(&rectangle(0.6)));
    assert!(gaps(&r).is_empty());
    assert_eq!(open_ends(&r), vec![(0, p(10.0, 10.0)), (3, p(10.6, 10.0))]);
}

/// AC 3 — a gap is *closer than* 0.5 mm: an opening of exactly `GAP_MM` is
/// two open ends.
#[test]
fn opening_of_exactly_gap_mm_is_two_open_ends() {
    let r = check_drawing(&doc_of(&rectangle(GAP_MM)));
    assert!(gaps(&r).is_empty(), "{r:?}");
    assert_eq!(open_ends(&r), vec![(0, p(10.0, 10.0)), (3, p(10.5, 10.0))]);
}

/// AC 2 — ends within `EPSILON` of each other meet.
#[test]
fn ends_within_epsilon_meet() {
    let r = check_drawing(&doc_of(&rectangle(EPSILON / 2.0)));
    assert_eq!(r, CheckReport::default());
}

/// AC 2 — ends `2·EPSILON` apart do not meet: they form a gap.
#[test]
fn ends_past_epsilon_do_not_meet() {
    let r = check_drawing(&doc_of(&rectangle(2.0 * EPSILON)));
    assert_eq!(gaps(&r).len(), 1);
}

/// AC 2 — the foot of a T-junction touches a line's middle, not an end, so it
/// is open.
#[test]
fn t_junction_end_is_open() {
    let doc = doc_of(&[line(10.0, 10.0, 50.0, 10.0), line(30.0, 10.0, 30.0, 40.0)]);
    let ends = open_ends(&check_drawing(&doc));
    assert_eq!(ends.len(), 4);
    assert!(ends.contains(&(1, p(30.0, 10.0))));
}

/// AC 3 — gaps pair nearest first: the 0.1 mm pair wins over the 0.2 mm one,
/// and the left-over end is open.
#[test]
fn gaps_pair_nearest_first() {
    let doc = doc_of(&[
        line(10.0, 10.0, 20.0, 10.0),
        line(20.1, 10.0, 30.0, 50.0),
        line(19.8, 10.0, 10.0, 80.0),
    ]);
    let r = check_drawing(&doc);
    let g = gaps(&r);
    assert_eq!(g.len(), 1, "{r:?}");
    assert_eq!((g[0].0, g[0].1), (0, 1));
    assert!(open_ends(&r).contains(&(2, p(19.8, 10.0))));
}

/// AC 2 — a circle has no ends; a closed contour plus a circle is clean.
#[test]
fn circles_have_no_ends() {
    let mut entities = rectangle(0.0);
    entities.push(circle(100.0, 100.0, 20.0));
    assert_eq!(check_drawing(&doc_of(&entities)), CheckReport::default());
}

/// Scope — entities on an Output-off layer are ignored.
#[test]
fn output_off_layer_is_ignored() {
    let mut doc = Document::with_bed(BED);
    let mut add = AddLayer::new("Draft", [9, 9, 9], false);
    add.do_(&mut doc);
    let off = add.id().expect("allocated by do_");
    doc.push_entity(line(10.0, 10.0, 50.0, 10.0), off);
    doc.push_entity(arc(100.0, 100.0, 10.0, 0.0, PI, true), off);
    assert_eq!(check_drawing(&doc), CheckReport::default());
}

fn duplicates(r: &CheckReport) -> Vec<(usize, usize)> {
    r.findings
        .iter()
        .filter_map(|f| match f {
            Finding::Duplicate { index, of, .. } => Some((*index, *of)),
            _ => None,
        })
        .collect()
}

/// AC 4 — a line drawn the other way round duplicates the first; the finding
/// sits at the later line's start.
#[test]
fn reversed_line_is_a_duplicate() {
    let doc = doc_of(&[line(10.0, 10.0, 50.0, 10.0), line(50.0, 10.0, 10.0, 10.0)]);
    let r = check_drawing(&doc);
    assert!(r.findings.contains(&Finding::Duplicate {
        index: 1,
        of: 0,
        at: p(50.0, 10.0)
    }));
}

/// AC 4 — the same circle twice is a duplicate, located at its centre.
#[test]
fn same_circle_is_a_duplicate() {
    let doc = doc_of(&[circle(100.0, 100.0, 20.0), circle(100.0, 100.0, 20.0)]);
    let r = check_drawing(&doc);
    assert_eq!(
        r.findings,
        vec![Finding::Duplicate {
            index: 1,
            of: 0,
            at: p(100.0, 100.0)
        }]
    );
}

/// AC 4 — a circle with another radius or centre is no duplicate.
#[test]
fn different_circle_is_not_a_duplicate() {
    let doc = doc_of(&[
        circle(100.0, 100.0, 20.0),
        circle(100.0, 100.0, 20.1),
        circle(100.1, 100.0, 20.0),
        circle(100.0, 100.1, 20.0),
    ]);
    assert_eq!(check_drawing(&doc), CheckReport::default());
}

/// AC 4 — a CW arc duplicates its CCW twin over the same span.
#[test]
fn cw_arc_duplicates_its_ccw_twin() {
    let doc = doc_of(&[
        arc(100.0, 100.0, 10.0, 0.0, FRAC_PI_2, true),
        arc(100.0, 100.0, 10.0, FRAC_PI_2, 0.0, false),
    ]);
    assert_eq!(duplicates(&check_drawing(&doc)), vec![(1, 0)]);
}

/// AC 4 — two arcs over the same angles but different directions, or over a
/// different span, are not duplicates.
#[test]
fn different_span_is_not_a_duplicate() {
    let doc = doc_of(&[
        arc(100.0, 100.0, 10.0, 0.0, FRAC_PI_2, true),
        arc(100.0, 100.0, 10.0, 0.0, FRAC_PI_2, false),
        arc(100.0, 100.0, 10.0, 0.0, PI / 3.0, true),
        arc(100.0, 100.0, 11.0, 0.0, FRAC_PI_2, true),
        arc(100.0, 101.0, 10.0, 0.0, FRAC_PI_2, true),
    ]);
    assert!(duplicates(&check_drawing(&doc)).is_empty());
}

/// AC 4 — a line that differs at one end is no duplicate.
#[test]
fn line_differing_at_one_end_is_not_a_duplicate() {
    let doc = doc_of(&[line(10.0, 10.0, 50.0, 10.0), line(10.0, 10.0, 50.0, 10.1)]);
    assert!(duplicates(&check_drawing(&doc)).is_empty());
    let doc = doc_of(&[line(10.0, 10.0, 50.0, 10.0), line(10.1, 10.0, 50.0, 10.0)]);
    assert!(duplicates(&check_drawing(&doc)).is_empty());
}

/// AC 4 — three copies give two findings, both against the lowest index.
#[test]
fn three_copies_point_at_the_lowest_index() {
    let l = line(10.0, 10.0, 50.0, 10.0);
    let r = check_drawing(&doc_of(&[l, l, l]));
    assert_eq!(duplicates(&r), vec![(1, 0), (2, 0)]);
}

/// AC 2/4 — a doubled open line still shows its own open ends: the later copy
/// leaves the endpoint analysis instead of closing the first.
#[test]
fn doubled_open_line_keeps_its_open_ends() {
    let doc = doc_of(&[line(10.0, 10.0, 50.0, 10.0), line(10.0, 10.0, 50.0, 10.0)]);
    let r = check_drawing(&doc);
    assert_eq!(open_ends(&r), vec![(0, p(10.0, 10.0)), (0, p(50.0, 10.0))]);
    assert_eq!(duplicates(&r), vec![(1, 0)]);
}

/// AC 4 — a closed contour drawn twice reports one duplicate per side and
/// no open end.
#[test]
fn doubled_rectangle_reports_duplicates_only() {
    let mut entities = rectangle(0.0);
    entities.extend(rectangle(0.0));
    let r = check_drawing(&doc_of(&entities));
    assert_eq!(duplicates(&r), vec![(4, 0), (5, 1), (6, 2), (7, 3)]);
    assert_eq!(r.findings.len(), 4);
}

/// AC 5 — a zero-length line is degenerate, located at its point, and has
/// no open ends.
#[test]
fn zero_length_line_is_degenerate() {
    let r = check_drawing(&doc_of(&[line(10.0, 10.0, 10.0, 10.0)]));
    assert_eq!(
        r.findings,
        vec![Finding::Degenerate {
            index: 0,
            at: p(10.0, 10.0)
        }]
    );
}

/// AC 5 — a line just longer than `EPSILON` is not degenerate.
#[test]
fn short_line_is_not_degenerate() {
    let r = check_drawing(&doc_of(&[line(10.0, 10.0, 10.0 + 2.0 * EPSILON, 10.0)]));
    assert!(
        !r.findings
            .iter()
            .any(|f| matches!(f, Finding::Degenerate { .. }))
    );
}

/// AC 5 — a zero-span arc is degenerate, located at its start point.
#[test]
fn zero_span_arc_is_degenerate() {
    let r = check_drawing(&doc_of(&[arc(100.0, 100.0, 10.0, 0.0, 0.0, true)]));
    assert_eq!(
        r.findings,
        vec![Finding::Degenerate {
            index: 0,
            at: p(110.0, 100.0)
        }]
    );
}

/// AC 5 — a zero-radius circle or arc is degenerate.
#[test]
fn zero_radius_circle_and_arc_are_degenerate() {
    let r = check_drawing(&doc_of(&[
        circle(100.0, 100.0, 0.0),
        arc(150.0, 100.0, 0.0, 0.0, PI, true),
    ]));
    assert_eq!(
        r.findings,
        vec![
            Finding::Degenerate {
                index: 0,
                at: p(100.0, 100.0)
            },
            Finding::Degenerate {
                index: 1,
                at: p(150.0, 100.0)
            },
        ]
    );
}

/// AC 2/5 — a degenerate line on a free end does not close it, and two
/// degenerate copies are reported as degenerate only.
#[test]
fn degenerate_entities_leave_the_other_analyses() {
    let doc = doc_of(&[
        line(10.0, 10.0, 50.0, 10.0),
        line(50.0, 10.0, 50.0, 10.0),
        line(50.0, 10.0, 50.0, 10.0),
    ]);
    let r = check_drawing(&doc);
    assert_eq!(open_ends(&r), vec![(0, p(10.0, 10.0)), (0, p(50.0, 10.0))]);
    assert!(duplicates(&r).is_empty());
    assert_eq!(r.findings.len(), 4);
}

/// AC 6 — a line past the right edge is off-bed, with its bbox.
#[test]
fn line_past_right_edge_is_off_bed() {
    let r = check_drawing(&doc_of(&[line(390.0, 10.0, 410.0, 10.0)]));
    assert!(r.findings.contains(&Finding::OffBed {
        index: 0,
        min: p(390.0, 10.0),
        max: p(410.0, 10.0)
    }));
}

/// AC 6 — an arc with both endpoints inside whose bulge crosses y = 0 is
/// off-bed.
#[test]
fn arc_bulge_below_zero_is_off_bed() {
    let r = check_drawing(&doc_of(&[arc(100.0, 5.0, 10.0, PI, 2.0 * PI, true)]));
    assert!(
        r.findings
            .iter()
            .any(|f| matches!(f, Finding::OffBed { index: 0, .. })),
        "{r:?}"
    );
}

/// AC 6 — a closed contour touching every bed edge exactly is clean.
#[test]
fn contour_on_the_bed_edges_is_clean() {
    let doc = doc_of(&[
        line(0.0, 0.0, 400.0, 0.0),
        line(400.0, 0.0, 400.0, 300.0),
        line(400.0, 300.0, 0.0, 300.0),
        line(0.0, 300.0, 0.0, 0.0),
    ]);
    assert_eq!(check_drawing(&doc), CheckReport::default());
}

/// AC 7 — a clean drawing prints exactly one line.
#[test]
fn clean_report_prints_no_problems_found() {
    let lines = check_drawing(&doc_of(&rectangle(0.0))).lines();
    assert_eq!(lines, vec!["CHECK: no problems found.".to_string()]);
    assert_eq!(
        CheckReport::default().lines(),
        vec!["CHECK: no problems found."]
    );
}

/// AC 1 — summary lines in kind order with counts and plurals, then one line
/// per finding by kind then index, coordinates at three decimals.
#[test]
fn report_lines_summary_then_findings() {
    let doc = doc_of(&[
        line(10.0, 10.0, 50.0, 10.0),      // 0: two open ends
        line(10.0, 10.0, 50.0, 10.0),      // 1: duplicate of 0
        circle(100.0, 100.0, 0.0),         // 2: degenerate
        line(390.0, 50.0, 410.0, 50.0),    // 3: open ends, off-bed
        line(100.0, 200.0, 150.0, 200.0),  // 4
        line(150.25, 200.0, 150.0, 250.0), // 5: gap with 4, open end
    ]);
    let lines = check_drawing(&doc).lines();
    assert_eq!(
        lines,
        vec![
            "CHECK: 6 open ends",
            "CHECK: 1 gap",
            "CHECK: 1 duplicate",
            "CHECK: 1 degenerate entity",
            "CHECK: 1 off-bed entity",
            "open end: entity 0 at (10.000, 10.000) mm",
            "open end: entity 0 at (50.000, 10.000) mm",
            "open end: entity 3 at (390.000, 50.000) mm",
            "open end: entity 3 at (410.000, 50.000) mm",
            "open end: entity 4 at (100.000, 200.000) mm",
            "open end: entity 5 at (150.000, 250.000) mm",
            "gap: entities 4 and 5, 0.250 mm wide at (150.125, 200.000) mm",
            "duplicate: entity 1 of entity 0 at (10.000, 10.000) mm",
            "degenerate: entity 2 at (100.000, 100.000) mm",
            "off-bed: entity 3 from (390.000, 50.000) to (410.000, 50.000) mm",
        ]
    );
}

/// AC 1 — singular and plural forms; kinds with no finding are omitted.
#[test]
fn report_summary_plurals_and_omissions() {
    let gaps2 = vec![
        Finding::Gap {
            a: 0,
            b: 1,
            mid: p(0.0, 0.0),
            width: 0.1,
        };
        2
    ];
    let lines = CheckReport { findings: gaps2 }.lines();
    assert_eq!(lines[0], "CHECK: 2 gaps");
    assert_eq!(lines.len(), 3);
    let one_open = CheckReport {
        findings: vec![Finding::OpenEnd {
            index: 7,
            at: p(1.23456, -2.0),
        }],
    };
    assert_eq!(
        one_open.lines(),
        vec![
            "CHECK: 1 open end",
            "open end: entity 7 at (1.235, -2.000) mm"
        ]
    );
    let dups = Finding::Duplicate {
        index: 2,
        of: 0,
        at: p(0.0, 0.0),
    };
    let deg = Finding::Degenerate {
        index: 3,
        at: p(0.0, 0.0),
    };
    let off = Finding::OffBed {
        index: 4,
        min: p(0.0, 0.0),
        max: p(1.0, 1.0),
    };
    let many = CheckReport {
        findings: vec![dups.clone(), dups, deg.clone(), deg, off.clone(), off],
    };
    assert_eq!(
        many.lines()[..3],
        [
            "CHECK: 2 duplicates",
            "CHECK: 2 degenerate entities",
            "CHECK: 2 off-bed entities"
        ]
    );
}
