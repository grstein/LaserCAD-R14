//! SVG export for LaserGRBL-compatible output.
//!
//! Provides [`export_svg`], a pure function that converts a [`Document`] into
//! a well-formed UTF-8 SVG string. The output satisfies LaserGRBL's strict
//! parser requirements: correct `xmlns`, millimetre `width`/`height`,
//! one colour group per layer, `fill="none"`, and true SVG arc paths (`A`
//! command — no bézier approximation). [`export_layer_svg`] writes one
//! layer alone for `File > Export layers` (LCV-156).
//!
//! **Y is mirrored on the way out.** The world is Y-up, SVG is Y-down, so
//! every emitted Y goes through [`crate::util::flip_y`]
//! (`y_svg = doc.bed_mm[1] - y_world`) and the canvas is the bed itself. The
//! mirror also reverses handedness, so the arc sweep flag is inverted with
//! respect to [`Arc::ccw`](crate::geometry::Arc::ccw). `src/io/svg/import.rs`
//! applies the same involution in reverse.
//!
//! The mirror axis is the **document's** bed height (LCV-114), never a
//! constant: a drawing made on a 180 mm-tall machine mirrored around 400 mm
//! produces a file that looks perfectly plausible in a text editor and cuts
//! 220 mm away from where the screen showed it.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.
//!
//! Introduced by demand LCV-056; Y mirror added by LCV-100, parameterised on
//! the document's bed by LCV-114, one group per layer by LCV-156.

use super::layers::open_group;
use crate::document::entity::Entity;
use crate::document::state::Document;
use crate::document::{Layer, LayerId};
use crate::util::flip_y;
use core::f64::consts::PI;

// ─── Public API ──────────────────────────────────────────────────────────────

/// Export a [`Document`] as the mother SVG (LCV-156, ADR 0012 §4).
///
/// The returned string is a self-contained UTF-8 XML fragment rooted at
/// `<svg>` with:
///
/// - `xmlns="http://www.w3.org/2000/svg"` on the root element.
/// - `width` and `height` in millimetres: always the document's bed size
///   (`doc.bed_mm`), for every document, empty or not.
/// - `viewBox="0 0 <width> <height>"` — the same two numbers in SVG
///   coordinates, no unit suffix (LCV-114).
/// - `fill="none"` on the root element.
/// - One `<g>` per layer, in layer order, empty layers included, carrying the
///   layer's name, color and Output flag, `stroke-width="0.1"` (mm), and
///   `data-current="1"` on the current layer only (see [`super::layers`]).
/// - Every entity, in index order, inside its own layer's group.
///
/// Every Y coordinate is mirrored through [`crate::util::flip_y`] **around
/// `doc.bed_mm[1]`**; X values, radii and the `large-arc-flag` are untouched.
/// Geometry outside the bed is emitted verbatim, never clipped or dropped.
///
/// The two bed numbers are formatted with `{}` (`f64`'s `Display`): a 400 mm
/// bed emits `400mm`, not `400.0000mm`. Coordinates keep their own `{:.4}`.
///
/// Pure function: no file I/O, no global state, no panics on any valid
/// [`Document`].
pub fn export_svg(doc: &Document) -> String {
    let mut out = open_svg(doc.bed_mm);
    for layer in doc.layers() {
        push_layer(&mut out, doc, layer, layer.id == doc.current_layer());
    }
    out.push_str("</svg>");
    out
}

/// Export one layer as a LaserGRBL file (LCV-156 AC 10): the same header as
/// [`export_svg`] and only `id`'s group, without `data-current`. An unknown
/// `id` yields the header alone.
pub fn export_layer_svg(doc: &Document, id: LayerId) -> String {
    let mut out = open_svg(doc.bed_mm);
    if let Some(layer) = doc.layer(id) {
        push_layer(&mut out, doc, layer, false);
    }
    out.push_str("</svg>");
    out
}

// ─── Private helpers ─────────────────────────────────────────────────────────

/// The root `<svg ...>` opening tag and newline: the canvas is always the
/// document's bed (LCV-100, parameterised by LCV-114).
fn open_svg(bed_mm: [f64; 2]) -> String {
    let mut out = String::with_capacity(1024);
    let [bed_w, bed_h] = bed_mm;
    out.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\"");
    push_attr(&mut out, "width", &format!("{bed_w}mm"));
    push_attr(&mut out, "height", &format!("{bed_h}mm"));
    push_attr(&mut out, "viewBox", &format!("0 0 {bed_w} {bed_h}"));
    push_attr(&mut out, "fill", "none");
    out.push_str(">\n");
    out
}

/// Append `layer`'s group holding every entity on it, in index order. An
/// entity without membership (never, for a document kept in lockstep) is
/// written with the first layer rather than dropped.
fn push_layer(out: &mut String, doc: &Document, layer: &Layer, current: bool) {
    open_group(out, layer, current);
    let first = doc.layers().first().map(|l| l.id);
    for (i, entity) in doc.entities.iter().enumerate() {
        if doc.entity_layer(i).or(first) == Some(layer.id) {
            out.push_str(&encode_entity(entity, doc.bed_mm[1]));
            out.push('\n');
        }
    }
    out.push_str("</g>\n");
}

/// Append ` key="value"` to `out`.
fn push_attr(out: &mut String, key: &str, value: &str) {
    out.push(' ');
    out.push_str(key);
    out.push_str("=\"");
    out.push_str(value);
    out.push('"');
}

/// Encode a single [`Entity`] as an SVG element string (no trailing newline).
///
/// The world → SVG mirror is applied here, per entity: Y values through
/// [`flip_y`] around `bed_height_mm` — the bed height of the document being
/// exported — and, because a mirror reverses handedness, the arc
/// `sweep-flag` inverted relative to `ccw`. Start and end points keep their
/// roles (they are not swapped) and `large-arc-flag` is unaffected, since
/// `Arc::sweep_angle()` is a magnitude.
fn encode_entity(entity: &Entity, bed_height_mm: f64) -> String {
    match entity {
        Entity::Line(line) => format!(
            "<line x1=\"{:.4}\" y1=\"{:.4}\" x2=\"{:.4}\" y2=\"{:.4}\"/>",
            line.p1.x,
            flip_y(line.p1.y, bed_height_mm),
            line.p2.x,
            flip_y(line.p2.y, bed_height_mm)
        ),
        Entity::Circle(circle) => format!(
            "<circle cx=\"{:.4}\" cy=\"{:.4}\" r=\"{:.4}\"/>",
            circle.center.x,
            flip_y(circle.center.y, bed_height_mm),
            circle.r
        ),
        Entity::Arc(arc) => {
            let sp = arc.start_point();
            let ep = arc.end_point();
            let large = if arc.sweep_angle() > PI { 1 } else { 0 };
            // Mirrored: a CCW world arc draws CW in SVG space.
            let sweep = if arc.ccw { 0 } else { 1 };
            format!(
                "<path d=\"M {:.4} {:.4} A {:.4} {:.4} 0 {} {} {:.4} {:.4}\"/>",
                sp.x,
                flip_y(sp.y, bed_height_mm),
                arc.r,
                arc.r,
                large,
                sweep,
                ep.x,
                flip_y(ep.y, bed_height_mm)
            )
        }
    }
}

// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
