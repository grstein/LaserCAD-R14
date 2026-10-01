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
