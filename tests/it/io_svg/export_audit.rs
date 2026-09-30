//! LCV-170 AC 5..8, AC 10 — export audit: every SVG LaserCAD writes is
//! well-formed SVG 2 within the `AGENTS.md` export contract, reopens to the
//! same document, and is byte-identical to the pre-LCV-170 output.

use lasercad::document::{Document, Entity, Layer, LayerId};
use lasercad::geometry::{Arc, Circle, Line, Vec2};
use lasercad::io::svg::{export_layer_svg, export_svg};
use std::f64::consts::PI;

const SVG_NS: &str = "http://www.w3.org/2000/svg";

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

fn layer(id: u32, name: &str, color: [u8; 3], output: bool) -> Layer {
    Layer {
        id: LayerId(id),
        name: name.to_owned(),
        color,
        output,
    }
}

fn line(x1: f64, y1: f64, x2: f64, y2: f64) -> Entity {
    Entity::Line(Line::new(Vec2::new(x1, y1), Vec2::new(x2, y2)))
}

fn circle(x: f64, y: f64, r: f64) -> Entity {
    Entity::Circle(Circle::new(Vec2::new(x, y), r))
}

fn arc(x: f64, y: f64, r: f64, start: f64, end: f64, ccw: bool) -> Entity {
    Entity::Arc(Arc::new(Vec2::new(x, y), r, start, end, ccw))
}

/// The audit set (AC 5): an empty document; each entity kind, including
/// small, large, clockwise, half-turn and off-bed arcs; layers with Output on
/// and off, an empty layer and a current layer that is not the first; names
/// with `& < > " '` and non-ASCII characters.
fn audit_set() -> Vec<(&'static str, Document)> {
    let kinds = Document::from_parts(
        [297.25, 210.0],
        vec![Layer::default_cut()],
        LayerId(0),
        vec![
            line(10.0, 20.0, 60.5, 20.0),
            line(-5.25, -1.0, 400.0, 250.0),
            circle(50.0, 50.0, 5.0),
            circle(0.123_456, 7.0, 0.05),
            arc(150.0, 90.0, 10.0, 0.0, 2.0, true),
            arc(150.0, 90.0, 10.0, 0.5, 5.5, true),
            arc(80.0, 40.0, 12.5, 1.0, -0.25, false),
            arc(80.0, 40.0, 12.5, 3.0, 0.1, false),
            arc(100.0, 100.0, 20.0, 0.0, PI, true),
            arc(-10.0, 300.0, 3.0, -1.0, 1.0, true),
        ],
        vec![LayerId(0); 10],
    )
    .expect("one default layer");
    let layered = Document::from_parts(
        [400.0, 300.0],
        vec![
            layer(0, "Cut", [255, 0, 0], true),
            layer(3, "A&<>\"'é", [0, 0, 255], false),
            layer(1, "Gravação 日本", [0, 170, 0], true),
            layer(7, "it's empty", [9, 9, 9], false),
        ],
        LayerId(1),
        vec![
            line(0.0, 0.0, 400.0, 300.0),
            circle(200.0, 150.0, 25.0),
            arc(30.0, 40.0, 8.0, 0.25, 4.0, true),
            line(12.3456, 0.0, 12.3456, 300.0),
        ],
        vec![LayerId(0), LayerId(3), LayerId(1), LayerId(3)],
    )
    .expect("valid layer set");
    vec![
        ("empty", Document::default()),
        ("kinds", kinds),
        ("layered", layered),
        ("golden", golden_doc()),
    ]
}

/// Every export of the audit set, labelled: each mother file and each
/// per-layer file.
fn audit_exports() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (label, doc) in audit_set() {
        out.push((format!("{label} mother"), export_svg(&doc)));
        for l in doc.layers() {
            let text = export_layer_svg(&doc, l.id);
            out.push((format!("{label} layer {:?}", l.name), text));
        }
    }
    out
}

/// AC 5 — every audit export is well-formed XML whose root is `svg` in the
/// SVG namespace.
#[test]
fn every_audit_export_parses_with_an_svg_root() {
    for (label, text) in audit_exports() {
        let xml = roxmltree::Document::parse(&text)
            .unwrap_or_else(|e| panic!("{label}: not well-formed: {e}\n{text}"));
        let root = xml.root_element();
        assert_eq!(root.tag_name().name(), "svg", "{label}");
        assert_eq!(root.tag_name().namespace(), Some(SVG_NS), "{label}");
    }
}
