//! LCV-160 — TRIM and EXTEND with arcs, driven through `TrimTool` and
//! `ExtendTool` on a `Document` + `History`. One test per acceptance criterion.

use core::f64::consts::{FRAC_PI_2, PI};

use lasercad::document::{Document, Entity, History, Layer, LayerId};
use lasercad::geometry::{Arc, Circle, Line, Vec2};
use lasercad::tools::{ExtendTool, Tool, TrimTool};

const TOL: f64 = 1e-7;
const CUT: LayerId = LayerId(0);
const MARK: LayerId = LayerId(1);

fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}
fn line(ax: f64, ay: f64, bx: f64, by: f64) -> Entity {
    Entity::Line(Line::new(v(ax, ay), v(bx, by)))
}
fn circle(cx: f64, cy: f64, r: f64) -> Entity {
    Entity::Circle(Circle::new(v(cx, cy), r))
}
fn arc(cx: f64, cy: f64, r: f64, start: f64, end: f64, ccw: bool) -> Entity {
    Entity::Arc(Arc::new(v(cx, cy), r, start, end, ccw))
}
fn polar(r: f64, angle: f64) -> Vec2 {
    v(r * angle.cos(), r * angle.sin())
}
fn doc_with(entities: Vec<Entity>) -> Document {
    let mut doc = Document::default();
    entities.into_iter().for_each(|e| doc.push_current(e));
    doc
}
fn trim_at(doc: &mut Document, hist: &mut History, pos: Vec2) {
    TrimTool.on_pointer_down(pos, false, doc, hist);
}
fn as_arc(e: &Entity) -> Arc {
    match e {
        Entity::Arc(a) => *a,
        other => panic!("expected Arc, got {other:?}"),
    }
}
fn as_line(e: &Entity) -> Line {
    match e {
        Entity::Line(l) => *l,
        other => panic!("expected Line, got {other:?}"),
    }
}
fn near(a: Vec2, b: Vec2) -> bool {
    a.approx_eq(b, TOL)
}

/// Assert `got` is the arc on `like`'s circle and direction from `start` to `end`.
fn assert_arc(got: &Entity, like: &Arc, start: Vec2, end: Vec2) {
    let a = as_arc(got);
    assert_eq!(
        (a.center, a.r, a.ccw),
        (like.center, like.r, like.ccw),
        "{a:?}"
    );
    assert!(
        near(a.start_point(), start),
        "start {:?} != {start:?}",
        a.start_point()
    );
    assert!(
        near(a.end_point(), end),
        "end {:?} != {end:?}",
        a.end_point()
    );
}

/// AC 1 — an Arc target cut by a Line, a Circle and an Arc cutter keeps the
/// sub-arc around the click, with the target's center, radius and direction.
#[test]
fn ac1_arc_target_keeps_sub_arc_around_click() {
    // Line cutter on a CW upper half (π → 0): keep π → π/2.
    let target = arc(0.0, 0.0, 10.0, PI, 0.0, false);
    let mut doc = doc_with(vec![target, line(0.0, 0.0, 0.0, 20.0)]);
    let mut hist = History::default();
    trim_at(&mut doc, &mut hist, polar(10.0, 0.75 * PI));
    assert_arc(
        &doc.entities[0],
        &as_arc(&target),
        v(-10.0, 0.0),
        v(0.0, 10.0),
    );

    // Circle cutter on an arc across ±π (3π/4 → −3π/4 CCW): keep the middle.
    let target = arc(0.0, 0.0, 10.0, 0.75 * PI, -0.75 * PI, true);
    let mut doc = doc_with(vec![target, circle(-10.0, 0.0, 5.0)]);
    let mut hist = History::default();
    trim_at(&mut doc, &mut hist, v(-10.0, 0.0));
    let (x, h) = (-8.75, (100.0f64 - 8.75 * 8.75).sqrt());
    assert_arc(&doc.entities[0], &as_arc(&target), v(x, h), v(x, -h));

    // Arc cutter on a CCW upper half: keep 0 → π/3.
    let target = arc(0.0, 0.0, 10.0, 0.0, PI, true);
    let cutter = arc(10.0, 0.0, 10.0, FRAC_PI_2, PI, true);
    let mut doc = doc_with(vec![target, cutter]);
    let mut hist = History::default();
    trim_at(&mut doc, &mut hist, polar(10.0, PI / 6.0));
    assert_arc(
        &doc.entities[0],
        &as_arc(&target),
        v(10.0, 0.0),
        polar(10.0, PI / 3.0),
    );
    assert_eq!(doc.entities[1], cutter);
}

/// AC 2 — a Line target cut by an Arc uses only the arc's cut points: the
/// parent circle's lower crossing (off the span) is ignored.
#[test]
fn ac2_line_target_uses_only_arc_cut_points() {
    let mut doc = doc_with(vec![
        line(5.0, -20.0, 5.0, 20.0),
        arc(0.0, 0.0, 10.0, 0.0, PI, true),
    ]);
    let mut hist = History::default();
    trim_at(&mut doc, &mut hist, v(5.0, -15.0));
    let l = as_line(&doc.entities[0]);
    let y = 75.0f64.sqrt();
    assert!(near(l.p1, v(5.0, -20.0)) && near(l.p2, v(5.0, y)), "{l:?}");
}

/// AC 3 — a Circle target cut at two points by a Circle or an Arc becomes the
/// arc between them that holds the click.
#[test]
fn ac3_circle_target_becomes_arc_holding_click() {
    let (x, y) = (5.0, 75.0f64.sqrt());
    for (cutter, click) in [
        (circle(10.0, 0.0, 10.0), v(-10.0, 0.0)),
        (
            arc(10.0, 0.0, 10.0, FRAC_PI_2, 1.5 * PI, true),
            v(10.0, 0.0),
        ),
    ] {
        let mut doc = doc_with(vec![circle(0.0, 0.0, 10.0), cutter]);
        let mut hist = History::default();
        trim_at(&mut doc, &mut hist, click);
        let a = as_arc(&doc.entities[0]);
        assert_eq!((a.center, a.r), (v(0.0, 0.0), 10.0));
        let ends = [a.start_point(), a.end_point()];
        assert!(ends.iter().any(|&p| near(p, v(x, y))), "{a:?}");
        assert!(ends.iter().any(|&p| near(p, v(x, -y))), "{a:?}");
        assert!(
            a.contains_angle(click.y.atan2(click.x)),
            "{a:?} misses the click"
        );
    }
}

/// AC 4 — no cut point on the target, or one cut point on a Circle target
/// (one cutter, or each of two cutters), leaves the document unchanged.
#[test]
fn ac4_no_cut_point_is_noop_without_undo_entry() {
    let cases = [
        // Arc target: the line meets its circle only off the span.
        (
            vec![
                arc(0.0, 0.0, 10.0, 0.0, PI, true),
                line(-20.0, -5.0, 20.0, -5.0),
            ],
            v(0.0, 10.0),
        ),
        // Circle target, tangent line: one cut point.
        (
            vec![circle(0.0, 0.0, 10.0), line(-20.0, 10.0, 20.0, 10.0)],
            v(0.0, -10.0),
        ),
        // Circle target, two arc cutters meeting it once each.
        (
            vec![
                circle(0.0, 0.0, 10.0),
                arc(10.0, 0.0, 10.0, FRAC_PI_2, PI, true),
                arc(10.0, 0.0, 10.0, PI, 1.5 * PI, true),
            ],
            v(-10.0, 0.0),
        ),
    ];
    for (entities, click) in cases {
        let mut doc = doc_with(entities.clone());
        let mut hist = History::default();
        trim_at(&mut doc, &mut hist, click);
        assert_eq!(doc.entities, entities);
        assert!(!hist.can_undo());
    }
}

/// AC 5 — EXTEND grows an Arc endpoint along its own circle, away from its
/// other end, to the nearest cut point on a Line, Circle or Arc boundary.
#[test]
fn ac5_extend_arc_endpoint_to_boundary() {
    let (x, y) = (-5.0, 75.0f64.sqrt());
    let quarter = arc(0.0, 0.0, 10.0, 0.0, FRAC_PI_2, true);
    let cw_quarter = arc(0.0, 0.0, 10.0, FRAC_PI_2, 0.0, false);
    let cases = [
        (quarter, line(-5.0, -20.0, -5.0, 20.0), v(x, y)),
        (quarter, circle(-10.0, 0.0, 10.0), v(x, y)),
        // Only the lower crossing lies on this boundary arc's span.
        (
            quarter,
            arc(-10.0, 0.0, 10.0, -FRAC_PI_2, 0.0, true),
            v(x, -y),
        ),
        // CW arc: the grown end is its start.
        (cw_quarter, line(-5.0, -20.0, -5.0, 20.0), v(x, y)),
    ];
    for (target, boundary, reach) in cases {
        let t = as_arc(&target);
        let (start, end) = if t.ccw {
            (t.start_point(), reach)
        } else {
            (reach, t.end_point())
        };
        let mut doc = doc_with(vec![target, boundary]);
        let mut hist = History::default();
        let mut tool = ExtendTool::default();
        tool.on_pointer_move(v(0.5, 10.5), &mut doc);
        let preview = tool.preview();
        assert_eq!(preview.len(), 1, "no preview for {boundary:?}");
        assert_arc(&preview[0], &t, start, end);
        tool.on_pointer_down(v(0.5, 10.5), false, &mut doc, &mut hist);
        assert_arc(&doc.entities[0], &t, start, end);
        assert_eq!(doc.entities[1], boundary);
    }
}

/// AC 6 — a Line extends to an Arc boundary using only the cut points on the
/// arc's span (the nearer crossing of its parent circle is off the span).
#[test]
fn ac6_extend_line_to_arc_boundary() {
    let mut doc = doc_with(vec![
        line(5.0, -20.0, 5.0, -15.0),
        arc(0.0, 0.0, 10.0, 0.0, PI, true),
    ]);
    let mut hist = History::default();
    let mut tool = ExtendTool::default();
    tool.on_pointer_move(v(5.0, -14.8), &mut doc);
    assert_eq!(tool.preview().len(), 1);
    tool.on_pointer_down(v(5.0, -14.8), false, &mut doc, &mut hist);
    let l = as_line(&doc.entities[0]);
    assert!(
        near(l.p1, v(5.0, -20.0)) && near(l.p2, v(5.0, 75.0f64.sqrt())),
        "{l:?}"
    );
}

/// AC 7 — a boundary reached only by closing the arc into a full turn gives
/// no preview, and a click changes nothing.
#[test]
fn ac7_extend_past_full_turn_is_rejected() {
    let entities = vec![
        arc(0.0, 0.0, 10.0, 0.0, FRAC_PI_2, true),
        line(0.0, 0.0, 20.0, 20.0),
    ];
    let mut doc = doc_with(entities.clone());
    let mut hist = History::default();
    let mut tool = ExtendTool::default();
    for pos in [v(0.5, 10.5), v(10.5, 0.5)] {
        tool.on_pointer_move(pos, &mut doc);
        assert!(tool.preview().is_empty(), "preview at {pos:?}");
        tool.on_pointer_down(pos, false, &mut doc, &mut hist);
    }
    assert_eq!(doc.entities, entities);
    assert!(!hist.can_undo());
}

fn two_layer_doc(entities: Vec<(Entity, LayerId)>) -> Document {
    let layer = |id, name: &str, color| Layer {
        id,
        name: name.to_owned(),
        color,
        output: true,
    };
    let layers = vec![
        layer(CUT, "Cut", [255, 0, 0]),
        layer(MARK, "Mark", [0, 0, 255]),
    ];
    let (es, ls) = entities.into_iter().unzip();
    Document::from_parts([400.0; 2], layers, CUT, es, ls).expect("valid layers")
}

/// AC 8 — undo restores the original exactly, redo reapplies the same
/// result, and the result stays on the target's (non-current) layer.
#[test]
fn ac8_undo_redo_exact_and_layer_kept() {
    type Act = fn(&mut Document, &mut History);
    let trim_arc: Act = |d, h| trim_at(d, h, polar(10.0, PI / 6.0));
    let trim_circle: Act = |d, h| trim_at(d, h, v(-10.0, 0.0));
    let trim_line: Act = |d, h| trim_at(d, h, v(5.0, -15.0));
    let extend: Act = |d, h| {
        let mut tool = ExtendTool::default();
        tool.on_pointer_move(v(0.5, 10.5), d);
        tool.on_pointer_down(v(0.5, 10.5), false, d, h);
    };
    let cases: [(Entity, Entity, Act); 5] = [
        (
            arc(0.0, 0.0, 10.0, 0.0, PI, true),
            line(5.0, 0.0, 5.0, 20.0),
            trim_arc,
        ),
        (
            circle(0.0, 0.0, 10.0),
            arc(10.0, 0.0, 10.0, FRAC_PI_2, 1.5 * PI, true),
            trim_circle,
        ),
        (
            line(5.0, -20.0, 5.0, 20.0),
            arc(0.0, 0.0, 10.0, 0.0, PI, true),
            trim_line,
        ),
        (
            arc(0.0, 0.0, 10.0, 0.0, FRAC_PI_2, true),
            line(-5.0, -20.0, -5.0, 20.0),
            extend,
        ),
        (
            arc(0.0, 0.0, 10.0, 0.0, FRAC_PI_2, true),
            circle(-10.0, 0.0, 10.0),
            extend,
        ),
    ];
    for (target, other, act) in cases {
        let mut doc = two_layer_doc(vec![(target, MARK), (other, CUT)]);
        let mut hist = History::default();
        act(&mut doc, &mut hist);
        let result = doc.entities[0];
        assert_ne!(result, target, "no change for {target:?}");
        assert_eq!(doc.entity_layer(0), Some(MARK));
        assert!(hist.undo(&mut doc));
        assert_eq!(doc.entities[0], target);
        assert_eq!(doc.entity_layer(0), Some(MARK));
        assert!(hist.redo(&mut doc));
        assert_eq!(doc.entities[0], result);
        assert_eq!(doc.entity_layer(0), Some(MARK));
    }
}

/// AC 9 — one TRIM click that meets several cutters is one undo step.
#[test]
fn ac9_multi_cutter_trim_is_one_undo_step() {
    let target = arc(0.0, 0.0, 10.0, 0.0, PI, true);
    let mut doc = doc_with(vec![
        target,
        line(-5.0, 0.0, -5.0, 20.0),
        line(5.0, 0.0, 5.0, 20.0),
    ]);
    let mut hist = History::default();
    trim_at(&mut doc, &mut hist, v(0.0, 10.0));
    let t = as_arc(&target);
    assert_arc(
        &doc.entities[0],
        &t,
        polar(10.0, PI / 3.0),
        polar(10.0, 2.0 * PI / 3.0),
    );
    assert_eq!(hist.len(), 1);
    assert!(hist.undo(&mut doc));
    assert_eq!(doc.entities[0], target);
    assert!(!hist.can_undo());
}
