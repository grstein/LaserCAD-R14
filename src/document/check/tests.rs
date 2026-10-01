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
    assert_eq!(check_drawing(&doc_of(&rectangle(0.0))), CheckReport::default());
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
