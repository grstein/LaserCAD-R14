//! LCV-175 AC 8–10, AC 12 — geometry outside any `<g data-layer>` lands on a
//! layer of its stroke (else fill) color; LaserCAD's own files reopen as
//! before.
//!
//! Kernel-only: no `App`, no egui, no filesystem.

use lasercad::document::{Layer, LayerId};
use lasercad::io::svg::{ImportedSvg, import_svg};

const HEADER: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100">"#;

fn import(body: &str) -> ImportedSvg {
    import_svg(&format!("{HEADER}\n{body}\n</svg>")).expect("imports")
}

fn line(attrs: &str) -> String {
    format!(r#"<line x1="1" y1="1" x2="2" y2="2" {attrs}/>"#)
}

/// `(name, color, output)` per layer, in order.
fn layers(imported: &ImportedSvg) -> Vec<(String, [u8; 3], bool)> {
    let row = |l: &Layer| (l.name.clone(), l.color, l.output);
    imported.layers.iter().map(row).collect()
}

/// Each entity's layer name, in entity order.
fn memberships(imported: &ImportedSvg) -> Vec<String> {
    let name = |id: LayerId| {
        let layer = imported.layers.iter().find(|l| l.id == id);
        layer.map_or_else(|| "?".to_owned(), |l| l.name.clone())
    };
    imported.entity_layers.iter().map(|&id| name(id)).collect()
}

fn row(name: &str, color: [u8; 3], output: bool) -> (String, [u8; 3], bool) {
    (name.to_owned(), color, output)
}

/// The two declared layers every mixed test starts from.
const DECLARED: &str =
    r##"<g data-layer="A" stroke="#00ff00" data-output="0"/><g data-layer="R" stroke="#ff0000"/>"##;

/// AC 8 — a stray red line reuses the declared `#ff0000` layer.
#[test]
fn a_stray_color_reuses_the_declared_layer_of_that_color() {
    let imported = import(&format!(r#"{DECLARED}{}"#, line(r#"stroke="red""#)));
    assert_eq!(memberships(&imported), ["R"]);
    assert_eq!(imported.layers.len(), 2);
}

/// AC 8 — new colors append `#rrggbb` layers with Output on, in order of
/// first appearance, after the declared ones.
#[test]
fn new_colors_append_hex_layers_in_first_appearance_order() {
    let body = format!(
        "{DECLARED}{}{}{}",
        line(r#"stroke="blue""#),
        line(r##"style="stroke:#00AA00""##),
        line(r#"stroke="rgb(0,0,255)""#),
    );
    let imported = import(&body);
    let want = [
        row("A", [0, 255, 0], false),
        row("R", [255, 0, 0], true),
        row("#0000ff", [0, 0, 255], true),
        row("#00aa00", [0, 170, 0], true),
    ];
    assert_eq!(layers(&imported), want);
    assert_eq!(memberships(&imported), ["#0000ff", "#00aa00", "#0000ff"]);
    assert_eq!(imported.current_layer, imported.layers[0].id);
}

/// AC 9 — `stroke:none` with a fill color goes to the fill's layer; the
/// outline is imported and `fill` reported.
#[test]
fn a_fill_color_stands_in_for_a_missing_stroke() {
    let body = format!(
        r#"{}<circle cx="5" cy="5" r="2" fill="blue"/>"#,
        line(r#"style="stroke:none;fill:blue""#)
    );
    let imported = import(&body);
    assert_eq!(memberships(&imported), ["#0000ff", "#0000ff"]);
    assert_eq!(imported.report, [("fill".to_owned(), 2)]);
}

/// AC 10 — an unstyled line goes to the first layer.
#[test]
fn an_unstyled_line_goes_to_the_first_layer() {
    let imported = import(&format!("{DECLARED}{}", line("")));
    assert_eq!(memberships(&imported), ["A"]);
    assert_eq!(imported.layers.len(), 2);
}

/// AC 10 — no layer declared and every entity colored: no `Cut`.
#[test]
fn all_colored_geometry_makes_no_default_layer() {
    let body = format!(
        "{}{}",
        line(r##"stroke="#00f""##),
        line(r##"stroke="#0000FF""##)
    );
    let imported = import(&body);
    assert_eq!(layers(&imported), [row("#0000ff", [0, 0, 255], true)]);
    assert_eq!(memberships(&imported), ["#0000ff", "#0000ff"]);
}

/// AC 8, AC 10 — no layer declared and some geometry uncolored: `Cut`
/// first, and a red stray lands on it (same color).
#[test]
fn mixed_geometry_keeps_cut_first() {
    let body = format!(
        "{}{}{}",
        line(r#"stroke="blue""#),
        line(""),
        line(r#"stroke="red""#)
    );
    let imported = import(&body);
    let want = [
        row("Cut", [255, 0, 0], true),
        row("#0000ff", [0, 0, 255], true),
    ];
    assert_eq!(layers(&imported), want);
    assert_eq!(memberships(&imported), ["#0000ff", "Cut", "Cut"]);
}

/// AC 8 — a `#rrggbb` name already taken by a layer of another color gets
/// a counter.
#[test]
fn a_taken_hex_name_gets_a_counter() {
    let body = format!(
        r##"<g data-layer="#ff0000" stroke="#00ff00"/><g data-layer="ff0000 2" stroke="#0000ff"/>{}"##,
        line(r#"stroke="red""#)
    );
    let imported = import(&body);
    assert_eq!(memberships(&imported), ["#ff0000 3"]);
    assert!(imported.clone().into_document().is_ok());
}

/// `Cut` + `Mark` (Output off, current) + an empty `Engrave`, entities
/// interleaved, on a 300 × 180 bed.
fn layered_doc() -> lasercad::document::Document {
    use lasercad::document::{AddLayer, Command, Document, Entity, SetCurrentLayer};
    use lasercad::geometry::{Arc, Circle, Line, Vec2};
    let mut doc = Document::with_bed([300.0, 180.0]);
    let cut = doc.current_layer();
    let mut add = |name: &str, color, output| {
        let mut cmd = AddLayer::new(name, color, output);
        cmd.do_(&mut doc);
        cmd.id().expect("allocated by do_")
    };
    let mark = add("Mark", [0, 0, 255], false);
    add("Engrave", [0, 170, 0], true);
    let line = Entity::Line(Line::new(Vec2::new(1.0, 2.0), Vec2::new(30.0, 40.0)));
    doc.push_entity(line, cut);
    let circle = Entity::Circle(Circle::new(Vec2::new(50.0, 50.0), 5.0));
    doc.push_entity(circle, mark);
    let arc = Entity::Arc(Arc::new(Vec2::new(150.0, 90.0), 10.0, 0.0, 2.0, true));
    doc.push_entity(arc, cut);
    SetCurrentLayer::new(mark).do_(&mut doc);
    doc
}

/// AC 12 — the mother SVG and every per-layer file reopen with identical
/// layers, colors, Output, current layer and memberships, report empty.
#[test]
fn lasercad_files_reopen_exactly() {
    use lasercad::io::svg::{export_layer_svg, export_svg};
    let doc = layered_doc();
    let rows = |layers: &[Layer]| -> Vec<_> {
        layers
            .iter()
            .map(|l| (l.name.clone(), l.color, l.output))
            .collect()
    };
    let mother = export_svg(&doc);
    let imported = import_svg(&mother).expect("imports");
    assert_eq!(layers(&imported), rows(doc.layers()));
    assert_eq!(imported.current_layer, doc.current_layer());
    assert_eq!(memberships(&imported), ["Cut", "Cut", "Mark"]);
    assert_eq!(imported.report, []);
    let back = imported.into_document().expect("valid layers");
    assert_eq!(export_svg(&back), mother, "re-save is byte-stable");
    for layer in doc.layers() {
        let one = import_svg(&export_layer_svg(&doc, layer.id)).expect("imports");
        assert_eq!(
            layers(&one),
            rows(std::slice::from_ref(layer)),
            "{}",
            layer.name
        );
        let count = (0..doc.entity_count())
            .filter(|&i| doc.entity_layer(i) == Some(layer.id))
            .count();
        assert_eq!(memberships(&one), vec![layer.name.clone(); count]);
        assert_eq!(one.report, [], "{}", layer.name);
    }
}

/// AC 12 — the `v03-mother-three-layers` corpus seed reopens as written.
#[test]
fn the_v03_mother_seed_reopens_exactly() {
    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/svg/v03-mother-three-layers.svg"
    ));
    let imported = import_svg(src).expect("imports");
    let want = [
        row("Cut", [255, 0, 0], true),
        row("Fine mark", [0, 0, 255], true),
        row("Guide", [0, 170, 0], false),
    ];
    assert_eq!(layers(&imported), want);
    assert_eq!(imported.current_layer, imported.layers[1].id);
    let want = ["Cut", "Cut", "Cut", "Fine mark", "Guide"];
    assert_eq!(memberships(&imported), want);
    assert_eq!(imported.report, []);
}

/// AC 12 — a sheet rule never recolors a `<g data-layer>`; its geometry
/// stays on it.
#[test]
fn a_sheet_rule_does_not_recolor_a_layer_group() {
    let body = format!(
        r##"<style>g{{stroke:blue}} *{{stroke:lime}}</style><g data-layer="A" stroke="#ff0000">{}</g>"##,
        line("")
    );
    let imported = import(&body);
    assert_eq!(layers(&imported), [row("A", [255, 0, 0], true)]);
    assert_eq!(memberships(&imported), ["A"]);
}
