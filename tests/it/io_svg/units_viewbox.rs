//! LCV-173 — SVG 2 lengths, units and the root viewBox, end to end through
//! `import_svg`.

use core::f64::consts::PI;
use lasercad::document::{Document, Entity};
use lasercad::geometry::{Arc, Circle, Line, Vec2};
use lasercad::io::svg::{export_svg, import_svg};

/// Every `f64` of `e` (an arc's `ccw` as 1.0/0.0), as bits.
fn bits(e: &Entity) -> Vec<u64> {
    let v = match *e {
        Entity::Line(l) => vec![l.p1.x, l.p1.y, l.p2.x, l.p2.y],
        Entity::Circle(c) => vec![c.center.x, c.center.y, c.r],
        Entity::Arc(a) => vec![
            a.center.x,
            a.center.y,
            a.r,
            a.start_angle,
            a.end_angle,
            f64::from(u8::from(a.ccw)),
        ],
    };
    v.into_iter().map(f64::to_bits).collect()
}

/// AC 11 — reopening LaserCAD's own export (`width="Wmm"`,
/// `viewBox="0 0 W H"`) yields exactly the entities and bed the importer
/// produced before LCV-173. The expected bits were captured from
/// `import_svg(&export_svg(doc))` at `feea39e`, the commit before this demand.
#[test]
fn own_export_reopens_bit_for_bit() {
    let mut doc = Document::with_bed([300.5, 180.25]);
    let v = Vec2::new;
    for e in [
        Entity::Line(Line::new(v(12.3456, 7.25), v(250.125, 170.0))),
        Entity::Line(Line::new(v(0.1, 0.2), v(-3.3, 1e-5))),
        Entity::Circle(Circle::new(v(150.3, 90.7), 33.3333)),
        Entity::Arc(Arc::new(v(100.1, 50.2), 20.7, 0.3, 2.9, true)),
        Entity::Arc(Arc::new(v(40.0, 120.0), 15.0, 1.0, 5.5, false)),
        Entity::Arc(Arc::new(v(200.0, 60.0), 10.0, 0.0, PI, true)),
    ] {
        doc.push_current(e);
    }
    let imported = import_svg(&export_svg(&doc)).unwrap();
    assert_eq!(
        imported.bed_mm.map(f64::to_bits),
        [300.5_f64, 180.25].map(f64::to_bits)
    );
    let want: [&[u64]; 6] = [
        &[
            0x4028b0f27bb2fec5,
            0x401d000000000000,
            0x406f440000000000,
            0x4065400000000000,
        ],
        &[
            0x3fb999999999999a,
            0x3fc9999999999800,
            0xc00a666666666666,
            0x0000000000000000,
        ],
        &[0x4062c9999999999a, 0x4056accccccccccd, 0x4040aaa9930be0df],
        &[
            0x40590666f22f3ba0,
            0x4049199ac8419778,
            0x4034b33333333333,
            0x3fd33332469ca797,
            0x40073333207682cb,
            0x3ff0000000000000,
        ],
        &[
            0x4043ffff3ae881ad,
            0x405e000075e1c84d,
            0x402e000000000000,
            0x3ff00000ebe9d344,
            0xbfe90fdf36f1bfdb,
            0x0000000000000000,
        ],
        &[
            0x4069000000000000,
            0x404e000000000000,
            0x4024000000000000,
            0x0000000000000000,
            0x400921fb54442d18,
            0x3ff0000000000000,
        ],
    ];
    assert_eq!(imported.entities.len(), want.len());
    for (i, (got, want)) in imported.entities.iter().zip(want).enumerate() {
        assert_eq!(bits(got), want, "entity {i}: {got:?}");
    }
    assert!(imported.report.is_empty(), "{:?}", imported.report);
}

/// Millimetres per px, 96 px = 1 in.
const MM_PER_PX: f64 = 25.4 / 96.0;

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// A 400 × 200 px page (one user unit = 1 px) holding `inner`.
fn px_page(inner: &str) -> Vec<Entity> {
    let src = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="400px" height="200px" viewBox="0 0 400 200">{inner}</svg>"#
    );
    let imported = import_svg(&src).unwrap();
    assert!(imported.report.is_empty(), "{:?}", imported.report);
    imported.entities
}

/// World point of an SVG point in px on the [`px_page`].
fn world(x_px: f64, y_px: f64) -> Vec2 {
    Vec2::new(x_px * MM_PER_PX, (200.0 - y_px) * MM_PER_PX)
}

fn assert_line(e: &Entity, a: Vec2, b: Vec2) {
    let Entity::Line(l) = e else {
        panic!("not a line: {e:?}")
    };
    assert!(
        l.p1.approx_eq(a, 1e-9) && l.p2.approx_eq(b, 1e-9),
        "{l:?} vs {a:?} {b:?}"
    );
}

/// AC 10 — `x1 y1 x2 y2` with absolute units land at their true size.
#[test]
fn line_attributes_with_absolute_units() {
    let es = px_page(r#"<line x1="10mm" y1="1cm" x2="1in" y2="72pt"/>"#);
    let mm = |x: f64, y: f64| world(x / MM_PER_PX, y / MM_PER_PX);
    assert_line(&es[0], mm(10.0, 10.0), mm(25.4, 25.4));
}

/// AC 10 — `%` resolves x against the viewport width, y against its height.
#[test]
fn line_percentages_resolve_against_the_viewport() {
    let es = px_page(r#"<line x1="50%" y1="25%" x2="100%" y2="0"/>"#);
    assert_line(&es[0], world(200.0, 50.0), world(400.0, 0.0));
}

/// AC 10 — `r` in `%` resolves against the normalized diagonal
/// `√(w² + h²) / √2`; `em` is 16 px and `ex` 8 px.
#[test]
fn circle_percent_radius_and_font_units() {
    let es = px_page(r#"<circle cx="1em" cy="2ex" r="10%"/><circle cx="0" cy="0" r="5mm"/>"#);
    let Entity::Circle(c) = es[0] else {
        panic!("not a circle: {:?}", es[0])
    };
    assert!(c.center.approx_eq(world(16.0, 16.0), 1e-9), "{c:?}");
    let diag = (400.0_f64.hypot(200.0)) / 2.0_f64.sqrt();
    assert!(near(c.r, 0.1 * diag * MM_PER_PX), "{c:?}");
    let Entity::Circle(c) = es[1] else {
        panic!("not a circle: {:?}", es[1])
    };
    assert!(near(c.r, 5.0), "{c:?}");
}
