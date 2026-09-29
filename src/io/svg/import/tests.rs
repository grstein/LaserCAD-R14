use super::*;
use core::f64::consts::{FRAC_PI_2, PI};

// Golden fixtures below are in the LCV-100 convention: SVG Y-down with the
// bed as the canvas, i.e. world Y = 400 − SVG Y.
const LINE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><line x1="1.0000" y1="2.0000" x2="11.0000" y2="7.0000"/></svg>"#;
const ARC_CCW_Q: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M 10.0000 400.0000 A 10.0000 10.0000 0 0 0 0.0000 390.0000"/></svg>"#;
const ARC_LARGE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M 110.0000 300.0000 A 10.0000 10.0000 0 1 0 100.0000 310.0000"/></svg>"#;
const ARC_CW_Q: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M 0.0000 390.0000 A 10.0000 10.0000 0 0 1 10.0000 400.0000"/></svg>"#;
const PATH_MALFORMED: &str =
    r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M 0 0 A notanumber 10 0 0 1 5 5"/></svg>"#;
const MIXED_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><line x1="1" y1="2" x2="3" y2="4"/><rect/><circle cx="5" cy="5" r="3"/></svg>"#;
const G_GROUPS_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><g><line x1="0" y1="0" x2="10" y2="10"/></g><g/><g/></svg>"#;

fn svg(inner: &str) -> String {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg">{inner}</svg>"#)
}

fn only_arc(src: &str) -> Arc {
    let es = import_svg(src).unwrap().entities;
    match es[0] {
        Entity::Arc(a) => a,
        _ => panic!("not an arc"),
    }
}

#[test]
fn empty_svg_returns_no_entities() {
    let es = import_svg(r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#)
        .unwrap()
        .entities;
    assert!(es.is_empty());
}

#[test]
fn invalid_xml_returns_xml_parse_error() {
    let r = import_svg("<svg><unclosed");
    assert!(matches!(r, Err(SvgImportError::XmlParse(_))));
}

#[test]
fn no_svg_root_returns_error() {
    let r = import_svg("<root/>");
    assert!(matches!(r, Err(SvgImportError::NoSvgRoot)));
}

#[test]
fn line_element_parsed_to_entity_line() {
    let es = import_svg(LINE_SVG).unwrap().entities;
    let Entity::Line(l) = es[0] else {
        panic!("not a line")
    };
    assert!((l.p1.x - 1.0).abs() < EPSILON && (l.p1.y - 398.0).abs() < EPSILON);
    assert!((l.p2.x - 11.0).abs() < EPSILON && (l.p2.y - 393.0).abs() < EPSILON);
}

#[test]
fn circle_element_parsed_to_entity_circle() {
    let es = import_svg(&svg(r#"<circle cx="5.0000" cy="5.0000" r="3.0000"/>"#))
        .unwrap()
        .entities;
    let Entity::Circle(c) = es[0] else {
        panic!("not a circle")
    };
    assert!((c.center.x - 5.0).abs() < EPSILON && (c.center.y - 395.0).abs() < EPSILON);
    assert!((c.r - 3.0).abs() < EPSILON);
}

/// LCV-100 AC 13 — X is untouched, Y is un-mirrored on both elements.
#[test]
fn import_line_and_circle_unflip_y() {
    let es = import_svg(&svg(
        r#"<line x1="1" y1="2" x2="11" y2="7"/><circle cx="5" cy="5" r="3"/>"#,
    ))
    .unwrap()
    .entities;
    let Entity::Line(l) = es[0] else {
        panic!("not a line")
    };
    assert!((l.p1.x - 1.0).abs() < EPSILON && (l.p1.y - 398.0).abs() < EPSILON);
    assert!((l.p2.x - 11.0).abs() < EPSILON && (l.p2.y - 393.0).abs() < EPSILON);
    let Entity::Circle(c) = es[1] else {
        panic!("not a circle")
    };
    assert!((c.center.x - 5.0).abs() < EPSILON && (c.center.y - 395.0).abs() < EPSILON);
    assert!((c.r - 3.0).abs() < EPSILON);
}

#[test]
fn arc_ccw_quarter_reconstructed_correctly() {
    let a = only_arc(ARC_CCW_Q);
    assert!(a.center.x.abs() < EPSILON && a.center.y.abs() < EPSILON);
    assert!((a.r - 10.0).abs() < EPSILON && a.start_angle.abs() < EPSILON);
    assert!((a.end_angle - FRAC_PI_2).abs() < EPSILON && a.ccw);
}

#[test]
fn arc_large_flag_selects_correct_center() {
    let a = only_arc(ARC_LARGE);
    assert!((a.center.x - 100.0).abs() < EPSILON && (a.center.y - 100.0).abs() < EPSILON);
    assert!((a.r - 10.0).abs() < EPSILON);
    assert!((a.sweep_angle() - 3.0 * FRAC_PI_2).abs() < EPSILON && a.ccw);
}

#[test]
fn arc_cw_sweep_flag_one_sets_ccw_false() {
    assert!(!only_arc(ARC_CW_Q).ccw);
}

/// LCV-100 AC 14 — each exported golden path reconstructs its source arc.
/// Raw `start_angle`/`end_angle` may differ by a multiple of 2π because the
/// importer normalises through `atan2`, so endpoints and sweep are compared
/// instead.
#[test]
fn import_arc_golden_paths_reconstruct_source_arcs() {
    let semicircle =
        svg(r#"<path d="M 10.0000 400.0000 A 10.0000 10.0000 0 0 0 -10.0000 400.0000"/>"#);
    let cases: [(&str, Arc); 4] = [
        (
            ARC_CCW_Q,
            Arc::new(Vec2::new(0.0, 0.0), 10.0, 0.0, FRAC_PI_2, true),
        ),
        (
            ARC_CW_Q,
            Arc::new(Vec2::new(0.0, 0.0), 10.0, FRAC_PI_2, 0.0, false),
        ),
        (
            &semicircle,
            Arc::new(Vec2::new(0.0, 0.0), 10.0, 0.0, PI, true),
        ),
        (
            ARC_LARGE,
            Arc::new(Vec2::new(100.0, 100.0), 10.0, 0.0, 3.0 * FRAC_PI_2, true),
        ),
    ];
    for (src, expected) in cases {
        let a = only_arc(src);
        assert!((a.center.x - expected.center.x).abs() < EPSILON, "{src}");
        assert!((a.center.y - expected.center.y).abs() < EPSILON, "{src}");
        assert!((a.r - expected.r).abs() < EPSILON, "{src}");
        let (sp, esp) = (a.start_point(), expected.start_point());
        let (ep, eep) = (a.end_point(), expected.end_point());
        assert!(
            (sp.x - esp.x).abs() < EPSILON && (sp.y - esp.y).abs() < EPSILON,
            "{src}"
        );
        assert!(
            (ep.x - eep.x).abs() < EPSILON && (ep.y - eep.y).abs() < EPSILON,
            "{src}"
        );
        assert_eq!(a.ccw, expected.ccw, "{src}");
        assert!(
            (a.sweep_angle() - expected.sweep_angle()).abs() < EPSILON,
            "{src}"
        );
    }
}

/// The lower half of the same semicircle differs only in the sweep flag and
/// must import as the CW arc.
#[test]
fn arc_semicircle_sweep_flag_selects_handedness() {
    let lower = only_arc(&svg(
        r#"<path d="M 10.0000 400.0000 A 10.0000 10.0000 0 0 1 -10.0000 400.0000"/>"#,
    ));
    assert!(!lower.ccw);
    assert!((lower.sweep_angle() - PI).abs() < EPSILON);
    assert!(lower.center.x.abs() < EPSILON && lower.center.y.abs() < EPSILON);
}

#[test]
fn circle_negative_radius_returns_malformed_attribute() {
    // Display: "<circle> attribute r="-1.0000" is not a valid number"
    let e = import_svg(&svg(r#"<circle cx="5" cy="5" r="-1.0000"/>"#)).unwrap_err();
    let s = e.to_string();
    assert!(s.contains("<circle>") && s.contains("r="));
}

#[test]
fn line_bad_attribute_returns_malformed_attribute() {
    // Display: "<line> attribute x1="abc" is not a valid number"
    let e = import_svg(&svg(r#"<line x1="abc" y1="0" x2="0" y2="0"/>"#)).unwrap_err();
    let s = e.to_string();
    assert!(s.contains("<line>") && s.contains("x1") && s.contains("abc"));
}

#[test]
fn path_with_non_numeric_a_command_returns_malformed_path() {
    let r = import_svg(PATH_MALFORMED);
    assert!(matches!(r, Err(SvgImportError::MalformedPath(_))));
}

#[test]
fn non_arc_path_silently_skipped() {
    let es = import_svg(&svg(r#"<path d="M 0 0 L 10 10"/>"#))
        .unwrap()
        .entities;
    assert!(es.is_empty());
}

#[test]
fn unknown_elements_silently_skipped() {
    let es = import_svg(MIXED_SVG).unwrap().entities;
    assert_eq!(es.len(), 2);
    assert!(matches!(es[0], Entity::Line(_)) && matches!(es[1], Entity::Circle(_)));
}

#[test]
fn entities_inside_g_groups_collected() {
    let es = import_svg(G_GROUPS_SVG).unwrap().entities;
    assert_eq!(es.len(), 1);
    assert!(matches!(es[0], Entity::Line(_)));
}

#[test]
fn round_trip_line_circle_arc() {
    use crate::document::state::Document;
    use crate::io::svg::export_svg;
    let mut doc = Document::default();
    let p1 = Vec2::new(1.0, 2.0);
    let p2 = Vec2::new(11.0, 7.0);
    let lin = Line::new(p1, p2);
    doc.entities.push(Entity::Line(lin));
    let cir = Circle::new(Vec2::new(5.0, 5.0), 3.0);
    doc.entities.push(Entity::Circle(cir));
    let arc = Arc::new(Vec2::default(), 10.0, 0.0, FRAC_PI_2, true);
    doc.entities.push(Entity::Arc(arc));
    let imp = import_svg(&export_svg(&doc, Preset::Cut)).unwrap().entities;
    assert_eq!(imp.len(), 3);
    if let (Entity::Line(l), Entity::Circle(c), Entity::Arc(a)) = (imp[0], imp[1], imp[2]) {
        assert!((l.p1.x - 1.0).abs() < EPSILON && (l.p1.y - 2.0).abs() < EPSILON);
        assert!((l.p2.x - 11.0).abs() < EPSILON && (l.p2.y - 7.0).abs() < EPSILON);
        assert!((c.center.x - 5.0).abs() < EPSILON && (c.center.y - 5.0).abs() < EPSILON);
        assert!((c.r - 3.0).abs() < EPSILON);
        assert!(a.center.x.abs() < EPSILON && a.center.y.abs() < EPSILON);
        assert!((a.r - 10.0).abs() < EPSILON && a.start_angle.abs() < EPSILON);
        assert!((a.end_angle - FRAC_PI_2).abs() < EPSILON && a.ccw);
    } else {
        panic!("unexpected entity variants in round-trip");
    }
}

/// LCV-114 AC 7/AC 8a — the declared bed comes back with the geometry,
/// and it is the axis the Y was un-mirrored around: `y_svg = 130` on a
/// 180 mm bed is `y_world = 50`, not the 270 a 400 mm mirror would give.
#[test]
fn import_reads_bed_from_width_height() {
    let src = r#"<svg xmlns="http://www.w3.org/2000/svg" width="300mm" height="180mm" viewBox="0 0 300 180"><line x1="10" y1="130" x2="250" y2="130"/></svg>"#;
    let imported = import_svg(src).unwrap();
    assert_eq!(imported.bed_mm, [300.0, 180.0]);
    let Entity::Line(l) = imported.entities[0] else {
        panic!("not a line")
    };
    assert!((l.p1.y - 50.0).abs() < EPSILON, "p1.y = {}", l.p1.y);
    assert!((l.p2.y - 50.0).abs() < EPSILON, "p2.y = {}", l.p2.y);
    assert!((l.p1.x - 10.0).abs() < EPSILON);
}

/// LCV-114 AC 8b — with no `width`/`height`, the `viewBox` is the bed and
/// the mirror axis.
#[test]
fn import_reads_bed_from_viewbox_when_dimensions_absent() {
    let src = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 180"><circle cx="5" cy="130" r="3"/></svg>"#;
    let imported = import_svg(src).unwrap();
    assert_eq!(imported.bed_mm, [300.0, 180.0]);
    let Entity::Circle(c) = imported.entities[0] else {
        panic!("not a circle")
    };
    assert!((c.center.y - 50.0).abs() < EPSILON, "cy = {}", c.center.y);
}

/// LCV-114 AC 8c — the back-compat guard: `LINE_SVG` carries neither a
/// dimension pair nor a `viewBox`, so it keeps importing exactly as it did
/// before this demand, at the default bed.
#[test]
fn import_falls_back_to_default_when_both_absent() {
    let imported = import_svg(LINE_SVG).unwrap();
    assert_eq!(
        imported.bed_mm,
        [
            crate::util::DEFAULT_BED_WIDTH_MM,
            crate::util::DEFAULT_BED_HEIGHT_MM
        ]
    );
    let Entity::Line(l) = imported.entities[0] else {
        panic!("not a line")
    };
    assert!((l.p1.y - 398.0).abs() < EPSILON);
}

/// LCV-114 AC 8 — the accepted numeric forms reach the importer, not just
/// the header parser.
#[test]
fn import_accepts_mm_suffix_and_whitespace() {
    let src = r#"<svg xmlns="http://www.w3.org/2000/svg" width=" 300.5 mm " height="180MM"/>"#;
    assert_eq!(import_svg(src).unwrap().bed_mm, [300.5, 180.0]);
    let bare = r#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="180"/>"#;
    assert_eq!(import_svg(bare).unwrap().bed_mm, [300.0, 180.0]);
}

/// LCV-114 AC 9 — one assertion per rejection case; each names the
/// offending attribute and keeps its raw value in the message, and the
/// import as a whole fails rather than importing at a guessed bed.
#[test]
fn import_rejects_zero_negative_oversized_and_garbage_dimensions() {
    for (attrs, attr, raw) in [
        (r#"width="0" height="180""#, "width", "0"),
        (r#"width="-300" height="180""#, "width", "-300"),
        (r#"width="5000" height="180""#, "width", "5000"),
        (r#"width="banana" height="180""#, "width", "banana"),
        (r#"width="300" height="0""#, "height", "0"),
        (r#"width="300" height="inf""#, "height", "inf"),
        (r#"viewBox="0 0 5000 180""#, "viewBox", "0 0 5000 180"),
        (r#"viewBox="10 0 300 180""#, "viewBox", "10 0 300 180"),
    ] {
        let src = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" {attrs}><line x1="0" y1="0" x2="1" y2="1"/></svg>"#
        );
        let err = import_svg(&src).unwrap_err();
        let SvgImportError::MalformedBedDimension { attr: got, value } = &err else {
            panic!("expected MalformedBedDimension for {attrs}, got {err:?}");
        };
        assert_eq!(*got, attr, "{attrs}");
        assert_eq!(value, raw, "{attrs}");
        assert!(err.to_string().contains(raw), "{err}");
    }
}

#[test]
fn import_svg_reachable_via_module_path() {
    let _f: fn(&str) -> Result<ImportedSvg, SvgImportError> = import_svg;
}

// ── LCV-115 — the file carries its export preset ──────────────────────

/// Wrap `inner` in a `<g id="…">` the way this crate's exporter does.
fn grouped(id: &str, inner: &str) -> String {
    svg(&format!(r#"<g id="{id}" fill="none">{inner}</g>"#))
}

const A_LINE: &str = r#"<line x1="0" y1="0" x2="10" y2="10"/>"#;

/// LCV-115 AC#8 — each recognised group id maps to its preset, and the
/// geometry still lands in `entities` either way.
#[test]
fn import_detects_preset_from_group_id() {
    for preset in Preset::ALL {
        let imported = import_svg(&grouped(preset.id(), A_LINE)).unwrap();
        assert_eq!(imported.preset, preset, "id={}", preset.id());
        assert_eq!(imported.entities.len(), 1, "id={}", preset.id());
    }
}

/// LCV-115 AC#8 — geometry outside any group, and an empty file, are
/// `Cut`: the safe reading is the one the operator already expects.
#[test]
fn import_defaults_to_cut_for_bare_geometry() {
    assert_eq!(import_svg(LINE_SVG).unwrap().preset, Preset::Cut);
    assert_eq!(import_svg(&svg("")).unwrap().preset, Preset::Cut);
    assert_eq!(import_svg(G_GROUPS_SVG).unwrap().preset, Preset::Cut);
}

/// LCV-115 AC#8 — an empty `cut` group followed by a populated `mark` one
/// reports `Mark`. This is exactly the file this crate's exporter writes
/// for a marking job, so it is the real-world case, not an edge case.
#[test]
fn import_ignores_empty_groups_and_takes_the_first_group_with_geometry() {
    let src = svg(&format!(
        r#"<g id="cut"></g><g id="mark">{A_LINE}</g><g id="engrave"></g>"#
    ));
    let imported = import_svg(&src).unwrap();
    assert_eq!(imported.preset, Preset::Mark);
    assert_eq!(imported.entities.len(), 1);

    // …and the first *populated* group wins when two are populated.
    let both = svg(&format!(
        r#"<g id="engrave">{A_LINE}</g><g id="mark">{A_LINE}</g>"#
    ));
    assert_eq!(import_svg(&both).unwrap().preset, Preset::Engrave);
}

/// LCV-115 AC#8 — an id this crate does not know is not a preset; nested
/// geometry inherits the nearest *recognised* enclosing group instead.
#[test]
fn import_defaults_to_cut_for_unknown_group_id() {
    let unknown = grouped("layer1", A_LINE);
    assert_eq!(import_svg(&unknown).unwrap().preset, Preset::Cut);

    let nested = svg(&format!(r#"<g id="mark"><g id="layer1">{A_LINE}</g></g>"#));
    assert_eq!(import_svg(&nested).unwrap().preset, Preset::Mark);
}

/// LCV-115 AC#8 — matching is on the exact id, not a prefix or a
/// case-insensitive fold: `Preset::from_group_id` is the single decoder
/// and it is exact.
#[test]
fn preset_group_ids_match_exactly() {
    for id in ["CUT", "cut-1", "marks", " mark", ""] {
        assert_eq!(Preset::from_group_id(id), None, "id={id:?}");
    }
    for preset in Preset::ALL {
        assert_eq!(Preset::from_group_id(preset.id()), Some(preset));
    }
}
