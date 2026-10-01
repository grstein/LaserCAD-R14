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

/// The contract attributes of each allowed element: `(required, optional)`.
/// `xmlns` is a namespace declaration, not an attribute, in `roxmltree`; the
/// root's namespace is checked in AC 5.
fn contract(element: &str) -> Option<(&'static [&'static str], &'static [&'static str])> {
    Some(match element {
        "svg" => (&["width", "height", "viewBox", "fill"], &[]),
        "g" => (
            &["data-layer", "stroke", "stroke-width", "data-output"],
            &["data-current"],
        ),
        "line" => (&["x1", "y1", "x2", "y2"], &[]),
        "circle" => (&["cx", "cy", "r"], &[]),
        // LCV-176 (ADR 0015): `transform="rotate(a cx cy)"` only when turned.
        "ellipse" => (&["cx", "cy", "rx", "ry"], &["transform"]),
        "path" => (&["d"], &[]),
        _ => return None,
    })
}

/// AC 6 — only `svg g line circle path`, all in the SVG namespace, each with
/// exactly its contract attributes; no text content; no namespace besides SVG.
#[test]
fn audit_exports_use_only_contract_elements_and_attributes() {
    for (label, text) in audit_exports() {
        let xml = roxmltree::Document::parse(&text).expect("AC 5");
        for node in xml.root().descendants() {
            if node.is_text() {
                let t = node.text().unwrap_or("");
                assert!(t.trim().is_empty(), "{label}: text {t:?}");
                continue;
            }
            if !node.is_element() {
                assert!(node.is_root(), "{label}: {node:?}");
                continue;
            }
            let name = node.tag_name().name();
            assert_eq!(
                node.tag_name().namespace(),
                Some(SVG_NS),
                "{label} <{name}>"
            );
            let (required, optional) =
                contract(name).unwrap_or_else(|| panic!("{label}: element <{name}>"));
            for a in node.attributes() {
                assert_eq!(a.namespace(), None, "{label} <{name}> {}", a.name());
                assert!(
                    required.contains(&a.name()) || optional.contains(&a.name()),
                    "{label}: <{name}> has {}",
                    a.name()
                );
            }
            for want in required {
                assert!(node.has_attribute(*want), "{label}: <{name}> lacks {want}");
            }
            let ns: Vec<_> = node.namespaces().map(|n| n.uri()).collect();
            assert_eq!(ns, [SVG_NS], "{label} <{name}> namespaces");
        }
        let root = xml.root_element();
        assert_eq!(root.attribute("fill"), Some("none"), "{label}");
        let current = xml
            .descendants()
            .filter(|n| n.attribute("data-current").is_some())
            .map(|n| n.attribute("data-current"))
            .collect::<Vec<_>>();
        assert!(
            current.len() <= 1 && current.iter().all(|c| *c == Some("1")),
            "{label}"
        );
    }
}

/// SVG 2 `number` (CSS syntax): `[+-]? (digits ('.' digits)? | '.' digits)
/// ([eE] [+-]? digits)?`, scanned by hand, and finite.
fn is_svg_number(s: &str) -> bool {
    fn digits(b: &[u8], i: &mut usize) -> usize {
        let start = *i;
        while b.get(*i).is_some_and(u8::is_ascii_digit) {
            *i += 1;
        }
        *i - start
    }
    let b = s.as_bytes();
    let mut i = 0;
    if matches!(b.first(), Some(b'+' | b'-')) {
        i += 1;
    }
    let int = digits(b, &mut i);
    if b.get(i) == Some(&b'.') {
        i += 1;
        if digits(b, &mut i) == 0 {
            return false;
        }
    } else if int == 0 {
        return false;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(b.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        if digits(b, &mut i) == 0 {
            return false;
        }
    }
    i == b.len() && s.parse::<f64>().is_ok_and(f64::is_finite)
}

#[test]
fn number_scanner_follows_svg_2() {
    for ok in [
        "0", "-0.0000", "12.3456", "+4", ".5", "1e3", "2.5E-4", "400",
    ] {
        assert!(is_svg_number(ok), "{ok}");
    }
    for bad in [
        "", "-", ".", "5.", "1e", "1e+", "NaN", "inf", "1,5", " 1", "1mm", "0x1", "--1", "1e999",
    ] {
        assert!(!is_svg_number(bad), "{bad}");
    }
}

/// `d` is exactly `M x y A r r 0 f f x y`: numbers, `r > 0` twice the same,
/// rotation `0`, flags `0|1`.
fn check_path(label: &str, d: &str) {
    let tok: Vec<&str> = d.split(' ').collect();
    assert_eq!(tok.len(), 11, "{label}: d={d:?}");
    assert_eq!(
        (tok[0], tok[3], tok[6]),
        ("M", "A", "0"),
        "{label}: d={d:?}"
    );
    for i in [1, 2, 4, 5, 9, 10] {
        assert!(is_svg_number(tok[i]), "{label}: d token {:?}", tok[i]);
    }
    for i in [7, 8] {
        assert!(matches!(tok[i], "0" | "1"), "{label}: flag {:?}", tok[i]);
    }
    assert_eq!(tok[4], tok[5], "{label}: rx = ry");
    assert!(
        tok[4].parse::<f64>().is_ok_and(|r| r > 0.0),
        "{label}: r {d:?}"
    );
}

/// AC 7 — every numeric value is a finite SVG 2 `number` and every `d` is a
/// single circular arc `M x y A r r 0 f f x y` with `r > 0`.
#[test]
fn audit_exports_write_svg_numbers_and_arc_paths() {
    for (label, text) in audit_exports() {
        let xml = roxmltree::Document::parse(&text).expect("AC 5");
        let num = |v: &str, what: &str| assert!(is_svg_number(v), "{label}: {what}={v:?}");
        for node in xml.descendants().filter(|n| n.is_element()) {
            let attr = |a: &str| node.attribute(a).unwrap_or("");
            match node.tag_name().name() {
                "svg" => {
                    for a in ["width", "height"] {
                        let v = attr(a).strip_suffix("mm").expect("mm unit");
                        num(v, a);
                    }
                    let vb: Vec<&str> = attr("viewBox").split(' ').collect();
                    assert_eq!(vb.len(), 4, "{label}: viewBox");
                    vb.iter().for_each(|v| num(v, "viewBox"));
                }
                "g" => num(attr("stroke-width"), "stroke-width"),
                "line" => ["x1", "y1", "x2", "y2"]
                    .iter()
                    .for_each(|a| num(attr(a), a)),
                "circle" => {
                    ["cx", "cy", "r"].iter().for_each(|a| num(attr(a), a));
                    assert!(attr("r").parse::<f64>().is_ok_and(|r| r > 0.0), "{label}");
                }
                "path" => check_path(&label, attr("d")),
                _ => {}
            }
        }
    }
}

/// Round-trip tolerance, mm: export writes four decimals (AC 8).
const TRIP_MM: f64 = 5e-4;

fn near(label: &str, what: &str, a: Vec2, b: Vec2) {
    assert!(
        (a.x - b.x).abs() <= TRIP_MM && (a.y - b.y).abs() <= TRIP_MM,
        "{label}: {what} {a:?} vs {b:?}"
    );
}

/// The entities on each layer, in layer order, each list in document order.
/// Export writes one group per layer, so entities interleaved across layers
/// come back grouped: order is kept within a layer, not across layers.
fn by_layer(doc: &Document) -> Vec<Vec<&Entity>> {
    doc.layers()
        .iter()
        .map(|l| {
            (0..doc.entity_count())
                .filter(|&i| doc.entity_layer(i) == Some(l.id))
                .map(|i| &doc.entities[i])
                .collect()
        })
        .collect()
}

/// AC 8 — export, `import_svg`, `into_document` restores the bed, the layers
/// (order, name, color, output, current) and the entities within 5e-4 mm,
/// each on its layer.
#[test]
fn audit_documents_survive_export_and_reopen() {
    for (label, doc) in audit_set() {
        let back = lasercad::io::svg::import_svg(&export_svg(&doc))
            .and_then(|imported| imported.into_document())
            .unwrap_or_else(|e| panic!("{label}: reopen failed: {e}"));
        assert_eq!(back.bed_mm, doc.bed_mm, "{label}: bed");
        let fields = |d: &Document| -> Vec<(String, [u8; 3], bool, bool)> {
            d.layers()
                .iter()
                .map(|l| (l.name.clone(), l.color, l.output, l.id == d.current_layer()))
                .collect()
        };
        assert_eq!(fields(&back), fields(&doc), "{label}: layers");
        assert_eq!(back.entity_count(), doc.entity_count(), "{label}");
        let (got_layers, want_layers) = (by_layer(&back), by_layer(&doc));
        let lens = |v: &[Vec<&Entity>]| v.iter().map(Vec::len).collect::<Vec<_>>();
        assert_eq!(lens(&got_layers), lens(&want_layers), "{label}: per layer");
        let pairs = got_layers
            .iter()
            .flatten()
            .zip(want_layers.iter().flatten());
        for (got, want) in pairs {
            match (got, want) {
                (Entity::Line(g), Entity::Line(w)) => {
                    near(label, "p1", g.p1, w.p1);
                    near(label, "p2", g.p2, w.p2);
                }
                (Entity::Circle(g), Entity::Circle(w)) => {
                    near(label, "center", g.center, w.center);
                    assert!((g.r - w.r).abs() <= TRIP_MM, "{label}: r");
                }
                (Entity::Arc(g), Entity::Arc(w)) => {
                    near(label, "center", g.center, w.center);
                    assert!((g.r - w.r).abs() <= TRIP_MM, "{label}: r");
                    near(label, "start", g.start_point(), w.start_point());
                    near(label, "end", g.end_point(), w.end_point());
                    assert_eq!(g.ccw, w.ccw, "{label}: ccw");
                }
                _ => panic!("{label}: kind {got:?} vs {want:?}"),
            }
        }
    }
}
