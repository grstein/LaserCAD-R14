//! LCV-177 (ADR 0016) — Bézier entities through SVG.
//!
//! Export (AC 11): a cubic is one `M x0 y0 C x1 y1 x2 y2 x3 y3` path and a
//! quadratic one `M x0 y0 Q x1 y1 x2 y2` path, every point in SVG
//! coordinates (`y` mirrored about the document's own bed height), four
//! decimals. AC 12 (bytes unchanged without Béziers) is pinned by
//! `export_audit.rs::export_bytes_unchanged_for_golden_document`.

use lasercad::document::{Document, Entity};
use lasercad::geometry::{Bezier, Vec2};
use lasercad::io::svg::export_svg;

/// Bed height of every export scene, mm; not the default, so a constant
/// height in the mirror shows.
const BED_H: f64 = 150.0;

fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

/// The one entity line `export_svg` writes for `b` on a 300 × 150 bed.
fn exported(b: Bezier) -> String {
    let mut doc = Document::with_bed([300.0, BED_H]);
    doc.push_current(Entity::Bezier(b));
    let text = export_svg(&doc);
    let body: Vec<&str> = text
        .lines()
        .filter(|l| !l.starts_with("<svg") && !l.starts_with("<g") && !l.starts_with("</"))
        .collect();
    assert_eq!(body.len(), 1, "{text}");
    body[0].to_owned()
}

/// AC 11 — a cubic writes `M … C …` with every `y` mirrored, four decimals.
#[test]
fn cubic_exports_one_c_path() {
    let b = Bezier::Cubic([
        v(10.0, 20.0),
        v(30.5, 80.25),
        v(70.0, -10.0),
        v(100.123_456, 40.0),
    ]);
    assert_eq!(
        exported(b),
        r#"<path d="M 10.0000 130.0000 C 30.5000 69.7500 70.0000 160.0000 100.1235 110.0000"/>"#
    );
}

/// AC 11 — a quadratic writes `M … Q …`, never elevated to a cubic.
#[test]
fn quadratic_exports_one_q_path() {
    let b = Bezier::Quadratic([v(5.0, 5.0), v(50.0, 140.0), v(95.5, 5.000_04)]);
    assert_eq!(
        exported(b),
        r#"<path d="M 5.0000 145.0000 Q 50.0000 10.0000 95.5000 145.0000"/>"#
    );
}
