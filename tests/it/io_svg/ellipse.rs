//! LCV-176 (ADR 0015) — ellipses and elliptical arcs through SVG.
//!
//! Export (AC 9, AC 10): a full ellipse is `<ellipse cx cy rx ry/>` plus
//! `transform="rotate(a cx cy)"` only when `a = −rotation` (degrees,
//! normalized into (−180, 180]) is not zero; an arc is one
//! `M sx sy A rx ry φ large sweep ex ey` path with `sweep` inverted by the
//! Y mirror and `large` iff the parametric sweep exceeds π.
//!
//! Import (AC 1): an `A` with rx ≠ ry after radius correction is one
//! elliptical arc through the segment's endpoints, on the flagged side and
//! in the flagged direction. (AC 2): `<ellipse>` is a full ellipse, a circle
//! when rx = ry; a missing or `auto` radius takes the other; rx or ry ≤ 0,
//! or neither given, is skipped and reported `ellipse (invalid radius)`.

use core::f64::consts::{FRAC_PI_2, FRAC_PI_6, PI, TAU};
use lasercad::document::{Document, Entity};
use lasercad::geometry::Circle;
use lasercad::geometry::{EPSILON, Ellipse, EllipseSpan, Vec2};
use lasercad::io::svg::{export_svg, import_svg};

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

/// AC 9 — the boundary: 5e-7° prints as `0.000000`, so it writes no
/// transform; an arc writes that φ as `0.000000`.
#[test]
fn a_rotation_printing_as_zero_at_the_boundary_is_no_turn() {
    let edge = -(5e-7f64).to_radians();
    assert_eq!(format!("{:.6}", (-edge).to_degrees()), "0.000000");
    assert_eq!(
        exported(full(edge)),
        r#"<ellipse cx="100.0000" cy="150.0000" rx="40.0000" ry="20.0000"/>"#
    );
    let span = Some(EllipseSpan::new(0.0, FRAC_PI_2, true));
    let arc = Ellipse {
        span,
        ..full(-edge)
    };
    assert!(
        exported(arc).contains(" 0.000000 0 0 "),
        "{}",
        exported(arc)
    );
}

/// AC 10 — an exact parametric half turn is not large: `large = 1` only
/// past π.
#[test]
fn an_elliptical_half_turn_is_not_large() {
    let half = |ccw: bool| Ellipse {
        span: Some(EllipseSpan::new(0.0, if ccw { PI } else { -PI }, ccw)),
        ..full(0.0)
    };
    assert!(
        exported(half(true)).contains(" 0.000000 0 0 "),
        "{}",
        exported(half(true))
    );
    assert!(
        exported(half(false)).contains(" 0.000000 0 1 "),
        "{}",
        exported(half(false))
    );
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

/// `body` inside a 300 × 200 mm root whose user unit is one millimetre.
fn import(body: &str) -> (Vec<Entity>, Vec<(String, usize)>) {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="300mm" height="{BED_H}mm" viewBox="0 0 300 {BED_H}">{body}</svg>"#
    );
    let imported = import_svg(&svg).unwrap_or_else(|e| panic!("{body}: {e:?}"));
    (imported.entities, imported.report)
}

/// The single ellipse `body` imports, with an empty report.
fn only_ellipse(body: &str) -> Ellipse {
    match import(body) {
        (es, report) if report.is_empty() => match es.as_slice() {
            [Entity::Ellipse(e)] => *e,
            other => panic!("{body}: {other:?}"),
        },
        (_, report) => panic!("{body}: {report:?}"),
    }
}

/// World point of an SVG point on the 1 mm-per-unit root.
fn world(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, BED_H - y)
}

/// Twice the signed area of the polygon through `e`'s span: positive when
/// the span turns counter-clockwise in the world.
fn turning(e: &Ellipse) -> f64 {
    let pts = e.polyline(1e-3);
    pts.windows(2)
        .map(|w| w[0].x * w[1].y - w[1].x * w[0].y)
        .sum::<f64>()
        + pts[pts.len() - 1].x * pts[0].y
        - pts[0].x * pts[pts.len() - 1].y
}

/// AC 1 — each `large`/`sweep` pair, with φ = 30°, absolute and relative:
/// one elliptical arc between the segment's endpoints, of radii 40 and 20,
/// large iff flagged, turning clockwise in the world iff `sweep = 1` (SVG
/// is Y-down), which re-exports as the same segment.
#[test]
fn elliptical_a_segments_import_as_elliptical_arcs() {
    for (large, sweep) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
        let abs = format!(r#"<path d="M 50 100 A 40 20 30 {large} {sweep} 90 120"/>"#);
        let rel = format!(r#"<path d="M 50 100 a 40 20 30 {large} {sweep} 40 20"/>"#);
        for body in [abs, rel] {
            let e = only_ellipse(&body);
            let (s, t) = (e.start_point().expect("arc"), e.end_point().expect("arc"));
            assert!(s.approx_eq(world(50.0, 100.0), EPSILON), "{body}: {s:?}");
            assert!(t.approx_eq(world(90.0, 120.0), EPSILON), "{body}: {t:?}");
            assert!((e.rx - 40.0).abs() <= EPSILON && (e.ry - 20.0).abs() <= EPSILON);
            assert_eq!(e.sweep() > PI, large == 1, "{body}: side");
            assert_eq!(turning(&e) < 0.0, sweep == 1, "{body}: direction");
            let back = format!(
                r#"<path d="M 50.0000 100.0000 A 40.0000 20.0000 30.000000 {large} {sweep} 90.0000 120.0000"/>"#
            );
            assert_eq!(exported(e), back, "{body}");
        }
    }
}

/// AC 1 — radii too small for the chord scale up together (SVG 2 §F.6.6):
/// a half ellipse of radii 50 and 25 centred on the chord.
#[test]
fn elliptical_a_with_short_radii_is_corrected() {
    let e = only_ellipse(r#"<path d="M 0 100 A 4 2 0 0 1 100 100"/>"#);
    assert!(
        e.start_point()
            .expect("arc")
            .approx_eq(world(0.0, 100.0), EPSILON)
    );
    assert!(
        e.end_point()
            .expect("arc")
            .approx_eq(world(100.0, 100.0), EPSILON)
    );
    assert!(
        (e.rx - 50.0).abs() <= 1e-6 && (e.ry - 25.0).abs() <= 1e-6,
        "{e:?}"
    );
    assert!(e.center.approx_eq(world(50.0, 100.0), 1e-6), "{e:?}");
    assert!((e.sweep() - PI).abs() <= 1e-6, "{e:?}");
}

/// AC 1 — equal radii still import a circular `Arc`, whatever φ, and no
/// `path elliptical arc` note is left.
#[test]
fn circular_a_stays_an_arc_and_no_elliptical_note() {
    let (es, report) = import(r#"<path d="M 10 50 A 10 10 30 0 1 30 50"/>"#);
    assert!(matches!(es.as_slice(), [Entity::Arc(_)]), "{es:?}");
    assert_eq!(report, []);
    let (es, report) = import(r#"<path d="M 0 0 A 10 5 0 0 1 10 0 M 0 9 A 3 1 45 1 0 4 9"/>"#);
    assert_eq!(es.len(), 2, "{es:?}");
    assert!(es.iter().all(|e| matches!(e, Entity::Ellipse(_))));
    assert!(report.is_empty(), "{report:?}");
}

/// AC 2 — `<ellipse>` with rx ≠ ry is a full ellipse, its `transform`
/// rotation mirrored into the world; it re-exports as written.
#[test]
fn ellipse_element_imports_a_full_ellipse() {
    let e = only_ellipse(r#"<ellipse cx="100" cy="50" rx="40" ry="20"/>"#);
    assert!(e.center.approx_eq(world(100.0, 50.0), EPSILON), "{e:?}");
    assert!((e.rx - 40.0).abs() <= EPSILON && (e.ry - 20.0).abs() <= EPSILON);
    assert!(e.rotation.abs() <= EPSILON && e.span.is_none(), "{e:?}");
    let body = r#"<ellipse cx="100" cy="50" rx="40" ry="20" transform="rotate(-30 100 50)"/>"#;
    let e = only_ellipse(body);
    assert!(e.center.approx_eq(world(100.0, 50.0), 1e-9), "{e:?}");
    assert!((e.rotation - FRAC_PI_6).abs() <= 1e-12, "{e:?}");
    assert_eq!(
        exported(e),
        r#"<ellipse cx="100.0000" cy="50.0000" rx="40.0000" ry="20.0000" transform="rotate(-30.000000 100.0000 50.0000)"/>"#
    );
}

/// AC 2 — equal radii, or one radius missing or `auto`, make a circle; a
/// `%` radius resolves against the viewport like any LCV-173 length.
#[test]
fn ellipse_element_radii_auto_missing_and_percent() {
    let circle = |body: &str| match import(body) {
        (es, report) if report.is_empty() => match es.as_slice() {
            [Entity::Circle(c)] => *c,
            other => panic!("{body}: {other:?}"),
        },
        (_, report) => panic!("{body}: {report:?}"),
    };
    let want = Circle::new(world(10.0, 20.0), 7.0);
    for body in [
        r#"<ellipse cx="10" cy="20" rx="7" ry="7"/>"#,
        r#"<ellipse cx="10" cy="20" rx="auto" ry="7"/>"#,
        r#"<ellipse cx="10" cy="20" rx="7"/>"#,
        r#"<ellipse cx="10" cy="20" ry="7" rx=" auto "/>"#,
    ] {
        assert_eq!(circle(body), want, "{body}");
    }
    let e = only_ellipse(r#"<ellipse cx="10" cy="20" rx="10%" ry="10%"/>"#);
    assert!(
        (e.rx - 30.0).abs() <= EPSILON && (e.ry - 20.0).abs() <= EPSILON,
        "{e:?}"
    );
}

/// AC 2 — rx or ry ≤ 0, or no radius at all, skips the element and reports
/// it once per element; the rest of the file still imports.
#[test]
fn ellipse_element_with_invalid_radius_is_skipped_and_reported() {
    let (es, report) = import(
        r#"<ellipse cx="1" cy="1" rx="0" ry="5"/><ellipse cx="1" cy="1" rx="5" ry="-2"/><ellipse cx="1" cy="1"/><ellipse cx="1" cy="1" rx="auto" ry="auto"/><line x1="0" y1="0" x2="5" y2="0"/>"#,
    );
    assert!(matches!(es.as_slice(), [Entity::Line(_)]), "{es:?}");
    assert_eq!(report, [("ellipse (invalid radius)".to_owned(), 4)]);
}
