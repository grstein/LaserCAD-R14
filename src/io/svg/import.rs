//! SVG import — pure function that parses a LaserCAD-exported SVG string.
//! Returns an [`ImportedSvg`] (entities plus the file's bed size); recognises
//! `<line>`, `<circle>`, `<path d="M…A…"/>`. Silently skips unknown elements;
//! geometry coordinates are bare mm values (no unit suffix).
//!
//! SVG is Y-down and the world is Y-up, so every parsed Y is un-mirrored
//! through [`crate::util::flip_y`] (`y_world = bed_height - y_svg`, the exact
//! inverse of `export.rs`'s map — `flip_y` is an involution). Because a
//! mirror reverses handedness, [`Arc::ccw`] is the **negation** of the SVG
//! sweep flag and the centre-selection sign is inverted accordingly.
//!
//! The mirror axis is the **file's own** bed height, read from the root
//! `<svg>` header by [`super::header::parse_bed`] (LCV-114): a file authored
//! at 300 × 180 must land back on the world coordinates it was exported from,
//! whatever bed the open document happens to be on.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`. — LCV-057,
//! Y mirror by LCV-100, bed by LCV-114.

use super::header::parse_bed;
use crate::document::entity::Entity;
use crate::geometry::{Arc, Circle, Line, Vec2, EPSILON};
use crate::util::flip_y;

/// Errors returned by [`import_svg`].
#[derive(Debug, thiserror::Error)]
pub enum SvgImportError {
    /// The input was not valid XML.
    #[error("XML parse error: {0}")]
    XmlParse(#[from] roxmltree::Error),
    /// The document root element was not `<svg>`.
    #[error("no <svg> root element found")]
    NoSvgRoot,
    /// A required numeric attribute could not be parsed, or `r ≤ 0` on `<circle>`.
    #[error("<{element}> attribute {attr}={value:?} is not a valid number")]
    MalformedAttribute {
        element: &'static str,
        attr: &'static str,
        value: String,
    },
    /// A `<path>` whose `d` begins with `M … A …` but contains a non-numeric token.
    #[error("malformed path data: {0:?}")]
    MalformedPath(String),
    /// A root `<svg>` bed attribute (`width`, `height` or `viewBox`) that is
    /// present but unusable: unparseable, non-finite, ≤ 0, or outside
    /// `1.0..=2000.0` mm (LCV-114 AC 9).
    ///
    /// Out-of-range values are rejected rather than clamped: a 5000 mm canvas
    /// silently held to 2000 mm would place every coordinate wrongly while
    /// looking plausible, whereas this error tells the truth and leaves the
    /// open document untouched.
    #[error("<svg> attribute {attr}={value:?} is not a usable bed size (expected 1..=2000 mm)")]
    MalformedBedDimension {
        /// The offending attribute name: `"width"`, `"height"` or `"viewBox"`.
        attr: &'static str,
        /// Its raw, unmodified attribute value.
        value: String,
    },
}

/// The result of a successful [`import_svg`]: the geometry, plus the bed size
/// the file declares (LCV-114 AC 7).
///
/// `bed_mm` is `[width, height]` in millimetres and is the axis pair the
/// entities were un-mirrored with, so installing both together —
/// `document.bed_mm = imported.bed_mm` **before** the entities — is what makes
/// an open / edit / re-save round-trip byte-stable.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedSvg {
    /// Recognised geometry, in document order.
    pub entities: Vec<Entity>,
    /// The bed size declared by the file's root `<svg>`, or the default bed
    /// when it declares none.
    pub bed_mm: [f64; 2],
}

/// Parse an SVG string and return its geometry together with its bed size.
///
/// Depth-first traversal; `<line>`, `<circle>`, `<path d="M…A…"/>` → entities.
/// Everything else is silently skipped. The bed comes from the root header
/// (see [`parse_bed`]) and is the axis every Y is un-mirrored around.
/// Returns the first error encountered, having mutated nothing: the caller's
/// document is untouched on `Err` (LCV-114 AC 9).
pub fn import_svg(src: &str) -> Result<ImportedSvg, SvgImportError> {
    let doc = roxmltree::Document::parse(src)?;
    let root = doc.root_element();
    if root.tag_name().name() != "svg" {
        return Err(SvgImportError::NoSvgRoot);
    }
    let bed_mm = parse_bed(root)?;
    let mut entities = Vec::new();
    collect(root, &mut entities, bed_mm[1])?;
    Ok(ImportedSvg { entities, bed_mm })
}

fn collect(
    node: roxmltree::Node<'_, '_>,
    out: &mut Vec<Entity>,
    bed_h: f64,
) -> Result<(), SvgImportError> {
    for child in node.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            "line" => out.push(parse_line(child, bed_h)?),
            "circle" => out.push(parse_circle(child, bed_h)?),
            "path" => {
                if let Some(e) = parse_path(child, bed_h)? {
                    out.push(e);
                }
            }
            _ => collect(child, out, bed_h)?,
        }
    }
    Ok(())
}

fn parse_line(n: roxmltree::Node<'_, '_>, bed_h: f64) -> Result<Entity, SvgImportError> {
    let (x1, y1) = (attr_f64(n, "line", "x1")?, attr_f64(n, "line", "y1")?);
    let (x2, y2) = (attr_f64(n, "line", "x2")?, attr_f64(n, "line", "y2")?);
    let (p1, p2) = (
        Vec2::new(x1, flip_y(y1, bed_h)),
        Vec2::new(x2, flip_y(y2, bed_h)),
    );
    Ok(Entity::Line(Line::new(p1, p2)))
}

fn parse_circle(n: roxmltree::Node<'_, '_>, bed_h: f64) -> Result<Entity, SvgImportError> {
    let (cx, cy) = (attr_f64(n, "circle", "cx")?, attr_f64(n, "circle", "cy")?);
    let r = attr_f64(n, "circle", "r")?;
    if r <= 0.0 {
        let v = n.attribute("r").unwrap_or("").to_string();
        return Err(malformed("circle", "r", v));
    }
    Ok(Entity::Circle(Circle::new(
        Vec2::new(cx, flip_y(cy, bed_h)),
        r,
    )))
}

fn malformed(element: &'static str, attr: &'static str, value: String) -> SvgImportError {
    SvgImportError::MalformedAttribute {
        element,
        attr,
        value,
    }
}

fn attr_f64(
    n: roxmltree::Node<'_, '_>,
    el: &'static str,
    a: &'static str,
) -> Result<f64, SvgImportError> {
    let raw = n.attribute(a).unwrap_or("");
    raw.parse::<f64>()
        .map_err(|_| malformed(el, a, raw.to_string()))
}

fn parse_path(n: roxmltree::Node<'_, '_>, bed_h: f64) -> Result<Option<Entity>, SvgImportError> {
    let Some(d) = n.attribute("d") else {
        return Ok(None);
    };
    let tok: Vec<&str> = d.split_ascii_whitespace().collect();
    if tok.len() < 11 || !tok[0].eq_ignore_ascii_case("m") || !tok[3].eq_ignore_ascii_case("a") {
        return Ok(None);
    }
    let sx = tok_f64(tok[1], d)?;
    let sy = tok_f64(tok[2], d)?;
    let rx = tok_f64(tok[4], d)?;
    let ry = tok_f64(tok[5], d)?;
    let xar = tok_f64(tok[6], d)?;
    let large_arc = tok_f64(tok[7], d)? != 0.0;
    let sweep_flag = tok_f64(tok[8], d)? != 0.0;
    let ex = tok_f64(tok[9], d)?;
    let ey = tok_f64(tok[10], d)?;
    if (rx - ry).abs() > EPSILON || xar.abs() > EPSILON {
        return Err(SvgImportError::MalformedPath(d.to_string()));
    }
    // Un-mirror both endpoints into world space first, then reconstruct the
    // centre there (LCV-057 §Arc reconstruction, mirrored by LCV-100).
    let (sy, ey) = (flip_y(sy, bed_h), flip_y(ey, bed_h));
    let (dx, dy) = (ex - sx, ey - sy);
    let chord = dx.hypot(dy);
    if chord < EPSILON || chord > 2.0 * rx + EPSILON {
        return Err(SvgImportError::MalformedPath(d.to_string()));
    }
    let (mx, my) = ((sx + ex) / 2.0, (sy + ey) / 2.0);
    let h = (rx * rx - (chord / 2.0).powi(2)).max(0.0).sqrt();
    let (ux, uy) = (-dy / chord, dx / chord);
    // Sign branches swapped relative to the un-mirrored reading: in world
    // space the SVG sweep flag denotes the opposite handedness.
    let sign: f64 = if large_arc == sweep_flag { 1.0 } else { -1.0 };
    let (cx, cy) = (mx + sign * h * ux, my + sign * h * uy);
    let (sa, ea) = ((sy - cy).atan2(sx - cx), (ey - cy).atan2(ex - cx));
    let ctr = Vec2::new(cx, cy);
    Ok(Some(Entity::Arc(Arc::new(ctr, rx, sa, ea, !sweep_flag))))
}

fn tok_f64(t: &str, d: &str) -> Result<f64, SvgImportError> {
    t.parse::<f64>()
        .map_err(|_| SvgImportError::MalformedPath(d.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::{FRAC_PI_2, PI};

    // Golden fixtures below are in the LCV-100 convention: SVG Y-down with the
    // bed as the canvas, i.e. world Y = 400 − SVG Y.
    const LINE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><line x1="1.0000" y1="2.0000" x2="11.0000" y2="7.0000"/></svg>"#;
    const ARC_CCW_Q: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M 10.0000 400.0000 A 10.0000 10.0000 0 0 0 0.0000 390.0000"/></svg>"#;
    const ARC_LARGE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M 110.0000 300.0000 A 10.0000 10.0000 0 1 0 100.0000 310.0000"/></svg>"#;
    const ARC_CW_Q: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M 0.0000 390.0000 A 10.0000 10.0000 0 0 1 10.0000 400.0000"/></svg>"#;
    const PATH_MALFORMED: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M 0 0 A notanumber 10 0 0 1 5 5"/></svg>"#;
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
}
