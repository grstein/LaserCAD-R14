//! SVG export for LaserGRBL-compatible output.
//!
//! Provides [`export_svg`], a pure function that converts a [`Document`] into
//! a well-formed UTF-8 SVG string. The output satisfies LaserGRBL's strict
//! parser requirements: correct `xmlns`, millimetre `width`/`height`,
//! per-preset colour groups, `fill="none"`, and true SVG arc paths (`A`
//! command — no bézier approximation).
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
//! the document's bed by LCV-114, group selection by [`Preset`] in LCV-115.

use crate::document::entity::Entity;
use crate::document::state::Document;
use crate::util::flip_y;
use core::f64::consts::PI;

const STROKE_WIDTH: &str = "0.1";

// ─── Export presets ──────────────────────────────────────────────────────────

/// Which LaserGRBL colour group the exported geometry is written into.
///
/// LaserGRBL assigns speed and power by stroke colour, so the group an entity
/// lands in *is* the machining profile: red cuts, blue marks, green engraves.
/// [`export_svg`] always emits all three groups in [`Preset::ALL`] order — the
/// selected one receives the geometry, the other two stay empty.
///
/// The choice is whole-document and made at export time (LCV-115): it lives on
/// `App::export_preset`, never on an [`Entity`] and never in the [`Document`],
/// so this type carries nothing but the choice itself. A future per-entity
/// demand reuses `id`, `color` and [`Preset::from_group_id`] unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Preset {
    /// Cutting profile — red. The default for every new session.
    #[default]
    Cut,
    /// Marking / scoring profile — blue.
    Mark,
    /// Raster engraving profile — green.
    Engrave,
}

impl Preset {
    /// All three presets, in the order [`export_svg`] emits their groups.
    pub const ALL: [Preset; 3] = [Preset::Cut, Preset::Mark, Preset::Engrave];

    /// The `id` attribute of this preset's `<g>` element.
    pub const fn id(self) -> &'static str {
        match self {
            Preset::Cut => "cut",
            Preset::Mark => "mark",
            Preset::Engrave => "engrave",
        }
    }

    /// The `stroke` attribute of this preset's `<g>` element — the LaserGRBL
    /// convention colour. Fixed: three groups, three colours, no picker.
    pub const fn color(self) -> &'static str {
        match self {
            Preset::Cut => "#ff0000",
            Preset::Mark => "#0000ff",
            Preset::Engrave => "#00aa00",
        }
    }

    /// The operator-facing name shown by the `File > Export preset ▸` menu.
    /// The single list of preset names in the tree (LCV-115 AC 6).
    pub const fn label(self) -> &'static str {
        match self {
            Preset::Cut => "Cut",
            Preset::Mark => "Mark",
            Preset::Engrave => "Engrave",
        }
    }

    /// The preset whose group carries `id`, or `None` for any other id.
    ///
    /// Drives the importer's preset detection (LCV-115 AC 8) from the same
    /// table [`Self::id`] writes with, so reader and writer cannot drift.
    pub fn from_group_id(id: &str) -> Option<Preset> {
        Preset::ALL.into_iter().find(|p| p.id() == id)
    }
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Export a [`Document`] as a LaserGRBL-compatible SVG string.
///
/// The returned string is a self-contained UTF-8 XML fragment rooted at
/// `<svg>` with:
///
/// - `xmlns="http://www.w3.org/2000/svg"` on the root element.
/// - `width` and `height` in millimetres: always the document's bed size
///   (`doc.bed_mm`), for every document, empty or not.
/// - `viewBox="0 0 <width> <height>"` — the same two numbers in SVG
///   coordinates, no unit suffix. The canvas is the machine bed, so a file
///   opened in LaserGRBL or Inkscape frames the geometry exactly as the
///   viewport showed it, and a re-import recovers the same bed (LCV-114).
/// - `fill="none"` on the root element.
/// - Three `<g>` children in fixed [`Preset::ALL`] order, each carrying its
///   own [`id`](Preset::id) and [`stroke`](Preset::color) plus
///   `stroke-width="0.1"` (mm).
/// - Every entity, in insertion order, inside the one group whose id equals
///   `preset.id()`; the other two groups are emitted empty. The preset is the
///   whole-document machining profile chosen at export time (LCV-115).
///
/// Every Y coordinate is mirrored through [`crate::util::flip_y`] **around
/// `doc.bed_mm[1]`**; X values, radii and the `large-arc-flag` are untouched.
/// Geometry outside the bed is emitted verbatim (with a Y outside
/// `[0, doc.bed_mm[1]]`), never clipped or dropped.
///
/// The two bed numbers are formatted with `{}` (`f64`'s `Display`), which is
/// what this header has always used: a 400 mm bed emits `400mm`, not
/// `400.0000mm`, so the default-bed output stays byte-identical to the
/// pre-LCV-114 exporter (AC 5 / AC 6). Coordinates keep their own `{:.4}`.
///
/// Pure function: no file I/O, no global state, no panics on any valid
/// [`Document`].
pub fn export_svg(doc: &Document, preset: Preset) -> String {
    let mut out = String::with_capacity(1024);

    // Root element — the canvas is always the document's bed (LCV-100,
    // parameterised by LCV-114).
    let [bed_w, bed_h] = doc.bed_mm;
    out.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\"");
    push_attr(&mut out, "width", &format!("{bed_w}mm"));
    push_attr(&mut out, "height", &format!("{bed_h}mm"));
    push_attr(&mut out, "viewBox", &format!("0 0 {bed_w} {bed_h}"));
    push_attr(&mut out, "fill", "none");
    out.push_str(">\n");

    // One group per preset, always all three, always in the same order
    // (LCV-056 AC#6). Only the selected one is filled (LCV-115).
    for group in Preset::ALL {
        open_group(&mut out, group);
        if group == preset {
            for entity in &doc.entities {
                out.push_str(&encode_entity(entity, bed_h));
                out.push('\n');
            }
        }
        out.push_str("</g>\n");
    }

    out.push_str("</svg>");
    out
}

// ─── Private helpers ─────────────────────────────────────────────────────────

/// Append ` key="value"` to `out`.
fn push_attr(out: &mut String, key: &str, value: &str) {
    out.push(' ');
    out.push_str(key);
    out.push_str("=\"");
    out.push_str(value);
    out.push('"');
}

/// Append a `<g id="…" stroke="…" stroke-width="…">` opening tag followed
/// by a newline. Both attribute values come from `group`, so the exporter
/// holds no colour or id literal of its own (LCV-115 AC 2).
fn open_group(out: &mut String, group: Preset) {
    out.push_str("<g");
    push_attr(out, "id", group.id());
    push_attr(out, "stroke", group.color());
    push_attr(out, "stroke-width", STROKE_WIDTH);
    out.push_str(">\n");
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
