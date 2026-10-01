//! LCV-156 — a v0.2 preset file (`<g id="cut|mark|engrave">`) still opens.
//!
//! The global export preset is gone (AC 15); a v0.2 file declares no layer,
//! so its geometry lands on one `#rrggbb` layer per stroke color it uses
//! (ADR 0012 §4 as amended by LCV-175), with every world coordinate intact,
//! and a re-save writes the layer format.
//!
//! Kernel-only: no `App`, no egui, no filesystem.

use lasercad::document::{Entity, Layer, LayerId};
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

/// A v0.2 marking file opens on one blue `#0000ff` layer (its empty
/// `cut`/`engrave` groups make none), keeps its bed and world coordinates,
/// and re-saves as that layer's group.
#[test]
fn v02_preset_file_opens_on_one_layer_per_color() {
    let doc = import_svg(V02_MARK_FILE)
        .expect("a v0.2 file imports")
        .into_document()
        .expect("color layer is valid");
    let blue = Layer {
        id: LayerId(0),
        name: "#0000ff".to_owned(),
        color: [0, 0, 255],
        output: true,
    };
    assert_eq!(doc.layers(), &[blue]);
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
        resaved.contains(r##"<g data-layer="#0000ff" stroke="#0000ff""##),
        "{resaved}"
    );
    assert!(!resaved.contains(" id="), "{resaved}");
}
