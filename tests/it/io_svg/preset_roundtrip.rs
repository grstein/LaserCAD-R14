//! LCV-156 — a v0.2 preset file (`<g id="cut|mark|engrave">`) still opens.
//!
//! The global export preset is gone (AC 15); migration of v0.2 files is out
//! of scope, so their geometry lands on the default `Cut` layer (ADR 0012 §4)
//! with every world coordinate intact, and a re-save writes the layer format.
//!
//! Kernel-only: no `App`, no egui, no filesystem.

use lasercad::document::{Entity, Layer};
use lasercad::geometry::{Line, Vec2};
use lasercad::io::svg::{export_svg, import_svg};

/// Millimetres.
const TOL_MM: f64 = 1e-9;

/// What the v0.2 exporter wrote for a marking job on a 300 × 180 bed: an
/// empty `cut` group, the geometry in `mark`, an empty `engrave`.
const V02_MARK_FILE: &str = concat!(
    "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"300mm\" height=\"180mm\"",
    " viewBox=\"0 0 300 180\" fill=\"none\">\n",
    "<g id=\"cut\" stroke=\"#ff0000\" stroke-width=\"0.1\">\n</g>\n",
    "<g id=\"mark\" stroke=\"#0000ff\" stroke-width=\"0.1\">\n",
    "<line x1=\"10.0000\" y1=\"130.0000\" x2=\"250.0000\" y2=\"60.0000\"/>\n",
    "</g>\n",
    "<g id=\"engrave\" stroke=\"#00aa00\" stroke-width=\"0.1\">\n</g>\n",
    "</svg>",
);

/// A v0.2 marking file opens on `Cut`, keeps its bed and world coordinates,
/// and re-saves as one `Cut` layer group.
#[test]
fn v02_preset_file_opens_on_the_default_layer() {
    let doc = import_svg(V02_MARK_FILE)
        .expect("a v0.2 file imports")
        .into_document()
        .expect("default layer is valid");
    assert_eq!(doc.layers(), &[Layer::default_cut()]);
    assert_eq!(doc.bed_mm, [300.0, 180.0]);
    assert_eq!(doc.entity_count(), 1);
    let Entity::Line(l) = doc.entities[0] else {
        panic!("expected a line")
    };
    let want = Line::new(Vec2::new(10.0, 50.0), Vec2::new(250.0, 120.0));
    assert!(
        l.p1.approx_eq(want.p1, TOL_MM) && l.p2.approx_eq(want.p2, TOL_MM),
        "{l:?}"
    );

    let resaved = export_svg(&doc);
    assert!(
        resaved.contains(r##"<g data-layer="Cut" stroke="#ff0000""##),
        "{resaved}"
    );
    assert!(!resaved.contains(" id="), "{resaved}");
}
