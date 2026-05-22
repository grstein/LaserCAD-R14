//! SVG importer for the strict subset emitted by [`super::export::export_svg`].
//!
//! Pure function [`import_svg`]: no file I/O, no `egui`/`eframe`/`rfd`. LCV-057.

use crate::document::Entity;
use crate::geometry::{Arc, Circle, Line, Vec2, EPSILON};

/// Errors returned by [`import_svg`].
#[derive(Debug, thiserror::Error)]
pub enum SvgImportError {
    /// The input string is not well-formed XML.
    #[error("XML parse error: {0}")]
    XmlParse(#[from] roxmltree::Error),

    /// The document root element is not `<svg>`.
    #[error("no <svg> root element found")]
    NoSvgRoot,

    /// A required numeric attribute on a recognised element could not be parsed.
    #[error("malformed attribute on <{element}> attr '{attr}': '{value}'")]
    MalformedAttribute {
        /// The element name (`"line"`, `"circle"`, …).
        element: &'static str,
        /// The attribute name (`"x1"`, `"r"`, …).
        attr: &'static str,
        /// The raw attribute value that failed to parse.
        value: String,
    },

    /// A `<path>` element whose `d` attribute starts with `M … A …` but
    /// contains a non-numeric token.
    #[error("malformed arc path: {0}")]
    MalformedPath(String),
}

/// Parse an SVG string and return the entities it contains.
///
/// Only the strict subset produced by [`super::export::export_svg`] is
/// recognised. Unknown elements are silently skipped; the traversal is
/// depth-first over the full XML tree.
///
/// Returns `Err` only on:
/// - Invalid XML (`SvgImportError::XmlParse`).
/// - Missing `<svg>` root (`SvgImportError::NoSvgRoot`).
/// - A required attribute on `<line>` / `<circle>` fails to parse as `f64`.
/// - A `<path d="M … A …">` contains a non-numeric token.
///
/// All other malformed or unknown content is silently skipped.
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

fn collect<'a, 'input: 'a>(
    node: roxmltree::Node<'a, 'input>,
    out: &mut Vec<Entity>,
) -> Result<(), SvgImportError> {
    for child in node.children() {
        if !child.is_element() {
            continue;
        }
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

fn parse_line(node: roxmltree::Node) -> Result<Entity, SvgImportError> {
    let x1 = f64_attr(node, "line", "x1")?;
    let y1 = f64_attr(node, "line", "y1")?;
    let x2 = f64_attr(node, "line", "x2")?;
    let y2 = f64_attr(node, "line", "y2")?;
    Ok(Entity::Line(Line::new(
        Vec2::new(x1, y1),
        Vec2::new(x2, y2),
    )))
}

fn parse_circle(node: roxmltree::Node) -> Result<Entity, SvgImportError> {
    let cx = f64_attr(node, "circle", "cx")?;
    let cy = f64_attr(node, "circle", "cy")?;
    let r_raw = node.attribute("r").unwrap_or("");
    let r: f64 = r_raw
        .trim()
        .parse()
        .map_err(|_| SvgImportError::MalformedAttribute {
            element: "circle",
            attr: "r",
            value: r_raw.to_owned(),
        })?;
    if r <= 0.0 {
        return Err(SvgImportError::MalformedAttribute {
            element: "circle",
            attr: "r",
            value: r_raw.to_owned(),
        });
    }
    Ok(Entity::Circle(Circle::new(Vec2::new(cx, cy), r)))
}

/// Returns `Ok(None)` for paths that don't match the `M … A …` shape,
/// `Ok(Some(arc))` on success, `Err` if the path looks like an arc but has
/// bad numeric tokens.
fn parse_path(node: roxmltree::Node) -> Result<Option<Entity>, SvgImportError> {
    let d = match node.attribute("d") {
        Some(v) => v,
        None => return Ok(None),
    };

    let tokens: Vec<&str> = d.split_ascii_whitespace().collect();

    // Silently skip if the path doesn't look like "M sx sy A …"
    if tokens.len() < 11 {
        return Ok(None);
    }
    if !tokens[0].eq_ignore_ascii_case("M") {
        return Ok(None);
    }
    if !tokens[3].eq_ignore_ascii_case("A") {
        return Ok(None);
    }

    // Parse all required numeric tokens (1,2 = start; 4,5 = rx,ry; 6 = rot;
    // 7,8 = large,sweep; 9,10 = end).
    let sx = f64_tok(tokens[1], d)?;
    let sy = f64_tok(tokens[2], d)?;
    let rx = f64_tok(tokens[4], d)?;
    let ry = f64_tok(tokens[5], d)?;
    let x_rot = f64_tok(tokens[6], d)?;
    let large_arc: u8 = tokens[7]
        .parse()
        .map_err(|_| SvgImportError::MalformedPath(d.to_owned()))?;
    let sweep: u8 = tokens[8]
        .parse()
        .map_err(|_| SvgImportError::MalformedPath(d.to_owned()))?;
    let ex = f64_tok(tokens[9], d)?;
    let ey = f64_tok(tokens[10], d)?;

    // Validate: only circular (rx == ry) and zero x-axis-rotation arcs.
    if (rx - ry).abs() > EPSILON {
        return Err(SvgImportError::MalformedPath(d.to_owned()));
    }
    if x_rot.abs() > EPSILON {
        return Err(SvgImportError::MalformedPath(d.to_owned()));
    }

    let arc = reconstruct_arc(
        Vec2::new(sx, sy),
        Vec2::new(ex, ey),
        rx,
        large_arc != 0,
        sweep != 0,
        d,
    )?;
    Ok(Some(Entity::Arc(arc)))
}

fn reconstruct_arc(
    start: Vec2,
    end: Vec2,
    r: f64,
    large_arc: bool,
    sweep: bool,
    raw_d: &str,
) -> Result<Arc, SvgImportError> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let chord = dx.hypot(dy);

    if chord < EPSILON {
        return Err(SvgImportError::MalformedPath(raw_d.to_owned()));
    }
    if chord > 2.0 * r + EPSILON {
        return Err(SvgImportError::MalformedPath(raw_d.to_owned()));
    }

    let mx = (start.x + end.x) / 2.0;
    let my = (start.y + end.y) / 2.0;
    let half_chord = chord / 2.0;
    let h = (r * r - half_chord * half_chord).max(0.0).sqrt();

    // Perpendicular unit vector (90° CCW from chord direction)
    let ux = -dy / chord;
    let uy = dx / chord;

    // Sign rule (from LCV-057): sign = if large_arc == sweep { -1 } else { 1 }
    let sign: f64 = if large_arc == sweep { -1.0 } else { 1.0 };

    let cx = mx + sign * h * ux;
    let cy = my + sign * h * uy;

    let start_angle = (start.y - cy).atan2(start.x - cx);
    let end_angle = (end.y - cy).atan2(end.x - cx);

    Ok(Arc::new(
        Vec2::new(cx, cy),
        r,
        start_angle,
        end_angle,
        sweep,
    ))
}

/// Parse a required `f64` attribute; missing or unparseable → `MalformedAttribute`.
fn f64_attr(
    node: roxmltree::Node,
    element: &'static str,
    attr: &'static str,
) -> Result<f64, SvgImportError> {
    let raw = node
        .attribute(attr)
        .ok_or_else(|| SvgImportError::MalformedAttribute {
            element,
            attr,
            value: String::new(),
        })?;
    raw.trim()
        .parse()
        .map_err(|_| SvgImportError::MalformedAttribute {
            element,
            attr,
            value: raw.to_owned(),
        })
}

/// Parse a token from a path `d` attribute; failure → `MalformedPath`.
fn f64_tok(token: &str, raw_d: &str) -> Result<f64, SvgImportError> {
    token
        .parse()
        .map_err(|_| SvgImportError::MalformedPath(raw_d.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile check — proves `import_svg` has the right signature.
    #[test]
    fn import_svg_fn_signature_compiles() {
        let _f: fn(&str) -> Result<Vec<Entity>, SvgImportError> = import_svg;
    }

    /// Empty SVG returns an empty entity list.
    #[test]
    fn empty_svg_returns_no_entities() {
        let r = import_svg(r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#).unwrap();
        assert!(r.is_empty());
    }

    /// Invalid XML propagates a parse error.
    #[test]
    fn invalid_xml_returns_xml_parse_error() {
        assert!(matches!(
            import_svg("<svg><unclosed"),
            Err(SvgImportError::XmlParse(_))
        ));
    }

    /// Non-SVG root element returns `NoSvgRoot`.
    #[test]
    fn no_svg_root_returns_error() {
        assert!(matches!(
            import_svg("<root/>"),
            Err(SvgImportError::NoSvgRoot)
        ));
    }
}
