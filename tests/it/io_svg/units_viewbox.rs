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
