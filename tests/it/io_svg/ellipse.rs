//! LCV-176 (ADR 0015) — ellipses and elliptical arcs through SVG.
//!
//! Export (AC 9, AC 10): a full ellipse is `<ellipse cx cy rx ry/>` plus
//! `transform="rotate(a cx cy)"` only when `a = −rotation` (degrees,
//! normalized into (−180, 180]) is not zero; an arc is one
//! `M sx sy A rx ry φ large sweep ex ey` path with `sweep` inverted by the
//! Y mirror and `large` iff the parametric sweep exceeds π.

use core::f64::consts::{FRAC_PI_2, FRAC_PI_6, PI, TAU};
use lasercad::document::{Document, Entity};
use lasercad::geometry::{Ellipse, EllipseSpan, Vec2};
use lasercad::io::svg::export_svg;

/// Bed height of every export scene, mm.
const BED_H: f64 = 200.0;

/// The one entity line `export_svg` writes for `e` on a 300 × 200 bed.
fn exported(e: Ellipse) -> String {
    let mut doc = Document::with_bed([300.0, BED_H]);
    doc.push_current(Entity::Ellipse(e));
    let text = export_svg(&doc);
    let body: Vec<&str> = text
        .lines()
        .filter(|l| !l.starts_with("<svg") && !l.starts_with("<g") && !l.starts_with("</"))
        .collect();
    assert_eq!(body.len(), 1, "{text}");
    body[0].to_owned()
}

fn full(rotation: f64) -> Ellipse {
    Ellipse::new(Vec2::new(100.0, 50.0), 40.0, 20.0, rotation, None)
}

/// AC 9 — rotation 0 writes no transform; 30° CCW in the world is −30° in
/// SVG; the angle is normalized into (−180, 180] and a rotation that prints
/// as zero writes no transform.
#[test]
fn full_ellipse_exports_with_rotate_only_when_turned() {
    let plain = r#"<ellipse cx="100.0000" cy="150.0000" rx="40.0000" ry="20.0000"/>"#;
    assert_eq!(exported(full(0.0)), plain);
    assert_eq!(exported(full(TAU)), plain, "a whole turn is no turn");
    assert_eq!(exported(full(1e-9)), plain, "prints as zero");
    assert_eq!(exported(full(-1e-9)), plain, "prints as zero");
    let turned = |a: &str| {
        format!(
            r#"<ellipse cx="100.0000" cy="150.0000" rx="40.0000" ry="20.0000" transform="rotate({a} 100.0000 150.0000)"/>"#
        )
    };
    assert_eq!(exported(full(FRAC_PI_6)), turned("-30.000000"));
    assert_eq!(exported(full(-FRAC_PI_2)), turned("90.000000"));
    assert_eq!(exported(full(PI)), turned("180.000000"));
    assert_eq!(exported(full(-PI)), turned("180.000000"));
    assert_eq!(exported(full(-3.0 * FRAC_PI_2)), turned("-90.000000"));
}

/// AC 10 — every `large`/`sweep` pair: `sweep = 0` for a CCW world span,
/// `large = 1` iff the parametric sweep exceeds π; φ = −rotation.
#[test]
fn elliptical_arc_exports_one_path_with_mirrored_sweep() {
    let cases = [
        (0.0, FRAC_PI_2, true, "0 0"),
        (0.0, FRAC_PI_2, false, "1 1"),
        (0.0, 4.0, true, "1 0"),
        (0.0, -1.0, false, "0 1"),
    ];
    for (start, end, ccw, flags) in cases {
        let e = Ellipse::new(
            Vec2::new(100.0, 50.0),
            40.0,
            20.0,
            FRAC_PI_6,
            Some(EllipseSpan::new(start, end, ccw)),
        );
        let (s, t) = (e.point(start), e.point(end));
        let want = format!(
            r#"<path d="M {:.4} {:.4} A 40.0000 20.0000 -30.000000 {flags} {:.4} {:.4}"/>"#,
            s.x,
            BED_H - s.y,
            t.x,
            BED_H - t.y
        );
        assert_eq!(exported(e), want, "{start}→{end} ccw={ccw}");
    }
    let a = Ellipse::new(
        Vec2::new(0.0, 0.0),
        4.0,
        2.0,
        0.0,
        Some(EllipseSpan::new(0.0, FRAC_PI_2, true)),
    );
    assert_eq!(
        exported(a),
        r#"<path d="M 4.0000 200.0000 A 4.0000 2.0000 0.000000 0 0 0.0000 198.0000"/>"#
    );
}
