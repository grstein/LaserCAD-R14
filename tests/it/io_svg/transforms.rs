//! LCV-173 — `transform` on elements and groups, circles and arcs under
//! similarity and non-similarity maps, and nested `<svg>` viewports, end to
//! end through `import_svg`. The page is 100 mm square with one user unit =
//! 1 mm, so a world point is `(x, 100 − y)`.

use lasercad::document::Entity;
use lasercad::geometry::{Circle, Vec2};
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
