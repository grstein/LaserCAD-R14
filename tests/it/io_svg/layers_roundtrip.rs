//! LCV-156 AC 9 — the mother SVG carries every layer and the membership.
//!
//! Kernel-only: no `App`, no egui, no filesystem.

use lasercad::document::{
    AddLayer, Command, Document, EditLayer, Entity, Layer, LayerId, SetCurrentLayer,
};
use lasercad::geometry::{Arc, Circle, Line, Vec2};
use lasercad::io::svg::{SvgImportError, export_svg, import_svg};

fn line(x: f64) -> Entity {
    Entity::Line(Line::new(Vec2::new(x, 10.0), Vec2::new(x + 5.0, 20.0)))
}

fn add(doc: &mut Document, name: &str, color: [u8; 3], output: bool) -> LayerId {
    let mut cmd = AddLayer::new(name, color, output);
    cmd.do_(doc);
    cmd.id().expect("allocated by do_")
}

/// `Cut` (red) + `Mark` (blue, Output off) + an empty `Engrave`, entities
/// interleaved, `Mark` current, on a 300 × 180 bed.
fn layered_doc() -> (Document, LayerId, LayerId, LayerId) {
    let mut doc = Document::with_bed([300.0, 180.0]);
    let cut = doc.current_layer();
    let mark = add(&mut doc, "Mark & <score>", [0, 0, 255], false);
    let engrave = add(&mut doc, "Engrave", [0, 170, 0], true);
    doc.push_entity(line(0.0), cut);
    doc.push_entity(
        Entity::Circle(Circle::new(Vec2::new(50.0, 50.0), 5.0)),
        mark,
    );
    doc.push_entity(line(100.0), cut);
    let arc = Arc::new(Vec2::new(150.0, 90.0), 10.0, 0.0, 2.0, true);
    doc.push_entity(Entity::Arc(arc), mark);
    SetCurrentLayer::new(mark).do_(&mut doc);
    (doc, cut, mark, engrave)
}

fn reopen(svg: &str) -> Document {
    import_svg(svg)
        .expect("imports")
        .into_document()
        .expect("valid layers")
}

fn memberships(doc: &Document) -> Vec<String> {
    (0..doc.entity_count())
        .map(|i| {
            let id = doc.entity_layer(i).expect("in range");
            doc.layer(id).expect("exists").name.clone()
        })
        .collect()
}

/// AC 9 — one `<g>` per layer, in layer order, empty layers included, each
/// carrying name, color and Output; only the current layer is marked.
#[test]
fn mother_svg_writes_one_group_per_layer_in_order() {
    let (doc, ..) = layered_doc();
    let svg = export_svg(&doc);
    let cut =
        svg.find(r##"<g data-layer="Cut" stroke="#ff0000" stroke-width="0.1" data-output="1">"##);
    let mark = svg.find(
        r##"<g data-layer="Mark &amp; &lt;score&gt;" stroke="#0000ff" stroke-width="0.1" data-output="0" data-current="1">"##,
    );
    let engrave = svg
        .find(r##"<g data-layer="Engrave" stroke="#00aa00" stroke-width="0.1" data-output="1">"##);
    let (cut, mark, engrave) = (cut.expect(&svg), mark.expect(&svg), engrave.expect(&svg));
    assert!(cut < mark && mark < engrave, "{svg}");
    assert_eq!(svg.matches("<g ").count(), 3, "{svg}");
    assert_eq!(svg.matches("data-current").count(), 1, "{svg}");
    assert!(!svg.contains(" id="), "no id attribute: {svg}");
    let cut_body = &svg[cut..mark];
    assert_eq!(cut_body.matches("<line").count(), 2, "{svg}");
    let mark_body = &svg[mark..engrave];
    assert!(
        mark_body.contains("<circle") && mark_body.contains("<path"),
        "{svg}"
    );
}

/// AC 9 — reopening restores layers (names, colors, Output, order), the
/// membership and the current layer, and re-saving is byte-stable.
#[test]
fn mother_svg_round_trips_layers_membership_and_current() {
    let (doc, ..) = layered_doc();
    let svg = export_svg(&doc);
    let back = reopen(&svg);
    let strip = |d: &Document| -> Vec<(String, [u8; 3], bool)> {
        d.layers()
            .iter()
            .map(|l| (l.name.clone(), l.color, l.output))
            .collect()
    };
    assert_eq!(strip(&back), strip(&doc));
    assert_eq!(
        back.layer(back.current_layer()).expect("current").name,
        "Mark & <score>"
    );
    // Entities come back grouped by layer, in layer order.
    assert_eq!(
        memberships(&back),
        ["Cut", "Cut", "Mark & <score>", "Mark & <score>"]
    );
    assert_eq!(back.bed_mm, [300.0, 180.0]);
    assert_eq!(export_svg(&back), svg, "re-save is byte-stable");
}

/// AC 9 — renamed and recolored layers survive too.
#[test]
fn edited_layer_round_trips() {
    let (mut doc, cut, ..) = layered_doc();
    let edited = Layer {
        name: "Outer cut".into(),
        color: [0x12, 0xab, 0xef],
        output: false,
        id: cut,
    };
    EditLayer::new(edited).do_(&mut doc);
    let back = reopen(&export_svg(&doc));
    let first = &back.layers()[0];
    assert_eq!(
        (first.name.as_str(), first.color, first.output),
        ("Outer cut", [0x12, 0xab, 0xef], false)
    );
}

const HEADER: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100" fill="none">"#;

fn file(body: &str) -> String {
    format!("{HEADER}\n{body}\n</svg>")
}

/// ADR 0012 §4 — geometry outside any layer group goes to the first layer;
/// a file with no layer group (a v0.2 file) gets the default `Cut` layer.
#[test]
fn stray_geometry_goes_to_the_first_layer() {
    let l = r#"<line x1="1" y1="1" x2="2" y2="2"/>"#;
    let body = format!(
        r##"{l}<g data-layer="A" stroke="#00FF00" data-output="0">{l}</g><g data-layer="B" stroke="#0000ff" data-current="1"/>{l}"##
    );
    let doc = reopen(&file(&body));
    assert_eq!(memberships(&doc), ["A", "A", "A"]);
    assert_eq!(doc.layers()[0].color, [0, 255, 0]);
    assert!(
        !doc.layers()[0].output && doc.layers()[1].output,
        "absent data-output = 1"
    );
    assert_eq!(doc.layer(doc.current_layer()).expect("current").name, "B");

    let v02 = file(&format!(
        r##"<g id="mark" stroke="#0000ff" stroke-width="0.1">{l}</g>"##
    ));
    let doc = reopen(&v02);
    assert_eq!(doc.layers(), &[Layer::default_cut()]);
    assert_eq!(memberships(&doc), ["Cut"]);
}

/// ADR 0012 §4 — no `data-current` means the first layer is current.
#[test]
fn first_layer_is_current_without_data_current() {
    let body = r##"<g data-layer="A" stroke="#010101"/><g data-layer="B" stroke="#020202"/>"##;
    let doc = reopen(&file(body));
    assert_eq!(doc.layer(doc.current_layer()).expect("current").name, "A");
}

/// AC 9 / ADR 0012 §4 — a bad layer group is refused with `MalformedLayer`.
#[test]
fn malformed_layers_are_refused() {
    let bad = [
        r##"<g data-layer="A" stroke="reddish"/>"##,
        r##"<g data-layer="A"/>"##,
        r##"<g data-layer="A" stroke="#ff0000" data-output="yes"/>"##,
        r##"<g data-layer="A" stroke="#ff0000"/><g data-layer="a" stroke="#00ff00"/>"##,
        r##"<g data-layer="A" stroke="#ff0000"/><g data-layer="B" stroke="#FF0000"/>"##,
        r##"<g data-layer="  " stroke="#ff0000"/>"##,
    ];
    for body in bad {
        let err = import_svg(&file(body)).map(|_| ()).expect_err(body);
        assert!(
            matches!(err, SvgImportError::MalformedLayer { .. }),
            "{body}: {err:?}"
        );
        assert!(!err.to_string().is_empty());
    }
}

/// The color of the only layer of a file whose body is `body`, or the error.
fn layer_color(header: &str, body: &str) -> Result<[u8; 3], SvgImportError> {
    let svg = format!("{header}\n{body}\n</svg>");
    let doc = import_svg(&svg)?.into_document().expect("valid layers");
    Ok(doc.layers()[0].color)
}

/// AC 16 — a layer's stroke is read from any CSS color: named keywords,
/// `#rgb`, `#rrggbb` in any case, `rgb()`, `hsl()`; `style="stroke:…"` wins
/// over the attribute. Export stays lowercase `#rrggbb`.
#[test]
fn layer_stroke_accepts_any_css_color() {
    let cases: [(&str, [u8; 3]); 20] = [
        (r##"stroke="#1F8""##, [0x11, 0xff, 0x88]),
        (r##"stroke="hsl(3.14159265rad 100% 50%)""##, [0, 255, 255]),
        (r##"stroke="hsl(200grad 100% 50%)""##, [0, 255, 255]),
        (r##"stroke="hsl(0, 100%, 75%)""##, [255, 128, 128]),
        (r##"stroke="hsl(30, 100%, 50%)""##, [255, 128, 0]),
        (r##"stroke="red""##, [255, 0, 0]),
        (r##"stroke="RebeccaPurple""##, [102, 51, 153]),
        (r##"stroke=" navy ""##, [0, 0, 128]),
        (r##"stroke="#0F0""##, [0, 255, 0]),
        (r##"stroke="#00AAff""##, [0, 170, 255]),
        (r##"stroke="rgb(255, 128, 0)""##, [255, 128, 0]),
        (r##"stroke="RGB(0 0 255 / 50%)""##, [0, 0, 255]),
        (r##"stroke="rgba(100%, 0%, 50%, 0.5)""##, [255, 0, 128]),
        (r##"stroke="rgb(300, -5, 12.6)""##, [255, 0, 13]),
        (r##"stroke="hsl(120, 100%, 25%)""##, [0, 128, 0]),
        (r##"stroke="hsl(0.5turn 100% 50%)""##, [0, 255, 255]),
        (r##"stroke="hsla(-120deg, 100%, 50%, 1)""##, [0, 0, 255]),
        (
            r##"style="fill:none; stroke: #123456" stroke="#ffffff""##,
            [0x12, 0x34, 0x56],
        ),
        (
            r##"style="fill:none;STROKE:teal !important;" "##,
            [0, 128, 128],
        ),
        (r##"style="stroke:red; stroke:blue""##, [0, 0, 255]),
    ];
    for (attrs, want) in cases {
        let body = format!(r##"<g data-layer="A" {attrs}/>"##);
        let got = layer_color(HEADER, &body).unwrap_or_else(|e| panic!("{attrs}: {e}"));
        assert_eq!(got, want, "{attrs}");
    }
    let doc = reopen(&file(r##"<g data-layer="A" stroke="#00AAff"/>"##));
    assert!(
        export_svg(&doc).contains(r##"stroke="#00aaff""##),
        "lowercase on export"
    );
}

/// AC 16 — a layer group with no stroke of its own inherits the nearest
/// ancestor `<g>`/`<svg>` stroke (style still winning there); its own stroke
/// beats any ancestor's.
#[test]
fn layer_stroke_inherits_from_an_ancestor() {
    let svg_stroke = HEADER.replace(r##"fill="none">"##, r##"fill="none" stroke="navy">"##);
    assert_ne!(svg_stroke, HEADER, "control: the header was rewritten");
    let cases: [(&str, &str, [u8; 3]); 5] = [
        (
            HEADER,
            r##"<g stroke="lime"><g data-layer="A"/></g>"##,
            [0, 255, 0],
        ),
        (
            HEADER,
            r##"<g stroke="red" style="stroke:blue"><g><g data-layer="A"/></g></g>"##,
            [0, 0, 255],
        ),
        (
            HEADER,
            r##"<g stroke="red"><g stroke="lime"><g data-layer="A"/></g></g>"##,
            [0, 255, 0],
        ),
        (
            HEADER,
            r##"<g stroke="lime"><g data-layer="A" stroke="#0000ff"/></g>"##,
            [0, 0, 255],
        ),
        (&svg_stroke, r##"<g data-layer="A"/>"##, [0, 0, 128]),
    ];
    for (header, body, want) in cases {
        let got = layer_color(header, body).unwrap_or_else(|e| panic!("{body}: {e}"));
        assert_eq!(got, want, "{body}");
    }
}

/// AC 16 — an invalid or unsupported stroke color is a malformed layer,
/// including an invalid `style` stroke over a valid attribute.
#[test]
fn unsupported_layer_colors_are_refused() {
    let bad = [
        r##"<g data-layer="A" stroke="none"/>"##,
        r##"<g data-layer="A" stroke="currentColor"/>"##,
        r##"<g data-layer="A" stroke="transparent"/>"##,
        r##"<g data-layer="A" stroke="url(#grad)"/>"##,
        r##"<g data-layer="A" stroke="#ff00"/>"##,
        r##"<g data-layer="A" stroke="#ff0000cc"/>"##,
        r##"<g data-layer="A" stroke="#gg0000"/>"##,
        r##"<g data-layer="A" stroke="rgb(1, 2)"/>"##,
        r##"<g data-layer="A" stroke="rgb (1, 2, 3)"/>"##,
        r##"<g data-layer="A" stroke="rgb(1, 2, x)"/>"##,
        r##"<g data-layer="A" stroke="hsl(1, 2%, 3%, 4, 5)"/>"##,
        r##"<g data-layer="A" stroke="cmyk(1, 2, 3)"/>"##,
        r##"<g data-layer="A" style="stroke:#zzz" stroke="red"/>"##,
        r##"<g data-layer="A" style="stroke:" stroke="red"/>"##,
    ];
    for body in bad {
        let err = layer_color(HEADER, body).expect_err(body);
        assert!(
            matches!(err, SvgImportError::MalformedLayer { .. }),
            "{body}: {err:?}"
        );
    }
    let ok = r##"<g data-layer="A" style="fill:none" stroke="red"/>"##;
    assert_eq!(layer_color(HEADER, ok).expect("control"), [255, 0, 0]);
}

/// ADR 0012 §4 — the first `data-current="1"` layer is current; a later
/// one does not take over.
#[test]
fn the_first_data_current_layer_wins() {
    let body = r##"<g data-layer="A" stroke="#010101"/><g data-layer="B" stroke="#020202" data-current="1"/><g data-layer="C" stroke="#030303" data-current="1"/>"##;
    let doc = reopen(&file(body));
    assert_eq!(doc.layer(doc.current_layer()).expect("current").name, "B");
}

/// ADR 0012 §3 — a double quote in a layer name is escaped in the attribute,
/// so the file stays well-formed and the name round-trips.
#[test]
fn a_quote_in_a_layer_name_is_escaped() {
    let mut doc = Document::default();
    add(&mut doc, r#"Say "hi""#, [0, 0, 255], true);
    let svg = export_svg(&doc);
    assert!(svg.contains(r#"data-layer="Say &quot;hi&quot;""#), "{svg}");
    assert_eq!(reopen(&svg).layers()[1].name, r#"Say "hi""#);
}
