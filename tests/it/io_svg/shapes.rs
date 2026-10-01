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

/// AC 4, AC 10 — a sharp `<rect>` is four lines in equivalent-path order
/// (top-left corner first, clockwise in SVG space); a missing `x`/`y` is 0
/// (AC 1); `rect` is no longer reported.
#[test]
fn a_sharp_rect_imports_four_lines() {
    let (es, report) =
        page(r#"<rect x="10" y="20" width="30" height="40"/><rect width="5" height="2"/>"#);
    assert!(report.is_empty(), "{report:?}");
    assert_eq!(
        es,
        [
            line((10.0, 20.0), (40.0, 20.0)),
            line((40.0, 20.0), (40.0, 60.0)),
            line((40.0, 60.0), (10.0, 60.0)),
            line((10.0, 60.0), (10.0, 20.0)),
            line((0.0, 0.0), (5.0, 0.0)),
            line((5.0, 0.0), (5.0, 2.0)),
            line((5.0, 2.0), (0.0, 2.0)),
            line((0.0, 2.0), (0.0, 0.0)),
        ]
    );
}

/// AC 2, AC 3 — a zero or missing `width`/`height` draws nothing
/// unreported; a negative or unparseable one is reported.
#[test]
fn a_degenerate_rect_is_skipped_and_only_an_invalid_one_reported() {
    let (es, report) = page(
        r#"<rect width="0" height="5"/><rect width="5" height="0"/><rect height="5"/>
           <rect width="5"/><rect/><rect width="-1" height="5"/><rect width="5" height="x"/>"#,
    );
    assert!(es.is_empty(), "{es:?}");
    assert_eq!(report, [entry("rect (invalid attribute)", 2)]);
}

/// The entity kinds of `es`, in order.
fn kinds(es: &[Entity]) -> Vec<&'static str> {
    es.iter()
        .map(|e| match e {
            Entity::Line(_) => "line",
            Entity::Arc(_) => "arc",
            Entity::Ellipse(_) => "ellipse",
            other => panic!("unexpected {other:?}"),
        })
        .collect()
}

/// AC 6 — equal radii give four quarter circular arcs between the sides,
/// starting with the top side and going clockwise in SVG space.
#[test]
fn a_rounded_rect_imports_quarter_arcs_and_sides() {
    let (es, report) = page(r#"<rect x="10" y="20" width="30" height="40" rx="5"/>"#);
    assert!(report.is_empty(), "{report:?}");
    let k = ["line", "arc"];
    assert_eq!(kinds(&es), [k, k, k, k].concat());
    let centers = [w(35.0, 25.0), w(35.0, 55.0), w(15.0, 55.0), w(15.0, 25.0)];
    for (i, c) in centers.into_iter().enumerate() {
        let Entity::Arc(a) = es[2 * i + 1] else {
            panic!("{:?}", es[2 * i + 1]);
        };
        assert!(
            a.center.approx_eq(c, TOL) && (a.r - 5.0).abs() < TOL,
            "{a:?}"
        );
        assert!((a.sweep_angle() - core::f64::consts::FRAC_PI_2).abs() < TOL);
        assert!(!a.ccw, "clockwise on screen is clockwise in the world");
    }
    assert_eq!(es[0], line((15.0, 20.0), (35.0, 20.0)));
    assert_eq!(es[6], line((10.0, 55.0), (10.0, 25.0)));
}

/// AC 6 — a radius of half the side leaves no zero-length side; a fully
/// round rect is four arcs.
#[test]
fn a_half_side_radius_leaves_no_zero_length_side() {
    let (es, _) = page(r#"<rect width="10" height="20" rx="5"/>"#);
    assert_eq!(kinds(&es), ["arc", "line", "arc", "arc", "line", "arc"]);
    let (es, _) = page(r#"<rect width="10" height="10" rx="5" ry="auto"/>"#);
    assert_eq!(kinds(&es), ["arc"; 4]);
}

/// AC 6 — unequal radii give four quarter elliptical arcs (LCV-176).
#[test]
fn unequal_radii_give_elliptical_corners() {
    let (es, report) = page(r#"<rect x="10" y="20" width="30" height="40" rx="3" ry="2"/>"#);
    assert!(report.is_empty(), "{report:?}");
    let k = ["line", "ellipse"];
    assert_eq!(kinds(&es), [k, k, k, k].concat());
    for e in es.iter().skip(1).step_by(2) {
        let Entity::Ellipse(e) = e else {
            panic!("{e:?}")
        };
        let (big, small) = (e.rx.max(e.ry), e.rx.min(e.ry));
        assert!(
            (big - 3.0).abs() < TOL && (small - 2.0).abs() < TOL,
            "{e:?}"
        );
        let span = e.span.expect("an arc");
        assert!(
            (e.sweep().abs() - core::f64::consts::FRAC_PI_2).abs() < TOL,
            "{span:?}"
        );
    }
}

/// AC 6 — a `<rect>` imports exactly like its SVG 2 §10.2 equivalent path.
#[test]
fn a_rect_matches_its_equivalent_path() {
    for (rect, d) in [
        (
            r#"<rect x="10" y="20" width="30" height="40" rx="5"/>"#,
            "M 15 20 H 35 A 5 5 0 0 1 40 25 V 55 A 5 5 0 0 1 35 60 H 15 A 5 5 0 0 1 10 55 V 25 A 5 5 0 0 1 15 20 Z",
        ),
        (
            r#"<rect x="10" y="20" width="30" height="40" rx="4" ry="2"/>"#,
            "M 14 20 H 36 A 4 2 0 0 1 40 22 V 58 A 4 2 0 0 1 36 60 H 14 A 4 2 0 0 1 10 58 V 22 A 4 2 0 0 1 14 20 Z",
        ),
        (
            r#"<rect x="10" y="20" width="30" height="40"/>"#,
            "M 10 20 H 40 V 60 H 10 Z",
        ),
    ] {
        let path = format!(r#"<path d="{d}"/>"#);
        assert_eq!(page(rect), page(&path), "{rect}");
    }
}

/// AC 7, AC 10 — a polyline is one line per pair of distinct consecutive
/// points; a duplicate point adds nothing; one point or none imports
/// nothing; neither is reported.
#[test]
fn a_polyline_imports_one_line_per_distinct_pair() {
    let (es, report) = page(
        r#"<polyline points="0,0 10,0 10,0 10,10,20 10"/><polyline points="5 5"/>
           <polyline points=""/><polyline/>"#,
    );
    assert!(report.is_empty(), "{report:?}");
    assert_eq!(
        es,
        [
            line((0.0, 0.0), (10.0, 0.0)),
            line((10.0, 0.0), (10.0, 10.0)),
            line((10.0, 10.0), (20.0, 10.0)),
        ]
    );
}

/// AC 7 — a polygon gets a closing line, unless its last point is its
/// first; glued signs and exponents parse as in path data.
#[test]
fn a_polygon_closes_unless_already_closed() {
    let (es, report) =
        page(r#"<polygon points="0,0 10,0 10,10"/><polygon points="20-0 3e1,0 30 10 20 0"/>"#);
    assert!(report.is_empty(), "{report:?}");
    assert_eq!(
        es,
        [
            line((0.0, 0.0), (10.0, 0.0)),
            line((10.0, 0.0), (10.0, 10.0)),
            line((10.0, 10.0), (0.0, 0.0)),
            line((20.0, 0.0), (30.0, 0.0)),
            line((30.0, 0.0), (30.0, 10.0)),
            line((30.0, 10.0), (20.0, 0.0)),
        ]
    );
}

/// AC 8 — an odd count or a bad token keeps the pairs before it and
/// reports a data error; the file still opens.
#[test]
fn bad_points_keep_the_pairs_before_the_error() {
    let (es, report) =
        page(r#"<polyline points="0 0 10 0 10"/><polygon points="0 20 10 20 10 30 x 5 5"/>"#);
    assert_eq!(
        es,
        [
            line((0.0, 0.0), (10.0, 0.0)),
            line((0.0, 20.0), (10.0, 20.0)),
            line((10.0, 20.0), (10.0, 30.0)),
            line((10.0, 30.0), (0.0, 20.0)),
        ]
    );
    assert_eq!(
        report,
        [
            entry("polyline (data error)", 1),
            entry("polygon (data error)", 1)
        ]
    );
}

/// AC 9 — shapes under a `transform` map like paths (LCV-173): a
/// translated group moves rect, polyline and polygon lines; `scale(2,1)`
/// turns round corners elliptical; `rotate(30)` keeps them circular.
#[test]
fn shapes_map_through_transforms() {
    let (es, report) = page(
        r#"<g transform="translate(10,20)"><rect width="10" height="5"/>
           <polyline points="0 0 5 5"/><polygon points="0 0 5 0 5 5"/></g>"#,
    );
    assert!(report.is_empty(), "{report:?}");
    assert_eq!(
        es,
        [
            line((10.0, 20.0), (20.0, 20.0)),
            line((20.0, 20.0), (20.0, 25.0)),
            line((20.0, 25.0), (10.0, 25.0)),
            line((10.0, 25.0), (10.0, 20.0)),
            line((10.0, 20.0), (15.0, 25.0)),
            line((10.0, 20.0), (15.0, 20.0)),
            line((15.0, 20.0), (15.0, 25.0)),
            line((15.0, 25.0), (10.0, 20.0)),
        ]
    );
    let (es, _) = page(r#"<rect width="20" height="20" rx="5" transform="scale(2,1)"/>"#);
    let corners: Vec<_> = es.iter().skip(1).step_by(2).collect();
    assert_eq!(corners.len(), 4, "{es:?}");
    for e in corners {
        let Entity::Ellipse(e) = e else {
            panic!("{e:?}")
        };
        let (big, small) = (e.rx.max(e.ry), e.rx.min(e.ry));
        assert!(
            (big - 10.0).abs() < TOL && (small - 5.0).abs() < TOL,
            "{e:?}"
        );
    }
    let (es, _) = page(r#"<rect width="20" height="20" rx="5" transform="rotate(30)"/>"#);
    let k = ["line", "arc"];
    assert_eq!(kinds(&es), [k, k, k, k].concat());
    for e in es.iter().skip(1).step_by(2) {
        let Entity::Arc(a) = e else { panic!("{e:?}") };
        assert!((a.r - 5.0).abs() < TOL, "{a:?}");
    }
}
