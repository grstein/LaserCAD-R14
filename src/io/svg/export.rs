//! SVG export for LaserGRBL-compatible output.
//!
//! Provides [`export_svg`], a pure function that converts a [`Document`] into
//! a well-formed UTF-8 SVG string. The output satisfies LaserGRBL's strict
//! parser requirements: correct `xmlns`, millimetre `width`/`height`,
//! per-preset colour groups, `fill="none"`, and true SVG arc paths (`A`
//! command — no bézier approximation).
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.
//!
//! Introduced by demand LCV-056.

use crate::document::entity::Entity;
use crate::document::state::Document;
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
/// - `width` and `height` in millimetres derived from `doc.bounds()`.
/// - `viewBox` in world coordinates (no Y-axis flip, no unit suffix).
/// - `fill="none"` on the root element.
/// - Three `<g>` children in fixed order:
///   1. `cut` — stroke `#ff0000`, all current entities.
///   2. `mark` — stroke `#0000ff`, empty (layer assignment is future work).
///   3. `engrave` — stroke `#00aa00`, empty.
/// - Each group carries `stroke-width="0.1"` (mm).
/// - All entities written into the `cut` group in insertion order.
///
/// When `doc.bounds()` returns `None` (empty document), the canvas defaults
/// to `width="100mm"`, `height="100mm"`, `viewBox="0 0 100 100"`.
///
/// Pure function: no file I/O, no global state, no panics on any valid
/// [`Document`].
pub fn export_svg(doc: &Document) -> String {
    let (width_str, height_str, viewbox_str) = canvas_attrs(doc);

    let mut out = String::with_capacity(1024);

    // Root element.
    out.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\"");
    push_attr(&mut out, "width", &width_str);
    push_attr(&mut out, "height", &height_str);
    push_attr(&mut out, "viewBox", &viewbox_str);
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

/// Derive `(width, height, viewBox)` string triples from the document bounds.
///
/// Uses exact integer literals for the fallback canvas so that AC#4 matches
/// `width="100mm"` without decimal padding.
fn canvas_attrs(doc: &Document) -> (String, String, String) {
    match doc.bounds() {
        Some((min, max)) => {
            let w = max.x - min.x;
            let h = max.y - min.y;
            (
                format!("{w:.4}mm"),
                format!("{h:.4}mm"),
                format!("{:.4} {:.4} {:.4} {:.4}", min.x, min.y, w, h),
            )
        }
        None => (
            "100mm".to_string(),
            "100mm".to_string(),
            "0 0 100 100".to_string(),
        ),
    }
}

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
fn encode_entity(entity: &Entity) -> String {
    match entity {
        Entity::Line(line) => format!(
            "<line x1=\"{:.4}\" y1=\"{:.4}\" x2=\"{:.4}\" y2=\"{:.4}\"/>",
            line.p1.x, line.p1.y, line.p2.x, line.p2.y
        ),
        Entity::Circle(circle) => format!(
            "<circle cx=\"{:.4}\" cy=\"{:.4}\" r=\"{:.4}\"/>",
            circle.center.x, circle.center.y, circle.r
        ),
        Entity::Arc(arc) => {
            let sp = arc.start_point();
            let ep = arc.end_point();
            let large = if arc.sweep_angle() > PI { 1 } else { 0 };
            let sweep = if arc.ccw { 1 } else { 0 };
            format!(
                "<path d=\"M {:.4} {:.4} A {:.4} {:.4} 0 {} {} {:.4} {:.4}\"/>",
                sp.x, sp.y, arc.r, arc.r, large, sweep, ep.x, ep.y
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

    /// AC#1, AC#2, AC#5 — root `<svg>` carries xmlns and fill="none".
    #[test]
    fn svg_root_has_required_attributes() {
        let svg = export_svg(&Document::default());
        assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(svg.contains("fill=\"none\""));
    }

    /// AC#4 — empty document uses exact fallback canvas literals (no decimal padding).
    #[test]
    fn empty_document_uses_fallback_canvas() {
        let svg = export_svg(&Document::default());
        assert!(svg.contains("width=\"100mm\""), "{svg}");
        assert!(svg.contains("height=\"100mm\""), "{svg}");
        assert!(svg.contains("viewBox=\"0 0 100 100\""), "{svg}");
    }

    /// AC#3 — non-empty document derives viewBox from bounds().
    #[test]
    fn non_empty_document_viewbox_matches_bounds() {
        let svg = export_svg(&doc_with(Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(50.0, 30.0),
        ))));
        assert!(svg.contains("width=\"50.0000mm\""), "{svg}");
        assert!(svg.contains("height=\"30.0000mm\""), "{svg}");
        assert!(
            svg.contains("viewBox=\"0.0000 0.0000 50.0000 30.0000\""),
            "{svg}"
        );
    }

    /// AC#6 — three groups always emitted with correct colours and stroke-width.
    #[test]
    fn three_groups_always_emitted() {
        let svg = export_svg(&Document::default());
        assert!(svg.contains("stroke=\"#ff0000\""), "{svg}");
        assert!(svg.contains("stroke=\"#0000ff\""), "{svg}");
        assert!(svg.contains("stroke=\"#00aa00\""), "{svg}");
        assert_eq!(svg.matches("stroke-width=\"0.1\"").count(), 3, "{svg}");
    }

    /// AC#7 — Line entity encodes to `<line …/>` inside the cut group.
    #[test]
    fn line_entity_encodes_to_svg_line() {
        let svg = export_svg(&doc_with(Entity::Line(Line::new(
            Vec2::new(1.0, 2.0),
            Vec2::new(11.0, 7.0),
        ))));
        let expected = "<line x1=\"1.0000\" y1=\"2.0000\" x2=\"11.0000\" y2=\"7.0000\"/>";
        assert!(svg.contains(expected), "{svg}");
        let line_pos = svg.find(expected).unwrap();
        let close_pos = svg.find("</g>").unwrap();
        assert!(line_pos < close_pos, "line outside cut group: {svg}");
    }

    /// AC#8 — Circle entity encodes to `<circle …/>`.
    #[test]
    fn circle_entity_encodes_to_svg_circle() {
        let svg = export_svg(&doc_with(Entity::Circle(Circle::new(
            Vec2::new(5.0, 5.0),
            3.0,
        ))));
        assert!(
            svg.contains("<circle cx=\"5.0000\" cy=\"5.0000\" r=\"3.0000\"/>"),
            "{svg}"
        );
    }

    /// AC#9 — CCW quarter-arc (r=10, 0→π/2) encodes with M, A, large=0, sweep=1.
    #[test]
    fn arc_ccw_quarter_encodes_correctly() {
        let svg = export_svg(&doc_with(Entity::Arc(Arc::new(
            Vec2::new(0.0, 0.0),
            10.0,
            0.0,
            FRAC_PI_2,
            true,
        ))));
        assert!(svg.contains("M 10.0000 0.0000"), "{svg}");
        assert!(svg.contains("A 10.0000 10.0000 0 0 1"), "{svg}");
        assert!(svg.contains("0.0000 10.0000"), "{svg}");
    }

    /// AC#10 — large-arc-flag is 1 for sweep > π.
    #[test]
    fn arc_large_flag_set_for_sweep_over_pi() {
        let svg = export_svg(&doc_with(Entity::Arc(Arc::new(
            Vec2::new(0.0, 0.0),
            10.0,
            0.0,
            3.0 * FRAC_PI_2,
            true,
        ))));
        assert!(svg.contains("A 10.0000 10.0000 0 1"), "{svg}");
    }

    /// AC#11 — CW arc produces sweep-flag 0.
    #[test]
    fn arc_cw_sweep_flag_is_zero() {
        let svg = export_svg(&doc_with(Entity::Arc(Arc::new(
            Vec2::new(0.0, 0.0),
            10.0,
            FRAC_PI_2,
            0.0,
            false,
        ))));
        assert!(svg.contains("A 10.0000 10.0000 0 0 0"), "{svg}");
    }

    /// AC#12 — no forbidden SVG elements (`<filter`, `<mask`, `<clipPath`, `<text`).
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

    /// AC#13 — `export_svg` is accessible via `crate::io::svg::export_svg`.
    #[test]
    fn export_svg_reachable_via_module_path() {
        use crate::io::svg::export_svg as reexported;
        let _ = reexported(&Document::default());
    }
}
