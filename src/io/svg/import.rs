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
//! The enclosing `<g id="…">` is read too (LCV-115): the file's export preset
//! is the one whose group first produced geometry, so an open / edit / re-save
//! round trip cannot silently demote a marking job to a cutting job.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`. — LCV-057,
//! Y mirror by LCV-100, bed by LCV-114, preset by LCV-115.

use super::export::Preset;
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

/// The result of a successful [`import_svg`]: the geometry, plus the two
/// file-level properties the opened document adopts — the bed size it declares
/// (LCV-114 AC 7) and its export preset (LCV-115 AC 8).
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
    /// The preset of the first recognised `<g>` whose subtree produced an
    /// entity, or [`Preset::Cut`] when the file has no recognised group.
    /// `App::export_preset` adopts it on open, so re-saving a marking file
    /// returns its geometry to the `mark` group instead of demoting it to a
    /// cut (LCV-115 AC 9).
    pub preset: Preset,
}

/// Parse an SVG string and return its geometry together with its bed size.
///
/// Depth-first traversal; `<line>`, `<circle>`, `<path d="M…A…"/>` → entities.
/// Everything else is silently skipped. The bed comes from the root header
/// (see [`parse_bed`]) and is the axis every Y is un-mirrored around; the
/// preset comes from the enclosing `<g id="…">` (see [`collect`]).
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
    let mut preset = None;
    collect(root, &mut entities, bed_mm[1], None, &mut preset)?;
    Ok(ImportedSvg {
        entities,
        bed_mm,
        // No recognised group produced geometry: bare geometry, an empty file
        // or an unknown group id all mean "cut" (LCV-115 AC 8).
        preset: preset.unwrap_or(Preset::Cut),
    })
}

/// Walk `node`'s element subtree, appending recognised geometry to `out`.
///
/// `group` is the preset of the nearest enclosing recognised `<g>`, and
/// `found` is the detection result: the first `group` that was `Some` when an
/// entity was appended wins, and it is never overwritten afterwards
/// (LCV-115 AC 8). Geometry outside any recognised group leaves `found`
/// untouched, so a file whose only `cut` group is empty still reports the
/// populated `mark` group that follows it — which is exactly the shape this
/// crate's own exporter writes.
fn collect(
    node: roxmltree::Node<'_, '_>,
    out: &mut Vec<Entity>,
    bed_h: f64,
    group: Option<Preset>,
    found: &mut Option<Preset>,
) -> Result<(), SvgImportError> {
    for child in node.children().filter(|n| n.is_element()) {
        let before = out.len();
        match child.tag_name().name() {
            "line" => out.push(parse_line(child, bed_h)?),
            "circle" => out.push(parse_circle(child, bed_h)?),
            "path" => {
                if let Some(e) = parse_path(child, bed_h)? {
                    out.push(e);
                }
            }
            _ => collect(child, out, bed_h, group_of(child, group), found)?,
        }
        if found.is_none() && out.len() > before {
            *found = group;
        }
    }
    Ok(())
}

/// The preset in force inside `node`: its own `id` when `node` is a `<g>`
/// carrying a recognised one, otherwise the enclosing `group` unchanged.
fn group_of(node: roxmltree::Node<'_, '_>, group: Option<Preset>) -> Option<Preset> {
    if node.tag_name().name() != "g" {
        return group;
    }
    node.attribute("id")
        .and_then(Preset::from_group_id)
        .or(group)
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
mod tests;
