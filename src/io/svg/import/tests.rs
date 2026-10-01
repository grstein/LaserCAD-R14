use super::*;
use crate::document::{Layer, LayerId};
use crate::geometry::{Arc, EPSILON};
use core::f64::consts::{FRAC_PI_2, PI};

/// Millimetres per px (96 px = 1 in): a unitless length is px (LCV-173).
const MM_PER_PX: f64 = 25.4 / 96.0;

/// A root `<svg>` around a literal, on a 400 mm bed whose user unit is 1 mm,
/// as LaserCAD writes it.
macro_rules! root {
    ($inner:literal) => {
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="400mm" height="400mm" viewBox="0 0 400 400">"#,
            $inner,
            "</svg>"
        )
    };
}

// Golden fixtures below are in the LCV-100 convention: SVG Y-down with the
// bed as the canvas, i.e. world Y = 400 − SVG Y.
const LINE_SVG: &str = root!(r#"<line x1="1.0000" y1="2.0000" x2="11.0000" y2="7.0000"/>"#);
const ARC_CCW_Q: &str =
    root!(r#"<path d="M 10.0000 400.0000 A 10.0000 10.0000 0 0 0 0.0000 390.0000"/>"#);
const ARC_LARGE: &str =
    root!(r#"<path d="M 110.0000 300.0000 A 10.0000 10.0000 0 1 0 100.0000 310.0000"/>"#);
const ARC_CW_Q: &str =
    root!(r#"<path d="M 0.0000 390.0000 A 10.0000 10.0000 0 0 1 10.0000 400.0000"/>"#);
const PATH_MALFORMED: &str = root!(r#"<path d="M 0 0 A notanumber 10 0 0 1 5 5"/>"#);
const MIXED_SVG: &str =
    root!(r#"<line x1="1" y1="2" x2="3" y2="4"/><rect/><circle cx="5" cy="5" r="3"/>"#);
const G_GROUPS_SVG: &str = root!(r#"<g><line x1="0" y1="0" x2="10" y2="10"/></g><g/><g/>"#);

fn svg(inner: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="400mm" height="400mm" viewBox="0 0 400 400">{inner}</svg>"#
    )
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

/// LCV-171 AC 8 — a file with nothing to ignore has an empty report.
#[test]
fn empty_svg_has_an_empty_report() {
    let imported = import_svg(r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#).unwrap();
    assert!(imported.report.is_empty());
}

#[test]
fn invalid_xml_returns_xml_parse_error() {
    let r = import_svg("<svg><unclosed");
    assert!(matches!(r, Err(SvgImportError::XmlParse(_))));
}

/// LCV-171 AC 1 — the root must be `svg` in the SVG namespace; a missing
/// or foreign `xmlns` is refused, a prefixed SVG root is accepted.
#[test]
fn no_svg_root_returns_error() {
    for src in [
        "<root/>",
        "<svg/>",
        r#"<svg xmlns="http://example.com/x"/>"#,
    ] {
        let r = import_svg(src);
        assert!(matches!(r, Err(SvgImportError::NoSvgRoot)), "{src}: {r:?}");
    }
    let prefixed = r#"<s:svg xmlns:s="http://www.w3.org/2000/svg"/>"#;
    assert!(import_svg(prefixed).is_ok());
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

/// LCV-172 AC 9 — two exported arc `d` strings joined into one `d` import as
/// exactly the two arcs each imports alone.
#[test]
fn joined_exported_arc_paths_import_as_the_same_arcs() {
    let d_of = |src: &str| {
        let doc = roxmltree::Document::parse(src).unwrap();
        let path = doc.descendants().find(|n| n.has_tag_name("path"));
        path.and_then(|n| n.attribute("d")).unwrap().to_owned()
    };
    let (a, b) = (d_of(ARC_CCW_Q), d_of(ARC_LARGE));
    let joined = import_svg(&svg(&format!(r#"<path d="{a} {b}"/>"#))).unwrap();
    let alone: Vec<Entity> = [ARC_CCW_Q, ARC_LARGE]
        .iter()
        .flat_map(|src| import_svg(src).unwrap().entities)
        .collect();
    assert_eq!(alone.len(), 2);
    assert_eq!(joined.entities, alone);
    assert!(joined.report.is_empty(), "{:?}", joined.report);
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

/// LCV-172 AC 8 — a non-numeric `A` argument is a data error: the file
/// opens, the path imports nothing and the error is reported (was
/// `path_with_non_numeric_a_command_returns_malformed_path`).
#[test]
fn path_with_non_numeric_a_command_reports_a_data_error() {
    let imported = import_svg(PATH_MALFORMED).unwrap();
    assert!(imported.entities.is_empty());
    assert_eq!(imported.report, [("path (data error)".to_owned(), 1)]);
}

/// LCV-172 AC 3 — a non-arc path imports its line (was
/// `non_arc_path_silently_skipped`); LCV-171 AC 6 — a path with no `d` is
/// still reported.
#[test]
fn line_path_imports_and_path_without_data_is_reported() {
    let imported = import_svg(&svg(r#"<path d="M 0 0 L 10 10"/>"#)).unwrap();
    assert_eq!(imported.entities.len(), 1);
    assert!(matches!(imported.entities[0], Entity::Line(_)));
    assert!(imported.report.is_empty());
    let imported = import_svg(&svg("<path/>")).unwrap();
    assert!(imported.entities.is_empty());
    assert_eq!(imported.report, [("path (unsupported data)".to_owned(), 1)]);
}

/// LCV-171 AC 5 — an unsupported element is skipped and reported (was
/// `unknown_elements_silently_skipped`).
#[test]
fn unknown_elements_are_skipped_and_reported() {
    let imported = import_svg(MIXED_SVG).unwrap();
    let es = imported.entities;
    assert_eq!(es.len(), 2);
    assert!(matches!(es[0], Entity::Line(_)) && matches!(es[1], Entity::Circle(_)));
    assert_eq!(imported.report, [("rect".to_owned(), 1)]);
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
    doc.push_current(Entity::Line(lin));
    let cir = Circle::new(Vec2::new(5.0, 5.0), 3.0);
    doc.push_current(Entity::Circle(cir));
    let arc = Arc::new(Vec2::default(), 10.0, 0.0, FRAC_PI_2, true);
    doc.push_current(Entity::Arc(arc));
    let imp = import_svg(&export_svg(&doc)).unwrap().entities;
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

/// LCV-114 AC 8b, LCV-173 AC 2 — with no `width`/`height`, the `viewBox`
/// read as px is the bed and the mirror axis.
#[test]
fn import_reads_bed_from_viewbox_when_dimensions_absent() {
    let src = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 180"><circle cx="5" cy="130" r="3"/></svg>"#;
    let imported = import_svg(src).unwrap();
    let bed = imported.bed_mm;
    assert!(
        (bed[0] - 300.0 * MM_PER_PX).abs() < EPSILON
            && (bed[1] - 180.0 * MM_PER_PX).abs() < EPSILON
    );
    let Entity::Circle(c) = imported.entities[0] else {
        panic!("not a circle")
    };
    assert!(
        (c.center.y - 50.0 * MM_PER_PX).abs() < EPSILON,
        "cy = {}",
        c.center.y
    );
    assert!((c.r - 3.0 * MM_PER_PX).abs() < EPSILON);
}

/// LCV-114 AC 8c, LCV-173 AC 4 — a root with neither a dimension pair nor
/// a `viewBox` sits on the default bed, one user unit being 1 px.
#[test]
fn import_falls_back_to_default_when_both_absent() {
    let src =
        r#"<svg xmlns="http://www.w3.org/2000/svg"><line x1="1" y1="2" x2="11" y2="7"/></svg>"#;
    let imported = import_svg(src).unwrap();
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
    assert!((l.p1.x - MM_PER_PX).abs() < EPSILON);
    assert!((l.p1.y - (400.0 - 2.0 * MM_PER_PX)).abs() < EPSILON);
}

/// LCV-114 AC 8 — the accepted numeric forms reach the importer, not just
/// the header parser.
#[test]
fn import_accepts_mm_suffix_and_whitespace() {
    let src = r#"<svg xmlns="http://www.w3.org/2000/svg" width=" 300.5 mm " height="180MM"/>"#;
    assert_eq!(import_svg(src).unwrap().bed_mm, [300.5, 180.0]);
    let bare = r#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="180"/>"#;
    let bed = import_svg(bare).unwrap().bed_mm;
    let px = [300.0 * MM_PER_PX, 180.0 * MM_PER_PX];
    assert!(
        (bed[0] - px[0]).abs() < EPSILON && (bed[1] - px[1]).abs() < EPSILON,
        "unitless is px: {bed:?}"
    );
}

/// LCV-114 AC 9 — one assertion per rejection case; each names the
/// offending attribute and keeps its raw value in the message, and the
/// import as a whole fails rather than importing at a guessed bed.
#[test]
fn import_rejects_zero_negative_oversized_and_garbage_dimensions() {
    for (attrs, attr, raw) in [
        (r#"width="0" height="180""#, "width", "0"),
        (r#"width="-300" height="180""#, "width", "-300"),
        (r#"width="5000mm" height="180""#, "width", "5000mm"),
        (r#"width="banana" height="180""#, "width", "banana"),
        (r#"width="300" height="0""#, "height", "0"),
        (r#"width="300" height="inf""#, "height", "inf"),
        (r#"viewBox="0 0 9000 180""#, "viewBox", "0 0 9000 180"),
        (r#"viewBox="0 0 0 180""#, "viewBox", "0 0 0 180"),
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

// ── LCV-156 — layers ──────────────────────────────────────────────────

/// Wrap `inner` in a `<g id="…">` the way the v0.2 exporter did.
fn grouped(id: &str, inner: &str) -> String {
    svg(&format!(r#"<g id="{id}" fill="none">{inner}</g>"#))
}

const A_LINE: &str = r#"<line x1="0" y1="0" x2="10" y2="10"/>"#;

/// A v0.2 file (preset groups, no layer group), bare geometry and an empty
/// file all open on the single default `Cut` layer.
#[test]
fn files_without_layer_groups_open_on_the_default_layer() {
    for src in [
        grouped("mark", A_LINE),
        LINE_SVG.to_owned(),
        svg(""),
        G_GROUPS_SVG.to_owned(),
    ] {
        let imported = import_svg(&src).unwrap();
        assert_eq!(imported.layers, vec![Layer::default_cut()], "{src}");
        assert_eq!(imported.current_layer, LayerId(0));
        assert_eq!(imported.entity_layers.len(), imported.entities.len());
        assert!(imported.entity_layers.iter().all(|&id| id == LayerId(0)));
    }
}

/// The innermost enclosing layer group owns the geometry; a plain `<g>`
/// inside a layer group inherits it.
#[test]
fn innermost_layer_group_owns_the_geometry() {
    let src = svg(&format!(
        r##"<g data-layer="A" stroke="#010101"><g>{A_LINE}</g><g data-layer="B" stroke="#020202">{A_LINE}</g>{A_LINE}</g>"##
    ));
    let imported = import_svg(&src).unwrap();
    assert_eq!(imported.entity_layers, [LayerId(0), LayerId(1), LayerId(0)]);
    let doc = imported.into_document().unwrap();
    assert_eq!(doc.layers().len(), 2);
}

/// Attribute values are XML-unescaped on the way in.
#[test]
fn layer_names_are_unescaped() {
    let src = svg(r##"<g data-layer="A &amp; &quot;B&quot;" stroke="#010101"/>"##);
    assert_eq!(import_svg(&src).unwrap().layers[0].name, "A & \"B\"");
}

/// SVG 2 F.6.6 — a chord past the diameter (here by the file's rounding)
/// scales the radius up to half the chord: the arc is a half turn.
#[test]
fn arc_chord_past_diameter_scales_radius_up() {
    let a = only_arc(&svg(
        r#"<path d="M 10.0001 400.0000 A 10.0000 10.0000 0 0 0 -10.0001 400.0000"/>"#,
    ));
    assert!((a.r - 10.0001).abs() < EPSILON);
    assert!((a.sweep_angle() - PI).abs() < EPSILON);
    assert!(a.center.x.abs() < EPSILON && a.center.y.abs() < EPSILON);
}

/// LCV-172 AC 6 — a zero radius draws a straight line (SVG 2 §F.6.6), it is
/// not scaled up (was `arc_zero_radius_returns_malformed_path`).
#[test]
fn arc_zero_radius_imports_a_line() {
    let imported = import_svg(&svg(r#"<path d="M 0 0 A 0 0 0 0 1 5 5"/>"#)).unwrap();
    assert!(imported.report.is_empty());
    let [Entity::Line(l)] = imported.entities.as_slice() else {
        panic!("{:?}", imported.entities);
    };
    assert!((l.p2.x - l.p1.x - 5.0).abs() < EPSILON);
    assert!((l.p1.y - l.p2.y - 5.0).abs() < EPSILON, "Y is mirrored");
}

// ── LCV-171 — import report and never-rendered elements ──────────────

/// The report as `(&str, usize)` pairs, for terse assertions.
fn report_of(src: &str) -> Vec<(String, usize)> {
    import_svg(src).unwrap().report
}

fn entry(label: &str, count: usize) -> (String, usize) {
    (label.to_owned(), count)
}

/// AC 2 — `a` and a nested `svg` are descended into; the prefixed SVG
/// namespace is the SVG namespace.
#[test]
fn a_and_nested_svg_are_descended_into() {
    for inner in [
        format!("<a>{A_LINE}</a>"),
        format!("<svg>{A_LINE}</svg>"),
        format!("<g><a><svg>{A_LINE}</svg></a></g>"),
        r#"<svg:line xmlns:svg="http://www.w3.org/2000/svg" x1="0" y1="0" x2="1" y2="1"/>"#
            .to_owned(),
    ] {
        let imported = import_svg(&svg(&inner)).unwrap();
        assert_eq!(imported.entities.len(), 1, "{inner}");
        assert!(imported.report.is_empty(), "{inner}");
    }
}

/// AC 4 — `title`, `desc`, `metadata` and anything outside the SVG namespace
/// are skipped with their subtree, and reported nowhere.
#[test]
fn silent_elements_and_foreign_namespaces_import_nothing_unreported() {
    for inner in [
        format!("<title>{A_LINE}</title>"),
        format!("<desc>t{A_LINE}</desc>"),
        format!("<metadata><g>{A_LINE}</g></metadata>"),
        format!(
            r#"<sodipodi:namedview xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd">{A_LINE}</sodipodi:namedview>"#
        ),
        r#"<foo:line xmlns:foo="http://example.com/foo" x1="0" y1="0" x2="1" y2="1"/>"#.to_owned(),
        format!(r#"<foo:g xmlns:foo="http://example.com/foo">{A_LINE}</foo:g>"#),
    ] {
        let imported = import_svg(&svg(&inner)).unwrap();
        assert!(imported.entities.is_empty(), "{inner}");
        assert!(imported.report.is_empty(), "{inner}");
    }
}

const NEVER_RENDERED: [&str; 9] = [
    "defs",
    "symbol",
    "clipPath",
    "mask",
    "marker",
    "pattern",
    "linearGradient",
    "radialGradient",
    "filter",
];

/// AC 3 — geometry inside a never-rendered element is not imported, and the
/// element is reported by name.
#[test]
fn never_rendered_elements_import_nothing_and_are_reported() {
    for name in NEVER_RENDERED {
        let src = svg(&format!(
            r#"<{name}>{A_LINE}<circle cx="1" cy="1" r="1"/></{name}>"#
        ));
        let imported = import_svg(&src).unwrap();
        assert!(imported.entities.is_empty(), "{name}");
        assert_eq!(imported.report, [entry(name, 1)], "{name}");
    }
}

/// AC 3 — reported iff it has an element child; a `stop` counts.
#[test]
fn never_rendered_elements_without_element_children_are_not_reported() {
    assert!(report_of(&svg("<defs/>")).is_empty());
    assert!(report_of(&svg("<clipPath> text </clipPath>")).is_empty());
    let gradient = svg("<linearGradient><stop/></linearGradient>");
    assert_eq!(report_of(&gradient), [entry("linearGradient", 1)]);
}

/// AC 5 — every other SVG element is skipped with its subtree and reported
/// by name, repeats counted under one entry in first-occurrence order.
#[test]
fn other_svg_elements_are_skipped_and_reported_with_counts() {
    let inner = format!(
        r##"<image href="x.png"/><text>{A_LINE}</text><use href="#l"/><switch>{A_LINE}</switch><image/><rect width="1" height="1"/><style>line {{}}</style><script/><foreignObject>{A_LINE}</foreignObject><text/>"##
    );
    let imported = import_svg(&svg(&inner)).unwrap();
    assert!(imported.entities.is_empty());
    let want = [
        entry("image", 2),
        entry("text", 2),
        entry("use", 1),
        entry("switch", 1),
        entry("rect", 1),
        entry("style", 1),
        entry("script", 1),
        entry("foreignObject", 1),
    ];
    assert_eq!(imported.report, want);
}

const REPORTED: [&str; 11] = [
    "fill",
    "clip-path",
    "mask",
    "filter",
    "marker-start",
    "marker-mid",
    "marker-end",
    "stroke-dasharray",
    "opacity",
    "display",
    "visibility",
];

/// `{p}` is where the property goes on each imported or descended host;
/// the second value is how many entities the host imports.
const HOSTS: [(&str, usize); 6] = [
    ("<svg {p}/>", 0),
    ("<g {p}/>", 0),
    ("<a {p}/>", 0),
    (r#"<line {p} x1="0" y1="0" x2="1" y2="1"/>"#, 1),
    (r#"<circle {p} cx="1" cy="1" r="1"/>"#, 1),
    (r#"<path {p} d="M 10 400 A 10 10 0 0 0 0 390"/>"#, 1),
];

/// AC 7 — every property is reported, as an attribute or a `style`
/// declaration, on every element that is imported or descended into.
#[test]
fn unapplied_properties_are_reported_on_imported_and_descended_elements() {
    for prop in REPORTED {
        for form in [
            format!(r#"{prop}="red""#),
            format!(r#"style="{prop}: red""#),
        ] {
            let root = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" {form}/>"#);
            assert_eq!(report_of(&root), [entry(prop, 1)], "{root}");
            for (host, count) in HOSTS {
                let src = svg(&host.replace("{p}", &form));
                let imported = import_svg(&src).unwrap();
                assert_eq!(imported.entities.len(), count, "{src}");
                assert_eq!(imported.report, [entry(prop, 1)], "{src}");
            }
        }
    }
}

/// AC 7 / AC 10 — `fill` is exempt only when `none`; `style` declarations
/// ignore case and `!important`.
#[test]
fn fill_none_is_exempt_and_style_ignores_case_and_important() {
    for quiet in [
        r#"<g fill="none"/>"#,
        r#"<g style="fill: NONE"/>"#,
        r#"<g style="stroke:#000; FILL:none !important"/>"#,
    ] {
        assert!(report_of(&svg(quiet)).is_empty(), "{quiet}");
    }
    assert_eq!(report_of(&svg(r#"<g fill="red"/>"#)), [entry("fill", 1)]);
    let styled = svg(r#"<g style="FILTER: none !important; Opacity:1"/>"#);
    assert_eq!(
        report_of(&styled),
        [entry("filter", 1), entry("opacity", 1)]
    );
}

/// AC 7 — properties on skipped elements are not read; a repeat counts twice
/// under one label.
#[test]
fn properties_on_skipped_elements_are_not_reported_and_repeats_count() {
    let image = svg(r#"<image opacity="0.5"/>"#);
    assert_eq!(report_of(&image), [entry("image", 1)]);
    let defs = svg(&format!(r#"<defs><g opacity="0.5">{A_LINE}</g></defs>"#));
    assert_eq!(report_of(&defs), [entry("defs", 1)]);
    let twice = svg(r#"<g opacity="0.5"><line opacity="0.5" x1="0" y1="0" x2="1" y2="1"/></g>"#);
    assert_eq!(report_of(&twice), [entry("opacity", 2)]);
}
