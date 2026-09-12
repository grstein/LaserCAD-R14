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
//! (`y_svg = BED_HEIGHT_MM - y_world`) and the canvas is the bed itself. The
//! mirror also reverses handedness, so the arc sweep flag is inverted with
//! respect to [`Arc::ccw`](crate::geometry::Arc::ccw). `src/io/svg/import.rs`
//! applies the same involution in reverse.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.
//!
//! Introduced by demand LCV-056; Y mirror added by LCV-100.

use crate::document::entity::Entity;
use crate::document::state::Document;
use crate::util::{flip_y, BED_HEIGHT_MM, BED_WIDTH_MM};
use core::f64::consts::PI;

// ─── Preset group colours ────────────────────────────────────────────────────

const CUT_COLOR: &str = "#ff0000";
const MARK_COLOR: &str = "#0000ff";
const ENGRAVE_COLOR: &str = "#00aa00";
const STROKE_WIDTH: &str = "0.1";

// ─── Public API ──────────────────────────────────────────────────────────────

/// Export a [`Document`] as a LaserGRBL-compatible SVG string.
///
/// The returned string is a self-contained UTF-8 XML fragment rooted at
/// `<svg>` with:
///
/// - `xmlns="http://www.w3.org/2000/svg"` on the root element.
/// - `width` and `height` in millimetres: always the bed size
///   ([`BED_WIDTH_MM`] × [`BED_HEIGHT_MM`]), for every document, empty or not.
/// - `viewBox="0 0 <BED_WIDTH_MM> <BED_HEIGHT_MM>"` — SVG coordinates, no unit
///   suffix. The canvas is the machine bed, so a file opened in LaserGRBL or
///   Inkscape frames the geometry exactly as the viewport showed it.
/// - `fill="none"` on the root element.
/// - Three `<g>` children in fixed order:
///   1. `cut` — stroke `#ff0000`, all current entities.
///   2. `mark` — stroke `#0000ff`, empty (layer assignment is future work).
///   3. `engrave` — stroke `#00aa00`, empty.
/// - Each group carries `stroke-width="0.1"` (mm).
/// - All entities written into the `cut` group in insertion order.
///
/// Every Y coordinate is mirrored through [`crate::util::flip_y`]; X values,
/// radii and the `large-arc-flag` are untouched. Geometry outside the bed is
/// emitted verbatim (with a Y outside `[0, BED_HEIGHT_MM]`), never clipped or
/// dropped.
///
/// Pure function: no file I/O, no global state, no panics on any valid
/// [`Document`].
pub fn export_svg(doc: &Document) -> String {
    let mut out = String::with_capacity(1024);

    // Root element — the canvas is always the bed (LCV-100).
    out.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\"");
    push_attr(&mut out, "width", &format!("{BED_WIDTH_MM}mm"));
    push_attr(&mut out, "height", &format!("{BED_HEIGHT_MM}mm"));
    push_attr(
        &mut out,
        "viewBox",
        &format!("0 0 {BED_WIDTH_MM} {BED_HEIGHT_MM}"),
    );
    push_attr(&mut out, "fill", "none");
    out.push_str(">\n");

    // Cut group — all entities.
    open_group(&mut out, "cut", CUT_COLOR);
    for entity in &doc.entities {
        out.push_str(&encode_entity(entity));
        out.push('\n');
    }
    out.push_str("</g>\n");

    // Mark group — empty.
    open_group(&mut out, "mark", MARK_COLOR);
    out.push_str("</g>\n");

    // Engrave group — empty.
    open_group(&mut out, "engrave", ENGRAVE_COLOR);
    out.push_str("</g>\n");

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
/// by a newline.
fn open_group(out: &mut String, id: &str, stroke: &str) {
    out.push_str("<g");
    push_attr(out, "id", id);
    push_attr(out, "stroke", stroke);
    push_attr(out, "stroke-width", STROKE_WIDTH);
    out.push_str(">\n");
}

/// Encode a single [`Entity`] as an SVG element string (no trailing newline).
///
/// The world → SVG mirror is applied here, per entity: Y values through
/// [`flip_y`], and — because a mirror reverses handedness — the arc
/// `sweep-flag` inverted relative to `ccw`. Start and end points keep their
/// roles (they are not swapped) and `large-arc-flag` is unaffected, since
/// `Arc::sweep_angle()` is a magnitude.
fn encode_entity(entity: &Entity) -> String {
    match entity {
        Entity::Line(line) => format!(
            "<line x1=\"{:.4}\" y1=\"{:.4}\" x2=\"{:.4}\" y2=\"{:.4}\"/>",
            line.p1.x,
            flip_y(line.p1.y),
            line.p2.x,
            flip_y(line.p2.y)
        ),
        Entity::Circle(circle) => format!(
            "<circle cx=\"{:.4}\" cy=\"{:.4}\" r=\"{:.4}\"/>",
            circle.center.x,
            flip_y(circle.center.y),
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
                flip_y(sp.y),
                arc.r,
                arc.r,
                large,
                sweep,
                ep.x,
                flip_y(ep.y)
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
        let svg = export_svg(&Document::default());
        assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(svg.contains("fill=\"none\""));
    }

    /// LCV-100 AC 11 — the canvas is the bed for every document, empty or not.
    /// Replaces LCV-056's bounds-derived canvas and its `100mm` empty-document
    /// fallback.
    #[test]
    fn canvas_is_always_the_bed() {
        for svg in [
            export_svg(&Document::default()),
            export_svg(&doc_with(Entity::Line(Line::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(50.0, 30.0),
            )))),
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
        let svg = export_svg(&Document::default());
        assert!(svg.contains("stroke=\"#ff0000\""), "{svg}");
        assert!(svg.contains("stroke=\"#0000ff\""), "{svg}");
        assert!(svg.contains("stroke=\"#00aa00\""), "{svg}");
        assert_eq!(svg.matches("stroke-width=\"0.1\"").count(), 3, "{svg}");
    }

    /// LCV-056 AC#7 — Line entity encodes to `<line …/>` inside the cut group.
    #[test]
    fn line_entity_encodes_to_svg_line() {
        let svg = export_svg(&doc_with(Entity::Line(Line::new(
            Vec2::new(1.0, 2.0),
            Vec2::new(11.0, 7.0),
        ))));
        let expected = "<line x1=\"1.0000\" y1=\"398.0000\" x2=\"11.0000\" y2=\"393.0000\"/>";
        assert!(svg.contains(expected), "{svg}");
        let line_pos = svg.find(expected).unwrap();
        let close_pos = svg.find("</g>").unwrap();
        assert!(line_pos < close_pos, "line outside cut group: {svg}");
    }

    /// LCV-100 AC 4 — `y1`/`y2` are mirrored, `x1`/`x2` are not.
    #[test]
    fn line_y_is_flipped_on_export() {
        let svg = export_svg(&doc_with(Entity::Line(Line::new(
            Vec2::new(10.0, 10.0),
            Vec2::new(10.0, 60.0),
        ))));
        assert!(
            svg.contains("<line x1=\"10.0000\" y1=\"390.0000\" x2=\"10.0000\" y2=\"340.0000\"/>"),
            "{svg}"
        );
    }

    /// LCV-056 AC#8 — Circle entity encodes to `<circle …/>`.
    #[test]
    fn circle_entity_encodes_to_svg_circle() {
        let svg = export_svg(&doc_with(Entity::Circle(Circle::new(
            Vec2::new(5.0, 5.0),
            3.0,
        ))));
        assert!(
            svg.contains("<circle cx=\"5.0000\" cy=\"395.0000\" r=\"3.0000\"/>"),
            "{svg}"
        );
    }

    /// LCV-100 AC 5 — `cy` is mirrored; `cx` and `r` are untouched.
    #[test]
    fn circle_cy_is_flipped_on_export() {
        let svg = export_svg(&doc_with(Entity::Circle(Circle::new(
            Vec2::new(100.0, 250.0),
            5.0,
        ))));
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
        let svg = export_svg(&doc_with(Entity::Arc(Arc::new(
            Vec2::new(0.0, 0.0),
            10.0,
            0.0,
            FRAC_PI_2,
            true,
        ))));
        assert_eq!(
            path_d(&svg),
            "M 10.0000 400.0000 A 10.0000 10.0000 0 0 0 0.0000 390.0000"
        );
    }

    /// LCV-100 AC 8 — CW quarter arc, golden `d` (sweep flag 1).
    #[test]
    fn arc_cw_quarter_golden() {
        let svg = export_svg(&doc_with(Entity::Arc(Arc::new(
            Vec2::new(0.0, 0.0),
            10.0,
            FRAC_PI_2,
            0.0,
            false,
        ))));
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
        let upper = export_svg(&doc_with(Entity::Arc(Arc::new(
            Vec2::new(0.0, 0.0),
            10.0,
            0.0,
            PI,
            true,
        ))));
        assert_eq!(
            path_d(&upper),
            "M 10.0000 400.0000 A 10.0000 10.0000 0 0 0 -10.0000 400.0000"
        );

        let lower = export_svg(&doc_with(Entity::Arc(Arc::new(
            Vec2::new(0.0, 0.0),
            10.0,
            0.0,
            PI,
            false,
        ))));
        assert_eq!(
            path_d(&lower),
            "M 10.0000 400.0000 A 10.0000 10.0000 0 0 1 -10.0000 400.0000"
        );
    }

    /// LCV-100 AC 10 — `large-arc-flag` is a magnitude and must not flip;
    /// only `sweep` does. Flag triple is `0 1 0`.
    #[test]
    fn arc_large_flag_survives_the_mirror() {
        let svg = export_svg(&doc_with(Entity::Arc(Arc::new(
            Vec2::new(100.0, 100.0),
            10.0,
            0.0,
            3.0 * FRAC_PI_2,
            true,
        ))));
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
        let svg = export_svg(&doc);
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
        let svg = export_svg(&doc);
        assert!(!svg.contains("<filter"));
        assert!(!svg.contains("<mask"));
        assert!(!svg.contains("<clipPath"));
        assert!(!svg.contains("<text"));
    }

    /// LCV-056 AC#13 — `export_svg` is accessible via `crate::io::svg::export_svg`.
    #[test]
    fn export_svg_reachable_via_module_path() {
        use crate::io::svg::export_svg as reexported;
        let _ = reexported(&Document::default());
    }
}
