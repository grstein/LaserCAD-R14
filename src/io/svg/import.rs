//! SVG import — pure function that parses a LaserCAD-exported SVG string.
//! Returns [`Vec<Entity>`]; recognises `<line>`, `<circle>`, `<path d="M…A…"/>`.
//! Silently skips unknown elements; coordinates are bare mm values (no unit suffix).
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`. — LCV-057.

use crate::document::entity::Entity;
use crate::geometry::{Arc, Circle, Line, Vec2, EPSILON};

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
}

/// Parse an SVG string and return all recognised geometry entities.
/// Depth-first traversal; `<line>`, `<circle>`, `<path d="M…A…"/>` → entities.
/// Everything else is silently skipped. Returns the first error encountered.
pub fn import_svg(src: &str) -> Result<Vec<Entity>, SvgImportError> {
    let doc = roxmltree::Document::parse(src)?;
    let root = doc.root_element();
    if root.tag_name().name() != "svg" {
        return Err(SvgImportError::NoSvgRoot);
    }
    let mut entities = Vec::new();
    collect(root, &mut entities)?;
    Ok(entities)
}

fn collect(node: roxmltree::Node<'_, '_>, out: &mut Vec<Entity>) -> Result<(), SvgImportError> {
    for child in node.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            "line" => out.push(parse_line(child)?),
            "circle" => out.push(parse_circle(child)?),
            "path" => {
                if let Some(e) = parse_path(child)? {
                    out.push(e);
                }
            }
            _ => collect(child, out)?,
        }
    }
    Ok(())
}

fn parse_line(n: roxmltree::Node<'_, '_>) -> Result<Entity, SvgImportError> {
    let (x1, y1) = (attr_f64(n, "line", "x1")?, attr_f64(n, "line", "y1")?);
    let (x2, y2) = (attr_f64(n, "line", "x2")?, attr_f64(n, "line", "y2")?);
    let (p1, p2) = (Vec2::new(x1, y1), Vec2::new(x2, y2));
    Ok(Entity::Line(Line::new(p1, p2)))
}

fn parse_circle(n: roxmltree::Node<'_, '_>) -> Result<Entity, SvgImportError> {
    let (cx, cy) = (attr_f64(n, "circle", "cx")?, attr_f64(n, "circle", "cy")?);
    let r = attr_f64(n, "circle", "r")?;
    if r <= 0.0 {
        let v = n.attribute("r").unwrap_or("").to_string();
        return Err(malformed("circle", "r", v));
    }
    Ok(Entity::Circle(Circle::new(Vec2::new(cx, cy), r)))
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

fn parse_path(n: roxmltree::Node<'_, '_>) -> Result<Option<Entity>, SvgImportError> {
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
    // Reconstruct arc center from endpoint encoding (LCV-057 §Arc reconstruction).
    let (dx, dy) = (ex - sx, ey - sy);
    let chord = dx.hypot(dy);
    if chord < EPSILON || chord > 2.0 * rx + EPSILON {
        return Err(SvgImportError::MalformedPath(d.to_string()));
    }
    let (mx, my) = ((sx + ex) / 2.0, (sy + ey) / 2.0);
    let h = (rx * rx - (chord / 2.0).powi(2)).max(0.0).sqrt();
    let (ux, uy) = (-dy / chord, dx / chord);
    let sign: f64 = if large_arc == sweep_flag { -1.0 } else { 1.0 };
    let (cx, cy) = (mx + sign * h * ux, my + sign * h * uy);
    let (sa, ea) = ((sy - cy).atan2(sx - cx), (ey - cy).atan2(ex - cx));
    let ctr = Vec2::new(cx, cy);
    Ok(Some(Entity::Arc(Arc::new(ctr, rx, sa, ea, sweep_flag))))
}

fn tok_f64(t: &str, d: &str) -> Result<f64, SvgImportError> {
    t.parse::<f64>()
        .map_err(|_| SvgImportError::MalformedPath(d.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::{FRAC_PI_2, PI};

    const LINE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><line x1="1.0000" y1="2.0000" x2="11.0000" y2="7.0000"/></svg>"#;
    const ARC_CCW_Q: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M 10.0000 0.0000 A 10.0000 10.0000 0 0 1 0.0000 10.0000"/></svg>"#;
    const ARC_LARGE_H: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M 10.0000 0.0000 A 10.0000 10.0000 0 1 1 -10.0000 0.0000"/></svg>"#;
    const ARC_CW_Q: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M 10.0000 0.0000 A 10.0000 10.0000 0 0 0 0.0000 -10.0000"/></svg>"#;
    const PATH_MALFORMED: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M 0 0 A notanumber 10 0 0 1 5 5"/></svg>"#;
    const MIXED_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><line x1="1" y1="2" x2="3" y2="4"/><rect/><circle cx="5" cy="5" r="3"/></svg>"#;
    const G_GROUPS_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><g><line x1="0" y1="0" x2="10" y2="10"/></g><g/><g/></svg>"#;

    fn svg(inner: &str) -> String {
        format!(r#"<svg xmlns="http://www.w3.org/2000/svg">{inner}</svg>"#)
    }

    #[test]
    fn empty_svg_returns_no_entities() {
        let es = import_svg(r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#).unwrap();
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
        let es = import_svg(LINE_SVG).unwrap();
        let Entity::Line(l) = es[0] else {
            panic!("not a line")
        };
        assert!((l.p1.x - 1.0).abs() < EPSILON && (l.p1.y - 2.0).abs() < EPSILON);
        assert!((l.p2.x - 11.0).abs() < EPSILON && (l.p2.y - 7.0).abs() < EPSILON);
    }

    #[test]
    fn circle_element_parsed_to_entity_circle() {
        let es = import_svg(&svg(r#"<circle cx="5.0000" cy="5.0000" r="3.0000"/>"#)).unwrap();
        let Entity::Circle(c) = es[0] else {
            panic!("not a circle")
        };
        assert!((c.center.x - 5.0).abs() < EPSILON && (c.center.y - 5.0).abs() < EPSILON);
        assert!((c.r - 3.0).abs() < EPSILON);
    }

    #[test]
    fn arc_ccw_quarter_reconstructed_correctly() {
        let es = import_svg(ARC_CCW_Q).unwrap();
        let Entity::Arc(a) = es[0] else {
            panic!("not an arc")
        };
        assert!(a.center.x.abs() < EPSILON && a.center.y.abs() < EPSILON);
        assert!((a.r - 10.0).abs() < EPSILON && a.start_angle.abs() < EPSILON);
        assert!((a.end_angle - FRAC_PI_2).abs() < EPSILON && a.ccw);
    }

    #[test]
    fn arc_large_flag_selects_correct_center() {
        let es = import_svg(ARC_LARGE_H).unwrap();
        let Entity::Arc(a) = es[0] else {
            panic!("not an arc")
        };
        assert!(a.center.x.abs() < EPSILON && a.center.y.abs() < EPSILON);
        assert!((a.r - 10.0).abs() < EPSILON && (a.end_angle - PI).abs() < EPSILON && a.ccw);
    }

    #[test]
    fn arc_cw_sweep_flag_zero_sets_ccw_false() {
        let es = import_svg(ARC_CW_Q).unwrap();
        assert!(matches!(es[0], Entity::Arc(a) if !a.ccw));
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
        let es = import_svg(&svg(r#"<path d="M 0 0 L 10 10"/>"#)).unwrap();
        assert!(es.is_empty());
    }

    #[test]
    fn unknown_elements_silently_skipped() {
        let es = import_svg(MIXED_SVG).unwrap();
        assert_eq!(es.len(), 2);
        assert!(matches!(es[0], Entity::Line(_)) && matches!(es[1], Entity::Circle(_)));
    }

    #[test]
    fn entities_inside_g_groups_collected() {
        let es = import_svg(G_GROUPS_SVG).unwrap();
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
        let imp = import_svg(&export_svg(&doc)).unwrap();
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

    #[test]
    fn import_svg_reachable_via_module_path() {
        let _f: fn(&str) -> Result<Vec<Entity>, SvgImportError> = import_svg;
    }
}
