use super::*;
use crate::document::state::Document;
use crate::geometry::{Arc, Circle, Line, Vec2};
use core::f64::consts::{FRAC_PI_2, PI};

fn doc_with(entity: Entity) -> Document {
    let mut doc = Document::default();
    doc.push_current(entity);
    doc
}

/// Extract the `d` attribute value of the single `<path>` in `svg`.
fn path_d(svg: &str) -> String {
    let start = svg.find("<path d=\"").expect("no <path> in output") + 9;
    let rest = &svg[start..];
    let end = rest.find('"').expect("unterminated d attribute");
    rest[..end].to_string()
}

/// LCV-056 AC#1, AC#2, AC#5 — root `<svg>` carries xmlns and fill="none".
#[test]
fn svg_root_has_required_attributes() {
    let svg = export_svg(&Document::default());
    assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(svg.contains("fill=\"none\""));
}

/// LCV-100 AC 11 — the canvas is the bed for every document, empty or not.
/// Replaces LCV-056's bounds-derived canvas and its `100mm` empty-document
/// fallback.
#[test]
fn canvas_is_always_the_bed() {
    for svg in [
        export_svg(&Document::default()),
        export_svg(&doc_with(Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(50.0, 30.0),
        )))),
    ] {
        assert!(svg.contains("width=\"400mm\""), "{svg}");
        assert!(svg.contains("height=\"400mm\""), "{svg}");
        assert!(svg.contains("viewBox=\"0 0 400 400\""), "{svg}");
        assert!(!svg.contains("100mm"), "{svg}");
    }
}

/// LCV-056 AC#6 — groups emitted with correct colours and
/// stroke-width. LCV-156: one group per layer, so a new document has one.
#[test]
fn one_group_per_layer_emitted() {
    let svg = export_svg(&Document::default());
    assert!(svg.contains("stroke=\"#ff0000\""), "{svg}");
    assert_eq!(svg.matches("<g ").count(), 1, "{svg}");
    assert_eq!(svg.matches("stroke-width=\"0.1\"").count(), 1, "{svg}");
}

/// LCV-056 AC#7 — Line entity encodes to `<line …/>` inside the cut group.
#[test]
fn line_entity_encodes_to_svg_line() {
    let svg = export_svg(&doc_with(Entity::Line(Line::new(
        Vec2::new(1.0, 2.0),
        Vec2::new(11.0, 7.0),
    ))));
    let expected = "<line x1=\"1.0000\" y1=\"398.0000\" x2=\"11.0000\" y2=\"393.0000\"/>";
    assert!(svg.contains(expected), "{svg}");
    let line_pos = svg.find(expected).unwrap();
    let close_pos = svg.find("</g>").unwrap();
    assert!(line_pos < close_pos, "line outside cut group: {svg}");
}

/// LCV-100 AC 4 — `y1`/`y2` are mirrored, `x1`/`x2` are not.
#[test]
fn line_y_is_flipped_on_export() {
    let svg = export_svg(&doc_with(Entity::Line(Line::new(
        Vec2::new(10.0, 10.0),
        Vec2::new(10.0, 60.0),
    ))));
    assert!(
        svg.contains("<line x1=\"10.0000\" y1=\"390.0000\" x2=\"10.0000\" y2=\"340.0000\"/>"),
        "{svg}"
    );
}

/// LCV-056 AC#8 — Circle entity encodes to `<circle …/>`.
#[test]
fn circle_entity_encodes_to_svg_circle() {
    let svg = export_svg(&doc_with(Entity::Circle(Circle::new(
        Vec2::new(5.0, 5.0),
        3.0,
    ))));
    assert!(
        svg.contains("<circle cx=\"5.0000\" cy=\"395.0000\" r=\"3.0000\"/>"),
        "{svg}"
    );
}

/// LCV-100 AC 5 — `cy` is mirrored; `cx` and `r` are untouched.
#[test]
fn circle_cy_is_flipped_on_export() {
    let svg = export_svg(&doc_with(Entity::Circle(Circle::new(
        Vec2::new(100.0, 250.0),
        5.0,
    ))));
    assert!(
        svg.contains("<circle cx=\"100.0000\" cy=\"150.0000\" r=\"5.0000\"/>"),
        "{svg}"
    );
}

/// LCV-100 AC 6, AC 7 — CCW quarter arc, golden `d`: mirrored endpoints
/// keep their roles, `large` stays 0, `sweep` is 0 because the mirror
/// reverses handedness.
#[test]
fn arc_ccw_quarter_golden() {
    let svg = export_svg(&doc_with(Entity::Arc(Arc::new(
        Vec2::new(0.0, 0.0),
        10.0,
        0.0,
        FRAC_PI_2,
        true,
    ))));
    assert_eq!(
        path_d(&svg),
        "M 10.0000 400.0000 A 10.0000 10.0000 0 0 0 0.0000 390.0000"
    );
}

/// LCV-100 AC 8 — CW quarter arc, golden `d` (sweep flag 1).
#[test]
fn arc_cw_quarter_golden() {
    let svg = export_svg(&doc_with(Entity::Arc(Arc::new(
        Vec2::new(0.0, 0.0),
        10.0,
        FRAC_PI_2,
        0.0,
        false,
    ))));
    assert_eq!(
        path_d(&svg),
        "M 0.0000 390.0000 A 10.0000 10.0000 0 0 1 10.0000 400.0000"
    );
}

/// LCV-100 AC 9 — semicircle: the chord is the diameter, so the centre is
/// unique and `large` is irrelevant; only the sweep flag picks the half.
/// The two goldens differ in exactly one character.
#[test]
fn arc_semicircle_sweep_selects_correct_half() {
    let upper = export_svg(&doc_with(Entity::Arc(Arc::new(
        Vec2::new(0.0, 0.0),
        10.0,
        0.0,
        PI,
        true,
    ))));
    assert_eq!(
        path_d(&upper),
        "M 10.0000 400.0000 A 10.0000 10.0000 0 0 0 -10.0000 400.0000"
    );

    let lower = export_svg(&doc_with(Entity::Arc(Arc::new(
        Vec2::new(0.0, 0.0),
        10.0,
        0.0,
        PI,
        false,
    ))));
    assert_eq!(
        path_d(&lower),
        "M 10.0000 400.0000 A 10.0000 10.0000 0 0 1 -10.0000 400.0000"
    );
}

/// LCV-100 AC 10 — `large-arc-flag` is a magnitude and must not flip;
/// only `sweep` does. Flag triple is `0 1 0`.
#[test]
fn arc_large_flag_survives_the_mirror() {
    let svg = export_svg(&doc_with(Entity::Arc(Arc::new(
        Vec2::new(100.0, 100.0),
        10.0,
        0.0,
        3.0 * FRAC_PI_2,
        true,
    ))));
    assert_eq!(
        path_d(&svg),
        "M 110.0000 300.0000 A 10.0000 10.0000 0 1 0 100.0000 310.0000"
    );
    assert!(svg.contains(" 0 1 0 "), "{svg}");
}

/// LCV-100 AC 12 — geometry outside the bed is emitted verbatim with a
/// negative Y; nothing is clipped or dropped.
#[test]
fn out_of_bed_geometry_is_emitted_verbatim() {
    let mut doc = doc_with(Entity::Line(Line::new(
        Vec2::new(0.0, 500.0),
        Vec2::new(10.0, 500.0),
    )));
    doc.push_current(Entity::Line(Line::new(
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 1.0),
    )));
    let svg = export_svg(&doc);
    assert!(
        svg.contains("<line x1=\"0.0000\" y1=\"-100.0000\" x2=\"10.0000\" y2=\"-100.0000\"/>"),
        "{svg}"
    );
    assert_eq!(svg.matches("<line").count(), doc.entities.len(), "{svg}");
}

/// LCV-056 AC#12 — no forbidden SVG elements (`<filter`, `<mask`,
/// `<clipPath`, `<text`).
#[test]
fn no_forbidden_svg_elements() {
    let mut doc = Document::default();
    doc.push_current(Entity::Line(Line::new(
        Vec2::new(0.0, 0.0),
        Vec2::new(10.0, 0.0),
    )));
    doc.push_current(Entity::Circle(Circle::new(Vec2::new(5.0, 5.0), 2.0)));
    doc.push_current(Entity::Arc(Arc::new(
        Vec2::new(0.0, 0.0),
        5.0,
        0.0,
        PI,
        true,
    )));
    let svg = export_svg(&doc);
    assert!(!svg.contains("<filter"));
    assert!(!svg.contains("<mask"));
    assert!(!svg.contains("<clipPath"));
    assert!(!svg.contains("<text"));
}

/// LCV-056 AC#13 — `export_svg` is accessible via `crate::io::svg::export_svg`.
#[test]
fn export_svg_reachable_via_module_path() {
    use crate::io::svg::export_svg as reexported;
    let _ = reexported(&Document::default());
    let _ = crate::io::export_svg(&Document::default());
}

// ── LCV-114 — the canvas is the document's bed ────────────────────────

/// LCV-114 AC 5 — the header is written from `doc.bed_mm`, in `mm`, with
/// a matching `viewBox`, and with no trailing `.0` on a whole number.
#[test]
fn export_header_uses_document_bed() {
    let doc = Document::with_bed([300.0, 200.0]);
    let svg = export_svg(&doc);
    assert!(svg.contains("width=\"300mm\""), "{svg}");
    assert!(svg.contains("height=\"200mm\""), "{svg}");
    assert!(svg.contains("viewBox=\"0 0 300 200\""), "{svg}");
    assert!(!svg.contains("400"), "no default bed may leak: {svg}");
}

/// LCV-114 AC 5 — a fractional bed keeps its decimals and still carries
/// no `{:.4}` padding.
#[test]
fn export_header_keeps_fractional_bed_dimensions() {
    let doc = Document::with_bed([300.5, 180.25]);
    let svg = export_svg(&doc);
    assert!(svg.contains("width=\"300.5mm\""), "{svg}");
    assert!(svg.contains("height=\"180.25mm\""), "{svg}");
    assert!(svg.contains("viewBox=\"0 0 300.5 180.25\""), "{svg}");
}

/// LCV-114 AC 6 — the mirror axis is the document's bed height: a line at
/// world `y = 50` on a 200 mm-tall bed emits `150`, not `350`. This is the
/// defect the demand exists to prevent — with a constant axis the file
/// still parses and still looks plausible.
#[test]
fn export_mirrors_around_document_bed_height() {
    let mut doc = doc_with(Entity::Line(Line::new(
        Vec2::new(10.0, 50.0),
        Vec2::new(20.0, 50.0),
    )));
    doc.bed_mm = [300.0, 200.0];
    let svg = export_svg(&doc);
    assert!(
        svg.contains("<line x1=\"10.0000\" y1=\"150.0000\" x2=\"20.0000\" y2=\"150.0000\"/>"),
        "{svg}"
    );
}

/// LCV-114 AC 6 — circles and arcs mirror around the same axis as lines.
#[test]
fn export_mirrors_circles_and_arcs_around_document_bed_height() {
    let mut doc = doc_with(Entity::Circle(Circle::new(Vec2::new(40.0, 30.0), 5.0)));
    doc.push_current(Entity::Arc(Arc::new(
        Vec2::new(0.0, 0.0),
        10.0,
        0.0,
        FRAC_PI_2,
        true,
    )));
    doc.bed_mm = [300.0, 180.0];
    let svg = export_svg(&doc);
    assert!(
        svg.contains("<circle cx=\"40.0000\" cy=\"150.0000\" r=\"5.0000\"/>"),
        "{svg}"
    );
    assert_eq!(
        path_d(&svg),
        "M 10.0000 180.0000 A 10.0000 10.0000 0 0 0 0.0000 170.0000"
    );
}
// ── LCV-156 — one group per layer ─────────────────────────────────────

/// LCV-156 AC 9 — byte identity of the mother SVG for a one-layer document:
/// the exact bytes LaserGRBL is handed, newlines included.
#[test]
fn default_document_export_is_byte_exact() {
    let doc = doc_with(Entity::Line(Line::new(
        Vec2::new(1.0, 2.0),
        Vec2::new(11.0, 7.0),
    )));
    let golden = concat!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"400mm\" height=\"400mm\"",
        " viewBox=\"0 0 400 400\" fill=\"none\">\n",
        "<g data-layer=\"Cut\" stroke=\"#ff0000\" stroke-width=\"0.1\" data-output=\"1\"",
        " data-current=\"1\">\n",
        "<line x1=\"1.0000\" y1=\"398.0000\" x2=\"11.0000\" y2=\"393.0000\"/>\n",
        "</g>\n",
        "</svg>",
    );
    assert_eq!(export_svg(&doc), golden);
}

/// LCV-156 AC 10 — a per-layer file is the same header and only that
/// layer's group, without `data-current`; an unknown id yields the header.
#[test]
fn layer_export_is_byte_exact() {
    let mut doc = Document::with_bed([300.0, 200.0]);
    let mut add = crate::document::AddLayer::new("Mark", [0, 0, 255], true);
    crate::document::Command::do_(&mut add, &mut doc);
    let mark = add.id().expect("allocated");
    doc.push_current(Entity::Line(Line::new(
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 1.0),
    )));
    doc.push_entity(Entity::Circle(Circle::new(Vec2::new(5.0, 5.0), 2.0)), mark);
    let golden = concat!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"300mm\" height=\"200mm\"",
        " viewBox=\"0 0 300 200\" fill=\"none\">\n",
        "<g data-layer=\"Mark\" stroke=\"#0000ff\" stroke-width=\"0.1\" data-output=\"1\">\n",
        "<circle cx=\"5.0000\" cy=\"195.0000\" r=\"2.0000\"/>\n",
        "</g>\n",
        "</svg>",
    );
    assert_eq!(export_layer_svg(&doc, mark), golden);
    let header_only = export_layer_svg(&doc, LayerId(99));
    assert!(
        header_only.ends_with("fill=\"none\">\n</svg>"),
        "{header_only}"
    );
}
