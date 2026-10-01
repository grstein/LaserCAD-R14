//! LCV-174 — basic shapes on import (SVG 2 ch. 10): missing positions are 0,
//! a zero or missing size draws nothing silently, a negative or unparseable
//! geometry attribute skips its element and is reported, and the file still
//! opens. The page is 100 mm square with one user unit = 1 mm, so a world
//! point is `(x, 100 − y)`.

use lasercad::document::Entity;
use lasercad::geometry::{Circle, Line, Vec2};
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

fn line(a: (f64, f64), b: (f64, f64)) -> Entity {
    Entity::Line(Line::new(w(a.0, a.1), w(b.0, b.1)))
}

/// AC 1 — a missing `x1 y1 x2 y2 cx cy` is 0.
#[test]
fn missing_positions_default_to_zero() {
    let (es, report) = page(
        r#"<line x2="5"/><line y1="3" x2="5" y2="3"/><circle r="3"/><ellipse rx="4" ry="2"/>"#,
    );
    assert!(report.is_empty(), "{report:?}");
    assert_eq!(
        es[..3],
        [
            line((0.0, 0.0), (5.0, 0.0)),
            line((0.0, 3.0), (5.0, 3.0)),
            Entity::Circle(Circle::new(w(0.0, 0.0), 3.0)),
        ]
    );
    let [.., Entity::Ellipse(e)] = es.as_slice() else {
        panic!("{es:?}");
    };
    assert!(e.center.approx_eq(w(0.0, 0.0), TOL), "{e:?}");
    assert!(
        (e.rx - 4.0).abs() < TOL && (e.ry - 2.0).abs() < TOL,
        "{e:?}"
    );
}

/// AC 2 — `r` missing or 0, and an ellipse whose resolved radius is 0 or
/// absent, import nothing and are not reported.
#[test]
fn a_zero_or_missing_radius_imports_nothing_unreported() {
    let (es, report) = page(
        r#"<circle cx="5" cy="5" r="0"/><circle cx="5" cy="5"/><circle r="0mm"/>
           <ellipse cx="5" cy="5" rx="0" ry="5"/><ellipse cx="5" cy="5"/>
           <ellipse rx="auto" ry="auto"/>"#,
    );
    assert!(es.is_empty(), "{es:?}");
    assert!(report.is_empty(), "{report:?}");
}

/// AC 3 — a negative radius or an unparseable position skips the element,
/// is reported once per element, and the rest of the file imports
/// (rewrites the retired LCV-057 AC 12/13, which failed the file).
#[test]
fn an_invalid_attribute_skips_the_element_and_is_reported() {
    let (es, report) = page(
        r#"<circle cx="0" cy="0" r="-1"/><line x1="abc" y1="0" x2="0" y2="0"/>
           <ellipse cx="1" cy="1" rx="-2"/><line x1="a" y1="b" x2="c" y2="d"/>
           <circle r="1px2"/><line x1="0" y1="0" x2="5" y2="0"/>"#,
    );
    assert_eq!(es, [line((0.0, 0.0), (5.0, 0.0))]);
    assert_eq!(
        report,
        [
            entry("circle (invalid attribute)", 2),
            entry("line (invalid attribute)", 2),
            entry("ellipse (invalid attribute)", 1),
        ]
    );
}
