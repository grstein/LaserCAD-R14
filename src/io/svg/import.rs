//! SVG import — pure function that parses a LaserCAD-exported SVG string.
//! Returns an [`ImportedSvg`] (entities plus the file's bed size); geometry
//! coordinates are bare mm values (no unit suffix). The root must be `svg` in
//! the [`SVG_NS`] namespace. Each element below it is, by namespace and local
//! name (LCV-171), and what it adds to [`ImportedSvg::report`]:
//!
//! | Outcome | Elements (SVG namespace unless noted) | Report |
//! |---|---|---|
//! | import | `line`, `circle`, `path` | properties; per `path`: curves not imported yet, `path (data error)`, or `path (unsupported data)` with no `d` |
//! | descend | `svg`, `g`, `a` | properties, then the children |
//! | never rendered | `defs symbol clipPath mask marker pattern linearGradient radialGradient filter` | name, iff it has an element child |
//! | silent | `title desc metadata`; any element outside the SVG namespace | nothing |
//! | other | every other SVG element, subtree included | name |
//!
//! "Properties" are the unapplied ones in [`report::REPORTED_PROPERTIES`].
//!
//! A `path`'s `d` is read with the full SVG 2 path-data grammar (LCV-172,
//! `super::path_data`): `M L H V Z A`, absolute or relative, every subpath.
//! [`path::path_entities`] turns it into lines and circular arcs; `C S Q T`
//! and elliptical arcs import nothing and are reported (`path C`, …,
//! `path elliptical arc`) until LCV-176/177. A syntax error keeps the
//! segments before it and reports `path (data error)`.
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
//! Layer groups are read too (LCV-156, ADR 0012 §4, see [`super::layers`]):
//! each entity belongs to its innermost enclosing `<g data-layer>`; geometry
//! outside any layer group goes to the file's first layer, and a file with no
//! layer group (a v0.2 file) gets the default `Cut` layer.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`. — LCV-057,
//! Y mirror by LCV-100, bed by LCV-114, layers by LCV-156.

use super::header::parse_bed;
use crate::document::entity::Entity;
use crate::document::{Document, Layer, LayerId};
use crate::geometry::{Circle, Line, Vec2};
use crate::util::flip_y;
use walk::Walk;

mod path;
pub(super) mod report;
mod walk;

/// The SVG namespace URI; only elements in it are SVG (LCV-171 AC 1).
pub const SVG_NS: &str = "http://www.w3.org/2000/svg";

/// Errors returned by [`import_svg`].
#[derive(Debug, thiserror::Error)]
pub enum SvgImportError {
    /// The input was not valid XML.
    #[error("XML parse error: {0}")]
    XmlParse(#[from] roxmltree::Error),
    /// The document root element was not `svg` in the [`SVG_NS`] namespace
    /// (a root without `xmlns` included).
    #[error("no <svg> root element in the SVG namespace found")]
    NoSvgRoot,
    /// A required numeric attribute could not be parsed, or `r ≤ 0` on `<circle>`.
    #[error("<{element}> attribute {attr}={value:?} is not a valid number")]
    MalformedAttribute {
        /// The element name, e.g. `circle`.
        element: &'static str,
        /// The attribute name, e.g. `r`.
        attr: &'static str,
        /// The raw attribute text that failed to parse.
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
    /// A `<g data-layer>` whose attributes are unusable, or that repeats
    /// another layer's name or color (LCV-156).
    #[error("layer {name:?}: {reason}")]
    MalformedLayer {
        /// The layer name as written in the file.
        name: String,
        /// Why it was refused.
        reason: String,
    },
}

/// The result of a successful [`import_svg`]: the geometry, its layers, and
/// the bed size the file declares (LCV-114 AC 7).
///
/// `bed_mm` is `[width, height]` in millimetres and is the axis pair the
/// entities were un-mirrored with. [`ImportedSvg::into_document`] installs
/// everything at once.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedSvg {
    /// Recognised geometry, in document order.
    pub entities: Vec<Entity>,
    /// The bed size declared by the file's root `<svg>`, or the default bed
    /// when it declares none.
    pub bed_mm: [f64; 2],
    /// The file's layers in document order (the default `Cut` layer when it
    /// declares none).
    pub layers: Vec<Layer>,
    /// The layer marked `data-current="1"`, else the first layer.
    pub current_layer: LayerId,
    /// One layer id per entity.
    pub entity_layers: Vec<LayerId>,
    /// What the file held that was not imported, as `(label, count)` in
    /// order of first occurrence, one entry per label (LCV-171 AC 8):
    /// skipped element names, path labels (`path (unsupported data)`,
    /// `path (data error)`, `path C`, …), and unapplied
    /// property names. Empty for a file LaserCAD wrote.
    pub report: Vec<(String, usize)>,
}

impl ImportedSvg {
    /// The opened document: bed, layers, current layer and membership.
    pub fn into_document(self) -> Result<Document, SvgImportError> {
        let current = self.current_layer;
        Document::from_parts(
            self.bed_mm,
            self.layers,
            current,
            self.entities,
            self.entity_layers,
        )
        .map_err(|e| SvgImportError::MalformedLayer {
            name: format!("#{}", current.0),
            reason: e.to_string(),
        })
    }
}

/// Parse an SVG string and return its geometry, layers and bed size.
///
/// Depth-first traversal; `<line>`, `<circle>`, `<path>` → entities.
/// What is skipped lands in the report (module docs). The bed comes from the root header
/// (see [`parse_bed`]) and is the axis every Y is un-mirrored around.
/// Returns the first error encountered, having mutated nothing: the caller's
/// document is untouched on `Err` (LCV-114 AC 9).
pub fn import_svg(src: &str) -> Result<ImportedSvg, SvgImportError> {
    let doc = roxmltree::Document::parse(src)?;
    let root = doc.root_element();
    if root.tag_name().name() != "svg" || root.tag_name().namespace() != Some(SVG_NS) {
        return Err(SvgImportError::NoSvgRoot);
    }
    let bed_mm = parse_bed(root)?;
    let mut walk = Walk::new(bed_mm[1]);
    walk.report.note_properties(root);
    walk.collect(root, None)?;
    let (layers, current_layer) = walk.layers.finish();
    Ok(ImportedSvg {
        entities: walk.entities,
        bed_mm,
        layers,
        current_layer,
        entity_layers: walk.entity_layers,
        report: walk.report.finish(),
    })
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

#[cfg(test)]
mod tests;
