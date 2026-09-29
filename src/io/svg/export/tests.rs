use super::*;
use crate::document::state::Document;
use crate::geometry::{Arc, Circle, Line, Vec2};
use core::f64::consts::{FRAC_PI_2, PI};

fn doc_with(entity: Entity) -> Document {
    let mut doc = Document::default();
    doc.entities.push(entity);
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
    let svg = export_svg(&Document::default(), Preset::Cut);
    assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(svg.contains("fill=\"none\""));
}

/// LCV-100 AC 11 — the canvas is the bed for every document, empty or not.
/// Replaces LCV-056's bounds-derived canvas and its `100mm` empty-document
/// fallback.
#[test]
fn canvas_is_always_the_bed() {
    for svg in [
        export_svg(&Document::default(), Preset::Cut),
        export_svg(
            &doc_with(Entity::Line(Line::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(50.0, 30.0),
            ))),
            Preset::Cut,
        ),
    ] {
        assert!(svg.contains("width=\"400mm\""), "{svg}");
        assert!(svg.contains("height=\"400mm\""), "{svg}");
        assert!(svg.contains("viewBox=\"0 0 400 400\""), "{svg}");
        assert!(!svg.contains("100mm"), "{svg}");
    }
}

/// LCV-056 AC#6 — three groups always emitted with correct colours and
/// stroke-width.
#[test]
fn three_groups_always_emitted() {
    let svg = export_svg(&Document::default(), Preset::Cut);
    assert!(svg.contains("stroke=\"#ff0000\""), "{svg}");
    assert!(svg.contains("stroke=\"#0000ff\""), "{svg}");
    assert!(svg.contains("stroke=\"#00aa00\""), "{svg}");
    assert_eq!(svg.matches("stroke-width=\"0.1\"").count(), 3, "{svg}");
}

/// LCV-056 AC#7 — Line entity encodes to `<line …/>` inside the cut group.
#[test]
fn line_entity_encodes_to_svg_line() {
    let svg = export_svg(
        &doc_with(Entity::Line(Line::new(
            Vec2::new(1.0, 2.0),
            Vec2::new(11.0, 7.0),
        ))),
        Preset::Cut,
    );
    let expected = "<line x1=\"1.0000\" y1=\"398.0000\" x2=\"11.0000\" y2=\"393.0000\"/>";
    assert!(svg.contains(expected), "{svg}");
    let line_pos = svg.find(expected).unwrap();
    let close_pos = svg.find("</g>").unwrap();
    assert!(line_pos < close_pos, "line outside cut group: {svg}");
}

/// LCV-100 AC 4 — `y1`/`y2` are mirrored, `x1`/`x2` are not.
#[test]
fn line_y_is_flipped_on_export() {
    let svg = export_svg(
        &doc_with(Entity::Line(Line::new(
            Vec2::new(10.0, 10.0),
            Vec2::new(10.0, 60.0),
        ))),
        Preset::Cut,
    );
    assert!(
        svg.contains("<line x1=\"10.0000\" y1=\"390.0000\" x2=\"10.0000\" y2=\"340.0000\"/>"),
        "{svg}"
    );
}

/// LCV-056 AC#8 — Circle entity encodes to `<circle …/>`.
#[test]
fn circle_entity_encodes_to_svg_circle() {
    let svg = export_svg(
        &doc_with(Entity::Circle(Circle::new(Vec2::new(5.0, 5.0), 3.0))),
        Preset::Cut,
    );
    assert!(
        svg.contains("<circle cx=\"5.0000\" cy=\"395.0000\" r=\"3.0000\"/>"),
        "{svg}"
    );
}

/// LCV-100 AC 5 — `cy` is mirrored; `cx` and `r` are untouched.
#[test]
fn circle_cy_is_flipped_on_export() {
    let svg = export_svg(
        &doc_with(Entity::Circle(Circle::new(Vec2::new(100.0, 250.0), 5.0))),
        Preset::Cut,
    );
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
    let svg = export_svg(
        &doc_with(Entity::Arc(Arc::new(
            Vec2::new(0.0, 0.0),
            10.0,
            0.0,
            FRAC_PI_2,
            true,
        ))),
        Preset::Cut,
    );
    assert_eq!(
        path_d(&svg),
        "M 10.0000 400.0000 A 10.0000 10.0000 0 0 0 0.0000 390.0000"
    );
}

/// LCV-100 AC 8 — CW quarter arc, golden `d` (sweep flag 1).
#[test]
fn arc_cw_quarter_golden() {
    let svg = export_svg(
        &doc_with(Entity::Arc(Arc::new(
            Vec2::new(0.0, 0.0),
            10.0,
            FRAC_PI_2,
            0.0,
            false,
        ))),
        Preset::Cut,
    );
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
    let upper = export_svg(
        &doc_with(Entity::Arc(Arc::new(
            Vec2::new(0.0, 0.0),
            10.0,
            0.0,
            PI,
            true,
        ))),
        Preset::Cut,
    );
    assert_eq!(
        path_d(&upper),
        "M 10.0000 400.0000 A 10.0000 10.0000 0 0 0 -10.0000 400.0000"
    );

    let lower = export_svg(
        &doc_with(Entity::Arc(Arc::new(
            Vec2::new(0.0, 0.0),
            10.0,
            0.0,
            PI,
            false,
        ))),
        Preset::Cut,
    );
    assert_eq!(
        path_d(&lower),
        "M 10.0000 400.0000 A 10.0000 10.0000 0 0 1 -10.0000 400.0000"
    );
}

/// LCV-100 AC 10 — `large-arc-flag` is a magnitude and must not flip;
/// only `sweep` does. Flag triple is `0 1 0`.
#[test]
fn arc_large_flag_survives_the_mirror() {
    let svg = export_svg(
        &doc_with(Entity::Arc(Arc::new(
            Vec2::new(100.0, 100.0),
            10.0,
            0.0,
            3.0 * FRAC_PI_2,
            true,
        ))),
        Preset::Cut,
    );
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
    doc.entities.push(Entity::Line(Line::new(
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 1.0),
    )));
    let svg = export_svg(&doc, Preset::Cut);
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
    doc.entities.push(Entity::Line(Line::new(
        Vec2::new(0.0, 0.0),
        Vec2::new(10.0, 0.0),
    )));
    doc.entities
        .push(Entity::Circle(Circle::new(Vec2::new(5.0, 5.0), 2.0)));
    doc.entities.push(Entity::Arc(Arc::new(
        Vec2::new(0.0, 0.0),
        5.0,
        0.0,
        PI,
        true,
    )));
    let svg = export_svg(&doc, Preset::Cut);
    assert!(!svg.contains("<filter"));
    assert!(!svg.contains("<mask"));
    assert!(!svg.contains("<clipPath"));
    assert!(!svg.contains("<text"));
}

/// LCV-056 AC#13 — `export_svg` is accessible via `crate::io::svg::export_svg`.
/// LCV-115 AC#1 — `Preset` rides the same re-export chain, up to `crate::io`.
#[test]
fn export_svg_reachable_via_module_path() {
    use crate::io::svg::export_svg as reexported;
    let _ = reexported(&Document::default(), crate::io::svg::Preset::Cut);
    let _ = crate::io::export_svg(&Document::default(), crate::io::Preset::Mark);
}

// ── LCV-114 — the canvas is the document's bed ────────────────────────

/// LCV-114 AC 5 — the header is written from `doc.bed_mm`, in `mm`, with
/// a matching `viewBox`, and with no trailing `.0` on a whole number.
#[test]
fn export_header_uses_document_bed() {
    let doc = Document {
        bed_mm: [300.0, 200.0],
        ..Document::default()
    };
    let svg = export_svg(&doc, Preset::Cut);
    assert!(svg.contains("width=\"300mm\""), "{svg}");
    assert!(svg.contains("height=\"200mm\""), "{svg}");
    assert!(svg.contains("viewBox=\"0 0 300 200\""), "{svg}");
    assert!(!svg.contains("400"), "no default bed may leak: {svg}");
}

/// LCV-114 AC 5 — a fractional bed keeps its decimals and still carries
/// no `{:.4}` padding.
#[test]
fn export_header_keeps_fractional_bed_dimensions() {
    let doc = Document {
        bed_mm: [300.5, 180.25],
        ..Document::default()
    };
    let svg = export_svg(&doc, Preset::Cut);
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
    let svg = export_svg(&doc, Preset::Cut);
    assert!(
        svg.contains("<line x1=\"10.0000\" y1=\"150.0000\" x2=\"20.0000\" y2=\"150.0000\"/>"),
        "{svg}"
    );
}

/// LCV-114 AC 6 — circles and arcs mirror around the same axis as lines.
#[test]
fn export_mirrors_circles_and_arcs_around_document_bed_height() {
    let mut doc = doc_with(Entity::Circle(Circle::new(Vec2::new(40.0, 30.0), 5.0)));
    doc.entities.push(Entity::Arc(Arc::new(
        Vec2::new(0.0, 0.0),
        10.0,
        0.0,
        FRAC_PI_2,
        true,
    )));
    doc.bed_mm = [300.0, 180.0];
    let svg = export_svg(&doc, Preset::Cut);
    assert!(
        svg.contains("<circle cx=\"40.0000\" cy=\"150.0000\" r=\"5.0000\"/>"),
        "{svg}"
    );
    assert_eq!(
        path_d(&svg),
        "M 10.0000 180.0000 A 10.0000 10.0000 0 0 0 0.0000 170.0000"
    );
}
// ── LCV-115 — the export preset selector ──────────────────────────────

/// AC 1 — every variant maps to its group id and its LaserGRBL colour.
#[test]
fn preset_ids_and_colors() {
    assert_eq!(Preset::Cut.id(), "cut");
    assert_eq!(Preset::Mark.id(), "mark");
    assert_eq!(Preset::Engrave.id(), "engrave");
    assert_eq!(Preset::Cut.color(), "#ff0000");
    assert_eq!(Preset::Mark.color(), "#0000ff");
    assert_eq!(Preset::Engrave.color(), "#00aa00");
    assert_eq!(Preset::Cut.label(), "Cut");
    assert_eq!(Preset::Mark.label(), "Mark");
    assert_eq!(Preset::Engrave.label(), "Engrave");
}

/// AC 1 / AC 5 — the default preset is `Cut`: an operator who never opens
/// the menu gets the profile they got before this demand existed.
#[test]
fn preset_default_is_cut() {
    assert_eq!(Preset::default(), Preset::Cut);
}

/// AC 1 / AC 3 — `ALL` is in cut, mark, engrave order, and that is the
/// order the groups actually appear in the output.
#[test]
fn preset_all_is_in_export_order() {
    assert_eq!(Preset::ALL, [Preset::Cut, Preset::Mark, Preset::Engrave]);
    let svg = export_svg(&Document::default(), Preset::Cut);
    let mut previous = 0usize;
    for preset in Preset::ALL {
        let at = svg
            .find(&format!("id=\"{}\"", preset.id()))
            .unwrap_or_else(|| panic!("{preset:?} group missing: {svg}"));
        assert!(at >= previous, "{preset:?} is out of order: {svg}");
        previous = at;
    }
}

/// AC 8 — `from_group_id` is the inverse of `id()` and rejects anything
/// else, which is what keeps the importer's detection honest.
#[test]
fn preset_from_group_id_round_trips_and_rejects_unknown_ids() {
    for preset in Preset::ALL {
        assert_eq!(Preset::from_group_id(preset.id()), Some(preset));
    }
    assert_eq!(Preset::from_group_id("layer1"), None);
    assert_eq!(Preset::from_group_id(""), None);
    assert_eq!(Preset::from_group_id("CUT"), None, "ids match exactly");
}

/// AC 2 / AC 3 — the selected group receives every entity; the other two
/// are emitted, in place, with the same attributes, and empty.
#[test]
fn export_places_geometry_in_the_selected_group() {
    let doc = doc_with(Entity::Line(Line::new(
        Vec2::new(1.0, 2.0),
        Vec2::new(11.0, 7.0),
    )));
    let geometry = "<line x1=\"1.0000\" y1=\"398.0000\" x2=\"11.0000\" y2=\"393.0000\"/>";

    for selected in Preset::ALL {
        let svg = export_svg(&doc, selected);
        assert_eq!(svg.matches(geometry).count(), 1, "{selected:?}: {svg}");

        for group in Preset::ALL {
            let open = format!(
                "<g id=\"{}\" stroke=\"{}\" stroke-width=\"0.1\">\n",
                group.id(),
                group.color()
            );
            let at = svg
                .find(&open)
                .unwrap_or_else(|| panic!("{group:?} group missing: {svg}"));
            let body_start = at + open.len();
            let body_end = svg[body_start..]
                .find("</g>")
                .unwrap_or_else(|| panic!("{group:?} group is unterminated: {svg}"))
                + body_start;
            let body = &svg[body_start..body_end];
            if group == selected {
                assert_eq!(
                    body,
                    format!("{geometry}\n"),
                    "{selected:?} must hold the geometry: {svg}"
                );
            } else {
                assert!(body.is_empty(), "{group:?} must stay empty: {svg}");
            }
        }
    }
}

/// AC 4 — byte identity for the default: this golden is the exact output
/// the pre-LCV-115 exporter produced for this document, down to the
/// newlines, so a refactor of the group loop cannot quietly reshape a
/// file LaserGRBL already accepts.
#[test]
fn cut_export_is_byte_identical_to_the_pre_preset_exporter() {
    let doc = doc_with(Entity::Line(Line::new(
        Vec2::new(1.0, 2.0),
        Vec2::new(11.0, 7.0),
    )));
    let golden = concat!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"400mm\" height=\"400mm\"",
        " viewBox=\"0 0 400 400\" fill=\"none\">\n",
        "<g id=\"cut\" stroke=\"#ff0000\" stroke-width=\"0.1\">\n",
        "<line x1=\"1.0000\" y1=\"398.0000\" x2=\"11.0000\" y2=\"393.0000\"/>\n",
        "</g>\n",
        "<g id=\"mark\" stroke=\"#0000ff\" stroke-width=\"0.1\">\n",
        "</g>\n",
        "<g id=\"engrave\" stroke=\"#00aa00\" stroke-width=\"0.1\">\n",
        "</g>\n",
        "</svg>",
    );
    assert_eq!(export_svg(&doc, Preset::Cut), golden);
}

/// AC 2 — the three group literals live **only** inside `impl Preset`.
///
/// The haystack is split into the two slices that must be clean — the
/// source before the impl and the source between the impl and the test
/// module — so this test's own body (which necessarily names every
/// literal) is outside both. Each absence claim is paired with a positive
/// control inside the impl, and with two controls proving the `after`
/// slice really contains the exporter: an absence assertion over an empty
/// or misplaced haystack would otherwise pass for free.
#[test]
fn group_literals_live_only_inside_the_preset_impl() {
    let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/io/svg/export.rs"));
    let impl_start = src.find("\nimpl Preset {").expect("impl Preset must exist");
    let impl_end = src[impl_start..]
        .find("\n}\n")
        .expect("impl Preset must be closed at column 0")
        + impl_start
        + 3;
    let cfg_test = src
        .find("\n#[cfg(test)]")
        .expect("export.rs must have a test module to bound the scan");
    assert!(impl_end < cfg_test, "the impl must precede the tests");

    let inside = &src[impl_start..impl_end];
    let before = &src[..impl_start];
    let after = &src[impl_end..cfg_test];

    assert!(
        after.contains("pub fn export_svg("),
        "positive control: the exporter must be in the scanned slice"
    );
    assert!(
        after.contains("open_group(&mut out, group);"),
        "positive control: the group loop must be in the scanned slice"
    );

    for literal in [
        "#ff0000",
        "#0000ff",
        "#00aa00",
        "\"cut\"",
        "\"mark\"",
        "\"engrave\"",
    ] {
        assert!(
            inside.contains(literal),
            "positive control: {literal} must be declared by impl Preset"
        );
        assert!(
            !before.contains(literal),
            "{literal} must not appear before impl Preset"
        );
        assert!(
            !after.contains(literal),
            "{literal} must not appear outside impl Preset (AC 2)"
        );
    }
}
