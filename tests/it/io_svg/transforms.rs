//! LCV-173 — `transform` on elements and groups, circles and arcs under
//! similarity and non-similarity maps, and nested `<svg>` viewports, end to
//! end through `import_svg`. The page is 100 mm square with one user unit =
//! 1 mm, so a world point is `(x, 100 − y)`.

use core::f64::consts::TAU;
use lasercad::document::Entity;
use lasercad::geometry::{Arc, Circle, Ellipse, Vec2};
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

fn ellipse(e: &Entity) -> Ellipse {
    match e {
        Entity::Ellipse(el) => *el,
        other => panic!("not an ellipse: {other:?}"),
    }
}

/// Twelve points of the user-space circle `(c, r)`, mapped to the world by
/// `f`, lie on `e`: five points fix a conic, so `e` is the exact image.
fn assert_image(e: &Ellipse, f: impl Fn(Vec2) -> Vec2, c: Vec2, r: f64) {
    for k in 0..12 {
        let a = f64::from(k) * TAU / 12.0;
        let p = f(c + Vec2::new(a.cos(), a.sin()) * r);
        let d = e.distance_to_point(p);
        assert!(d <= TOL, "{p:?} is {d} mm off {e:?}");
    }
}

/// LCV-176 AC 3 (replacing LCV-173 AC 7) — under a non-uniform scale or a
/// skew a circle or arc imports as the exact ellipse or elliptical arc,
/// unreported; the path's current point still advances.
#[test]
fn circles_and_arcs_under_non_similarities_import_as_ellipses() {
    let (es, report) = page(
        r#"<g transform="scale(2 1)">
             <circle cx="10" cy="10" r="5"/>
             <path d="M 10 50 A 10 10 0 0 1 30 50 L 40 50"/>
           </g>
           <circle transform="skewX(30)" cx="10" cy="10" r="5"/>"#,
    );
    assert!(report.is_empty(), "{report:?}");
    assert_eq!(es.len(), 4, "{es:?}");
    let scale = |p: Vec2| w(2.0 * p.x, p.y);
    let full = ellipse(&es[0]);
    assert_eq!(full.span, None);
    assert_image(&full, scale, Vec2::new(10.0, 10.0), 5.0);
    // The half turn from page (10, 50) over the top of the screen to
    // (30, 50), stretched: from (20, 50) through (40, 40) to (60, 50).
    let arc = ellipse(&es[1]);
    assert_image(
        &Ellipse { span: None, ..arc },
        scale,
        Vec2::new(20.0, 50.0),
        10.0,
    );
    let (from, to) = (arc.start_point(), arc.end_point());
    assert!(
        from.is_some_and(|p| p.approx_eq(w(20.0, 50.0), TOL)),
        "{arc:?}"
    );
    assert!(
        to.is_some_and(|p| p.approx_eq(w(60.0, 50.0), TOL)),
        "{arc:?}"
    );
    assert!(arc.distance_to_point(w(40.0, 40.0)) <= TOL, "{arc:?}");
    assert!(arc.distance_to_point(w(40.0, 60.0)) > 1.0, "{arc:?}");
    assert_line(&es[2], w(60.0, 50.0), w(80.0, 50.0));
    let t = 30f64.to_radians().tan();
    let skewed = ellipse(&es[3]);
    assert_eq!(skewed.span, None);
    assert_image(
        &skewed,
        |p| w(p.x + t * p.y, p.y),
        Vec2::new(10.0, 10.0),
        5.0,
    );
    assert!((skewed.rx * skewed.ry - 25.0).abs() <= TOL, "{skewed:?}");
}

/// LCV-176 AC 3 — `preserveAspectRatio="none"` with unequal scales turns a
/// circle into the exact ellipse.
#[test]
fn par_none_with_unequal_scales_imports_an_ellipse() {
    let src = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="50mm" viewBox="0 0 100 100" preserveAspectRatio="none"><circle cx="50" cy="50" r="5"/><line x1="0" y1="100" x2="100" y2="0"/></svg>"#;
    let imported = import_svg(src).unwrap();
    assert!(imported.report.is_empty(), "{:?}", imported.report);
    assert_eq!(imported.entities.len(), 2);
    let e = ellipse(&imported.entities[0]);
    let to_world = |p: Vec2| Vec2::new(p.x, 50.0 - 0.5 * p.y);
    assert_image(&e, to_world, Vec2::new(50.0, 50.0), 5.0);
    assert!(
        (e.rx - 5.0).abs() <= TOL && (e.ry - 2.5).abs() <= TOL,
        "{e:?}"
    );
    assert_line(
        &imported.entities[1],
        Vec2::new(0.0, 0.0),
        Vec2::new(100.0, 50.0),
    );
}

/// AC 9 — a nested `<svg>` maps its `viewBox` onto its `x y width height`
/// per `preserveAspectRatio`, its own `transform` applies in the parent
/// space, its content is imported whole (no clipping), and it is reported.
#[test]
fn nested_svg_is_a_further_mapping_unclipped_and_reported() {
    // viewBox 10 × 10 onto (10, 20, 40, 20), xMaxYMid meet: scale 2, content
    // 20 × 20 pushed right: (0, 0) → (30, 20). Then translate(1 2), then the
    // group's translate(5 5).
    let (es, report) = page(
        r#"<g transform="translate(5 5)">
             <svg x="10" y="20" width="40" height="20" viewBox="0 0 10 10"
                  preserveAspectRatio="xMaxYMid meet" transform="translate(1 2)">
               <line x1="0" y1="0" x2="10" y2="10"/>
               <line x1="0" y1="0" x2="20" y2="0"/>
               <circle cx="5" cy="5" r="1"/>
             </svg>
           </g>"#,
    );
    assert_eq!(report, [entry("svg (not clipped)", 1)]);
    assert_line(&es[0], w(36.0, 27.0), w(56.0, 47.0));
    // Past the nested viewport's right edge (x = 55 in the page): kept.
    assert_line(&es[1], w(36.0, 27.0), w(76.0, 27.0));
    let c = circle(&es[2]);
    assert!(
        c.center.approx_eq(w(46.0, 37.0), TOL) && (c.r - 2.0).abs() < TOL,
        "{c:?}"
    );
}

/// AC 9, AC 10 — without a viewBox a nested `<svg>` only translates by
/// `x y` (`%` of the parent viewport); `width`/`height` default to 100% and
/// are the viewport its own `%` lengths resolve against.
#[test]
fn nested_svg_without_view_box_translates_and_sets_the_viewport() {
    let (es, report) = page(
        r#"<svg x="10%" y="5"><line x1="0" y1="0" x2="1" y2="0"/></svg>
           <svg x="50" width="50%" height="20"><line x1="0" y1="0" x2="100%" y2="100%"/></svg>"#,
    );
    assert_eq!(report, [entry("svg (not clipped)", 2)]);
    assert_line(&es[0], w(10.0, 5.0), w(11.0, 5.0));
    assert_line(&es[1], w(50.0, 0.0), w(100.0, 20.0));
}

/// AC 5, AC 8, LCV-171 AC 7 — a `transform` style declaration (any ASCII
/// case) is applied like the attribute and wins over it (SVG 2); an
/// unparseable one is reported and ignored.
#[test]
fn a_css_transform_is_applied_and_wins_over_the_attribute() {
    let (es, report) = page(
        r#"<g style="transform:translate(10,10)"><line x1="1" y1="1" x2="2" y2="1"/></g>
           <g transform="translate(50,50)" style="fill:none; TRANSFORM : scale(2)">
             <line x1="1" y1="1" x2="2" y2="1"/></g>
           <g style="transform: none" transform="translate(5,5)"><line x1="1" y1="1" x2="2" y2="1"/></g>"#,
    );
    assert!(report.is_empty(), "{report:?}");
    assert_line(&es[0], w(11.0, 11.0), w(12.0, 11.0));
    assert_line(&es[1], w(2.0, 2.0), w(4.0, 2.0));
    assert_line(&es[2], w(1.0, 1.0), w(2.0, 1.0));

    let (es, report) =
        page(r#"<g style="transform: rotate(45deg)"><line x1="1" y1="1" x2="2" y2="1"/></g>"#);
    assert_eq!(report, [entry("transform (invalid)", 1)]);
    assert_line(&es[0], w(1.0, 1.0), w(2.0, 1.0));
}

/// AC 9 — a nested `<svg>` with a zero width or height renders nothing: it
/// imports nothing and is reported as singular, like a singular transform.
#[test]
fn a_zero_size_nested_svg_imports_nothing_and_is_reported() {
    let (es, report) = page(
        r#"<svg x="10" y="10" width="0" height="20" viewBox="0 0 10 10"><line x1="0" y1="0" x2="10" y2="10"/></svg>
           <svg x="10" y="10" width="20" height="0%"><line x1="0" y1="0" x2="10" y2="10"/></svg>"#,
    );
    assert!(es.is_empty(), "{es:?}");
    assert_eq!(report, [entry("transform (singular)", 2)]);
}

/// AC 9 — a nested `<svg>` with a negative width or height is an error in
/// SVG 2: it imports nothing (never a mirrored copy) and is reported.
#[test]
fn a_negative_size_nested_svg_imports_nothing_and_is_reported() {
    let (es, report) = page(
        r#"<svg x="10" y="10" width="-20" height="20" viewBox="0 0 10 10"><line x1="0" y1="0" x2="10" y2="10"/></svg>
           <svg x="10" y="10" width="20" height="-5"><line x1="0" y1="0" x2="10" y2="10"/></svg>"#,
    );
    assert!(es.is_empty(), "{es:?}");
    assert_eq!(report, [entry("transform (singular)", 2)]);
}
