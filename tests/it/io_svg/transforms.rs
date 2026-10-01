//! LCV-173 — `transform` on elements and groups, circles and arcs under
//! similarity and non-similarity maps, and nested `<svg>` viewports, end to
//! end through `import_svg`. The page is 100 mm square with one user unit =
//! 1 mm, so a world point is `(x, 100 − y)`.

use lasercad::document::Entity;
use lasercad::geometry::{Arc, Circle, Vec2};
use lasercad::io::svg::import_svg;

const TOL: f64 = 1e-9;

/// `(entities, report)` of a 100 mm page holding `inner`.
fn page(inner: &str) -> (Vec<Entity>, Vec<(String, usize)>) {
    let src = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100">{inner}</svg>"#
    );
    let imported = import_svg(&src).unwrap();
    (imported.entities, imported.report)
}

/// World point of SVG page point `(x, y)`.
fn w(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, 100.0 - y)
}

fn entry(label: &str, count: usize) -> (String, usize) {
    (label.to_owned(), count)
}

fn assert_line(e: &Entity, a: Vec2, b: Vec2) {
    let Entity::Line(l) = e else {
        panic!("not a line: {e:?}")
    };
    assert!(
        l.p1.approx_eq(a, TOL) && l.p2.approx_eq(b, TOL),
        "{l:?} vs {a:?} {b:?}"
    );
}

fn circle(e: &Entity) -> Circle {
    match e {
        Entity::Circle(c) => *c,
        other => panic!("not a circle: {other:?}"),
    }
}

/// AC 5 — nested groups compose their transforms onto lines, circles and
/// path segments; an applied `transform` is not reported.
#[test]
fn nested_groups_compose_onto_lines_circles_and_paths() {
    let (es, report) = page(
        r#"<g transform="translate(10,20)"><g transform="scale(2)">
             <line x1="1" y1="1" x2="5" y2="1"/>
             <circle cx="5" cy="5" r="2"/>
             <path d="M 0 0 L 5 0"/>
           </g></g>"#,
    );
    assert!(report.is_empty(), "{report:?}");
    assert_eq!(es.len(), 3);
    assert_line(&es[0], w(12.0, 22.0), w(20.0, 22.0));
    let c = circle(&es[1]);
    assert!(
        c.center.approx_eq(w(20.0, 30.0), TOL) && (c.r - 4.0).abs() < TOL,
        "{c:?}"
    );
    assert_line(&es[2], w(10.0, 20.0), w(20.0, 20.0));
}

/// AC 5 — an element's own transform applies inside its parent's, and a
/// list applies right to left.
#[test]
fn element_transform_and_list_order() {
    let (es, report) = page(
        r#"<g transform="translate(50 0)">
             <line transform="rotate(90)" x1="10" y1="0" x2="20" y2="0"/>
             <line transform="translate(0,10) scale(3)" x1="1" y1="1" x2="2" y2="1"/>
           </g>"#,
    );
    assert!(report.is_empty(), "{report:?}");
    assert_line(&es[0], w(50.0, 10.0), w(50.0, 20.0));
    assert_line(&es[1], w(53.0, 13.0), w(56.0, 13.0));
}

/// AC 8 — an unparseable `transform` is treated as absent and reported as
/// `transform (invalid)`; the element still imports.
#[test]
fn an_invalid_transform_is_ignored_and_reported() {
    let (es, report) = page(
        r#"<g transform="red"><line x1="1" y1="1" x2="2" y2="1"/></g>
           <line transform="scale(2" x1="1" y1="1" x2="2" y2="1"/>"#,
    );
    assert_line(&es[0], w(1.0, 1.0), w(2.0, 1.0));
    assert_line(&es[1], w(1.0, 1.0), w(2.0, 1.0));
    assert_eq!(report, [entry("transform (invalid)", 2)]);
}

/// AC 5 (amends LCV-171 AC 7) — `transform` is no longer an unapplied
/// property, whether valid or empty.
#[test]
fn transform_is_no_longer_reported_as_a_property() {
    for host in [
        r#"<g transform="scale(2)"/>"#,
        r#"<g transform=""/>"#,
        r#"<line transform="skewX(10)" x1="0" y1="0" x2="1" y2="1"/>"#,
    ] {
        let (_, report) = page(host);
        assert!(report.is_empty(), "{host}: {report:?}");
    }
}

/// A singular transform renders nothing (SVG 2) and is reported.
#[test]
fn a_singular_transform_imports_nothing_and_is_reported() {
    let (es, report) = page(
        r#"<g transform="scale(0)"><line x1="1" y1="1" x2="2" y2="1"/></g>
           <line transform="matrix(1 2 2 4 0 0)" x1="1" y1="1" x2="2" y2="1"/>"#,
    );
    assert!(es.is_empty(), "{es:?}");
    assert_eq!(report, [entry("transform (singular)", 2)]);
}

fn arc(e: &Entity) -> Arc {
    match e {
        Entity::Arc(a) => *a,
        other => panic!("not an arc: {other:?}"),
    }
}

/// `a`'s centre, radius, endpoints (world) and handedness.
fn assert_arc(a: Arc, center: Vec2, r: f64, from: Vec2, to: Vec2, ccw: bool) {
    let ok = a.center.approx_eq(center, TOL)
        && (a.r - r).abs() < TOL
        && a.start_point().approx_eq(from, TOL)
        && a.end_point().approx_eq(to, TOL)
        && a.ccw == ccw;
    assert!(
        ok,
        "{a:?}: want centre {center:?} r {r} {from:?} → {to:?} ccw {ccw}"
    );
}

/// AC 6 — a circle under rotation and uniform scale, and under a
/// reflection: centre mapped, radius scaled.
#[test]
fn circles_under_similarities() {
    let (es, report) = page(
        r#"<g transform="translate(50 50) rotate(30) scale(2)"><circle cx="0" cy="0" r="5"/></g>
           <g transform="translate(60 0) scale(-1,1)"><circle cx="10" cy="20" r="3"/></g>"#,
    );
    assert!(report.is_empty(), "{report:?}");
    let c = circle(&es[0]);
    assert!(
        c.center.approx_eq(w(50.0, 50.0), TOL) && (c.r - 10.0).abs() < TOL,
        "{c:?}"
    );
    let c = circle(&es[1]);
    assert!(
        c.center.approx_eq(w(50.0, 20.0), TOL) && (c.r - 3.0).abs() < TOL,
        "{c:?}"
    );
}

/// AC 6 — a path arc under a uniform scale keeps its handedness with the
/// radius scaled; under a reflection (either axis) `ccw` is inverted.
#[test]
fn arcs_under_similarities() {
    // Untransformed: (10, 50) → (30, 50), r 10, sweep 1: clockwise over the
    // top of the screen, centre (20, 50).
    let d = r#"<path d="M 10 50 A 10 10 0 0 1 30 50"/>"#;
    let (es, report) = page(&format!(
        r#"<g transform="translate(0,-50) scale(2)">{d}</g>
           <g transform="translate(100 0) scale(-1 1)">{d}</g>
           <g transform="translate(0 100) scale(1 -1)">{d}</g>"#
    ));
    assert!(report.is_empty(), "{report:?}");
    assert_arc(
        arc(&es[0]),
        w(40.0, 50.0),
        20.0,
        w(20.0, 50.0),
        w(60.0, 50.0),
        false,
    );
    // Mirrored in x: (90, 50) → (70, 50) still over the top, now
    // counter-clockwise.
    assert_arc(
        arc(&es[1]),
        w(80.0, 50.0),
        10.0,
        w(90.0, 50.0),
        w(70.0, 50.0),
        true,
    );
    // Mirrored in y: (10, 50) → (30, 50) under the bottom, counter-clockwise.
    let a = arc(&es[2]);
    assert_arc(a, w(20.0, 50.0), 10.0, w(10.0, 50.0), w(30.0, 50.0), true);
    let low = a.bbox().0.y;
    assert!(
        (low - 40.0).abs() < TOL,
        "the bulge is below the chord: {a:?}"
    );
}

/// AC 7 — under a non-uniform scale or a skew a circle or arc imports
/// nothing and is reported; the path's current point still advances.
#[test]
fn circles_and_arcs_under_non_similarities_are_reported() {
    let (es, report) = page(
        r#"<g transform="scale(2 1)">
             <circle cx="10" cy="10" r="5"/>
             <path d="M 10 50 A 10 10 0 0 1 30 50 L 40 50"/>
           </g>
           <circle transform="skewX(20)" cx="10" cy="10" r="5"/>"#,
    );
    assert_eq!(es.len(), 1, "{es:?}");
    assert_line(&es[0], w(60.0, 50.0), w(80.0, 50.0));
    assert_eq!(
        report,
        [
            entry("circle (non-uniform transform)", 2),
            entry("arc (non-uniform transform)", 1),
        ]
    );
}

/// AC 7 — `preserveAspectRatio="none"` with unequal scales is a
/// non-uniform map too.
#[test]
fn par_none_with_unequal_scales_is_non_uniform() {
    let src = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="50mm" viewBox="0 0 100 100" preserveAspectRatio="none"><circle cx="50" cy="50" r="5"/><line x1="0" y1="100" x2="100" y2="0"/></svg>"#;
    let imported = import_svg(src).unwrap();
    assert_eq!(imported.entities.len(), 1);
    assert_line(
        &imported.entities[0],
        Vec2::new(0.0, 0.0),
        Vec2::new(100.0, 50.0),
    );
    assert_eq!(
        imported.report,
        [entry("circle (non-uniform transform)", 1)]
    );
}
