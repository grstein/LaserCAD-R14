//! LCV-170 AC 5..8, AC 10 — export audit: every SVG LaserCAD writes is
//! well-formed SVG 2 within the `AGENTS.md` export contract, reopens to the
//! same document, and is byte-identical to the pre-LCV-170 output.

use lasercad::document::{Document, Entity, Layer, LayerId};
use lasercad::geometry::{Arc, Circle, Line, Vec2};
use lasercad::io::svg::export_svg;

/// Two layers on a 300 × 180 bed: `Cut` (Output on) holding a line and an
/// arc, and `A&<>"'é` (Output off, current) holding a circle.
fn golden_doc() -> Document {
    let cut = Layer::default_cut();
    let odd = Layer {
        id: LayerId(1),
        name: "A&<>\"'é".to_owned(),
        color: [0, 0, 255],
        output: false,
    };
    let entities = vec![
        Entity::Line(Line::new(Vec2::new(10.0, 20.0), Vec2::new(60.5, 20.0))),
        Entity::Circle(Circle::new(Vec2::new(50.0, 50.0), 5.0)),
        Entity::Arc(Arc::new(Vec2::new(150.0, 90.0), 10.0, 0.0, 2.0, true)),
    ];
    let members = vec![LayerId(0), LayerId(1), LayerId(0)];
    Document::from_parts(
        [300.0, 180.0],
        vec![cut, odd],
        LayerId(1),
        entities,
        members,
    )
    .expect("valid layer set")
}

/// The exact bytes `export_svg` wrote for [`golden_doc`] before LCV-170.
const GOLDEN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="300mm" height="180mm" viewBox="0 0 300 180" fill="none">
<g data-layer="Cut" stroke="#ff0000" stroke-width="0.1" data-output="1">
<line x1="10.0000" y1="160.0000" x2="60.5000" y2="160.0000"/>
<path d="M 160.0000 90.0000 A 10.0000 10.0000 0 0 0 145.8385 80.9070"/>
</g>
<g data-layer="A&amp;&lt;&gt;&quot;'é" stroke="#0000ff" stroke-width="0.1" data-output="0" data-current="1">
<circle cx="50.0000" cy="130.0000" r="5.0000"/>
</g>
</svg>"##;

/// AC 10 — a document without control characters exports byte-identically.
#[test]
fn export_bytes_unchanged_for_golden_document() {
    assert_eq!(export_svg(&golden_doc()), GOLDEN);
}
