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
mod tests {
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
        let src = include_str!("export.rs");
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
}
