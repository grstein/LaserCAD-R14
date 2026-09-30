//! LCV-171 AC 10 — files LaserCAD writes reopen with an empty import report:
//! the root `fill="none"` is exempt and `stroke`/`stroke-width` are not
//! reported properties.

use core::f64::consts::FRAC_PI_2;

use lasercad::document::{AddLayer, Command, Document, Entity};
use lasercad::geometry::{Arc, Circle, Line, Vec2};
use lasercad::io::svg::{export_layer_svg, export_svg, import_svg};

/// A line, a circle, a CCW and a CW arc on two layers: `Cut` (current) and
/// `Off` (Output off).
fn two_layer_doc() -> Document {
    let mut doc = Document::with_bed([300.0, 180.0]);
    let cut = doc.current_layer();
    let mut add = AddLayer::new("Off", [9, 9, 9], false);
    add.do_(&mut doc);
    let off = add.id().expect("allocated by do_");
    let line = Line::new(Vec2::new(10.0, 20.0), Vec2::new(60.0, 20.0));
    doc.push_entity(Entity::Line(line), cut);
    let circle = Circle::new(Vec2::new(50.0, 50.0), 5.0);
    doc.push_entity(Entity::Circle(circle), off);
    let ccw = Arc::new(Vec2::new(100.0, 100.0), 10.0, 0.0, FRAC_PI_2, true);
    doc.push_entity(Entity::Arc(ccw), cut);
    let cw = Arc::new(Vec2::new(150.0, 100.0), 10.0, FRAC_PI_2, 0.0, false);
    doc.push_entity(Entity::Arc(cw), off);
    doc
}

#[test]
fn exported_files_reopen_with_an_empty_report() {
    let doc = two_layer_doc();
    let mut files = vec![export_svg(&doc)];
    files.extend(doc.layers().iter().map(|l| export_layer_svg(&doc, l.id)));
    assert_eq!(files.len(), 3);
    for svg in files {
        let imported = import_svg(&svg).unwrap();
        assert!(!imported.entities.is_empty(), "{svg}");
        assert_eq!(imported.report, [], "{svg}");
    }
}
